//! `viewer.rs` — Phase 3 (directory hardening): viewer-tier field masking.
//!
//! The public directory must render only non-harvestable fields to anonymous
//! visitors and bots, while a logged-in **owner of a claimed listing** sees the
//! full record. This module owns:
//!   * [`DirViewer`] — the request-scoped identity extension. It is constructed
//!     by `main.rs` (the only bridge to quiz2/forge auth) and injected as an
//!     axum `Extension`. This crate stays dependency-isolated: it never imports
//!     quiz2 or forge — it only reads the resolved `user_id`.
//!   * [`ViewerTier`] — `Public` | `Owner`, resolved per (viewer, company).
//!   * [`MaskedCompany`] — a 1:1 mirror of [`DirectoryCompany`] with the
//!     non-public fields nulled/reduced at `Public` tier. Because it is a real
//!     owned struct, the masked-out fields are physically absent from the
//!     serialized Tera/JSON output — the template cannot "leak" them back.
//!   * A 5-second TTL cache for owner resolution keyed on (user_id, company_id).
//!
//! Anonymous viewers (no `user_id`) short-circuit to `Public` with no DB lookup.

use crate::store::{DirectoryApplicator, DirectoryCompany, DirectoryOfficer, DirectoryStore};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Request-scoped viewer identity. Constructed by `main.rs` from the validated
/// `liq_session` JWT (same mechanism quiz2 uses) and injected as an axum
/// `Extension<DirViewer>`. `user_id == None` means anonymous (or bot).
#[derive(Debug, Clone, Default)]
pub struct DirViewer {
    pub user_id: Option<String>,
    /// The logged-in account email (from the validated session claims),
    /// resolved by `main.rs`. Used as the authoritative destination for the
    /// claim verification email. `None` when anonymous or unavailable.
    pub email: Option<String>,
}

impl DirViewer {
    pub fn anonymous() -> Self {
        Self { user_id: None, email: None }
    }
    pub fn for_user(user_id: Option<String>) -> Self {
        Self { user_id, email: None }
    }
    /// Construct from a resolved session: user_id plus the account email.
    pub fn for_user_with_email(user_id: Option<String>, email: Option<String>) -> Self {
        Self { user_id, email }
    }
}

/// The visibility tier applied to a company record for the current viewer.
/// `RevealUser` is intentionally NOT modeled here — per-field reveal is the
/// Phase 2 `POST /directory/reveal` endpoint (transient, per-click), not a
/// persistent viewer tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewerTier {
    Public,
    Owner,
}

// ── Owner-resolution TTL cache ───────────────────────────────────────────────
//
// Authenticated requests would otherwise hit `is_verified_owner` (a SELECT) on
// every render. A tiny 5s TTL cache keyed on (user_id, company_id) collapses
// repeated lookups for the same authed viewer. Anonymous viewers never reach
// the cache (they short-circuit to Public above). No new dependency — a
// Mutex<HashMap> with Instant timestamps.

const OWNER_TTL: Duration = Duration::from_secs(5);

static OWNER_CACHE: Mutex<Option<HashMap<(String, String), (Instant, bool)>>> = Mutex::new(None);

fn cached_is_owner(user_id: &str, company_id: &str) -> Option<bool> {
    let guard = OWNER_CACHE.lock().ok()?;
    let map = guard.as_ref()?;
    let (ts, val) = map.get(&(user_id.to_string(), company_id.to_string()))?;
    if ts.elapsed() < OWNER_TTL {
        Some(*val)
    } else {
        None
    }
}

/// Drop the entire owner-resolution cache. Called right after a claim is
/// verified so the next render for that member resolves the freshly-verified
/// Owner tier immediately instead of waiting out the 5s TTL on a stale `false`.
pub fn invalidate_owner_cache() {
    if let Ok(mut guard) = OWNER_CACHE.lock() {
        *guard = None;
    }
}

