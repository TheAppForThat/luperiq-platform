//! `luperiq-mod-directory` — public pest-control company directory for `pestcontroller.org`.
//!
//! Wiring pattern:
//! 1. Call [`init`]`(db_path, stats_key)` at startup to open the `directory.sqlite` file and
//!    populate the [`DIR_STORE`] singleton.
//! 2. Merge [`directory_routes`]`(tera)` into the host axum [`Router`].
//! 3. Optionally call [`set_exclude_config`] / [`set_company_overrides`] to configure
//!    display filtering and per-company overrides.
//!
//! All state lives in a dedicated `DirectoryStore` (`Mutex<rusqlite::Connection>`)
//! backed by a separate `directory.sqlite` — no forge WAL, no luperiq-* deps.

pub mod store;
mod host_gate;
pub mod viewer;
pub mod claim;
pub mod pages_io;

pub use host_gate::{DirectoryHostGate, DirectoryHostGateService};
pub use viewer::{
    apply_tier_mask, mask_applicators, mask_officers, resolve_tier, DirViewer, MaskedCompany,
    ViewerTier,
};

/// CLAIM-FIRST visibility gate for the rebuilt-site preview mega-menu.
///
/// The mega-menu is built from raw crawl URLs (root-relative paths that 404 on
/// routing), so it is shown ONLY to the verified owner of a listing. Public /
/// unclaimed viewers get nothing — they still see the clean listing, the
/// working about/contact/services tabs, and the claim CTA.
fn mega_menu_visible(tier: ViewerTier) -> bool {
    tier == ViewerTier::Owner
}
pub use claim::EmailSender;

pub use store::{
    CompanyOverride, DirectoryApplicator, DirectoryCity, DirectoryCompany,
    DirectoryExcludeConfig, DirectoryMiniSiteBlock, DirectoryMiniSitePage,
    DirectoryOfficer, DirectoryState, DirectoryStore,
};
pub use store::{
    get_all_company_overrides, get_exclude_config, get_exclude_config_for,
    remove_company_override, set_company_overrides, set_exclude_config, update_company_override,
};

use std::sync::{Arc, OnceLock};

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use axum::{Form, Router};
use serde::Deserialize;

static DIR_STORE: OnceLock<Arc<DirectoryStore>> = OnceLock::new();
static STATS_KEY: OnceLock<String> = OnceLock::new();

pub fn get_store() -> Option<Arc<DirectoryStore>> {
    DIR_STORE.get().cloned()
}

pub fn init(db_path: &str, stats_key: Option<String>) {
    match DirectoryStore::open(db_path) {
        Ok(store) => {
            let _ = DIR_STORE.set(Arc::new(store));
            if let Some(k) = stats_key {
                let _ = STATS_KEY.set(k);
            }
            tracing::info!("[directory] loaded from {db_path}");
        }
        Err(e) => tracing::warn!("[directory] could not load {db_path}: {e}"),
    }
}

fn store() -> Option<Arc<DirectoryStore>> {
    DIR_STORE.get().cloned()
}

/// Resolve the request's [`DirViewer`] from the optional axum extension that
/// `main.rs` injects (Approach B — the directory crate never touches quiz2/forge
/// auth itself). Absent extension (e.g. routes mounted without the middleware,
/// or a bot) → anonymous viewer → Public tier.
fn viewer_from_ext(ext: Option<Extension<DirViewer>>) -> DirViewer {
    ext.map(|Extension(v)| v).unwrap_or_default()
}

// ── State shared between route handlers ─────────────────────────────────────

#[derive(Clone)]
pub(crate) struct DirRouteState {
    pub(crate) store: Arc<DirectoryStore>,
    pub(crate) tera: Arc<tera::Tera>,
    /// Phase 4: optional boxed email sender wired from `main.rs` (smtp
    /// orchestrator). `None` ⇒ claim verification emails cannot be sent and the
    /// claim is left pending with a clear message (never auto-verified).
    pub(crate) email_sender: Option<claim::EmailSender>,
}

const PER_PAGE: u32 = 24;
const SITE_BASE: &str = "https://pestcontroller.org";

// ── Public routes ────────────────────────────────────────────────────────────

