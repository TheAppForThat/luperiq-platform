//! Company Profile Module — business identity fact sheet with semi-automated import.
//!
//! Provides:
//! - **CompanyProfile**: Singleton aggregate with brand identity, tone, team bios,
//!   certifications, USPs, social links, contact info, and review highlights.
//! - **Semi-automated import**: Extract data from Google Business, Facebook, website URLs.
//! - **AI extraction**: Parse free-form owner conversation transcripts for profile data.
//! - **Questionnaire**: Guided setup wizard with ~20 questions.
//! - **Import job workflow**: Review-before-apply pattern for all import sources.
//!
//! Admin views: Company Profile (Data section)
//!
//! Security notes:
//! - Admin UI uses DOM methods (createElement/textContent) for XSS safety
//! - All write endpoints are admin-authenticated via middleware in main.rs
//! - URL extractors are best-effort and data always goes through admin review

pub mod admin_js;
pub mod conversation;
pub mod extractors;
pub mod import;
pub mod profile;
pub mod questionnaire;

use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use luperiq_module_api::{
    AdminView, AiFeatureConfig, AiFeatureRegistry, AppContext, CmsModule, SharedJournal,
};

// ── AI provider abstraction ─────────────────────────────────────────

/// Response from an AI generation call.
pub struct CompanyAiResponse {
    pub content: String,
}

/// Trait for AI text generation, abstracting the CMS's `AiClient`.
///
/// The CMS wires in the real implementation; without one, AI endpoints return
/// "AI not configured".
pub trait CompanyAiProvider: Send + Sync + 'static {
    fn generate(
        &self,
        system: &str,
        user_message: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<CompanyAiResponse, String>> + Send + '_>,
    >;
}

/// Shared handle to an optional AI provider.
pub type OptCompanyAiProvider = Option<Arc<dyn CompanyAiProvider>>;

// ── Shared state for company profile handlers ────────────────────────

#[derive(Clone)]
pub struct CompanyProfileState {
    pub journal: SharedJournal,
    pub ai_provider: OptCompanyAiProvider,
}

// ── Module definition ─────────────────────────────────────────────────

pub struct CompanyProfileModule {
    pub ai_provider: OptCompanyAiProvider,
}

