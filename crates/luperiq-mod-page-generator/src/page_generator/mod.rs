//! SEO Page Generator Module — bulk page generation with cross-product support.
//!
//! Generates hundreds of SEO-optimized pages from industry item x city combinations.
//! The admin selects items (pest types, service types, menu categories, etc.) and
//! enters service areas/cities, then the module generates cross-product pages with
//! silo linking:
//!
//! - **Item hub pages** — `/termite-control` (one per item)
//! - **City hub pages** — `/pest-control-dallas` (one per city)
//! - **Cross-product pages** — `/termite-control-dallas` (item x city)
//! - **Category hub pages** — `/crawling-insect-control` (one per item category)
//! - **Category x city pages** — `/crawling-insect-control-dallas` (category x city)
//!
//! Two modes:
//! - **Template mode** — instant, uses `{{variable}}` substitution, free/cheap
//! - **AI mode** — generates unique content per page via AI, deducts credits
//!
//! Industry-agnostic: any module implementing `IndustryPageGenProvider` can register
//! items for page generation (pest types, HVAC equipment, plumbing services, etc.).
//!
//! Security notes:
//! - Admin UI uses DOM methods (createElement/textContent) for XSS safety
//! - All write endpoints are admin-authenticated via middleware in main.rs
//! - Bulk generation requires typing "GENERATE" to confirm (safety gate)

pub mod admin_js;
pub mod industry;
// providers.rs stays in the CMS crate — concrete industry implementations
pub mod templates;

use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use luperiq_mod_content_sources::content_sources::query::PlannedPageSources;
use luperiq_mod_location_profile::location_profile::profile::{load_all_locations, LocationProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use luperiq_forge::{ApexEvent, ForgeContent, ForgeContentManager};

use luperiq_module_api::{AdminView, AppContext, CmsModule, NexusNetworkConfig, SharedJournal};

/// AI response from the page generator's perspective.
#[derive(Debug, Clone)]
pub struct PageGenAiResponse {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Abstract AI client trait — CMS wires in the real implementation.
pub trait PageGenAiClient: Send + Sync + 'static {
    fn is_configured(&self) -> bool;
    fn status(&self) -> serde_json::Value;
    fn generate(
        &self,
        system: &str,
        user_message: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<PageGenAiResponse, String>> + Send + '_>,
    >;
}

/// Type alias used in state.
type AiClient = dyn PageGenAiClient;

pub use industry::{
    IndustryItem, IndustryPageGenConfig, IndustryPageGenProvider, PageGenProviderRegistry,
};

// ── Aggregate type constants ────────────────────────────────────────

// AGG_SEO_META is owned and published by luperiq-mod-seo (seo/mod.rs).
// Using the crate's own const avoids silent desync if the key is ever renamed.
use luperiq_mod_seo::seo::AGG_SEO_META;
const AGG_PAGE_GEN_BATCH: &str = "PageGenBatch";

/// Tombstone value for filtering deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Shared state ────────────────────────────────────────────────────

#[derive(Clone)]
struct PageGenState {
    journal: SharedJournal,
    ai_client: Option<Arc<AiClient>>,
    nexus_config: Option<NexusNetworkConfig>,
    provider_registry: Arc<PageGenProviderRegistry>,
}

// ── Module definition ───────────────────────────────────────────────

pub struct PageGeneratorModule {
    pub provider_registry: Arc<PageGenProviderRegistry>,
    pub ai_client: Option<Arc<AiClient>>,
}

impl CmsModule for PageGeneratorModule {
    fn slug(&self) -> &str {
        "page-generator"
    }
    fn name(&self) -> &str {
        "SEO Page Generator"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Bulk SEO page generation from industry item x city cross-products with silo linking."
    }

    fn category(&self) -> &str {
        "Content"
    }

    fn dependencies(&self) -> &[&str] {
        &["seo"]
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        let state = PageGenState {
            journal: ctx.journal.clone(),
            ai_client: self.ai_client.clone(),
            nexus_config: ctx.nexus_config.clone(),
            provider_registry: self.provider_registry.clone(),
        };

        let router = Router::new()
            // New generic endpoints
            .route("/api/modules/page-generator/items", get(list_items))
            .route("/api/modules/page-generator/config", get(get_config))
            .route(
                "/api/modules/page-generator/industries",
                get(list_industries),
            )
            // Legacy endpoint — maps to list_items for backward compat
            .route("/api/modules/page-generator/pest-types", get(list_items))
            .route(
                "/api/modules/page-generator/preview",
                post(preview_generation),
            )
            .route("/api/modules/page-generator/generate", post(generate_pages))
            .route("/api/modules/page-generator/batches", get(list_batches))
            .route("/api/modules/page-generator/ai/status", get(ai_status))
            .with_state(state);

        Some(router)
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![AdminView {
            id: "page-generator".into(),
            label: "SEO Page Generator".into(),
            section: "Content".into(),
        }]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::ADMIN_JS.to_string())
    }

    fn admin_css(&self) -> Option<String> {
        Some(ADMIN_CSS.to_string())
    }
}

// ── Domain types ────────────────────────────────────────────────────

/// Page types we can generate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PageKind {
    /// One per item (e.g. "Termite Control", "AC Repair")
    ItemHub,
    /// One per city (e.g. "Pest Control in Dallas", "HVAC Service in Dallas")
    CityHub,
    /// Item x City cross-product (e.g. "Termite Control in Dallas")
    ItemCity,
    /// One per item category (e.g. "Crawling Insect Control")
    CategoryHub,
    /// Category x City (e.g. "Crawling Insect Control in Dallas")
    CategoryCity,
}

impl PageKind {
    fn label(&self) -> &str {
        match self {
            Self::ItemHub => "Item Hub",
            Self::CityHub => "City Hub",
            Self::ItemCity => "Item x City",
            Self::CategoryHub => "Category Hub",
            Self::CategoryCity => "Category x City",
        }
    }
}

/// A real photo (from the SEO Photo Library) attached to a planned page.
/// Carries just the fields the template renderer needs — never the full
/// `PhotoLibraryEntry`, which is owned by `luperiq-mod-seo`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct SeoPhoto {
    pub(crate) image_url: String,
    pub(crate) alt: String,
    pub(crate) location_zip: Option<String>,
    pub(crate) pest_type: Option<String>,
}

