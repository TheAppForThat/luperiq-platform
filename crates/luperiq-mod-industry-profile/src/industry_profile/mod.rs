//! Industry Profile Module — centrally maintained data sheets per industry.
//!
//! Provides:
//! - **IndustryProfile**: Rich aggregate with terminology, compliance, services,
//!   equipment, materials, SEO keywords, content guidelines, seasonal patterns,
//!   pricing norms, pain points, trust factors, and Schema.org types.
//! - **Seed data**: 8 realistic industry profiles (HVAC, pest control, plumbing,
//!   electrical, landscaping, dog waste removal, law office, pawn shop).
//! - **CSV import**: Bulk import SEO keywords from CSV into any profile.
//!
//! Admin views: Industry Profiles (Data section)
//!
//! Security notes:
//! - Admin UI uses DOM methods (createElement/textContent) for XSS safety
//! - All write endpoints are admin-authenticated via middleware in main.rs

pub mod admin_js;
pub mod profile;
pub mod seed;
pub mod seo_import;

use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};

use luperiq_module_api::{
    AdminView, AiFeatureConfig, AiFeatureRegistry, AppContext, CmsModule, SharedJournal,
};

// ── Module definition ─────────────────────────────────────────────────

pub struct IndustryProfileModule;

impl CmsModule for IndustryProfileModule {
    fn slug(&self) -> &str {
        "industry-profile"
    }
    fn name(&self) -> &str {
        "Industry Profiles"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Centrally maintained industry data sheets with terminology, compliance, services, SEO keywords, and content guidelines."
    }
    fn category(&self) -> &str {
        "Data"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(industry_profile_router(
            ctx.journal.clone(),
            ctx.ai_features.clone(),
        ))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![AdminView {
            id: "industry-profiles".into(),
            label: "Industry Profiles".into(),
            section: "Data".into(),
        }]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::INDUSTRY_PROFILES_ADMIN_JS.to_string())
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
struct CreateProfilePayload {
    name: String,
    slug: String,
    description: Option<String>,
    category: Option<String>,
    #[serde(default)]
    terminology: Vec<profile::IndustryTerm>,
    #[serde(default)]
    compliance_requirements: Vec<profile::ComplianceReq>,
    #[serde(default)]
    common_services: Vec<profile::CommonService>,
    #[serde(default)]
    equipment_categories: Vec<profile::EquipmentCategory>,
    #[serde(default)]
    material_categories: Vec<profile::MaterialCategory>,
    #[serde(default)]
    seo_keywords: Vec<profile::SeoKeyword>,
    #[serde(default)]
    content_guidelines: Vec<profile::ContentGuideline>,
    #[serde(default)]
    seasonal_patterns: Vec<profile::SeasonalPattern>,
    #[serde(default)]
    pricing_norms: profile::PricingNorms,
    #[serde(default)]
    customer_pain_points: Vec<String>,
    #[serde(default)]
    trust_factors: Vec<String>,
    #[serde(default)]
    competitor_terms: Vec<String>,
    #[serde(default)]
    schema_org_types: Vec<String>,
    #[serde(default = "default_true")]
    active: bool,
}

#[derive(Deserialize)]
struct UpdateProfilePayload {
    name: Option<String>,
    slug: Option<String>,
    description: Option<String>,
    category: Option<String>,
    terminology: Option<Vec<profile::IndustryTerm>>,
    compliance_requirements: Option<Vec<profile::ComplianceReq>>,
    common_services: Option<Vec<profile::CommonService>>,
    equipment_categories: Option<Vec<profile::EquipmentCategory>>,
    material_categories: Option<Vec<profile::MaterialCategory>>,
    seo_keywords: Option<Vec<profile::SeoKeyword>>,
    content_guidelines: Option<Vec<profile::ContentGuideline>>,
    seasonal_patterns: Option<Vec<profile::SeasonalPattern>>,
    pricing_norms: Option<profile::PricingNorms>,
    customer_pain_points: Option<Vec<String>>,
    trust_factors: Option<Vec<String>>,
    competitor_terms: Option<Vec<String>>,
    schema_org_types: Option<Vec<String>>,
    active: Option<bool>,
}

fn default_true() -> bool {
    true
}

/// Returns `true` if `s` is a valid profile slug (ASCII alphanumerics and hyphens only).
///
/// Slugs must be non-empty; callers are responsible for trimming/lowercasing before
/// passing to this function.
fn is_valid_slug(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

// ── Router ────────────────────────────────────────────────────────────

fn industry_profile_router(journal: SharedJournal, ai_features: AiFeatureRegistry) -> Router {
    // Register AI features
    {
        let features = ai_features.clone();
        tokio::task::spawn(async move {
            let mut reg = features.lock().await;
            reg.insert("industry_ai_guidelines".into(), AiFeatureConfig {
                system_prompt: "You are an expert content strategist for service industry businesses. Based on the provided industry profile, generate comprehensive content guidelines including recommended page types, word counts, tone notes, and section structures. Return as a JSON array of objects with keys: page_type, word_count_min, word_count_max, tone_notes, recommended_sections (array of strings).".to_string(),
                max_input_len: 6000,
                credit_cost: 3,
                escalation_credit_cost: 1,
                result_parser: |s| {
                    let trimmed = s.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {e}"))
                },
            });
        });
    }

    Router::new()
        .route(
            "/api/modules/industry-profile/profiles",
            get(list_profiles).post(create_profile),
        )
        .route(
            "/api/modules/industry-profile/profiles/{slug}",
            get(get_profile).put(update_profile).delete(delete_profile),
        )
        .route(
            "/api/modules/industry-profile/profiles/{slug}/import-seo",
            post(import_seo),
        )
        .route("/api/modules/industry-profile/seed", post(seed_profiles))
        .with_state(journal)
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/modules/industry-profile/profiles — list all profiles.
async fn list_profiles(State(journal): State<SharedJournal>) -> Json<ApiResult> {
    let j = journal.lock().await;
    let mut profiles = profile::load_all_profiles(&j);
    profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let items: Vec<serde_json::Value> = profiles
        .iter()
        .map(|p| serde_json::to_value(p).unwrap_or_default())
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} profile(s)", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

/// GET /api/modules/industry-profile/profiles/{slug} — get a single profile.
async fn get_profile(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
) -> Json<ApiResult> {
    let j = journal.lock().await;
    match profile::load_profile_by_slug(&j, &slug) {
        Some(p) => Json(ApiResult {
            ok: true,
            message: "Profile found".into(),
            data: Some(serde_json::to_value(&p).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: format!("Profile '{}' not found", slug),
            data: None,
        }),
    }
}

/// POST /api/modules/industry-profile/profiles — create a new profile.
async fn create_profile(
    State(journal): State<SharedJournal>,
    axum::extract::Json(payload): axum::extract::Json<CreateProfilePayload>,
) -> Json<ApiResult> {
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Profile name is required".into(),
            data: None,
        });
    }

    let slug = payload.slug.trim().to_lowercase();
    if slug.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Profile slug is required".into(),
            data: None,
        });
    }

    // Validate slug format (alphanumeric + hyphens only)
    if !is_valid_slug(&slug) {
        return Json(ApiResult {
            ok: false,
            message: "Slug must contain only letters, numbers, and hyphens".into(),
            data: None,
        });
    }

    let category = payload.category.as_deref().unwrap_or("other").to_string();
    if !profile::VALID_CATEGORIES.contains(&category.as_str()) {
        return Json(ApiResult {
            ok: false,
            message: format!(
                "Invalid category '{}'. Must be one of: {}",
                category,
                profile::VALID_CATEGORIES.join(", ")
            ),
            data: None,
        });
    }

    // Check for existing profile with same slug
    {
        let j = journal.lock().await;
        if profile::load_profile_by_slug(&j, &slug).is_some() {
            return Json(ApiResult {
                ok: false,
                message: format!("Profile with slug '{}' already exists", slug),
                data: None,
            });
        }
    }

    let id = ulid::Ulid::new().to_string();
    let p = profile::IndustryProfile {
        id: id.clone(),
        slug: slug.clone(),
        name,
        description: payload.description.unwrap_or_default(),
        category,
        terminology: payload.terminology,
        compliance_requirements: payload.compliance_requirements,
        common_services: payload.common_services,
        equipment_categories: payload.equipment_categories,
        material_categories: payload.material_categories,
        seo_keywords: payload.seo_keywords,
        content_guidelines: payload.content_guidelines,
        seasonal_patterns: payload.seasonal_patterns,
        pricing_norms: payload.pricing_norms,
        customer_pain_points: payload.customer_pain_points,
        trust_factors: payload.trust_factors,
        competitor_terms: payload.competitor_terms,
        schema_org_types: payload.schema_org_types,
        active: payload.active,
    };

    let mut j = journal.lock().await;
    if let Err(e) = profile::persist_profile(&mut j, &p) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to create profile: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Created profile '{}'", p.name),
        data: Some(serde_json::json!({ "id": id, "slug": slug })),
    })
}

