//! Location Profile Module — data sheets per service area.
//!
//! Provides:
//! - **LocationProfile**: Rich aggregate with demographics, climate, competitors,
//!   local keywords, regulations, and area descriptions.
//! - **Census import**: Merge census/demographic data into existing profiles.
//! - **Competitor import**: Replace competitor lists from JSON arrays.
//!
//! Admin views: Location Profiles (Data section)
//!
//! Security notes:
//! - Admin UI uses DOM methods (createElement/textContent) for XSS safety
//! - All write endpoints are admin-authenticated via middleware in main.rs

pub mod admin_js;
pub mod census;
pub mod competitors;
pub mod profile;

use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

use luperiq_module_api::{
    AdminView, AiFeatureConfig, AiFeatureRegistry, AppContext, CmsModule, SharedJournal,
};

// ── Module definition ─────────────────────────────────────────────────

pub struct LocationProfileModule;

impl CmsModule for LocationProfileModule {
    fn slug(&self) -> &str {
        "location-profile"
    }
    fn name(&self) -> &str {
        "Location Profiles"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Data sheets per service area with demographics, competitors, local keywords, and regulations."
    }
    fn category(&self) -> &str {
        "Data"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(location_profile_router(
            ctx.journal.clone(),
            ctx.ai_features.clone(),
        ))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![AdminView {
            id: "location-profiles".into(),
            label: "Location Profiles".into(),
            section: "Data".into(),
        }]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::LOCATION_PROFILES_ADMIN_JS.to_string())
    }
}

// ── API result type ───────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

// ── Request types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct CreateLocationPayload {
    city: String,
    state: String,
    slug: String,
    county: Option<String>,
    #[serde(default)]
    zip_codes: Vec<String>,
    metro_area: Option<String>,
    population: Option<u64>,
    median_income: Option<u64>,
    housing_units: Option<u64>,
    owner_occupied_pct: Option<f64>,
    median_home_age: Option<u32>,
    climate_zone: Option<String>,
    #[serde(default)]
    weather_patterns: Vec<profile::WeatherPattern>,
    #[serde(default)]
    local_keywords: Vec<profile::LocalKeyword>,
    #[serde(default)]
    local_competitors: Vec<profile::LocalCompetitor>,
    #[serde(default)]
    local_regulations: Vec<profile::LocalRegulation>,
    cost_of_living_index: Option<f64>,
    #[serde(default)]
    area_description: String,
    #[serde(default)]
    neighborhoods: Vec<String>,
    #[serde(default = "default_true")]
    active: bool,
}

/// Payload for updating an existing location profile.
///
/// # Two-level optionality (`Option<Option<T>>`)
///
/// Fields typed `Option<Option<T>>` distinguish three intents:
/// - `None` (field absent from JSON) → **leave field unchanged**.
/// - `Some(Some(v))` → **set field to `v`**.
/// - `Some(None)` → **explicitly clear the field** (set it to `null`/`None`).
///
/// Collapsing these to `Option<T>` would break the "clear field" path; the
/// two-level form is intentional and must be preserved.
#[derive(Deserialize)]
struct UpdateLocationPayload {
    city: Option<String>,
    state: Option<String>,
    slug: Option<String>,
    county: Option<Option<String>>,
    zip_codes: Option<Vec<String>>,
    metro_area: Option<Option<String>>,
    population: Option<Option<u64>>,
    median_income: Option<Option<u64>>,
    housing_units: Option<Option<u64>>,
    owner_occupied_pct: Option<Option<f64>>,
    median_home_age: Option<Option<u32>>,
    climate_zone: Option<Option<String>>,
    weather_patterns: Option<Vec<profile::WeatherPattern>>,
    local_keywords: Option<Vec<profile::LocalKeyword>>,
    local_competitors: Option<Vec<profile::LocalCompetitor>>,
    local_regulations: Option<Vec<profile::LocalRegulation>>,
    cost_of_living_index: Option<Option<f64>>,
    area_description: Option<String>,
    neighborhoods: Option<Vec<String>>,
    active: Option<bool>,
}

fn default_true() -> bool {
    true
}

// ── Router ────────────────────────────────────────────────────────────