/// A single page to be generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PlannedPage {
    pub(crate) kind: PageKind,
    pub(crate) title: String,
    pub(crate) slug: String,
    pub(crate) focus_keyword: String,
    pub(crate) meta_description: String,
    /// Parent hub slug for silo linking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent_slug: Option<String>,
    /// Related page slugs for cross-linking
    #[serde(default)]
    pub(crate) related_slugs: Vec<String>,
    /// Assembled content sources for AI prompts (LuperIQ facts, customer facts, raw reference).
    #[serde(default)]
    pub(crate) sources: PlannedPageSources,
    /// Location intelligence injected into AI prompts when a LocationProfile exists for this city.
    #[serde(default)]
    pub(crate) location_context: String,
    /// Phase 7 / 2026-05-27 — approved photos from the SEO Photo Library
    /// matched to this page's pest + city. Empty when no matching approved
    /// photos exist or when this page type doesn't use real photos
    /// (everything except `ItemCity` / `CityHub` for now).
    #[serde(default)]
    pub(crate) seo_photos: Vec<SeoPhoto>,
}

/// Request payload for preview and generate endpoints.
#[derive(Debug, Deserialize)]
struct GenerateRequest {
    /// Item slugs to include (generic name)
    #[serde(default)]
    item_slugs: Vec<String>,
    /// Backward compat: pest_slugs maps to item_slugs
    #[serde(default)]
    pest_slugs: Vec<String>,
    /// Custom item entries (name only, we generate slug)
    #[serde(default)]
    custom_items: Vec<CustomItem>,
    /// Backward compat: custom_pests maps to custom_items
    #[serde(default)]
    custom_pests: Vec<CustomItem>,
    /// City/area names
    cities: Vec<String>,
    /// State abbreviation (e.g. "TX")
    #[serde(default)]
    state_abbr: String,
    /// Business brand name
    #[serde(default)]
    brand: String,
    /// Business phone
    #[serde(default)]
    phone: String,
    /// Page types to generate
    #[serde(default)]
    page_types: Vec<String>,
    /// "template" or "ai"
    #[serde(default = "default_mode")]
    mode: String,
    /// Safety confirmation — must be "GENERATE"
    #[serde(default)]
    confirmation: String,
    /// License key for AI credit deduction (client nodes)
    #[serde(default)]
    license_key: Option<String>,
    /// Maximum credits to charge (from the UI quote — server will never exceed this)
    #[serde(default)]
    max_credits: Option<u32>,
    /// Industry slug to use (e.g. "pest-control", "hvac"). Defaults to first registered.
    #[serde(default)]
    industry: String,
}

impl GenerateRequest {
    /// Merge backward-compat fields into the canonical fields.
    fn merged_item_slugs(&self) -> Vec<String> {
        let mut slugs = self.item_slugs.clone();
        slugs.extend(self.pest_slugs.clone());
        slugs
    }

    fn merged_custom_items(&self) -> Vec<CustomItem> {
        let mut items = self.custom_items.clone();
        items.extend(self.custom_pests.clone());
        items
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CustomItem {
    name: String,
    #[serde(default)]
    category: String,
}

fn default_mode() -> String {
    "template".into()
}

/// Batch record saved to the journal after generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchRecord {
    batch_id: String,
    pages_created: u32,
    mode: String,
    /// Number of items (pest types, service types, etc.) in this batch.
    /// Backward compat: old records stored this as `pest_count`.
    #[serde(alias = "pest_count")]
    item_count: u32,
    city_count: u32,
    page_types: Vec<String>,
    ai_tokens_used: u32,
    ai_credits_charged: u32,
    errors: Vec<String>,
    created_at: u64,
    #[serde(default)]
    industry: String,
}

/// SEO meta aggregate (matches seo module's structure).
#[derive(Debug, Serialize, Deserialize)]
struct SeoMeta {
    content_id: String,
    title: String,
    description: String,
    #[serde(default)]
    focus_keyword: String,
}

/// Standard API response envelope.
#[derive(Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

// ── Utility functions ───────────────────────────────────────────────

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>()
        .join("-")
}

fn new_id() -> String {
    ulid::Ulid::new().to_string()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Resolve active industry provider ────────────────────────────────

fn resolve_provider<'a>(
    registry: &'a PageGenProviderRegistry,
    industry_slug: &str,
) -> Option<&'a dyn IndustryPageGenProvider> {
    if industry_slug.is_empty() {
        registry.first()
    } else {
        registry.get(industry_slug)
    }
}


// ── Location context builder ────────────────────────────────────────

/// Build a pre-rendered location intelligence string for injection into AI prompts.
/// Called once per city; the result is cloned into each city-bound PlannedPage.
fn build_location_context(profile: &LocationProfile) -> String {
    let mut ctx = format!("--- Location Profile: {}, {} ---\n", profile.city, profile.state);

    if !profile.area_description.is_empty() {
        ctx.push_str(&format!("Area Overview: {}\n\n", profile.area_description));
    }

    if !profile.weather_patterns.is_empty() {
        ctx.push_str("Seasonal Patterns:\n");
        for wp in &profile.weather_patterns {
            let high = wp.avg_high_f.map(|t| format!("{}°F avg high", t)).unwrap_or_default();
            ctx.push_str(&format!(
                "  {} ({}): {}\n",
                capitalize(&wp.season),
                high,
                wp.description
            ));
        }
        ctx.push('\n');
    }

    if !profile.local_keywords.is_empty() {
        ctx.push_str("High-Value Local Search Terms:\n");
        for kw in profile.local_keywords.iter().take(10) {
            let vol = kw
                .search_volume
                .map(|v| format!(" ({}/mo)", v))
                .unwrap_or_default();
            ctx.push_str(&format!("  - {}{}\n", kw.keyword, vol));
        }
        ctx.push('\n');
    }

    if !profile.local_competitors.is_empty() {
        ctx.push_str("Local Competitors:\n");
        for comp in profile.local_competitors.iter().take(5) {
            let rating = comp
                .rating
                .map(|r| format!(" ({:.1}★", r))
                .unwrap_or_default();
            let reviews = if comp.review_count.is_some() {
                format!(", {} reviews)", comp.review_count.unwrap_or(0))
            } else if comp.rating.is_some() {
                ")".to_string()
            } else {
                String::new()
            };
            ctx.push_str(&format!("  - {}{}{}\n", comp.name, rating, reviews));
        }
        ctx.push('\n');
    }

    if !profile.local_regulations.is_empty() {
        ctx.push_str("Local Regulations:\n");
        for reg in &profile.local_regulations {
            ctx.push_str(&format!(
                "  - {} ({}): {}\n",
                reg.name, reg.authority, reg.description
            ));
        }
        ctx.push('\n');
    }

    if !profile.neighborhoods.is_empty() {
        ctx.push_str(&format!(
            "Local Neighborhoods/Areas: {}\n",
            profile.neighborhoods.join(", ")
        ));
    }

    ctx
}