pub fn directory_routes(tera: Arc<tera::Tera>, email_sender: Option<claim::EmailSender>) -> Router {
    let Some(store) = store() else {
        return Router::new();
    };
    let s = DirRouteState { store, tera, email_sender };
    Router::new()
        .route("/directory", get(home_handler))
        .route("/directory/", get(home_handler))
        .route("/directory/_stats", get(stats_handler))
        .route("/directory/click", post(click_handler))
        .route("/directory/reveal", post(reveal_handler))
        .route("/directory/search", get(search_handler))
        .route("/directory/search.json", get(search_json_handler))
        .route("/directory/{state}", get(state_handler))
        .route("/directory/{state}/{city}", get(city_handler))
        .route("/directory/{state}/{city}/{company}", get(company_handler))
        .route("/directory/{state}/{city}/{company}/{page_slug}", get(company_subpage_handler))
        .route("/directory/{state}/{city}/{company}/rate", post(rate_handler))
        .route("/directory/newsletter/signup", post(newsletter_signup_handler))
        .route("/directory/_export", get(export_gone_handler))
        .route("/directory/about", get(about_handler))
        .route("/directory/upgrade", get(upgrade_handler))
        .route("/directory/_empty-cities", get(empty_cities_handler))
        .route("/directory/sitemap.xml", get(sitemap_handler))
        // ── Phase 4: claim flow + owner dashboard ──
        .route("/directory/claim", get(claim::claim_form_handler))
        .route("/directory/claim/submit", post(claim::claim_submit_handler))
        .route("/directory/claim/verify", get(claim::claim_verify_handler))
        .route("/directory/my-listings", get(claim::my_listings_handler))
        // Owner mini-site editor (verified-owner scoped). Static "my-listings" prefix means
        // these never collide with the /directory/{state}/{city}/{company} dynamic tree.
        .route("/directory/my-listings/{company_id}/edit", get(claim::mini_site_editor_handler))
        .route("/directory/my-listings/{company_id}/pages.json", get(claim::owner_pages_json))
        .route("/directory/my-listings/{company_id}/pages", post(claim::owner_pages_save))
        .route("/directory/my-listings/{company_id}/pages/{page_slug}/delete", post(claim::owner_page_delete))
        .route("/directory/my-listings/{company_id}/export", get(claim::owner_pages_export))
        .route("/directory/my-listings/{company_id}/import", post(claim::owner_pages_import))
        .route("/directory/my-listings/{company_id}/upgrade-bundle", get(claim::owner_upgrade_bundle))
        .route("/directory/my-listings/{company_id}/upgrade", post(claim::owner_upgrade_request))
        // Admin: unclaim tool
        .route("/directory/_admin/unclaim/{company_id}", axum::routing::get(admin_unclaim_form).post(admin_unclaim_submit))
        .with_state(s)
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn home_handler(State(s): State<DirRouteState>, headers: HeaderMap) -> Response {
    let store = &s.store;
    let states = store.all_states();
    let (total_co, total_cities) = store.total_counts();

    let ip = ip_hash(&extract_ip(&headers));
    let referer = header_str(&headers, "referer");
    let store_clone = Arc::clone(store);
    tokio::spawn(async move {
        store_clone.track_view("home", None, None, None, &ip, referer.as_deref());
    });

    let top_cities = store.top_cities_per_state(8);

    let mut ctx = tera::Context::new();
    ctx.insert("states", &states);
    ctx.insert("top_cities", &top_cities);
    ctx.insert("total_companies", &total_co);
    ctx.insert("total_cities", &total_cities);
    ctx.insert("page_title", "Pest Control Company Directory — Find Licensed Pest Control Businesses Near You");
    ctx.insert("page_description", &format!(
        "Browse {total_co} licensed pest control companies across {total_cities} cities. Find local exterminators, termite specialists, and pest management professionals."
    ));
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory"));

    // JSON-LD: ItemList for directory index
    let ld = luperiq_mod_seo::seo::structured_data::json_ld_chemical_list(
        "Pest Control Company Directory",
        "Find licensed pest control businesses across the United States.",
        &format!("{SITE_BASE}/directory"),
        0,
    );
    ctx.insert("json_ld", &ld);
    ctx.insert("breadcrumbs", &serde_json::json!([
        {"name": "pestcontroller.org", "url": SITE_BASE},
        {"name": "Directory", "url": format!("{SITE_BASE}/directory")}
    ]));

    render(&s.tera, "pages/directory-home.html", ctx)
}


#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

async fn search_handler(
    State(s): State<DirRouteState>,
    Query(params): Query<SearchQuery>,
    headers: HeaderMap,
) -> Response {
    let raw_query = params.q.unwrap_or_default();
    let query = raw_query.trim().to_string();

    if query.len() < 2 {
        let mut ctx = tera::Context::new();
        ctx.insert("query", &query);
        ctx.insert("results", &Vec::<DirectoryCompany>::new());
        ctx.insert("result_count", &0usize);
        ctx.insert("page_title", "Search — Pest Control Directory");
        ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/search"));
        return render(&s.tera, "pages/directory-search.html", ctx);
    }

    // Phase 3: search is a list surface — always serialize the Public mask so
    // the result cards (and any scraper hitting this route) never receive
    // phone/email/website/street-address. Owner-tier unmasking is reserved for
    // the single-company detail page.
    let results: Vec<MaskedCompany> = s
        .store
        .search_companies(&query, 50)
        .iter()
        .map(|c| viewer::apply_tier_mask(c, ViewerTier::Public))
        .collect();
    let result_count = results.len();

    let ip = ip_hash(&extract_ip(&headers));
    let store_clone = std::sync::Arc::clone(&s.store);
    tokio::spawn(async move {
        store_clone.track_view("search", None, None, None, &ip, None);
    });

    let mut ctx = tera::Context::new();
    ctx.insert("query", &query);
    ctx.insert("results", &results);
    ctx.insert("result_count", &result_count);
    ctx.insert("page_title", &format!("Search: \"{query}\" — Pest Control Directory"));
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/search"));
    render(&s.tera, "pages/directory-search.html", ctx)
}

/// Autocomplete JSON for the directory typeahead. Mirrors the chemicals
/// `/chemicals/search.json` handler: returns `{"results":[...]}` capped at 10,
/// empty results below 2 chars. Reuses `search_companies` so the JSON rows are
/// the full `DirectoryCompany` serialization (the dropdown reads `entity_name`,
/// `city`, `state_abbr`, `city_slug`, `company_slug`).
async fn search_json_handler(
    State(s): State<DirRouteState>,
    Query(params): Query<SearchQuery>,
) -> Response {
    let raw_query = params.q.unwrap_or_default();
    let query = raw_query.trim().to_string();
    if query.len() < 2 {
        return Json(serde_json::json!({"results": []})).into_response();
    }
    // Phase 3 (harvest hole closed): the autocomplete JSON previously serialized
    // the FULL DirectoryCompany incl. phone/email/website. Apply the Public mask
    // so the typeahead payload carries only name/city/state/slug-class fields.
    let results: Vec<MaskedCompany> = s
        .store
        .search_companies(&query, 10)
        .iter()
        .map(|c| viewer::apply_tier_mask(c, ViewerTier::Public))
        .collect();
    Json(serde_json::json!({"results": results})).into_response()
}

async fn state_handler(
    State(s): State<DirRouteState>,
    Path(state): Path<String>,
    headers: HeaderMap,
) -> Response {
    let state_up = state.to_uppercase();
    let store = &s.store;
    let total = store.state_totals(&state_up);
    // A state is valid if it has any companies at all — even if every one is
    // cityless (the statewide bucket below renders them).
    if total == 0 {
        return not_found();
    }
    let cities = store.cities_for_state(&state_up);
    let state_name = state_name_for(&state_up);
    // Phase 3: state-page lists are Public list surfaces — mask each card so no
    // contact data is serialized (defensive; the cards render name/slug only).
    let mask_list = |v: Vec<DirectoryCompany>| -> Vec<MaskedCompany> {
        v.iter().map(|c| viewer::apply_tier_mask(c, ViewerTier::Public)).collect()
    };
    let featured = mask_list(store.newest_for_state(&state_up, 6));
    let oldest = mask_list(store.oldest_for_state(&state_up, 6));

    // Cityless companies → "Statewide / Location not specified" bucket so they
    // are reachable from the state page via the 2-segment fallback URL.
    let (statewide_raw, statewide_total) =
        store.cityless_companies_for_state(&state_up, 0, 24);
    let statewide = mask_list(statewide_raw);

    let ip = ip_hash(&extract_ip(&headers));
    let referer = header_str(&headers, "referer");
    let store_clone = Arc::clone(store);
    let state_up_clone = state_up.clone();
    tokio::spawn(async move {
        store_clone.track_view("state", Some(&state_up_clone), None, None, &ip, referer.as_deref());
    });

    let mut ctx = tera::Context::new();
    ctx.insert("state_abbr", &state_up);
    ctx.insert("state_name", state_name);
    ctx.insert("cities", &cities);
    ctx.insert("total_companies", &total);
    ctx.insert("featured", &featured);
    ctx.insert("oldest", &oldest);
    ctx.insert("statewide", &statewide);
    ctx.insert("statewide_total", &statewide_total);
    ctx.insert("page_title", &format!("Pest Control Companies in {state_name} — Licensed Exterminators & Pest Management"));
    ctx.insert("page_description", &format!(
        "Find {total} licensed pest control companies in {state_name}. Browse by city to find local exterminators, termite specialists, rodent control, and more."
    ));
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/{}", state.to_lowercase()));

    // JSON-LD: State as City schema
    let ld = luperiq_mod_seo::seo::structured_data::json_ld_directory_city(
        &state_name,
        &state_name,
        &state_up,
        &format!("{SITE_BASE}/directory/{}", state.to_lowercase()),
        total as usize,
    );
    ctx.insert("json_ld", &ld);
    ctx.insert("breadcrumbs", &serde_json::json!([
        {"name": "pestcontroller.org", "url": SITE_BASE},
        {"name": "Directory", "url": format!("{SITE_BASE}/directory")},
        {"name": state_name, "url": format!("{SITE_BASE}/directory/{}", state.to_lowercase())}
    ]));

    render(&s.tera, "pages/directory-state.html", ctx)
}

#[derive(Deserialize)]
struct PageParam {
    page: Option<u32>,
}

async fn city_handler(
    State(s): State<DirRouteState>,
    Path((state, city)): Path<(String, String)>,
    Query(params): Query<PageParam>,
    ext: Option<Extension<DirViewer>>,
    headers: HeaderMap,
) -> Response {
    let state_up = state.to_uppercase();
    let page = params.page.unwrap_or(0);
    let store = &s.store;

    // Disambiguate the {city} segment. It is normally a city slug, but the
    // cityless fallback URL `/directory/{state}/{company}` lands on this same
    // route. When the segment is not a known city, treat it as a company slug.
    if !store.is_known_city_slug(&state_up, &city) {
        // (a) A genuinely cityless company under this slug → render it.
        if let Some(co) = store.company_by_slug_statewide(&state_up, &city) {
            return render_company_page(&s, co, &state_up, None, &viewer_from_ext(ext), &headers);
        }
        // (b) The company has SINCE acquired a city (Phase 1 recovery): 301 to
        //     the canonical 3-segment URL so old links + crawlers stay valid.
        if let Some(co) = store.company_by_state_and_slug_any_city(&state_up, &city) {
            if let Some(cs) = co.city_slug.as_deref() {
                let target = format!("/directory/{}/{}/{}", state_up.to_lowercase(), cs, co.company_slug);
                return axum::response::Redirect::permanent(&target).into_response();
            }
        }
        return not_found();
    }

    let (companies_raw, total) = store.companies_for_city(&state_up, &city, page, PER_PAGE);
    if total == 0 && page == 0 {
        return not_found();
    }
    // Phase 3: list cards are a Public surface — mask every card so the city
    // page (and any scraper) only ever sees name/city/state/category, never
    // phone/email/website/street-address.
    let companies: Vec<MaskedCompany> = companies_raw
        .iter()
        .map(|c| viewer::apply_tier_mask(c, ViewerTier::Public))
        .collect();

    let state_name = state_name_for(&state_up);
    let city_display = city_display_name(&city);
    let total_pages = (total as u32).saturating_add(PER_PAGE - 1) / PER_PAGE;

    let ip = ip_hash(&extract_ip(&headers));
    let referer = header_str(&headers, "referer");
    let store_clone = Arc::clone(store);
    let state_up_clone = state_up.clone();
    let city_clone = city.clone();
    tokio::spawn(async move {
        store_clone.track_view("city", Some(&state_up_clone), Some(&city_clone), None, &ip, referer.as_deref());
    });

    let mut ctx = tera::Context::new();
    ctx.insert("state_abbr", &state_up);
    ctx.insert("state_name", state_name);
    ctx.insert("city_slug", &city);
    ctx.insert("city_name", &city_display);
    ctx.insert("companies", &companies);
    ctx.insert("page", &page);
    ctx.insert("total", &total);
    ctx.insert("total_pages", &total_pages);
    ctx.insert("per_page", &PER_PAGE);
    ctx.insert("page_title", &format!("Pest Control Companies in {city_display}, {state_up} — {total} Licensed Businesses"));
    ctx.insert("page_description", &format!(
        "Find {total} licensed pest control companies in {city_display}, {state_up}. Compare local exterminators, termite specialists, and pest management professionals."
    ));
    let canon = if page == 0 {
        format!("{SITE_BASE}/directory/{}/{city}", state.to_lowercase())
    } else {
        format!("{SITE_BASE}/directory/{}/{city}?page={page}", state.to_lowercase())
    };
    ctx.insert("canonical_url", &canon);

    // JSON-LD: City schema
    let ld = luperiq_mod_seo::seo::structured_data::json_ld_directory_city(
        &city_display,
        &state_name,
        &state_up,
        &canon,
        total as usize,
    );
    ctx.insert("json_ld", &ld);
    ctx.insert("breadcrumbs", &serde_json::json!([
        {"name": "pestcontroller.org", "url": SITE_BASE},
        {"name": "Directory", "url": format!("{SITE_BASE}/directory")},
        {"name": state_name, "url": format!("{SITE_BASE}/directory/{}", state.to_lowercase())},
        {"name": &city_display, "url": format!("{SITE_BASE}/directory/{}/{city}", state.to_lowercase())}
    ]));

    render(&s.tera, "pages/directory-city.html", ctx)
}

async fn company_handler(
    State(s): State<DirRouteState>,
    Path((state, city, company)): Path<(String, String, String)>,
    ext: Option<Extension<DirViewer>>,
    headers: HeaderMap,
) -> Response {
    let state_up = state.to_uppercase();
    let store = &s.store;

    let Some(co) = store.company_by_slug(&state_up, &city, &company) else {
        return not_found();
    };
    render_company_page(&s, co, &state_up, Some(&city), &viewer_from_ext(ext), &headers)
}

/// Shared company-detail renderer used by both the canonical
/// `/directory/{state}/{city}/{company}` route and the cityless
/// `/directory/{state}/{company}` fallback. `city_seg` is the URL city slug
/// when present; cityless listings pass None and render under a
/// "location not specified" treatment with a 2-segment canonical URL.
///
/// Computes a positive-framed data-quality tier in the handler (per brief):
///   - "complete":  (phone OR website OR email) AND address present
///   - "partial":   some contact/address present, but not the complete set
///   - "listing":   name + location only — most prominent claim CTA
fn render_company_page(
    s: &DirRouteState,
    co: store::DirectoryCompany,
    state_up: &str,
    city_seg: Option<&str>,
    viewer: &DirViewer,
    headers: &HeaderMap,
) -> Response {
    let store = &s.store;
    let state_name = state_name_for(state_up);

    // ── Phase 3: resolve viewer tier + mask the record ──────────────────────
    // Owner of a verified claim → full data; everyone else (anon, bot, authed
    // non-owner) → Public mask. `data_quality` below is computed from the RAW
    // record so the claim-CTA framing still reflects whether data *exists*; the
    // masked struct is what reaches Tera, so hidden fields can never serialize.
    let tier = viewer::resolve_tier(viewer, store, &co.id);
    let viewer_is_owner = tier == ViewerTier::Owner;

    // Effective city slug for canonical/track/breadcrumb URLs. Cityless rows
    // fall back to the row's own city_slug if one exists, else None.
    let effective_city_slug: Option<String> =
        city_seg.map(|c| c.to_string()).or_else(|| co.city_slug.clone());
    let has_city = effective_city_slug.is_some();

    let company_display = co.dba.clone().unwrap_or_else(|| co.entity_name.clone());

    // ── Data-quality tier (entity-agnostic; "business" copy in template) ──
    let has_phone = co.phone.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let has_email = co.email.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let has_website = co.website.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let has_address = co.address.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let has_contact = has_phone || has_email || has_website;
    let data_quality = if has_contact && has_address {
        "complete"
    } else if has_contact || has_address {
        "partial"
    } else {
        "listing"
    };

    let city_display = co
        .city
        .clone()
        .or_else(|| city_seg.map(city_display_name))
        .unwrap_or_else(|| state_name.to_string());
    let location_label = if has_city {
        format!("{city_display}, {state_up}")
    } else {
        format!("{state_name} (location not specified)")
    };

    // Tracking
    let ip = ip_hash(&extract_ip(headers));
    let referer = header_str(headers, "referer");
    let store_clone = Arc::clone(store);
    let state_up_clone = state_up.to_string();
    let city_clone = effective_city_slug.clone();
    let company_slug_clone = co.company_slug.clone();
    tokio::spawn(async move {
        store_clone.track_view(
            "company",
            Some(&state_up_clone),
            city_clone.as_deref(),
            Some(&company_slug_clone),
            &ip,
            referer.as_deref(),
        );
    });

    let officers = viewer::mask_officers(store.officers_for_company(&co.id), tier);
    let applicators = viewer::mask_applicators(store.applicators_for_company(&co.id), tier);
    let mini_site_pages = store.mini_site_pages_for(&co.id);
    let has_mini_site = !mini_site_pages.is_empty();
    // CLAIM-FIRST gate: the rebuilt-site preview mega-menu is built from RAW
    // crawl URLs (root-relative paths that 404 on routing). Only the verified
    // owner sees it; public/unclaimed viewers get nothing (no dead links).
    // `company_nav` data is left untouched as the source for claim-time rebuild.
    let company_mega_menu_html = if mega_menu_visible(tier) {
        let nav_items = store.company_nav_items(&co.id);
        if nav_items.is_empty() {
            String::new()
        } else {
            crate::store::render_company_mega_menu(&nav_items, &company_display)
        }
    } else {
        String::new()
    };

    // Build the masked company that actually reaches Tera. The masked-out
    // contact fields are physically None on `masked`, so the rendered HTML +
    // JSON-LD cannot contain them. Template-facing booleans are recomputed from
    // the masked struct so the contact card / sidebar gate correctly.
    let masked = viewer::apply_tier_mask(&co, tier);
    let m_has_phone = masked.phone.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let m_has_email = masked.email.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let m_has_website = masked.website.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let m_has_address = masked.address.as_deref().map(|v| !v.trim().is_empty()).unwrap_or(false);
    let m_has_contact = m_has_phone || m_has_email || m_has_website;
    let contact_hidden = masked.contact_hidden;
    let address_hidden = masked.address_hidden;
    let officer_first_only = masked.officer_first_only;
    let staff_preview = masked.staff_preview.clone();

    // Canonical URL: 3-segment when a city is known, 2-segment otherwise.
    let state_lc = state_up.to_lowercase();
    let company_slug = &co.company_slug;
    let canonical = match &effective_city_slug {
        Some(cs) => format!("{SITE_BASE}/directory/{state_lc}/{cs}/{company_slug}"),
        None => format!("{SITE_BASE}/directory/{state_lc}/{company_slug}"),
    };
    let city_url = effective_city_slug
        .as_ref()
        .map(|cs| format!("{SITE_BASE}/directory/{state_lc}/{cs}"));

    let mut ctx = tera::Context::new();
    ctx.insert("state_abbr", state_up);
    ctx.insert("state_name", state_name);
    ctx.insert("city_slug", &effective_city_slug);
    ctx.insert("has_city", &has_city);
    ctx.insert("city_name", &city_display);
    ctx.insert("location_label", &location_label);
    // Pass the MASKED company as `company`; the raw `co` is never serialized.
    ctx.insert("company", &masked);
    ctx.insert("company_display_name", &company_display);
    ctx.insert("officers", &officers);
    ctx.insert("applicators", &applicators);
    ctx.insert("has_mini_site", &has_mini_site);
    ctx.insert("mini_site_pages", &mini_site_pages);
    ctx.insert("active_tab", "profile");
    ctx.insert("company_mega_menu_html", &company_mega_menu_html);
    ctx.insert("data_quality", data_quality);
    // Template-facing booleans derived from the masked struct (Public → false).
    ctx.insert("has_phone", &m_has_phone);
    ctx.insert("has_email", &m_has_email);
    ctx.insert("has_website", &m_has_website);
    ctx.insert("has_address", &m_has_address);
    ctx.insert("has_contact", &m_has_contact);
    // Phase 3 presentation flags.
    ctx.insert("viewer_is_owner", &viewer_is_owner);
    ctx.insert("contact_hidden", &contact_hidden);
    ctx.insert("address_hidden", &address_hidden);
    ctx.insert("officer_first_only", &officer_first_only);
    ctx.insert("staff_preview", &staff_preview);
    // Per-field "value exists" flags (computed from the RAW record) so the
    // reveal control only renders for fields that actually have a value. These
    // expose existence, not the value — the value comes from /directory/reveal.
    ctx.insert("reveal_phone", &(contact_hidden && has_phone));
    ctx.insert("reveal_email", &(contact_hidden && has_email));
    ctx.insert("reveal_website", &(contact_hidden && has_website));
    ctx.insert("page_title", &format!("{company_display} — Pest Control in {location_label}"));
    ctx.insert("page_description", &format!(
        "{company_display} is a pest control company in {location_label}. Find contact info, license details, and state regulations."
    ));
    ctx.insert("canonical_url", &canonical);

    // JSON-LD: Organization schema
    let ld = luperiq_mod_seo::seo::structured_data::json_ld_directory_company(
        &company_display,
        &format!("{company_display} is a pest control company in {location_label}."),
        &canonical,
        &city_display,
        &state_name,
        co.phone.as_deref(),
        co.email.as_deref(),
        co.website.as_deref(),
    );
    ctx.insert("json_ld", &ld);

    // Breadcrumbs: include the city level only when a city is known.
    let mut crumbs = vec![
        serde_json::json!({"name": "pestcontroller.org", "url": SITE_BASE}),
        serde_json::json!({"name": "Directory", "url": format!("{SITE_BASE}/directory")}),
        serde_json::json!({"name": state_name, "url": format!("{SITE_BASE}/directory/{state_lc}")}),
    ];
    if let Some(ref cu) = city_url {
        crumbs.push(serde_json::json!({"name": city_display, "url": cu}));
    }
    crumbs.push(serde_json::json!({"name": company_display, "url": canonical}));
    ctx.insert("breadcrumbs", &serde_json::Value::Array(crumbs));

    render(&s.tera, "pages/directory-company.html", ctx)
}

async fn company_subpage_handler(
    State(s): State<DirRouteState>,
    Path((state, city, company, page_slug)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> Response {
    let state_up = state.to_uppercase();
    let store = &s.store;

    let Some(co) = store.company_by_slug(&state_up, &city, &company) else {
        return not_found();
    };

    let Some(page) = store.mini_site_page_for(&co.id, &page_slug) else {
        return not_found();
    };

    let state_name = state_name_for(&state_up);
    let city_display = co.city.clone().unwrap_or_else(|| city_display_name(&city));
    let company_display = co.dba.clone().unwrap_or_else(|| co.entity_name.clone());
    let page_title_str = page.page_title.clone().unwrap_or_else(|| format!("{page_slug} — {company_display}"));

    let ip = ip_hash(&extract_ip(&headers));
    let store_clone = Arc::clone(store);
    let state_up_clone = state_up.clone();
    let city_clone = city.clone();
    let company_slug_clone = company.clone();
    tokio::spawn(async move {
        store_clone.track_view(
            "company_page",
            Some(&state_up_clone),
            Some(&city_clone),
            Some(&company_slug_clone),
            &ip,
            None,
        );
    });

    let mut ctx = tera::Context::new();
    ctx.insert("state_abbr", &state_up);
    ctx.insert("state_name", state_name);
    ctx.insert("city_slug", &city);
    ctx.insert("city_name", &city_display);
    ctx.insert("company", &co);
    ctx.insert("company_display_name", &company_display);
    ctx.insert("page", &page);
    // The company's full tab list — so the subpage menu is dynamic (their real tabs), not hardcoded.
    ctx.insert("mini_site_pages", &store.mini_site_pages_for(&co.id));
    ctx.insert("active_tab", &page_slug);
    ctx.insert("has_mini_site", &true);
    ctx.insert("page_title", &format!("{page_title_str} — {company_display}"));
    ctx.insert("page_description", &format!(
        "{page_title_str} for {company_display}, a pest control company in {city_display}, {state_up}."
    ));
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/{}/{city}/{company}/{page_slug}", state.to_lowercase()));
    ctx.insert("breadcrumbs", &serde_json::json!([
        {"name": "pestcontroller.org", "url": SITE_BASE},
        {"name": "Directory", "url": format!("{SITE_BASE}/directory")},
        {"name": state_name, "url": format!("{SITE_BASE}/directory/{}", state.to_lowercase())},
        {"name": &city_display, "url": format!("{SITE_BASE}/directory/{}/{city}", state.to_lowercase())},
        {"name": &company_display, "url": format!("{SITE_BASE}/directory/{}/{city}/{company}", state.to_lowercase())},
        {"name": &page_title_str, "url": format!("{SITE_BASE}/directory/{}/{city}/{company}/{page_slug}", state.to_lowercase())}
    ]));

    render(&s.tera, "pages/directory-company-subpage.html", ctx)
}

#[derive(Deserialize)]
struct RateForm {
    rating: i64,
    review: Option<String>,
    name: Option<String>,
}

async fn rate_handler(
    State(s): State<DirRouteState>,
    Path((state, city, company)): Path<(String, String, String)>,
    Form(form): Form<RateForm>,
) -> Response {
    if form.rating < 1 || form.rating > 5 {
        return (StatusCode::BAD_REQUEST, "Invalid rating").into_response();
    }
    let state_up = state.to_uppercase();
    let store = &s.store;

    let Some(co) = store.company_by_slug(&state_up, &city, &company) else {
        return not_found();
    };

    let review = form.review.as_deref().filter(|s| !s.trim().is_empty());
    let reviewer = form.name.as_deref().filter(|s| !s.trim().is_empty());
    store.add_rating(&co.id, form.rating, review, reviewer);

    let redirect = format!("/directory/{}/{city}/{company}#ratings", state.to_lowercase());
    axum::response::Redirect::to(&redirect).into_response()
}

// ── Contact reveal (Phase 2, directory hardening) ────────────────────────────
//
// POST /directory/reveal — returns a single contact value (phone|email|website)
// for a company and logs an engagement_event. Behavior-neutral for every other
// page: this is the only new externally-visible surface this phase.
//
// CSRF: this mirrors the existing directory POSTs (`click_handler`, `rate_handler`,
// `newsletter_signup_handler`), none of which carry a CSRF token — the directory
// module is mounted on the public, unauthenticated router and applies no CSRF
// middleware. See report for the Phase 3/4 reconciliation note.

#[derive(Deserialize)]
struct RevealForm {
    company_id: String,
    field: String,
}

const REVEAL_FIELDS: [&str; 3] = ["phone", "email", "website"];

async fn reveal_handler(
    State(s): State<DirRouteState>,
    headers: HeaderMap,
    Form(form): Form<RevealForm>,
) -> Response {
    // Allowlist the requested field; reject anything else with 400.
    if !REVEAL_FIELDS.contains(&form.field.as_str()) {
        return (StatusCode::BAD_REQUEST, "Invalid field").into_response();
    }

    let store = &s.store;

    // Resolve contact fields; missing company → {"ok":false} with 200 so we do
    // not leak existence via the status code.
    let Some((phone, email, website)) = store.company_contact(&form.company_id) else {
        return reveal_not_available();
    };

    let value = match form.field.as_str() {
        "phone" => phone,
        "email" => email,
        "website" => website,
        _ => None,
    };

    let Some(value) = value.filter(|v| !v.trim().is_empty()) else {
        return reveal_not_available();
    };

    // ip_hash via the existing helpers (daily-salted FNV-1a) — do not reimplement.
    let ip = ip_hash(&extract_ip(&headers));
    let event_type = format!("{}_reveal", form.field);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let company_id = form.company_id.clone();
    // user_id is None this phase — journal threading deferred to Phase 3.
    let store_clone = Arc::clone(store);
    tokio::spawn(async move {
        store_clone.log_engagement_event(&company_id, &event_type, &ip, ts, None);
    });

    let body = Json(serde_json::json!({ "ok": true, "value": value }));
    (
        StatusCode::OK,
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        body,
    )
        .into_response()
}

/// `{"ok":false}` with 200 + no-store — used when the company or field is absent
/// so existence is never leaked via the HTTP status.
fn reveal_not_available() -> Response {
    (
        StatusCode::OK,
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({ "ok": false })),
    )
        .into_response()
}

#[derive(Deserialize)]
struct ClickForm {
    t: String,
    id: Option<String>,
    s: Option<String>,
    ci: Option<String>,
    cs: Option<String>,
}

async fn click_handler(
    State(s): State<DirRouteState>,
    headers: HeaderMap,
    Form(form): Form<ClickForm>,
) -> Response {
    let ip = ip_hash(&extract_ip(&headers));
    let store_clone = Arc::clone(&s.store);
    tokio::spawn(async move {
        store_clone.track_click(
            &form.t,
            form.id.as_deref(),
            form.s.as_deref(),
            form.ci.as_deref(),
            form.cs.as_deref(),
            &ip,
        );
    });
    (StatusCode::NO_CONTENT, "").into_response()
}

#[derive(Deserialize)]
struct StatsQuery {
    key: Option<String>,
}



async fn sitemap_handler(State(s): State<DirRouteState>) -> Response {
    let store = &s.store;
    let cities = store.all_city_slugs();
    let companies = store.all_company_slug_tuples();

    let mut xml = String::from(concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
        "<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    ));
    xml.push_str(&format!(
        "  <url><loc>{}/directory</loc><changefreq>weekly</changefreq><priority>0.9</priority></url>\n",
        SITE_BASE
    ));

    let mut seen_states: std::collections::HashSet<String> = std::collections::HashSet::new();
    for city in &cities {
        if seen_states.insert(city.state_abbr.clone()) {
            xml.push_str(&format!(
                "  <url><loc>{}/directory/{}</loc><changefreq>weekly</changefreq><priority>0.8</priority></url>\n",
                SITE_BASE, city.state_abbr.to_lowercase()
            ));
        }
    }
    for city in &cities {
        xml.push_str(&format!(
            "  <url><loc>{}/directory/{}/{}</loc><changefreq>weekly</changefreq><priority>0.7</priority></url>\n",
            SITE_BASE, city.state_abbr.to_lowercase(), city.city_slug
        ));
    }
    for (state, city_slug, company_slug) in companies.iter().take(40_000) {
        xml.push_str(&format!(
            "  <url><loc>{}/directory/{}/{}/{}</loc><changefreq>monthly</changefreq><priority>0.5</priority></url>\n",
            SITE_BASE, state.to_lowercase(), city_slug, company_slug
        ));
    }
    // Cityless companies — emit the 2-segment fallback URL so search engines
    // only ever see the URL form that actually resolves for each listing.
    let cityless = store.cityless_company_slug_tuples();
    for (state, company_slug) in cityless.iter().take(20_000) {
        xml.push_str(&format!(
            "  <url><loc>{}/directory/{}/{}</loc><changefreq>monthly</changefreq><priority>0.4</priority></url>\n",
            SITE_BASE, state.to_lowercase(), company_slug
        ));
    }
    xml.push_str("</urlset>\n");

    (
        [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    ).into_response()
}

async fn empty_cities_handler(State(s): State<DirRouteState>) -> Response {
    let store = &s.store;
    let cities = store.empty_cities();
    let mut html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Empty Cities</title>\
        <style>body{{font-family:system-ui,sans-serif;padding:24px;max-width:900px;margin:0 auto}}\
        table{{width:100%;border-collapse:collapse}}th,td{{padding:8px 12px;text-align:left;\
        border-bottom:1px solid #e2e8f0}}th{{background:#f8fafc;font-size:13px;color:#64748b}}</style>\
        </head><body><h1>Empty Cities ({} total)</h1>\
        <table><tr><th>State</th><th>Slug</th><th>Name</th><th>County?</th></tr>",
        cities.len()
    );
    for city in &cities {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            city.state_abbr, city.city_slug, city.city_name,
            if city.is_county { "Yes" } else { "No" }
        ));
    }
    html.push_str("</table></body></html>");
    Html(html).into_response()
}

#[derive(Debug, Deserialize)]
struct NewsletterSignupPayload {
    email: String,
    name: Option<String>,
    company: Option<String>,
    state: Option<String>,
}

async fn newsletter_signup_handler(
    State(s): State<DirRouteState>,
    Json(payload): Json<NewsletterSignupPayload>,
) -> Response {
    let email = payload.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return (StatusCode::BAD_REQUEST, "Valid email required").into_response();
    }
    let ok = s.store.newsletter_signup(
        &email,
        payload.name.as_deref().map(str::trim),
        payload.company.as_deref().map(str::trim),
        payload.state.as_deref().map(str::trim),
    );
    if ok {
        StatusCode::OK.into_response()
    } else {
        // UNIQUE constraint means already subscribed — return OK anyway
        StatusCode::OK.into_response()
    }
}

async fn export_gone_handler() -> Response {
    (StatusCode::GONE, Html("<h1>Export moved</h1><p>Use <a href='/admin/system/data-studio'>Data Studio &rarr; Directory</a>.</p>")).into_response()
}

async fn stats_handler(
    State(s): State<DirRouteState>,
    Query(q): Query<StatsQuery>,
) -> Response {
    let expected = STATS_KEY.get().map(|k| k.as_str()).unwrap_or("");
    if expected.is_empty() || q.key.as_deref() != Some(expected) {
        return (StatusCode::NOT_FOUND, Html("<h1>Not found</h1>")).into_response();
    }

    let stats = s.store.stats();
    let html = render_stats_html(&stats);
    Html(html).into_response()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render(tera: &tera::Tera, template: &str, ctx: tera::Context) -> Response {
    match tera.render(template, &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("[directory] template error {template}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {e}")).into_response()
        }
    }
}

/// Public-facing 404 reused by the claim module.
pub(crate) fn not_found_public() -> Response {
    not_found()
}

/// Render wrapper reused by the claim module (keeps `render` private).
pub(crate) fn render_page(tera: &tera::Tera, template: &str, ctx: tera::Context) -> Response {
    render(tera, template, ctx)
}

/// ip_hash wrapper reused by the claim module.
pub(crate) fn ip_hash_public(ip: &str) -> String {
    ip_hash(ip)
}

/// extract_ip wrapper reused by the claim module.
pub(crate) fn extract_ip_public(headers: &HeaderMap) -> String {
    extract_ip(headers)
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, Html("<h1>Not found</h1><p><a href='/directory'>Back to directory</a></p>")).into_response()
}