impl CmsModule for CompanyProfileModule {
    fn slug(&self) -> &str {
        "company-profile"
    }
    fn name(&self) -> &str {
        "Company Profile"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Business identity fact sheet with brand, tone, team, USPs, social links, and semi-automated import."
    }
    fn category(&self) -> &str {
        "Data"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(company_profile_router(
            CompanyProfileState {
                journal: ctx.journal.clone(),
                ai_provider: self.ai_provider.clone(),
            },
            ctx.ai_features.clone(),
        ))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![AdminView {
            id: "company-profile".into(),
            label: "Company Profile".into(),
            section: "Data".into(),
        }]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::COMPANY_PROFILE_ADMIN_JS.to_string())
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
struct UrlImportPayload {
    url: String,
}

#[derive(Deserialize)]
struct ConversationPayload {
    transcript: String,
}

// ── Router ────────────────────────────────────────────────────────────

fn company_profile_router(state: CompanyProfileState, ai_features: AiFeatureRegistry) -> Router {
    // Register AI features
    {
        let features = ai_features.clone();
        tokio::task::spawn(async move {
            let mut reg = features.lock().await;
            reg.insert("company_ai_bio".into(), AiFeatureConfig {
                system_prompt: "You are a professional copywriter for small businesses. Write a compelling company bio/about section based on the provided business details. Keep it 2-3 paragraphs, warm but professional. Highlight the company's story, values, and what makes them unique. Return just the text.".to_string(),
                max_input_len: 4000,
                credit_cost: 2,
                escalation_credit_cost: 1,
                result_parser: |s| Ok(serde_json::Value::String(s.trim().to_string())),
            });
            reg.insert("company_ai_usp".into(), AiFeatureConfig {
                system_prompt: "You are a marketing strategist for small businesses. Based on the provided business details, generate 3-5 unique selling propositions (USPs). Each should be one concise, compelling sentence that differentiates this business from competitors. Return as a JSON array of strings.".to_string(),
                max_input_len: 4000,
                credit_cost: 2,
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
            "/api/modules/company-profile/profile",
            get(get_profile).post(save_profile),
        )
        .route(
            "/api/modules/company-profile/questionnaire",
            get(get_questionnaire).post(submit_questionnaire),
        )
        .route(
            "/api/modules/company-profile/import/google",
            post(import_google),
        )
        .route(
            "/api/modules/company-profile/import/facebook",
            post(import_facebook),
        )
        .route(
            "/api/modules/company-profile/import/website",
            post(import_website),
        )
        .route(
            "/api/modules/company-profile/extract-conversation",
            post(extract_conversation),
        )
        .route("/api/modules/company-profile/imports", get(list_imports))
        .route(
            "/api/modules/company-profile/imports/{id}/apply",
            post(apply_import),
        )
        .route(
            "/api/modules/company-profile/site-visibility",
            get(get_site_visibility).post(set_site_visibility),
        )
        .with_state(state)
}

// ── Site Visibility ───────────────────────────────────────────────────

/// WAL aggregate type for site-settings records.
///
/// Shared with `luperiq-cms` — use this const rather than raw string literals
/// so a key rename gets a compile-time error everywhere this crate is used.
pub const SITE_SETTINGS_AGG: &str = "SiteSettings";

/// Aggregate ID for the site visibility toggle (singleton per site).
pub const SITE_VISIBILITY_ID: &str = "visibility";

/// GET /api/modules/company-profile/site-visibility — check if site is public
async fn get_site_visibility(State(state): State<CompanyProfileState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let is_public = j
        .get_latest(SITE_SETTINGS_AGG, SITE_VISIBILITY_ID)
        .and_then(|e| serde_json::from_slice::<serde_json::Value>(&e.payload).ok())
        .and_then(|v| v.get("public").and_then(|p| p.as_bool()))
        .unwrap_or(false);
    Json(serde_json::json!({ "ok": true, "public": is_public }))
}

/// POST /api/modules/company-profile/site-visibility — toggle site visibility
/// Body: `{ "public": true }` or `{ "public": false }`
async fn set_site_visibility(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let is_public = body
        .get("public")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let payload = serde_json::json!({ "public": is_public });
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let mut j = state.journal.lock().await;
    match j.append(luperiq_forge::ApexEvent::new(
        SITE_SETTINGS_AGG,
        SITE_VISIBILITY_ID,
        bytes,
    )) {
        Ok(_) => Json(serde_json::json!({ "ok": true, "public": is_public })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
    }
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/modules/company-profile/profile — get the company profile (or 404).
async fn get_profile(State(state): State<CompanyProfileState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    match profile::load_company_profile(&j) {
        Some(p) => Json(ApiResult {
            ok: true,
            message: "Profile found".into(),
            data: Some(serde_json::to_value(&p).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: "No company profile has been created yet".into(),
            data: None,
        }),
    }
}

/// POST /api/modules/company-profile/profile — create or update the company profile.
async fn save_profile(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    let mut p = profile::load_company_profile(&j).unwrap_or_default();
    let is_new = p.name.is_empty();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(obj) = payload.as_object() {
        merge_into_profile(&mut p, obj);
    }

    p.updated_at = now;
    if is_new {
        p.created_at = now;
    }

    if p.name.trim().is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Business name is required".into(),
            data: None,
        });
    }

    if let Err(e) = profile::persist_company_profile(&mut j, &p) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save profile: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: if is_new {
            "Company profile created".into()
        } else {
            "Company profile updated".into()
        },
        data: Some(serde_json::json!({ "id": "global" })),
    })
}

async fn get_questionnaire(State(_state): State<CompanyProfileState>) -> Json<ApiResult> {
    let questions = questionnaire::get_questions();
    Json(ApiResult {
        ok: true,
        message: format!("{} questions", questions.len()),
        data: Some(serde_json::to_value(&questions).unwrap_or_default()),
    })
}

async fn submit_questionnaire(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(answers): axum::extract::Json<Vec<questionnaire::QuestionnaireAnswer>>,
) -> Json<ApiResult> {
    if answers.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "No answers provided".into(),
            data: None,
        });
    }