/// Build a lookup map keyed by all sensible slugified forms of the city name.
fn build_location_map(
    profiles: Vec<LocationProfile>,
    state_abbr: &str,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for p in profiles {
        if !p.active {
            continue;
        }
        let ctx = build_location_context(&p);
        // Index by: "austin", "austin-tx", explicit slug
        let city_slug = slugify(&p.city);
        let state_lower = p.state.to_lowercase();
        map.insert(city_slug.clone(), ctx.clone());
        map.insert(format!("{}-{}", city_slug, state_lower), ctx.clone());
        if !p.slug.is_empty() && p.slug != city_slug {
            map.insert(p.slug.clone(), ctx.clone());
        }
        // If state_abbr provided, also try "austin-tx" even if profile.state is "Texas"
        if !state_abbr.is_empty() {
            let abbr = state_abbr.to_lowercase();
            if abbr != state_lower {
                map.insert(format!("{}-{}", city_slug, abbr), ctx.clone());
            }
        }
    }
    map
}

// ── SEO photo attachment (Phase 7 / 2026-05-27) ─────────────────────

/// Maximum number of approved photos surfaced per generated page. Keeps
/// generated pages snappy even when the library is dense.
const MAX_SEO_PHOTOS_PER_PAGE: usize = 4;

/// Build a `city_slug → Vec<zip_code>` map from the active LocationProfiles.
/// Keys mirror `build_location_map` so the same lookup keys work.
fn build_city_zip_map(profiles: &[LocationProfile], state_abbr: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for p in profiles {
        if !p.active {
            continue;
        }
        let zips = p.zip_codes.clone();
        let city_slug = slugify(&p.city);
        let state_lower = p.state.to_lowercase();
        map.entry(city_slug.clone()).or_default().extend(zips.iter().cloned());
        map.entry(format!("{}-{}", city_slug, state_lower))
            .or_default()
            .extend(zips.iter().cloned());
        if !p.slug.is_empty() && p.slug != city_slug {
            map.entry(p.slug.clone()).or_default().extend(zips.iter().cloned());
        }
        if !state_abbr.is_empty() {
            let abbr = state_abbr.to_lowercase();
            if abbr != state_lower {
                map.entry(format!("{}-{}", city_slug, abbr))
                    .or_default()
                    .extend(zips.iter().cloned());
            }
        }
    }
    // Dedup each value list.
    for zips in map.values_mut() {
        zips.sort();
        zips.dedup();
    }
    map
}

/// Pull approved photos from the SEO library and stamp them onto the
/// planned pages where they belong. Only `ItemCity` and `CityHub` pages
/// get photos for now — that's where geographic specificity is meaningful.
async fn attach_seo_photos(
    planned: &mut [PlannedPage],
    journal: &SharedJournal,
    cities: &[String],
    state_abbr: &str,
) {
    let j = journal.lock().await;
    let profiles = load_all_locations(&j);
    let zip_map = build_city_zip_map(&profiles, state_abbr);

    for page in planned.iter_mut() {
        if !matches!(page.kind, PageKind::ItemCity | PageKind::CityHub) {
            continue;
        }
        let (item_name_lower, city_slug) = page_item_city_for_zip_lookup(page, cities, state_abbr);

        let mut zips: Vec<String> = Vec::new();
        if !city_slug.is_empty() {
            if let Some(z) = zip_map.get(&city_slug) {
                zips.extend(z.iter().cloned());
            }
            if let Some(z) = zip_map.get(&format!(
                "{}-{}",
                city_slug,
                state_abbr.to_lowercase()
            )) {
                zips.extend(z.iter().cloned());
            }
        }
        zips.sort();
        zips.dedup();

        let pest_query: Option<&str> = if matches!(page.kind, PageKind::ItemCity) && !item_name_lower.is_empty() {
            Some(item_name_lower.as_str())
        } else {
            None
        };

        let approved = luperiq_mod_seo::seo::photo_library::query_approved_for_generator(
            &j,
            pest_query,
            &zips,
            true,
            MAX_SEO_PHOTOS_PER_PAGE,
        );

        page.seo_photos = approved
            .into_iter()
            .map(|e| SeoPhoto {
                image_url: e.image_url,
                alt: e
                    .caption
                    .filter(|s| !s.is_empty())
                    .or(e.notes)
                    .unwrap_or_else(|| page.title.clone()),
                location_zip: e.location_zip,
                pest_type: e.pest_type,
            })
            .collect();
    }
}

/// Reverse-derive `(item_name_lowercased, city_slug)` from a planned page.
/// Used to drive the photo-library query without re-threading the original
/// `IndustryItem` / city tuple all the way down here.
fn page_item_city_for_zip_lookup(
    page: &PlannedPage,
    cities: &[String],
    _state_abbr: &str,
) -> (String, String) {
    // ItemCity slugs look like `<item>-<city>` or `<item>-<city>-<state>`.
    // CityHub slugs look like `<prefix>-<city>` where prefix is e.g.
    // "pest-control" / "hvac-service".
    let slug = page.slug.to_lowercase();
    for c in cities {
        let cs = slugify(c);
        if cs.is_empty() {
            continue;
        }
        // Try suffix `-<city>` and `-<city>-<state>` variants.
        let suffix1 = format!("-{}", cs);
        if let Some(idx) = slug.rfind(&suffix1) {
            // Anything before suffix is item+state OR hub prefix.
            let head = &slug[..idx];
            let item = head.replace('-', " ");
            return (item, cs);
        }
    }
    (String::new(), String::new())
}

// ── Plan generation (shared between preview and generate) ───────────