fn store_is_owner(user_id: &str, company_id: &str, val: bool) {
    if let Ok(mut guard) = OWNER_CACHE.lock() {
        let map = guard.get_or_insert_with(HashMap::new);
        // Opportunistic eviction so the map cannot grow without bound under churn.
        if map.len() > 4096 {
            map.retain(|_, (ts, _)| ts.elapsed() < OWNER_TTL);
        }
        map.insert(
            (user_id.to_string(), company_id.to_string()),
            (Instant::now(), val),
        );
    }
}

/// Resolve the [`ViewerTier`] for a viewer against one company.
///
/// `Owner` iff the viewer has a `user_id` AND `store.is_verified_owner(company,
/// user)` is true. Anonymous viewers (and any non-owner) get `Public`. Until
/// Phase 4 writes `directory_claims`, no row is ever a verified owner, so this
/// returns `Public` for everyone — which is the intended pre-Phase-4 behavior.
pub fn resolve_tier(viewer: &DirViewer, store: &DirectoryStore, company_id: &str) -> ViewerTier {
    let Some(uid) = viewer.user_id.as_deref() else {
        // Anonymous: no lookup, no cache.
        return ViewerTier::Public;
    };
    if let Some(is_owner) = cached_is_owner(uid, company_id) {
        return if is_owner { ViewerTier::Owner } else { ViewerTier::Public };
    }
    let is_owner = store.is_verified_owner(company_id, uid);
    store_is_owner(uid, company_id, is_owner);
    if is_owner {
        ViewerTier::Owner
    } else {
        ViewerTier::Public
    }
}

// ── MaskedCompany ────────────────────────────────────────────────────────────
//
// A real owned mirror of DirectoryCompany. At Public tier the harvestable
// fields are physically removed (set to None / reduced), so the serialized
// output handed to Tera and to search.json cannot contain them. At Owner tier
// it is a faithful copy of the source record.
//
// `staff_preview` / `contact_hidden` / `address_hidden` / `officer_first_only`
// are Phase-3 presentation flags consumed by the template; they have no
// analogue on the source struct.

#[derive(Debug, Clone, Serialize)]
pub struct MaskedCompany {
    pub id: String,
    pub state_abbr: String,
    pub state_name: String,
    // Location — city/state/ZIP kept (SEO); street address hidden at Public.
    pub city: Option<String>,
    pub city_slug: Option<String>,
    pub is_county_location: bool,
    pub county: Option<String>,
    // Names — kept (SEO).
    pub entity_name: String,
    pub dba: Option<String>,
    pub company_slug: String,
    // Contact — hidden at Public (revealed via /directory/reveal).
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    // Address — street hidden at Public.
    pub address: Option<String>,
    // SOS data — registered-agent / file-number / agent address hidden at Public.
    pub entity_type: Option<String>,
    pub formation_date: Option<String>,
    pub status: Option<String>,
    pub expiration_date: Option<String>,
    pub file_number: Option<String>,
    pub registered_agent: Option<String>,
    pub agent_address: Option<String>,
    // Pest license data — public record, kept.
    pub pest_license_num: Option<String>,
    pub pest_license_type: Option<String>,
    pub pest_categories: Option<String>,
    pub pest_categories_decoded: Option<String>,
    pub pest_license_expires: Option<String>,
    pub pest_operator: Option<String>,
    pub pest_source_url: Option<String>,
    // Staff counts — applicator COUNT kept (SEO); full lists become a preview.
    pub applicator_count: i64,
    pub technician_count: i64,
    pub apprentice_count: i64,
    // Source links.
    pub source: String,
    pub source_url: Option<String>,
    pub sos_lookup_url: Option<String>,
    pub pest_lookup_url: Option<String>,
    // Enrichment / claiming.
    pub listing_tier: i64,
    pub claimed_by: Option<String>,
    pub rating_count: i64,
    pub rating_sum: i64,
    // Computed fields.
    pub is_active: bool,
    pub has_pest_license: bool,
    pub avg_rating: Option<f64>,
    pub formation_year: Option<String>,
    pub state_regs_url: Option<String>,
    pub staff_summary: Option<String>,
    pub pest_license_expires_display: Option<String>,
    pub formation_date_display: Option<String>,
    pub expiration_date_display: Option<String>,
    pub pest_license_issued: Option<String>,
    pub pest_license_issued_display: Option<String>,
    pub pest_license_renewed: Option<String>,
    pub pest_license_renewed_display: Option<String>,
    pub pest_insurance_expires: Option<String>,
    pub pest_insurance_expires_display: Option<String>,
    pub pest_responsible_applicator: Option<String>,
    pub pest_responsible_applicator_license: Option<String>,
    pub pest_spcb_id: Option<String>,
    pub is_canary: bool,
    // ── Phase 3 presentation flags (no source analogue) ──
    /// Phone/email/website were withheld for this viewer (reveal control shown).
    pub contact_hidden: bool,
    /// Street address withheld; city/state/ZIP still shown.
    pub address_hidden: bool,
    /// Officer/operator names reduced to first name only.
    pub officer_first_only: bool,
    /// Positive-framed staff teaser shown instead of full staff lists.
    pub staff_preview: Option<String>,
}

