//! LocationProfile aggregate — data sheets per service area.
//!
//! Stores demographics, climate, competitors, local keywords, regulations,
//! and area descriptions for each location a business serves.
//!
//! Each profile is keyed by a URL-safe slug (e.g. "austin-tx", "miami-fl")
//! and persisted via the ForgeJournal event-sourcing pattern.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for location profiles in the ForgeJournal.
///
/// **Cross-crate callers must import this const** rather than copy the string literal.
/// Callers: `luperiq-mod-seo`, `luperiq-cms` (site_pages/seo.rs and site_blueprint/rollback.rs),
/// `luperiq-mod-site-blueprint`. If renamed here, all callers will get a compile error
/// rather than a silent runtime mismatch.
pub const AGG_LOCATION: &str = "LocProf:Profile";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Primary aggregate ────────────────────────────────────────────────

/// A comprehensive location data sheet that drives geo-targeted content,
/// local SEO optimization, competitor analysis, and service area planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationProfile {
    pub id: String,
    /// URL-safe slug, e.g. "austin-tx", "miami-fl".
    pub slug: String,
    /// City name.
    pub city: String,
    /// State abbreviation or full name.
    pub state: String,
    /// County name (optional).
    pub county: Option<String>,
    /// ZIP codes served in this location.
    pub zip_codes: Vec<String>,
    /// Metropolitan statistical area name (optional).
    pub metro_area: Option<String>,
    /// Total population.
    pub population: Option<u64>,
    /// Median household income in dollars.
    pub median_income: Option<u64>,
    /// Total housing units.
    pub housing_units: Option<u64>,
    /// Percentage of owner-occupied housing (0.0 - 100.0).
    pub owner_occupied_pct: Option<f64>,
    /// Median age of homes in years.
    pub median_home_age: Option<u32>,
    /// Climate zone classification (e.g. "4A - Mixed Humid").
    pub climate_zone: Option<String>,
    /// Seasonal weather patterns for content context.
    pub weather_patterns: Vec<WeatherPattern>,
    /// Geo-targeted keywords for local SEO.
    pub local_keywords: Vec<LocalKeyword>,
    /// Known competitors in this service area.
    pub local_competitors: Vec<LocalCompetitor>,
    /// Local regulations and licensing requirements.
    pub local_regulations: Vec<LocalRegulation>,
    /// Cost of living index relative to national average (100.0 = average).
    pub cost_of_living_index: Option<f64>,
    /// 2-3 paragraph description for content generation context.
    pub area_description: String,
    /// Notable neighborhoods or sub-areas.
    pub neighborhoods: Vec<String>,
    /// Whether the location is active and usable.
    pub active: bool,
}

// ── Sub-types ────────────────────────────────────────────────────────

/// Seasonal weather pattern for a location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherPattern {
    /// Season: "summer", "winter", "spring", "fall".
    pub season: String,
    /// Average daily high temperature in Fahrenheit.
    pub avg_high_f: Option<i32>,
    /// Average daily low temperature in Fahrenheit.
    pub avg_low_f: Option<i32>,
    /// Description of typical weather for this season.
    pub description: String,
}

/// A geo-modified keyword for local SEO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalKeyword {
    /// The keyword phrase, e.g. "hvac repair {city}".
    pub keyword: String,
    /// Estimated monthly search volume (if known).
    pub search_volume: Option<u32>,
    /// Geo modifier template: "{city}", "{county}", "{metro}".
    pub geo_modifier: String,
}

/// A local competitor in this service area.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalCompetitor {
    /// Business name.
    pub name: String,
    /// Website URL (optional).
    pub website: Option<String>,
    /// Average rating (e.g. 4.5 out of 5.0).
    pub rating: Option<f64>,
    /// Number of reviews.
    pub review_count: Option<u32>,
    /// Areas of specialization.
    pub specialties: Vec<String>,
}

/// A local regulation or licensing requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRegulation {
    /// Regulation name.
    pub name: String,
    /// Description of the regulation.
    pub description: String,
    /// Authority level: "city", "county", "state".
    pub authority: String,
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load all non-deleted location profiles from the journal.
pub fn load_all_locations(j: &luperiq_forge::ForgeJournal) -> Vec<LocationProfile> {
    j.latest_by_aggregate_type(AGG_LOCATION)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<LocationProfile>(&e.payload).ok())
        .collect()
}

/// Load a single location profile by its slug.
///
/// Because profiles are stored with the slug as the aggregate_id,
/// this is a direct key lookup rather than a scan.
pub fn load_location_by_slug(
    j: &luperiq_forge::ForgeJournal,
    slug: &str,
) -> Option<LocationProfile> {
    j.get_latest(AGG_LOCATION, slug)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<LocationProfile>(&e.payload).ok())
}

/// Persist a location profile to the journal.
///
/// The profile's `slug` is used as the aggregate_id for direct lookups.
pub fn persist_location(
    j: &mut luperiq_forge::ForgeJournal,
    location: &LocationProfile,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(location).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_LOCATION, &location.slug, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete a location profile by slug.
pub fn delete_location(j: &mut luperiq_forge::ForgeJournal, slug: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_LOCATION, slug, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