fn location_profile_router(journal: SharedJournal, ai_features: AiFeatureRegistry) -> Router {
    // Register AI features
    {
        let features = ai_features.clone();
        tokio::task::spawn(async move {
            let mut reg = features.lock().await;
            reg.insert("location_ai_description".into(), AiFeatureConfig {
                system_prompt: "You are a local SEO expert and copywriter. Based on the provided location details, write a compelling area description that incorporates local keywords and highlights why this is a great service area. Include mentions of neighborhoods, demographics, and local factors. Keep it 2-3 paragraphs. Return just the text.".to_string(),
                max_input_len: 4000,
                credit_cost: 2,
                escalation_credit_cost: 1,
                result_parser: |s| Ok(serde_json::Value::String(s.trim().to_string())),
            });
        });
    }

    Router::new()
        .route(
            "/api/modules/location-profile/locations",
            get(list_locations).post(create_location),
        )
        .route(
            "/api/modules/location-profile/locations/{slug}",
            get(get_location)
                .put(update_location)
                .delete(delete_location),
        )
        .route(
            "/api/modules/location-profile/locations/{slug}/import-census",
            post(import_census),
        )
        .route(
            "/api/modules/location-profile/locations/{slug}/import-competitors",
            post(import_competitors),
        )
        .with_state(journal)
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/modules/location-profile/locations — list all location profiles.
async fn list_locations(State(journal): State<SharedJournal>) -> Json<ApiResult> {
    let j = journal.lock().await;
    let mut locations = profile::load_all_locations(&j);
    locations.sort_by(|a, b| {
        a.city
            .to_lowercase()
            .cmp(&b.city.to_lowercase())
            .then_with(|| a.state.to_lowercase().cmp(&b.state.to_lowercase()))
    });

    let items: Vec<serde_json::Value> = locations
        .iter()
        .map(|l| serde_json::to_value(l).unwrap_or_default())
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} location(s)", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

/// GET /api/modules/location-profile/locations/{slug} — get a single location profile.
async fn get_location(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
) -> Json<ApiResult> {
    let j = journal.lock().await;
    match profile::load_location_by_slug(&j, &slug) {
        Some(l) => match serde_json::to_value(&l) {
            Ok(v) => Json(ApiResult {
                ok: true,
                message: "Location found".into(),
                data: Some(v),
            }),
            Err(e) => Json(ApiResult {
                ok: false,
                message: format!("Location '{}' data is corrupt and cannot be serialized: {e}", slug),
                data: None,
            }),
        },
        None => Json(ApiResult {
            ok: false,
            message: format!("Location '{}' not found", slug),
            data: None,
        }),
    }
}

/// POST /api/modules/location-profile/locations — create a new location profile.
async fn create_location(
    State(journal): State<SharedJournal>,
    axum::extract::Json(payload): axum::extract::Json<CreateLocationPayload>,
) -> Json<ApiResult> {
    let city = payload.city.trim().to_string();
    if city.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "City is required".into(),
            data: None,
        });
    }

    let state = payload.state.trim().to_string();
    if state.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "State is required".into(),
            data: None,
        });
    }

    let slug = payload.slug.trim().to_lowercase();
    if slug.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Slug is required".into(),
            data: None,
        });
    }

    // Validate slug format (alphanumeric + hyphens only)
    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Json(ApiResult {
            ok: false,
            message: "Slug must contain only letters, numbers, and hyphens".into(),
            data: None,
        });
    }

    // Check for existing location with same slug
    {
        let j = journal.lock().await;
        if profile::load_location_by_slug(&j, &slug).is_some() {
            return Json(ApiResult {
                ok: false,
                message: format!("Location with slug '{}' already exists", slug),
                data: None,
            });
        }
    }

    let id = ulid::Ulid::new().to_string();
    let loc = profile::LocationProfile {
        id: id.clone(),
        slug: slug.clone(),
        city,
        state,
        county: payload.county,
        zip_codes: payload.zip_codes,
        metro_area: payload.metro_area,
        population: payload.population,
        median_income: payload.median_income,
        housing_units: payload.housing_units,
        owner_occupied_pct: payload.owner_occupied_pct,
        median_home_age: payload.median_home_age,
        climate_zone: payload.climate_zone,
        weather_patterns: payload.weather_patterns,
        local_keywords: payload.local_keywords,
        local_competitors: payload.local_competitors,
        local_regulations: payload.local_regulations,
        cost_of_living_index: payload.cost_of_living_index,
        area_description: payload.area_description,
        neighborhoods: payload.neighborhoods,
        active: payload.active,
    };

    let mut j = journal.lock().await;
    if let Err(e) = profile::persist_location(&mut j, &loc) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to create location: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Created location '{}, {}'", loc.city, loc.state),
        data: Some(serde_json::json!({ "id": id, "slug": slug })),
    })
}