fn plan_pages(
    items: &[IndustryItem],
    cities: &[String],
    state_abbr: &str,
    brand: &str,
    page_types: &[String],
    config: &IndustryPageGenConfig,
    customer_sources: &std::collections::HashMap<
        String,
        Vec<luperiq_mod_content_sources::content_sources::types::ContentSource>,
    >,
    location_map: &HashMap<String, String>,
) -> Vec<PlannedPage> {
    let mut pages: Vec<PlannedPage> = Vec::new();
    let state_suffix = if state_abbr.is_empty() {
        String::new()
    } else {
        format!(", {}", state_abbr.to_uppercase())
    };

    let verb = &config.service_verb;
    let verb_cap = capitalize(verb);
    let city_hub_prefix = &config.city_hub_prefix;
    let industry_name = &config.industry_name;

    // If an item slug or name already ends with the verb (e.g. "urgent-care"
    // with verb "care", "ac-repair" with verb "repair"), don't append it
    // again — that produces awkward URLs/titles like "urgent-care-care".
    let item_slug_with_verb = |item_slug: &str| -> String {
        let needle = format!("-{verb}");
        if item_slug == verb || item_slug.ends_with(&needle) {
            item_slug.to_string()
        } else {
            format!("{item_slug}-{verb}")
        }
    };
    let item_name_with_verb = |item_name: &str| -> String {
        let lower = item_name.to_lowercase();
        let needle = format!(" {verb}");
        if lower == verb.as_str() || lower.ends_with(&needle) {
            item_name.to_string()
        } else {
            format!("{item_name} {verb_cap}")
        }
    };
    let item_phrase_with_verb = |item_name: &str| -> String {
        let lower = item_name.to_lowercase();
        let needle = format!(" {verb}");
        if lower == verb.as_str() || lower.ends_with(&needle) {
            lower
        } else {
            format!("{lower} {verb}")
        }
    };

    // Backward compat: accept both pest_hub/item_hub, pest_city/item_city
    let do_item_hub = page_types.is_empty()
        || page_types.contains(&"item_hub".to_string())
        || page_types.contains(&"pest_hub".to_string());
    let do_city_hub = page_types.is_empty() || page_types.contains(&"city_hub".to_string());
    let do_item_city = page_types.is_empty()
        || page_types.contains(&"item_city".to_string())
        || page_types.contains(&"pest_city".to_string());
    let do_category_hub = page_types.contains(&"category_hub".to_string());
    let do_category_city = page_types.contains(&"category_city".to_string());

    // ── Item hub pages ──────────────────────────────────────────────
    if do_item_hub {
        for item in items {
            let slug = item_slug_with_verb(&item.slug);
            let title = item_name_with_verb(&item.name);
            let focus = item_phrase_with_verb(&item.name);
            let meta = if brand.is_empty() {
                format!(
                    "Professional {} services. Identification, treatment, and prevention.",
                    focus
                )
            } else {
                format!(
                    "{} provides expert {}. Identification, treatment plans, and prevention.",
                    brand, focus
                )
            };

            pages.push(PlannedPage {
                kind: PageKind::ItemHub,
                title,
                slug,
                focus_keyword: focus,
                meta_description: truncate(&meta, 160),
                parent_slug: None,
                related_slugs: vec![],
                sources: luperiq_mod_content_sources::content_sources::query::assemble_sources(
                    &item.fact_sheet,
                    customer_sources
                        .get(&item.slug)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]),
                ),
                location_context: String::new(),
                seo_photos: Vec::new(),
            });
        }
    }

    // ── City hub pages ──────────────────────────────────────────────
    if do_city_hub {
        for city in cities {
            let city_slug = slugify(city);
            let slug = format!("{}-{}", city_hub_prefix, city_slug);
            let title = format!("{} in {}{}", industry_name, city, state_suffix);
            let focus = format!(
                "{} in {}",
                industry_name.to_lowercase(),
                city.to_lowercase()
            );
            let meta = if brand.is_empty() {
                format!(
                    "Local {} services in {}{}. Request a free consultation today.",
                    industry_name.to_lowercase(),
                    city,
                    state_suffix
                )
            } else {
                format!(
                    "{} offers professional {} in {}{}. Call for a free consultation.",
                    brand,
                    industry_name.to_lowercase(),
                    city,
                    state_suffix
                )
            };

            let loc_ctx = {
                let key1 = slugify(city);
                let key2 = if state_abbr.is_empty() {
                    String::new()
                } else {
                    format!("{}-{}", key1, state_abbr.to_lowercase())
                };
                location_map
                    .get(&key1)
                    .or_else(|| location_map.get(&key2))
                    .cloned()
                    .unwrap_or_default()
            };
            pages.push(PlannedPage {
                kind: PageKind::CityHub,
                title,
                slug,
                focus_keyword: focus,
                meta_description: truncate(&meta, 160),
                parent_slug: None,
                related_slugs: vec![],
                sources: PlannedPageSources::default(),
                location_context: loc_ctx,
                seo_photos: Vec::new(),
            });
        }
    }

    // ── Cross-product: item x city ──────────────────────────────────
    if do_item_city {
        for item in items {
            let item_hub_slug = item_slug_with_verb(&item.slug);
            let item_phrase = item_phrase_with_verb(&item.name);
            let item_title = item_name_with_verb(&item.name);
            for city in cities {
                let city_slug = slugify(city);
                let city_hub_slug = format!("{}-{}", city_hub_prefix, city_slug);
                let slug = format!("{}-{}", item_hub_slug, city_slug);
                let title = format!("{} in {}{}", item_title, city, state_suffix);
                let focus = format!("{} in {}", item_phrase, city.to_lowercase());
                let meta = if brand.is_empty() {
                    format!(
                        "Need {} in {}{}? Get expert treatment and service.",
                        item_phrase, city, state_suffix
                    )
                } else {
                    format!(
                        "{} provides {} in {}{}. Call for a consultation.",
                        brand, item_phrase, city, state_suffix
                    )
                };

                let loc_ctx_ic = {
                    let key1 = slugify(city);
                    let key2 = if state_abbr.is_empty() {
                        String::new()
                    } else {
                        format!("{}-{}", key1, state_abbr.to_lowercase())
                    };
                    location_map
                        .get(&key1)
                        .or_else(|| location_map.get(&key2))
                        .cloned()
                        .unwrap_or_default()
                };
                pages.push(PlannedPage {
                    kind: PageKind::ItemCity,
                    title,
                    slug,
                    focus_keyword: focus,
                    meta_description: truncate(&meta, 160),
                    parent_slug: Some(item_hub_slug.clone()),
                    related_slugs: vec![city_hub_slug],
                    sources: luperiq_mod_content_sources::content_sources::query::assemble_sources(
                        &item.fact_sheet,
                        customer_sources
                            .get(&item.slug)
                            .map(|v| v.as_slice())
                            .unwrap_or(&[]),
                    ),
                    location_context: loc_ctx_ic,
                    seo_photos: Vec::new(),
                });
            }
        }
    }

    // ── Category hubs ───────────────────────────────────────────────
    if do_category_hub || do_category_city {
        let mut categories: Vec<String> = items.iter().map(|it| it.category.clone()).collect();
        categories.sort();
        categories.dedup();

        if do_category_hub {
            for cat in &categories {
                let cat_slug = slugify(cat);
                let slug = format!("{}-{}", cat_slug, verb);
                let title = format!("{} {}", cat, verb_cap);
                let focus = format!("{} {}", cat.to_lowercase(), verb);
                let meta = format!(
                    "Complete {} {} services. Expert identification and treatment.",
                    cat.to_lowercase(),
                    verb
                );

                pages.push(PlannedPage {
                    kind: PageKind::CategoryHub,
                    title,
                    slug,
                    focus_keyword: focus,
                    meta_description: truncate(&meta, 160),
                    parent_slug: None,
                    related_slugs: vec![],
                    sources: PlannedPageSources::default(),
                    location_context: String::new(),
                    seo_photos: Vec::new(),
                });
            }
        }

        // Category x city
        if do_category_city {
            for cat in &categories {
                let cat_slug = slugify(cat);
                let cat_hub_slug = format!("{}-{}", cat_slug, verb);
                for city in cities {
                    let city_slug = slugify(city);
                    let slug = format!("{}-{}-{}", cat_slug, verb, city_slug);
                    let title = format!("{} {} in {}{}", cat, verb_cap, city, state_suffix);
                    let focus =
                        format!("{} {} in {}", cat.to_lowercase(), verb, city.to_lowercase());
                    let meta = format!(
                        "{} {} services in {}{}. Expert treatment and service.",
                        cat, verb, city, state_suffix
                    );

                    let loc_ctx_cc = {
                        let key1 = slugify(city);
                        let key2 = if state_abbr.is_empty() {
                            String::new()
                        } else {
                            format!("{}-{}", key1, state_abbr.to_lowercase())
                        };
                        location_map
                            .get(&key1)
                            .or_else(|| location_map.get(&key2))
                            .cloned()
                            .unwrap_or_default()
                    };
                    pages.push(PlannedPage {
                        kind: PageKind::CategoryCity,
                        title,
                        slug,
                        focus_keyword: focus,
                        meta_description: truncate(&meta, 160),
                        parent_slug: Some(cat_hub_slug.clone()),
                        related_slugs: vec![format!("{}-{}", city_hub_prefix, city_slug)],
                        sources: PlannedPageSources::default(),
                        location_context: loc_ctx_cc,
                        seo_photos: Vec::new(),
                    });
                }
            }
        }
    }

    // ── Build silo cross-links ──────────────────────────────────────
    let all_slugs: Vec<String> = pages.iter().map(|p| p.slug.clone()).collect();

    for page in &mut pages {
        match page.kind {
            PageKind::ItemHub => {
                // Link to all item_city pages that start with the same item slug
                let prefix = page.slug.clone();
                page.related_slugs = all_slugs
                    .iter()
                    .filter(|s| s.starts_with(&format!("{}-", prefix)) && **s != prefix)
                    .cloned()
                    .collect();
            }
            PageKind::CityHub => {
                // Link to all item_city pages that end with this city slug
                let city_suffix = page
                    .slug
                    .strip_prefix(&format!("{}-", city_hub_prefix))
                    .unwrap_or("");
                if !city_suffix.is_empty() {
                    page.related_slugs = all_slugs
                        .iter()
                        .filter(|s| s.ends_with(&format!("-{}", city_suffix)) && **s != page.slug)
                        .cloned()
                        .collect();
                }
            }
            _ => {}
        }
    }

    pages
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut truncated = s[..max - 3].to_string();
        truncated.push_str("...");
        truncated
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
// NOTE: templates.rs also defines an identical capitalize — consider making one
// pub(super) and removing the other once templates.rs is refactored.