impl MaskedCompany {
    fn from_full(c: &DirectoryCompany) -> Self {
        // Owner tier: faithful copy. Public masking is layered on afterwards.
        MaskedCompany {
            id: c.id.clone(),
            state_abbr: c.state_abbr.clone(),
            state_name: c.state_name.clone(),
            city: c.city.clone(),
            city_slug: c.city_slug.clone(),
            is_county_location: c.is_county_location,
            county: c.county.clone(),
            entity_name: c.entity_name.clone(),
            dba: c.dba.clone(),
            company_slug: c.company_slug.clone(),
            phone: c.phone.clone(),
            email: c.email.clone(),
            website: c.website.clone(),
            address: c.address.clone(),
            entity_type: c.entity_type.clone(),
            formation_date: c.formation_date.clone(),
            status: c.status.clone(),
            expiration_date: c.expiration_date.clone(),
            file_number: c.file_number.clone(),
            registered_agent: c.registered_agent.clone(),
            agent_address: c.agent_address.clone(),
            pest_license_num: c.pest_license_num.clone(),
            pest_license_type: c.pest_license_type.clone(),
            pest_categories: c.pest_categories.clone(),
            pest_categories_decoded: c.pest_categories_decoded.clone(),
            pest_license_expires: c.pest_license_expires.clone(),
            pest_operator: c.pest_operator.clone(),
            pest_source_url: c.pest_source_url.clone(),
            applicator_count: c.applicator_count,
            technician_count: c.technician_count,
            apprentice_count: c.apprentice_count,
            source: c.source.clone(),
            source_url: c.source_url.clone(),
            sos_lookup_url: c.sos_lookup_url.clone(),
            pest_lookup_url: c.pest_lookup_url.clone(),
            listing_tier: c.listing_tier,
            claimed_by: c.claimed_by.clone(),
            rating_count: c.rating_count,
            rating_sum: c.rating_sum,
            is_active: c.is_active,
            has_pest_license: c.has_pest_license,
            avg_rating: c.avg_rating,
            formation_year: c.formation_year.clone(),
            state_regs_url: c.state_regs_url.clone(),
            staff_summary: c.staff_summary.clone(),
            pest_license_expires_display: c.pest_license_expires_display.clone(),
            formation_date_display: c.formation_date_display.clone(),
            expiration_date_display: c.expiration_date_display.clone(),
            pest_license_issued: c.pest_license_issued.clone(),
            pest_license_issued_display: c.pest_license_issued_display.clone(),
            pest_license_renewed: c.pest_license_renewed.clone(),
            pest_license_renewed_display: c.pest_license_renewed_display.clone(),
            pest_insurance_expires: c.pest_insurance_expires.clone(),
            pest_insurance_expires_display: c.pest_insurance_expires_display.clone(),
            pest_responsible_applicator: c.pest_responsible_applicator.clone(),
            pest_responsible_applicator_license: c.pest_responsible_applicator_license.clone(),
            pest_spcb_id: c.pest_spcb_id.clone(),
            is_canary: c.is_canary,
            contact_hidden: false,
            address_hidden: false,
            officer_first_only: false,
            staff_preview: None,
        }
    }
}