/// PUT /api/modules/location-profile/locations/{slug} — update an existing location.
async fn update_location(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
    axum::extract::Json(payload): axum::extract::Json<UpdateLocationPayload>,
) -> Json<ApiResult> {
    let mut j = journal.lock().await;

    let mut loc = match profile::load_location_by_slug(&j, &slug) {
        Some(l) => l,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Location '{}' not found", slug),
                data: None,
            });
        }
    };

    if let Some(city) = &payload.city {
        loc.city = city.trim().to_string();
    }
    if let Some(state) = &payload.state {
        loc.state = state.trim().to_string();
    }
    if let Some(new_slug) = &payload.slug {
        let ns = new_slug.trim().to_lowercase();
        if !ns.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Json(ApiResult {
                ok: false,
                message: "Slug must contain only letters, numbers, and hyphens".into(),
                data: None,
            });
        }
        // If slug is changing, tombstone old and use new slug
        if ns != slug {
            if profile::load_location_by_slug(&j, &ns).is_some() {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Location with slug '{}' already exists", ns),
                    data: None,
                });
            }
            // Tombstone old slug
            if let Err(e) = profile::delete_location(&mut j, &slug) {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Failed to remove old slug: {e}"),
                    data: None,
                });
            }
            loc.slug = ns;
        }
    }
    if let Some(v) = payload.county {
        loc.county = v;
    }
    if let Some(v) = payload.zip_codes {
        loc.zip_codes = v;
    }
    if let Some(v) = payload.metro_area {
        loc.metro_area = v;
    }
    if let Some(v) = payload.population {
        loc.population = v;
    }
    if let Some(v) = payload.median_income {
        loc.median_income = v;
    }
    if let Some(v) = payload.housing_units {
        loc.housing_units = v;
    }
    if let Some(v) = payload.owner_occupied_pct {
        loc.owner_occupied_pct = v;
    }
    if let Some(v) = payload.median_home_age {
        loc.median_home_age = v;
    }
    if let Some(v) = payload.climate_zone {
        loc.climate_zone = v;
    }
    if let Some(v) = payload.weather_patterns {
        loc.weather_patterns = v;
    }
    if let Some(v) = payload.local_keywords {
        loc.local_keywords = v;
    }
    if let Some(v) = payload.local_competitors {
        loc.local_competitors = v;
    }
    if let Some(v) = payload.local_regulations {
        loc.local_regulations = v;
    }
    if let Some(v) = payload.cost_of_living_index {
        loc.cost_of_living_index = v;
    }
    if let Some(v) = &payload.area_description {
        loc.area_description = v.clone();
    }
    if let Some(v) = payload.neighborhoods {
        loc.neighborhoods = v;
    }
    if let Some(active) = payload.active {
        loc.active = active;
    }

    if let Err(e) = profile::persist_location(&mut j, &loc) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to update: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Updated location '{}, {}'", loc.city, loc.state),
        data: None,
    })
}

/// DELETE /api/modules/location-profile/locations/{slug} — tombstone-delete a location.
async fn delete_location(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
) -> Json<ApiResult> {
    let mut j = journal.lock().await;

    if profile::load_location_by_slug(&j, &slug).is_none() {
        return Json(ApiResult {
            ok: false,
            message: format!("Location '{}' not found", slug),
            data: None,
        });
    }

    if let Err(e) = profile::delete_location(&mut j, &slug) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to delete: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Location '{}' deleted", slug),
        data: None,
    })
}

/// POST /api/modules/location-profile/locations/{slug}/import-census — merge census data.
async fn import_census(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
    axum::extract::Json(payload): axum::extract::Json<census::CensusImportPayload>,
) -> Json<ApiResult> {
    let mut j = journal.lock().await;

    let mut loc = match profile::load_location_by_slug(&j, &slug) {
        Some(l) => l,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Location '{}' not found", slug),
                data: None,
            });
        }
    };

    let updated = census::merge_census(&mut loc, &payload);

    if let Err(e) = profile::persist_location(&mut j, &loc) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Updated {} census field(s) for '{}'", updated, slug),
        data: Some(serde_json::json!({ "fields_updated": updated })),
    })
}

/// POST /api/modules/location-profile/locations/{slug}/import-competitors — replace competitors.
async fn import_competitors(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
    axum::extract::Json(entries): axum::extract::Json<Vec<competitors::CompetitorImportEntry>>,
) -> Json<ApiResult> {
    // Validate entries
    let validated = match competitors::validate_competitors(entries) {
        Ok(v) => v,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Validation error: {e}"),
                data: None,
            });
        }
    };

    let count = validated.len();

    let mut j = journal.lock().await;

    let mut loc = match profile::load_location_by_slug(&j, &slug) {
        Some(l) => l,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Location '{}' not found", slug),
                data: None,
            });
        }
    };

    // Replace the competitor list entirely
    loc.local_competitors = validated;

    if let Err(e) = profile::persist_location(&mut j, &loc) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Imported {} competitor(s) for '{}'", count, slug),
        data: Some(serde_json::json!({ "competitors_imported": count })),
    })
}