// ── Handlers ────────────────────────────────────────────────────────

/// GET /api/modules/page-generator/items  (also served at /pest-types for backward compat)
///
/// Returns all active items from the active industry provider.
/// Accepts optional `?industry=slug` query param.
async fn list_items(
    State(state): State<PageGenState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let industry_slug = params.get("industry").map(|s| s.as_str()).unwrap_or("");
    let provider = match resolve_provider(&state.provider_registry, industry_slug) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "No industry provider registered.".into(),
                data: None,
            })
        }
    };

    let j = state.journal.lock().await;
    let items = provider.load_items(&j);

    Json(ApiResult {
        ok: true,
        message: format!(
            "{} items available ({})",
            items.len(),
            provider.page_gen_config().industry_name
        ),
        data: Some(serde_json::json!(items)),
    })
}

/// GET /api/modules/page-generator/config
///
/// Returns the IndustryPageGenConfig for the active industry.
async fn get_config(
    State(state): State<PageGenState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let industry_slug = params.get("industry").map(|s| s.as_str()).unwrap_or("");
    let provider = match resolve_provider(&state.provider_registry, industry_slug) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "No industry provider registered.".into(),
                data: None,
            })
        }
    };

    let config = provider.page_gen_config();
    Json(ApiResult {
        ok: true,
        message: format!("Config for {}", config.industry_name),
        data: Some(serde_json::to_value(&config).unwrap_or_default()),
    })
}

/// GET /api/modules/page-generator/industries
///
/// Lists all registered industry providers.
async fn list_industries(State(state): State<PageGenState>) -> Json<ApiResult> {
    let industries: Vec<serde_json::Value> = state
        .provider_registry
        .list()
        .iter()
        .map(|p| {
            let config = p.page_gen_config();
            serde_json::json!({
                "slug": p.industry_slug(),
                "name": config.industry_name,
                "item_singular": config.item_singular,
                "item_plural": config.item_plural,
                "service_verb": config.service_verb,
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} industries available", industries.len()),
        data: Some(serde_json::json!(industries)),
    })
}

/// POST /api/modules/page-generator/preview
///
/// Plans and returns the pages that would be generated, without creating them.
async fn preview_generation(
    State(state): State<PageGenState>,
    axum::extract::Json(req): axum::extract::Json<GenerateRequest>,
) -> Json<ApiResult> {
    let provider = match resolve_provider(&state.provider_registry, &req.industry) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "No industry provider registered.".into(),
                data: None,
            })
        }
    };

    let config = provider.page_gen_config();
    let j = state.journal.lock().await;
    let all_items = provider.load_items(&j);
    drop(j);

    // Resolve selected items + custom ones
    let merged_slugs = req.merged_item_slugs();
    let mut selected: Vec<IndustryItem> = all_items
        .into_iter()
        .filter(|it| merged_slugs.contains(&it.slug))
        .collect();

    for custom in req.merged_custom_items() {
        selected.push(IndustryItem {
            id: new_id(),
            name: custom.name.clone(),
            slug: slugify(&custom.name),
            category: if custom.category.is_empty() {
                "Custom".to_string()
            } else {
                custom.category.clone()
            },
            description: String::new(),
            active: true,
            fact_sheet: String::new(),
        });
    }

    if selected.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: format!("Select at least one {}.", config.item_singular),
            data: None,
        });
    }

    if req.cities.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Enter at least one city or service area.".into(),
            data: None,
        });
    }

    let location_map = {
        let j_loc = state.journal.lock().await;
        let profiles = load_all_locations(&j_loc);
        drop(j_loc);
        build_location_map(profiles, &req.state_abbr)
    };

    let pages = plan_pages(
        &selected,
        &req.cities,
        &req.state_abbr,
        &req.brand,
        &req.page_types,
        &config,
        &std::collections::HashMap::new(),
        &location_map,
    );

    // Compute credit cost for AI mode
    let pricing = {
        let j_pricing = state.journal.lock().await;
        luperiq_mod_content_sources::content_sources::pricing::load_pricing(&j_pricing)
    };
    let credits_per_page = pricing.credits_per_page;
    let credits_per_seo = pricing.credits_per_seo;
    let total_pages = pages.len() as u32;
    let total_credits = total_pages * (credits_per_page + credits_per_seo);

    // Summarize by kind
    let mut by_kind: HashMap<String, u32> = HashMap::new();
    for p in &pages {
        *by_kind.entry(p.kind.label().to_string()).or_default() += 1;
    }

    Json(ApiResult {
        ok: true,
        message: format!("{} pages planned", pages.len()),
        data: Some(serde_json::json!({
            "total_pages": total_pages,
            "pages": pages,
            "by_kind": by_kind,
            "ai_credits_estimate": total_credits,
            "credits_per_page": credits_per_page,
            "credits_per_seo": credits_per_seo,
            "industry": provider.industry_slug(),
        })),
    })
}