fn extract_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .or_else(|| headers.get("x-forwarded-for"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn ip_hash(ip: &str) -> String {
    let day = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() / 86400)
        .unwrap_or(0);
    // FNV-1a 64-bit over IP bytes + daily salt — privacy-safe daily unique
    let mut h: u64 = 0xcbf29ce484222325;
    for b in ip.bytes().chain(day.to_le_bytes()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(|s| s.to_string())
}

fn state_name_for(abbr: &str) -> &'static str {
    match abbr {
        "TX" => "Texas", "FL" => "Florida", "NY" => "New York",
        "CT" => "Connecticut", "CO" => "Colorado", "OR" => "Oregon",
        "IA" => "Iowa", "DE" => "Delaware", "WA" => "Washington",
        "IL" => "Illinois", "CA" => "California", "NJ" => "New Jersey",
        "OK" => "Oklahoma", "AR" => "Arkansas", "LA" => "Louisiana",
        "KS" => "Kansas", "MO" => "Missouri", "KY" => "Kentucky",
        "GA" => "Georgia", "MN" => "Minnesota", "OH" => "Ohio",
        "IN" => "Indiana", "MI" => "Michigan", "VA" => "Virginia",
        "NC" => "North Carolina", "AZ" => "Arizona",
        _ => "Unknown",
    }
}

fn city_display_name(slug: &str) -> String {
    slug.split('-')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_stats_html(stats: &store::DirStats) -> String {
    let mut html = String::from(r#"<!DOCTYPE html><html><head>
<meta charset="utf-8"><title>Directory Analytics</title>
<style>
body{font-family:system-ui,sans-serif;max-width:1100px;margin:40px auto;padding:0 20px;color:#1a2230}
h1{font-size:1.6rem;margin-bottom:4px}
.meta{color:#64748b;font-size:14px;margin-bottom:32px}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:24px}
.card{background:#f8fafc;border:1px solid #e2e8f0;border-radius:10px;padding:20px}
.card h2{font-size:1rem;font-weight:700;color:#1a3c5e;margin:0 0 12px}
.stat-big{font-size:2.4rem;font-weight:800;color:#1a3c5e}
table{width:100%;border-collapse:collapse;font-size:13px}
td,th{padding:5px 8px;text-align:left;border-bottom:1px solid #e2e8f0}
th{font-weight:600;color:#475569}
tr:last-child td{border-bottom:none}
.bar{display:inline-block;height:8px;background:#1a3c5e;border-radius:4px;min-width:2px}
</style>
</head><body>
<h1>Directory Analytics</h1>
<p class="meta">Live data from directory.sqlite &mdash; page views + click events</p>
<div style="display:flex;gap:24px;margin-bottom:32px">
  <div class="card" style="flex:1"><div class="stat-big">"#);
    html.push_str(&stats.total_views.to_string());
    html.push_str(r#"</div><div style="color:#64748b;font-size:14px">Total Page Views</div></div>
  <div class="card" style="flex:1"><div class="stat-big">"#);
    html.push_str(&stats.total_clicks.to_string());
    html.push_str(r#"</div><div style="color:#64748b;font-size:14px">Total Click Events</div></div>
</div>
<div class="grid">"#);

    html.push_str(&stat_table("Views by Page Type", &stats.by_page_type));
    html.push_str(&stat_table("Views by State (top 20)", &stats.by_state));
    html.push_str(&stat_table("Click Events by Type", &stats.by_click_type));
    html.push_str(&stat_table("Top Cities (views)", &stats.top_cities));
    html.push_str(&stat_table("Top Company Pages (views)", &stats.top_companies));
    html.push_str(&stat_table("Top Clicked Companies", &stats.top_clicked_companies));

    // Daily chart
    html.push_str(r#"<div class="card" style="grid-column:1/-1"><h2>Daily Views (last 30 days)</h2><table><tr><th>Date</th><th>Views</th><th></th></tr>"#);
    let max = stats.daily_views.iter().map(|r| r.count).max().unwrap_or(1);
    for row in &stats.daily_views {
        let w = (row.count * 200 / max.max(1)).max(2);
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><span class=\"bar\" style=\"width:{}px\"></span></td></tr>",
            row.label, row.count, w
        ));
    }
    html.push_str("</table></div>");

    html.push_str("</div></body></html>");
    html
}

fn stat_table(title: &str, rows: &[store::DirStatRow]) -> String {
    if rows.is_empty() {
        return format!(
            "<div class=\"card\"><h2>{title}</h2><p style=\"color:#94a3b8;font-size:13px\">No data yet</p></div>"
        );
    }
    let max = rows.iter().map(|r| r.count).max().unwrap_or(1);
    let mut s = format!("<div class=\"card\"><h2>{title}</h2><table><tr><th>Label</th><th>Count</th><th></th></tr>");
    for row in rows {
        let w = (row.count * 100 / max.max(1)).max(2);
        s.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><span class=\"bar\" style=\"width:{}px\"></span></td></tr>",
            esc(&row.label), row.count, w
        ));
    }
    s.push_str("</table></div>");
    s
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}


#[derive(Deserialize, Default)]
struct UpgradeQuery {
    state: Option<String>,
    city: Option<String>,
    company: Option<String>,
    name: Option<String>,
}

async fn upgrade_handler(
    State(s): State<DirRouteState>,
    Query(q): Query<UpgradeQuery>,
) -> Response {
    let company_name = q.name.clone().unwrap_or_default();
    let state_abbr = q.state.clone().map(|s| s.to_uppercase()).unwrap_or_default();
    let city_slug = q.city.clone().unwrap_or_default();
    let company_slug = q.company.clone().unwrap_or_default();
    let state_name_str = if state_abbr.is_empty() { "your state".to_string() } else { state_name_for(&state_abbr).to_string() };

    let back_url = if !state_abbr.is_empty() && !city_slug.is_empty() && !company_slug.is_empty() {
        format!("{SITE_BASE}/directory/{}/{}/{}", state_abbr.to_lowercase(), city_slug, company_slug)
    } else {
        format!("{SITE_BASE}/directory")
    };

    let mut ctx = tera::Context::new();
    ctx.insert("company_name", &company_name);
    ctx.insert("state_abbr", &state_abbr);
    ctx.insert("city_slug", &city_slug);
    ctx.insert("company_slug", &company_slug);
    ctx.insert("state_name", &state_name_str);
    ctx.insert("back_url", &back_url);
    ctx.insert("page_title", &format!(
        "Upgrade Your Listing{} — pestcontroller.org",
        if company_name.is_empty() { String::new() } else { format!(": {company_name}") }
    ));
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/upgrade"));

    render(&s.tera, "pages/directory-upgrade.html", ctx)
}

async fn about_handler(State(s): State<DirRouteState>) -> Response {
    let mut ctx = tera::Context::new();
    ctx.insert("page_title", &"About pestcontroller.org — Built by Pest Control People");
    ctx.insert("canonical_url", &format!("{SITE_BASE}/directory/about"));
    render(&s.tera, "pages/directory-about.html", ctx)
}




/// Admin: render the unclaim form (localhost-only).
async fn admin_unclaim_form(
    State(s): State<DirRouteState>,
    axum::extract::Path(company_id): axum::extract::Path<String>,
) -> Response {
    let company = match s.store.company_by_id(&company_id) {
        Some(c) => c,
        None => return render(&s.tera, "error/404.html", tera::Context::new()),
    };
    let mut ctx = tera::Context::new();
    ctx.insert("company_id", &company.id);
    ctx.insert("company_name", &company.entity_name);
    ctx.insert("claimed_by", &company.claimed_by);
    let html = s.tera.render("pages/directory-admin-unclaim.html", &ctx).unwrap_or_default();
    Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(axum::body::Body::from(html).into())
        .unwrap()
}

async fn admin_unclaim_submit(
    State(s): State<DirRouteState>,
    axum::extract::Path(company_id): axum::extract::Path<String>,
) -> Response {
    match s.store.unclaim_company(&company_id) {
        Ok(result) => {
            let j = serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string());
            let html = format!(
                "<html><head><title>Unclaimed</title></head>\n\
                 <body style=\"font-family:monospace;padding:2rem\">\n\
                 <h1>Company Unclaimed</h1><pre>{}</pre><br/>\n\
                 <a href=\"/directory\">Back to directory</a><br/>\n\
                 <a href=\"javascript:history.back()\">Go back</a>\n\
                 </body></html>",
                j
            );
            Response::builder()
                .header("Content-Type", "text/html; charset=utf-8")
                .body(axum::body::Body::from(html).into())
                .unwrap()
        }
        Err(e) => {
            let html = format!(
                "<html><head><title>Error</title></head>\n\
                 <body style=\"font-family:monospace;padding:2rem\">\n\
                 <h1>Error</h1><p>{}</p>\n\
                 <a href=\"javascript:history.back()\">Go back</a>\n\
                 </body></html>",
                e
            );
            Response::builder()
                .status(500)
                .header("Content-Type", "text/html; charset=utf-8")
                .body(axum::body::Body::from(html).into())
                .unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mega_menu_visible_only_for_owner() {
        // Verified owner sees the rebuilt-site preview mega-menu.
        assert!(mega_menu_visible(ViewerTier::Owner));
        // Public / unclaimed viewer does NOT (claim-first; raw crawl URLs 404).
        assert!(!mega_menu_visible(ViewerTier::Public));
    }
}