/// Reduce a person name to its first token only (e.g. "Jane A. Smith" → "Jane").
/// Used to soften owner/operator/registered-agent exposure at Public tier.
fn first_name_only(name: &str) -> String {
    name.split_whitespace().next().unwrap_or("").to_string()
}

/// Build a positive-framed staff teaser from the head-count, e.g.
/// "We hold 4 team members — claim this listing to manage". Returns None when
/// the company lists no staff at all (nothing to tease).
fn staff_preview_for(c: &DirectoryCompany) -> Option<String> {
    let n = c.applicator_count + c.technician_count + c.apprentice_count;
    if n <= 0 {
        return None;
    }
    Some(format!(
        "We hold {n} team member{} — claim this listing to manage",
        if n == 1 { "" } else { "s" }
    ))
}

/// Apply the [`ViewerTier`] visibility policy to a company.
///
/// * `Owner`  → faithful copy, all flags false / no preview.
/// * `Public` → harvestable fields nulled or reduced:
///   - HIDE: phone, email, website (→ None, `contact_hidden`), street address
///     (→ None, `address_hidden`, city/state/ZIP retained), file number,
///     registered agent + agent address.
///   - REDUCE: pest operator + responsible applicator + registered agent to
///     first name only (`officer_first_only`); formation_date → year only.
///   - STAFF: full staff lists are replaced by `staff_preview` (the handler
///     suppresses the officer/applicator vecs accordingly); applicator COUNT
///     is retained for SEO.
///   - KEEP (SEO): entity_name/dba, category, city/state/ZIP, license
///     type/number, applicator count.
pub fn apply_tier_mask(co: &DirectoryCompany, tier: ViewerTier) -> MaskedCompany {
    let mut m = MaskedCompany::from_full(co);
    if tier == ViewerTier::Owner {
        return m;
    }

    // ── Public tier ──
    // HIDE contact — physically removed; reveal endpoint serves them per-click.
    let had_contact = m.phone.is_some() || m.email.is_some() || m.website.is_some();
    m.phone = None;
    m.email = None;
    m.website = None;
    m.contact_hidden = had_contact;

    // HIDE street address; keep city/state/ZIP (already separate columns).
    let had_address = m.address.is_some();
    m.address = None;
    m.address_hidden = had_address;

    // HIDE registered-agent / file-number detail (harvestable PII / lead data).
    m.file_number = None;
    m.agent_address = None;

    // REDUCE owner/officer/agent names to first name only.
    m.officer_first_only = true;
    m.pest_operator = m.pest_operator.as_deref().map(first_name_only);
    m.registered_agent = m.registered_agent.as_deref().map(first_name_only);
    m.pest_responsible_applicator =
        m.pest_responsible_applicator.as_deref().map(first_name_only);
    // The responsible-applicator license number is a harvestable identifier.
    m.pest_responsible_applicator_license = None;

    // REDUCE formation_date to year only (keep the SEO-relevant "Est. YYYY").
    m.formation_date = m.formation_year.clone();
    m.formation_date_display = m.formation_year.clone();

    // STAFF: replace the full lists with a positive-framed preview. Applicator
    // COUNT is retained on the struct for the SEO badge.
    m.staff_preview = staff_preview_for(co);

    m
}

/// Apply the tier policy to the officer list. `Public` tier yields an empty vec
/// (the template shows `staff_preview` instead); `Owner` tier returns the full
/// list unchanged.
pub fn mask_officers(officers: Vec<DirectoryOfficer>, tier: ViewerTier) -> Vec<DirectoryOfficer> {
    match tier {
        ViewerTier::Owner => officers,
        ViewerTier::Public => Vec::new(),
    }
}

/// Apply the tier policy to the applicator list. `Public` tier yields an empty
/// vec (the template shows the count + `staff_preview`); `Owner` returns full.
pub fn mask_applicators(
    applicators: Vec<DirectoryApplicator>,
    tier: ViewerTier,
) -> Vec<DirectoryApplicator> {
    match tier {
        ViewerTier::Owner => applicators,
        ViewerTier::Public => Vec::new(),
    }
}