/// POST /api/modules/page-generator/generate
///
/// Generates all planned pages, creates content + SEO meta in the journal.
/// Requires `confirmation: "GENERATE"` as a safety gate.
async fn generate_pages(
    State(state): State<PageGenState>,
    axum::extract::Json(req): axum::extract::Json<GenerateRequest>,
) -> Json<ApiResult> {
    // Safety gate
    if req.confirmation != "GENERATE" {
        return Json(ApiResult {
            ok: false,
            message: "Type GENERATE to confirm bulk page creation.".into(),
            data: None,
        });
    }

    let provider = match resolve_provider(&state.provider_registry, &req.industry) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "No industry provider registered.".into(),
                data: None,
            })
        }
    };

    let config = provider.page_gen_config();

    let j_lock = state.journal.lock().await;
    let all_items = provider.load_items(&j_lock);
    drop(j_lock);

    // Resolve items
    let merged_slugs = req.merged_item_slugs();
    let mut selected: Vec<IndustryItem> = all_items
        .into_iter()
        .filter(|it| merged_slugs.contains(&it.slug))
        .collect();

    for custom in req.merged_custom_items() {
        selected.push(IndustryItem {
            id: new_id(),
            name: custom.name.clone(),
            slug: slugify(&custom.name),
            category: if custom.category.is_empty() {
                "Custom".to_string()
            } else {
                custom.category.clone()
            },
            description: String::new(),
            active: true,
            fact_sheet: String::new(),
        });
    }

    if selected.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: format!("Select at least one {}.", config.item_singular),
            data: None,
        });
    }

    if req.cities.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Enter at least one city or service area.".into(),
            data: None,
        });
    }

    // Load customer content sources for selected items
    let customer_sources: std::collections::HashMap<
        String,
        Vec<luperiq_mod_content_sources::content_sources::types::ContentSource>,
    > = {
        let j_sources = state.journal.lock().await;
        let mut map = std::collections::HashMap::new();
        for item in &selected {
            let sources =
                luperiq_mod_content_sources::content_sources::query::get_sources_for_topic(
                    &j_sources,
                    provider.industry_slug(),
                    &item.slug,
                );
            if !sources.is_empty() {
                map.insert(item.slug.clone(), sources);
            }
        }
        drop(j_sources);
        map
    };

    let location_map = {
        let j_loc = state.journal.lock().await;
        let profiles = load_all_locations(&j_loc);
        drop(j_loc);
        build_location_map(profiles, &req.state_abbr)
    };

    let mut planned = plan_pages(
        &selected,
        &req.cities,
        &req.state_abbr,
        &req.brand,
        &req.page_types,
        &config,
        &customer_sources,
        &location_map,
    );

    // Phase 7 / 2026-05-27 — attach approved SEO photos to city-bound pages.
    // We re-lock the journal once, build a city→zips lookup from the
    // LocationProfiles, then query luperiq-mod-seo's photo library per page.
    attach_seo_photos(
        &mut planned,
        &state.journal,
        &req.cities,
        &req.state_abbr,
    )
    .await;

    let now = now_secs();
    let is_ai = req.mode == "ai";
    let mut errors: Vec<String> = Vec::new();
    let mut ai_tokens_used: u32 = 0;
    let mut total_credits: u32 = 0;

    // Build template context for this industry
    let tpl_ctx = templates::TemplateContext {
        industry_name: config.industry_name.clone(),
        item_singular: config.item_singular.clone(),
        item_plural: config.item_plural.clone(),
        service_verb: config.service_verb.clone(),
        city_hub_prefix: config.city_hub_prefix.clone(),
    };

    // ── AI mode: reserve credits and generate content ───────────────
    let mut content_items: Vec<(String, String, String, String)> = Vec::new(); // (title, slug, body, excerpt)

    if is_ai {
        let ai_client = match &state.ai_client {
            Some(c) if c.is_configured() => c.clone(),
            _ => {
                return Json(ApiResult {
                    ok: false,
                    message: "AI content generation is not configured.".into(),
                    data: None,
                })
            }
        };

        let pricing = {
            let j_pricing = state.journal.lock().await;
            luperiq_mod_content_sources::content_sources::pricing::load_pricing(&j_pricing)
        };
        total_credits = planned.len() as u32 * (pricing.credits_per_page + pricing.credits_per_seo);
        // Cap at the quoted amount from the UI (never charge more than quoted)
        if let Some(max) = req.max_credits {
            if max > 0 && max < total_credits {
                total_credits = max;
            }
        }

        // Reserve credits for client nodes
        if let Some(nexus) = &state.nexus_config {
            if nexus.role.as_deref() == Some("client") {
                let central_url = match nexus.central_url.as_deref() {
                    Some(u) => u,
                    None => {
                        return Json(ApiResult {
                            ok: false,
                            message: "No central_url configured for client node.".into(),
                            data: None,
                        })
                    }
                };
                let key = req
                    .license_key
                    .as_deref()
                    .or(nexus.license_key.as_deref())
                    .unwrap_or("");

                let client = match reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                {
                    Ok(c) => c,
                    Err(e) => {
                        return Json(ApiResult {
                            ok: false,
                            message: format!("HTTP client init failed: {e}"),
                            data: None,
                        });
                    }
                };

                let resp = client
                    .post(format!("{central_url}/api/modules/nexus/credits/deduct"))
                    .json(&serde_json::json!({
                        "license_key": key,
                        "operation": "ai_seo_page",
                        "amount": total_credits,
                        "module_key": "page-generator",
                    }))
                    .send()
                    .await;

                match resp {
                    Ok(r) => {
                        if let Ok(body) = r.json::<serde_json::Value>().await {
                            if !body["ok"].as_bool().unwrap_or(false) {
                                let msg = body["message"]
                                    .as_str()
                                    .unwrap_or("Credit deduction failed");
                                return Json(ApiResult {
                                    ok: false,
                                    message: format!("Credit check failed: {msg}"),
                                    data: None,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        return Json(ApiResult {
                            ok: false,
                            message: format!("Failed to contact Central: {e}"),
                            data: None,
                        });
                    }
                }
            }
        }

        // Generate AI content for each page (outside journal lock)
        for page in &planned {
            let system_prompt = templates::ai_system_prompt(&tpl_ctx);
            let user_msg = templates::ai_user_prompt(page, &req.brand, &req.state_abbr, &tpl_ctx);

            match ai_client.generate(system_prompt, &user_msg).await {
                Ok(resp) => {
                    ai_tokens_used += resp.input_tokens + resp.output_tokens;
                    let excerpt = truncate(&page.meta_description, 200);
                    let body = templates::normalize_ai_body(
                        &resp.content,
                        page,
                        &req.brand,
                        &req.phone,
                        &req.state_abbr,
                        &tpl_ctx,
                    );
                    content_items.push((page.title.clone(), page.slug.clone(), body, excerpt));
                }
                Err(e) => {
                    // Fall back to template
                    errors.push(format!(
                        "AI failed for '{}', using template: {}",
                        page.slug, e
                    ));
                    let body = templates::template_body(
                        page,
                        &req.brand,
                        &req.phone,
                        &req.state_abbr,
                        &tpl_ctx,
                    );
                    let excerpt = truncate(&page.meta_description, 200);
                    content_items.push((page.title.clone(), page.slug.clone(), body, excerpt));
                }
            }

            // 1-second delay between AI calls
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    } else {
        // Template mode: build content from templates
        for page in &planned {
            let body =
                templates::template_body(page, &req.brand, &req.phone, &req.state_abbr, &tpl_ctx);
            let excerpt = truncate(&page.meta_description, 200);
            content_items.push((page.title.clone(), page.slug.clone(), body, excerpt));
        }
    }

    // ── Lock journal and save everything ────────────────────────────
    let mut j = state.journal.lock().await;
    let mut pages_created: u32 = 0;
    let mut slug_to_id: HashMap<String, String> = HashMap::new();

    for (title, slug, body, excerpt) in &content_items {
        let content = ForgeContent {
            content_id: String::new(),
            content_type: "page".to_string(),
            title: title.clone(),
            slug: slug.clone(),
            body_json: body.clone(),
            excerpt: Some(excerpt.clone()),
            author_id: "system".to_string(),
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
            published_at: None,
        };

        let mut mgr = ForgeContentManager::new(&mut j);
        match mgr.create_content(&content) {
            Ok(content_id) => {
                slug_to_id.insert(slug.clone(), content_id.clone());
                // Publish immediately
                let mut mgr2 = ForgeContentManager::new(&mut j);
                if let Err(e) = mgr2.publish_content(&content_id) {
                    errors.push(format!("Failed to publish '{}': {}", slug, e));
                }
                pages_created += 1;
            }
            Err(e) => {
                errors.push(format!("Failed to create '{}': {}", slug, e));
            }
        }
    }

    // ── Create SEO meta for each page ───────────────────────────────
    let mut seo_created: u32 = 0;

    let seo_brand_fallback = config.industry_name.clone();
    for page in &planned {
        let content_id = match slug_to_id.get(&page.slug) {
            Some(id) => id.clone(),
            None => continue,
        };

        let meta = SeoMeta {
            content_id: content_id.clone(),
            title: truncate(
                &format!(
                    "{} | {}",
                    page.title,
                    if req.brand.is_empty() {
                        &seo_brand_fallback
                    } else {
                        &req.brand
                    }
                ),
                60,
            ),
            description: page.meta_description.clone(),
            focus_keyword: page.focus_keyword.clone(),
        };

        let payload_bytes = match serde_json::to_vec(&meta) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "SEO serialization error for '{}': {}",
                    page.slug, e
                ));
                continue;
            }
        };

        let event = ApexEvent::new(AGG_SEO_META, &content_id, payload_bytes);
        match j.append(event) {
            Ok(_) => seo_created += 1,
            Err(e) => {
                errors.push(format!(
                    "Failed to create SEO meta for '{}': {}",
                    page.slug, e
                ));
            }
        }
    }

    // ── Record the batch ────────────────────────────────────────────
    let batch_id = new_id();
    let batch = BatchRecord {
        batch_id: batch_id.clone(),
        pages_created,
        mode: req.mode.clone(),
        item_count: selected.len() as u32,
        city_count: req.cities.len() as u32,
        page_types: req.page_types.clone(),
        ai_tokens_used,
        ai_credits_charged: if is_ai { total_credits } else { 0 },
        errors: errors.clone(),
        created_at: now,
        industry: provider.industry_slug().to_string(),
    };

    if let Ok(bytes) = serde_json::to_vec(&batch) {
        let event = ApexEvent::new(AGG_PAGE_GEN_BATCH, &batch_id, bytes);
        if let Err(e) = j.append(event) {
            errors.push(format!("Failed to save batch record: {}", e));
        }
    }

    let message = if errors.is_empty() {
        format!(
            "{} pages created with {} SEO entries ({} mode, {})",
            pages_created, seo_created, req.mode, config.industry_name
        )
    } else {
        format!(
            "{} pages created, {} errors ({} mode, {})",
            pages_created,
            errors.len(),
            req.mode,
            config.industry_name
        )
    };

    // Build sharing offer if customer sources were used
    let customer_source_ids: Vec<String> = customer_sources
        .values()
        .flat_map(|sources| sources.iter().map(|s| s.source_id.clone()))
        .collect();

    let mut response_data = serde_json::json!({
        "batch_id": batch_id,
        "pages_created": pages_created,
        "seo_entries_created": seo_created,
        "total_planned": planned.len(),
        "mode": req.mode,
        "ai_tokens_used": ai_tokens_used,
        "ai_credits_charged": if is_ai { total_credits } else { 0 },
        "errors": errors,
        "industry": provider.industry_slug(),
    });

    if !customer_source_ids.is_empty() && is_ai {
        let pricing = {
            let j_pricing = state.journal.lock().await;
            luperiq_mod_content_sources::content_sources::pricing::load_pricing(&j_pricing)
        };
        response_data["sharing_offer"] = serde_json::json!({
            "eligible": true,
            "source_ids": customer_source_ids,
            "refund_trusted_source_pct": pricing.refund_trusted_source_pct,
            "refund_anonymized_pct": pricing.refund_anonymized_pct,
            "credits_used": total_credits,
        });
    }

    Json(ApiResult {
        ok: true,
        message,
        data: Some(response_data),
    })
}