/// PUT /api/modules/industry-profile/profiles/{slug} — update an existing profile.
async fn update_profile(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
    axum::extract::Json(payload): axum::extract::Json<UpdateProfilePayload>,
) -> Json<ApiResult> {
    let mut j = journal.lock().await;

    let mut p = match profile::load_profile_by_slug(&j, &slug) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Profile '{}' not found", slug),
                data: None,
            });
        }
    };

    if let Some(name) = &payload.name {
        p.name = name.trim().to_string();
    }
    if let Some(new_slug) = &payload.slug {
        let ns = new_slug.trim().to_lowercase();
        if !is_valid_slug(&ns) {
            return Json(ApiResult {
                ok: false,
                message: "Slug must contain only letters, numbers, and hyphens".into(),
                data: None,
            });
        }
        // If slug is changing, tombstone old and use new slug
        if ns != slug {
            if profile::load_profile_by_slug(&j, &ns).is_some() {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Profile with slug '{}' already exists", ns),
                    data: None,
                });
            }
            // Tombstone old slug
            if let Err(e) = profile::delete_profile(&mut j, &slug) {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Failed to remove old slug: {e}"),
                    data: None,
                });
            }
            p.slug = ns;
        }
    }
    if let Some(desc) = &payload.description {
        p.description = desc.clone();
    }
    if let Some(category) = &payload.category {
        if !profile::VALID_CATEGORIES.contains(&category.as_str()) {
            return Json(ApiResult {
                ok: false,
                message: format!(
                    "Invalid category '{}'. Must be one of: {}",
                    category,
                    profile::VALID_CATEGORIES.join(", ")
                ),
                data: None,
            });
        }
        p.category = category.clone();
    }
    if let Some(v) = payload.terminology {
        p.terminology = v;
    }
    if let Some(v) = payload.compliance_requirements {
        p.compliance_requirements = v;
    }
    if let Some(v) = payload.common_services {
        p.common_services = v;
    }
    if let Some(v) = payload.equipment_categories {
        p.equipment_categories = v;
    }
    if let Some(v) = payload.material_categories {
        p.material_categories = v;
    }
    if let Some(v) = payload.seo_keywords {
        p.seo_keywords = v;
    }
    if let Some(v) = payload.content_guidelines {
        p.content_guidelines = v;
    }
    if let Some(v) = payload.seasonal_patterns {
        p.seasonal_patterns = v;
    }
    if let Some(v) = payload.pricing_norms {
        p.pricing_norms = v;
    }
    if let Some(v) = payload.customer_pain_points {
        p.customer_pain_points = v;
    }
    if let Some(v) = payload.trust_factors {
        p.trust_factors = v;
    }
    if let Some(v) = payload.competitor_terms {
        p.competitor_terms = v;
    }
    if let Some(v) = payload.schema_org_types {
        p.schema_org_types = v;
    }
    if let Some(active) = payload.active {
        p.active = active;
    }

    if let Err(e) = profile::persist_profile(&mut j, &p) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to update: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Updated profile '{}'", p.name),
        data: None,
    })
}