    let extracted_data = questionnaire::build_profile_from_answers(&answers);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let job = import::CompanyImportJob {
        id: ulid::Ulid::new().to_string(),
        source: "questionnaire".into(),
        source_url: None,
        status: "review".into(),
        extracted_data: Some(extracted_data),
        error: None,
        created_at: now,
        completed_at: now,
    };

    let mut j = state.journal.lock().await;
    if let Err(e) = import::persist_import(&mut j, &job) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save import job: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: "Questionnaire submitted. Review and apply the imported data.".into(),
        data: Some(serde_json::json!({ "import_id": job.id })),
    })
}

async fn import_google(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(payload): axum::extract::Json<UrlImportPayload>,
) -> Json<ApiResult> {
    run_url_import(state, "google_business", &payload.url).await
}

async fn import_facebook(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(payload): axum::extract::Json<UrlImportPayload>,
) -> Json<ApiResult> {
    run_url_import(state, "facebook", &payload.url).await
}

async fn import_website(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(payload): axum::extract::Json<UrlImportPayload>,
) -> Json<ApiResult> {
    run_url_import(state, "website", &payload.url).await
}

async fn run_url_import(state: CompanyProfileState, source: &str, url: &str) -> Json<ApiResult> {
    let url = url.trim();
    if url.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "URL is required".into(),
            data: None,
        });
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Json(ApiResult {
            ok: false,
            message: "URL must start with http:// or https://".into(),
            data: None,
        });
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let job_id = ulid::Ulid::new().to_string();

    let mut job = import::CompanyImportJob {
        id: job_id.clone(),
        source: source.to_string(),
        source_url: Some(url.to_string()),
        status: "extracting".into(),
        extracted_data: None,
        error: None,
        created_at: now,
        completed_at: 0,
    };

    {
        let mut j = state.journal.lock().await;
        if let Err(e) = import::persist_import(&mut j, &job) {
            return Json(ApiResult {
                ok: false,
                message: format!("Failed to create import job: {e}"),
                data: None,
            });
        }
    }

    let result = match source {
        "google_business" => extractors::extract_google_business(url).await,
        "facebook" => extractors::extract_facebook(url).await,
        "website" => extractors::extract_website(url).await,
        _ => Err(format!("Unknown source: {source}")),
    };

    let completed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    match result {
        Ok(data) => {
            job.status = "review".into();
            job.extracted_data = Some(data);
            job.completed_at = completed_at;
        }
        Err(e) => {
            job.status = "failed".into();
            job.error = Some(e.clone());
            job.completed_at = completed_at;
        }
    }

    let mut j = state.journal.lock().await;
    if let Err(e) = import::persist_import(&mut j, &job) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to update import job: {e}"),
            data: None,
        });
    }

    match &job.status[..] {
        "review" => Json(ApiResult {
            ok: true,
            message: format!(
                "Data extracted from {}. Review and apply when ready.",
                source
            ),
            data: Some(serde_json::json!({ "import_id": job_id })),
        }),
        _ => Json(ApiResult {
            ok: false,
            message: job.error.unwrap_or_else(|| "Extraction failed".into()),
            data: Some(serde_json::json!({ "import_id": job_id })),
        }),
    }
}