/// GET /api/modules/page-generator/batches
///
/// Lists previous batch generation records.
async fn list_batches(State(state): State<PageGenState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_PAGE_GEN_BATCH);

    let mut batches: Vec<serde_json::Value> = events
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| {
            let b: BatchRecord = serde_json::from_slice(&e.payload).ok()?;
            Some(serde_json::json!({
                "batch_id": b.batch_id,
                "pages_created": b.pages_created,
                "mode": b.mode,
                "item_count": b.item_count,
                "city_count": b.city_count,
                "ai_tokens_used": b.ai_tokens_used,
                "ai_credits_charged": b.ai_credits_charged,
                "error_count": b.errors.len(),
                "created_at": b.created_at,
                "industry": b.industry,
            }))
        })
        .collect();

    batches.sort_by(|a, b| {
        let ta = a["created_at"].as_u64().unwrap_or(0);
        let tb = b["created_at"].as_u64().unwrap_or(0);
        tb.cmp(&ta)
    });

    Json(ApiResult {
        ok: true,
        message: format!("{} batches", batches.len()),
        data: Some(serde_json::json!(batches)),
    })
}

/// GET /api/modules/page-generator/ai/status
async fn ai_status(State(state): State<PageGenState>) -> Json<ApiResult> {
    match &state.ai_client {
        Some(client) if client.is_configured() => Json(ApiResult {
            ok: true,
            message: "AI content generation available".into(),
            data: Some(client.status()),
        }),
        _ => Json(ApiResult {
            ok: false,
            message: "AI not configured.".into(),
            data: None,
        }),
    }
}