/// DELETE /api/modules/industry-profile/profiles/{slug} — tombstone-delete a profile.
async fn delete_profile(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
) -> Json<ApiResult> {
    let mut j = journal.lock().await;

    if profile::load_profile_by_slug(&j, &slug).is_none() {
        return Json(ApiResult {
            ok: false,
            message: format!("Profile '{}' not found", slug),
            data: None,
        });
    }

    if let Err(e) = profile::delete_profile(&mut j, &slug) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to delete: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Profile '{}' deleted", slug),
        data: None,
    })
}

/// POST /api/modules/industry-profile/profiles/{slug}/import-seo — import SEO keywords from CSV.
async fn import_seo(
    State(journal): State<SharedJournal>,
    Path(slug): Path<String>,
    body: String,
) -> Json<ApiResult> {
    if body.trim().is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "CSV body is empty".into(),
            data: None,
        });
    }

    // Parse CSV
    let new_keywords = match seo_import::parse_seo_csv(&body) {
        Ok(kw) => kw,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("CSV parse error: {e}"),
                data: None,
            });
        }
    };

    if new_keywords.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "No keywords found in CSV".into(),
            data: None,
        });
    }

    let mut j = journal.lock().await;

    let mut p = match profile::load_profile_by_slug(&j, &slug) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Profile '{}' not found", slug),
                data: None,
            });
        }
    };

    let total_parsed = new_keywords.len();
    let added = seo_import::merge_keywords(&mut p.seo_keywords, new_keywords);

    if let Err(e) = profile::persist_profile(&mut j, &p) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!(
            "Imported {} new keyword(s) ({} parsed, {} duplicates skipped). Total: {}",
            added,
            total_parsed,
            total_parsed - added,
            p.seo_keywords.len()
        ),
        data: Some(serde_json::json!({
            "added": added,
            "parsed": total_parsed,
            "total": p.seo_keywords.len(),
        })),
    })
}

/// POST /api/modules/industry-profile/seed — seed 8 default industry profiles.
async fn seed_profiles(State(journal): State<SharedJournal>) -> Json<ApiResult> {
    let profiles = seed::seed_profiles();
    let mut j = journal.lock().await;

    let mut created = 0;
    let mut skipped = 0;

    for p in &profiles {
        if profile::load_profile_by_slug(&j, &p.slug).is_some() {
            skipped += 1;
            continue;
        }
        match profile::persist_profile(&mut j, p) {
            Ok(()) => created += 1,
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Failed to seed '{}': {e}", p.slug),
                    data: None,
                });
            }
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("Seeded {} profile(s), {} already existed", created, skipped),
        data: Some(serde_json::json!({
            "created": created,
            "skipped": skipped,
            "total": profiles.len(),
        })),
    })
}