async fn extract_conversation(
    State(state): State<CompanyProfileState>,
    axum::extract::Json(payload): axum::extract::Json<ConversationPayload>,
) -> Json<ApiResult> {
    let ai_provider = match &state.ai_provider {
        Some(c) => c.clone(),
        None => return Json(ApiResult { ok: false, message: "AI is not configured. Set up an AI provider in cms.toml to use conversation extraction.".into(), data: None }),
    };

    let transcript = payload.transcript.trim();
    if transcript.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Transcript is required".into(),
            data: None,
        });
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let job_id = ulid::Ulid::new().to_string();

    let mut job = import::CompanyImportJob {
        id: job_id.clone(),
        source: "conversation".into(),
        source_url: None,
        status: "extracting".into(),
        extracted_data: None,
        error: None,
        created_at: now,
        completed_at: 0,
    };

    {
        let mut j = state.journal.lock().await;
        if let Err(e) = import::persist_import(&mut j, &job) {
            return Json(ApiResult {
                ok: false,
                message: format!("Failed to create import job: {e}"),
                data: None,
            });
        }
    }

    let result = conversation::extract_from_conversation(&*ai_provider, transcript).await;
    let completed_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match result {
        Ok(data) => {
            job.status = "review".into();
            job.extracted_data = Some(data);
            job.completed_at = completed_at;
        }
        Err(e) => {
            job.status = "failed".into();
            job.error = Some(e);
            job.completed_at = completed_at;
        }
    }

    let mut j = state.journal.lock().await;
    if let Err(e) = import::persist_import(&mut j, &job) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to update import job: {e}"),
            data: None,
        });
    }

    match &job.status[..] {
        "review" => Json(ApiResult {
            ok: true,
            message: "AI extraction complete. Review and apply when ready.".into(),
            data: Some(serde_json::json!({ "import_id": job_id })),
        }),
        _ => Json(ApiResult {
            ok: false,
            message: job.error.unwrap_or_else(|| "AI extraction failed".into()),
            data: Some(serde_json::json!({ "import_id": job_id })),
        }),
    }
}

async fn list_imports(State(state): State<CompanyProfileState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let jobs = import::load_all_imports(&j);
    let items: Vec<serde_json::Value> = jobs
        .iter()
        .map(|j| serde_json::to_value(j).unwrap_or_default())
        .collect();
    Json(ApiResult {
        ok: true,
        message: format!("{} import(s)", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

async fn apply_import(
    State(state): State<CompanyProfileState>,
    Path(id): Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    let mut job = match import::load_import(&j, &id) {
        Some(j) => j,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("Import job '{}' not found", id),
                data: None,
            })
        }
    };

    if job.status != "review" {
        return Json(ApiResult {
            ok: false,
            message: format!(
                "Import job is in '{}' status, must be 'review' to apply",
                job.status
            ),
            data: None,
        });
    }

    let extracted = match &job.extracted_data {
        Some(data) => data.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Import job has no extracted data".into(),
                data: None,
            })
        }
    };

    let mut p = profile::load_company_profile(&j).unwrap_or_default();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let is_new = p.name.is_empty();

    if let Some(obj) = extracted.as_object() {
        merge_into_profile(&mut p, obj);
    }
    p.updated_at = now;
    if is_new {
        p.created_at = now;
    }

    if let Err(e) = profile::persist_company_profile(&mut j, &p) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save profile: {e}"),
            data: None,
        });
    }

    job.status = "applied".into();
    job.completed_at = now;
    if let Err(e) = import::persist_import(&mut j, &job) {
        eprintln!("Warning: failed to update import job status: {e}");
    }

    Json(ApiResult {
        ok: true,
        message: "Import data applied to company profile".into(),
        data: None,
    })
}

// ── Profile merge helper ─────────────────────────────────────────────