// ── Admin CSS ───────────────────────────────────────────────────────

const ADMIN_CSS: &str = r##"
/* ── SEO Page Generator Module ───────────────────────────────────── */
.pg-section { margin-bottom: 24px; }
.pg-section h3 { font-size: 16px; font-weight: 600; margin-bottom: 8px; }
.pg-industry-select {
    display: flex; align-items: center; gap: 12px;
    margin-bottom: 16px; padding: 12px 16px;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 8px;
}
.pg-industry-select select {
    padding: 6px 12px; border: 1px solid var(--border);
    border-radius: 6px; font-size: 14px;
    background: var(--surface); color: var(--text);
}
.pg-pest-grid, .pg-item-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 8px;
    margin-bottom: 16px;
}
.pg-pest-item, .pg-item-entry {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    transition: border-color 0.2s, background 0.2s;
    font-size: 14px;
}
.pg-pest-item:hover, .pg-item-entry:hover { border-color: var(--accent); }
.pg-pest-item.selected, .pg-item-entry.selected { border-color: var(--accent); background: rgba(59,130,246,0.08); }
.pg-pest-item input[type="checkbox"], .pg-item-entry input[type="checkbox"] { margin: 0; }
.pg-pest-category, .pg-item-category {
    font-size: 11px;
    color: var(--text-muted);
    padding: 2px 6px;
    background: var(--surface);
    border-radius: 4px;
    margin-left: auto;
}
.pg-cities-input {
    width: 100%;
    padding: 10px 14px;
    border: 1px solid var(--border);
    border-radius: 8px;
    font-size: 14px;
    background: var(--surface);
    color: var(--text);
    box-sizing: border-box;
}
.pg-cities-input:focus { outline: none; border-color: var(--accent); }
.pg-cities-help { font-size: 12px; color: var(--text-muted); margin-top: 4px; }
.pg-options { display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 16px; }
.pg-option-chip {
    display: flex; align-items: center; gap: 6px;
    padding: 6px 14px; border: 1px solid var(--border);
    border-radius: 20px; cursor: pointer; font-size: 13px;
    transition: border-color 0.2s, background 0.2s;
    user-select: none;
}
.pg-option-chip:hover { border-color: var(--accent); }
.pg-option-chip.selected { border-color: var(--accent); background: rgba(59,130,246,0.08); }
.pg-option-chip input { margin: 0; }
.pg-preview-box {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 16px;
    max-height: 400px;
    overflow-y: auto;
}
.pg-preview-summary {
    display: flex; gap: 16px; flex-wrap: wrap;
    padding: 12px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 16px;
}
.pg-preview-stat { text-align: center; }
.pg-preview-stat .num { font-size: 28px; font-weight: 700; color: var(--accent); }
.pg-preview-stat .lbl { font-size: 12px; color: var(--text-muted); }
.pg-page-row {
    display: flex; align-items: center; gap: 8px;
    padding: 6px 0; border-bottom: 1px solid var(--border);
    font-size: 13px;
}
.pg-page-row:last-child { border-bottom: none; }
.pg-page-kind {
    font-size: 10px; font-weight: 600;
    padding: 2px 6px; border-radius: 4px;
    text-transform: uppercase; white-space: nowrap;
}
.pg-kind-item_hub { background: #dbeafe; color: #1e40af; }
.pg-kind-city_hub { background: #dcfce7; color: #166534; }
.pg-kind-item_city { background: #fef3c7; color: #92400e; }
.pg-kind-category_hub { background: #f3e8ff; color: #6b21a8; }
.pg-kind-category_city { background: #ffe4e6; color: #9f1239; }
.pg-page-slug { color: var(--text-muted); font-family: monospace; font-size: 12px; }
.pg-ai-toggle {
    display: flex; align-items: center; gap: 12px;
    padding: 16px; background: var(--surface);
    border: 1px solid var(--border); border-radius: 8px;
    margin-bottom: 16px;
}
.pg-ai-cost {
    background: #fef3c7; border: 1px solid #f59e0b;
    border-radius: 8px; padding: 12px 16px;
    font-size: 13px; color: #92400e; margin-bottom: 16px;
}
.pg-confirm-input {
    padding: 8px 12px; border: 1px solid var(--border);
    border-radius: 6px; font-size: 14px; width: 200px;
    background: var(--surface); color: var(--text);
}
.pg-confirm-input:focus { outline: none; border-color: var(--accent); }
.pg-result {
    text-align: center; padding: 32px;
}
.pg-result-icon { font-size: 64px; margin-bottom: 16px; }
.pg-custom-pest-row, .pg-custom-item-row {
    display: flex; gap: 8px; align-items: center; margin-bottom: 8px;
}
.pg-custom-pest-row input, .pg-custom-item-row input {
    padding: 6px 10px; border: 1px solid var(--border);
    border-radius: 6px; font-size: 13px;
    background: var(--surface); color: var(--text);
}
.pg-custom-pest-row input:focus, .pg-custom-item-row input:focus { outline: none; border-color: var(--accent); }
.pg-brand-row {
    display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 16px;
}
.pg-brand-row input {
    flex: 1; min-width: 150px;
    padding: 8px 12px; border: 1px solid var(--border);
    border-radius: 6px; font-size: 14px;
    background: var(--surface); color: var(--text); box-sizing: border-box;
}
.pg-brand-row input:focus { outline: none; border-color: var(--accent); }
.pg-batch-list { margin-top: 24px; }
.pg-batch-item {
    display: flex; justify-content: space-between; align-items: center;
    padding: 10px 14px; border: 1px solid var(--border);
    border-radius: 8px; margin-bottom: 6px; font-size: 13px;
}
.pg-batch-mode {
    font-size: 11px; font-weight: 600; padding: 2px 8px;
    border-radius: 10px;
}
.pg-batch-mode.template { background: #dbeafe; color: #1e40af; }
.pg-batch-mode.ai { background: #fef3c7; color: #92400e; }
"##;