fn merge_into_profile(
    p: &mut profile::CompanyProfile,
    obj: &serde_json::Map<String, serde_json::Value>,
) {
    if let Some(v) = obj.get("name").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.name = v.to_string();
        }
    }
    if let Some(v) = obj.get("legal_name") {
        if v.is_null() {
            p.legal_name = None;
        } else if let Some(s) = v.as_str() {
            p.legal_name = if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            };
        }
    }
    if let Some(v) = obj.get("industry_slug").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.industry_slug = v.to_string();
        }
    }
    if let Some(v) = obj.get("tagline").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.tagline = v.to_string();
        }
    }
    if let Some(v) = obj.get("story").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.story = v.to_string();
        }
    }
    if let Some(v) = obj.get("tone").and_then(|v| v.as_str()) {
        let valid_tones = [
            "professional",
            "friendly",
            "casual",
            "authoritative",
            "playful",
        ];
        if valid_tones.contains(&v) {
            p.tone = v.to_string();
        }
    }
    if let Some(v) = obj.get("service_philosophy").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.service_philosophy = v.to_string();
        }
    }
    if let Some(v) = obj.get("phone").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.phone = v.to_string();
        }
    }
    if let Some(v) = obj.get("email").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.email = v.to_string();
        }
    }
    if let Some(v) = obj.get("address").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.address = v.to_string();
        }
    }
    if let Some(v) = obj.get("city").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.city = v.to_string();
        }
    }
    if let Some(v) = obj.get("state").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.state = v.to_string();
        }
    }
    if let Some(v) = obj.get("zip").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.zip = v.to_string();
        }
    }
    if let Some(v) = obj.get("service_area_description").and_then(|v| v.as_str()) {
        if !v.is_empty() {
            p.service_area_description = v.to_string();
        }
    }

    if let Some(v) = obj.get("logo_url") {
        p.logo_url = v.as_str().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });
    }
    if let Some(v) = obj.get("favicon_url") {
        p.favicon_url = v.as_str().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });
    }
    if let Some(v) = obj.get("owner_name") {
        p.owner_name = v.as_str().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });
    }
    if let Some(v) = obj.get("owner_title") {
        p.owner_title = v.as_str().and_then(|s| {
            if s.is_empty() {
                None
            } else {
                Some(s.to_string())
            }
        });
    }

    if let Some(v) = obj.get("years_in_business") {
        if v.is_null() {
            p.years_in_business = None;
        } else if let Some(n) = v.as_u64() {
            p.years_in_business = Some(n as u32);
        }
    }

    if let Some(v) = obj.get("voice_notes").and_then(|v| v.as_array()) {
        p.voice_notes = v
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = obj.get("certifications").and_then(|v| v.as_array()) {
        p.certifications = v
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = obj.get("license_numbers").and_then(|v| v.as_array()) {
        p.license_numbers = v
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = obj.get("unique_selling_points").and_then(|v| v.as_array()) {
        p.unique_selling_points = v
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(v) = obj.get("location_slugs").and_then(|v| v.as_array()) {
        p.location_slugs = v
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();
    }

    if let Some(v) = obj.get("brand_colors").and_then(|v| v.as_object()) {
        if let Some(c) = v.get("primary").and_then(|c| c.as_str()) {
            if !c.is_empty() {
                p.brand_colors.primary = c.to_string();
            }
        }
        if let Some(c) = v.get("secondary").and_then(|c| c.as_str()) {
            if !c.is_empty() {
                p.brand_colors.secondary = c.to_string();
            }
        }
        if let Some(c) = v.get("accent").and_then(|c| c.as_str()) {
            if !c.is_empty() {
                p.brand_colors.accent = c.to_string();
            }
        }
    }

    if let Some(v) = obj.get("social_links").and_then(|v| v.as_object()) {
        if let Some(s) = v.get("google_business") {
            p.social_links.google_business = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("facebook") {
            p.social_links.facebook = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("instagram") {
            p.social_links.instagram = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("twitter") {
            p.social_links.twitter = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("youtube") {
            p.social_links.youtube = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("linkedin") {
            p.social_links.linkedin = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("yelp") {
            p.social_links.yelp = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
        if let Some(s) = v.get("nextdoor") {
            p.social_links.nextdoor = s.as_str().and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            });
        }
    }

    if let Some(v) = obj.get("team_bios").and_then(|v| v.as_array()) {
        let mut team = Vec::new();
        for item in v {
            if let Ok(member) = serde_json::from_value::<profile::TeamMember>(item.clone()) {
                team.push(member);
            }
        }
        p.team_bios = team;
    }

    if let Some(v) = obj.get("review_highlights").and_then(|v| v.as_array()) {
        let mut reviews = Vec::new();
        for item in v {
            if let Ok(review) = serde_json::from_value::<profile::ReviewHighlight>(item.clone()) {
                reviews.push(review);
            }
        }
        p.review_highlights = reviews;
    }
}
