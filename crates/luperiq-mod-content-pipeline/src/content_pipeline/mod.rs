//! Content Pipeline Module — two-stage AI content generation with SEO optimization.
//!
//! Combines Company Profile + Industry Profile + Location Profile + SEO Guidelines
//! + Fact Packs into rich context for AI content generation. This is the core value
//! engine — it generates entire websites' worth of content for service businesses.
//!
//! Key design principle: Surfer SEO guidelines and Fact Packs are invisible
//! server-side reference data. The customer never sees them. They feed the AI
//! content generation pipeline behind the scenes.
//!
//! ## Aggregates
//! - `CntPipe:Job` — ContentJob lifecycle tracking
//! - `CntPipe:Template` — Handlebars prompt templates
//! - `CntPipe:SeoGuide` — Surfer-style SEO guidelines
//! - `CntPipe:FactPack` — Structured reference data with citations
//!
//! Security notes:
//! - Admin UI uses DOM methods (createElement/textContent) for XSS safety
//! - All write endpoints are admin-authenticated via middleware in main.rs
//! - SEO data and fact packs are admin-only, never exposed to customers

pub mod admin_js;
pub mod ai_client;
pub mod batch;
pub mod context;
pub mod generator;
pub mod jobs;
pub mod models;
pub mod seo_data;
pub mod templates;

use axum::extract::{Path, State};
use axum::response::Json;
use axum::routing::{delete, get, post, put};
use axum::Router;
use serde::Deserialize;
use std::sync::Arc;

use ai_client::AiClient;
use luperiq_module_api::{AdminView, AiFeatureConfig, AppContext, CmsModule, SharedJournal};

// ── Shared state ────────────────────────────────────────────────────

#[derive(Clone)]
struct PipelineState {
    journal: SharedJournal,
    ai_client: Option<Arc<AiClient>>,
    nexus_role: String,
}

// ── Module definition ───────────────────────────────────────────────

pub struct ContentPipelineModule;

impl CmsModule for ContentPipelineModule {
    fn slug(&self) -> &str {
        "content-pipeline"
    }
    fn name(&self) -> &str {
        "Content Pipeline"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Two-stage AI content generation with SEO optimization, fact packs, and multi-profile context assembly."
    }

    fn category(&self) -> &str {
        "Content"
    }

    fn dependencies(&self) -> &[&str] {
        &["company-profile", "industry-profile", "location-profile"]
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        let state = PipelineState {
            journal: ctx.journal.clone(),
            ai_client: AppContext::service::<AiClient>(&ctx.ai_client),
            nexus_role: ctx
                .nexus_config
                .as_ref()
                .and_then(|cfg| cfg.role.clone())
                .unwrap_or_default(),
        };

        // Register AI features
        {
            let features = ctx.ai_features.clone();
            tokio::task::spawn(async move {
                let mut reg = features.lock().await;
                reg.insert("content_ai_outline".into(), AiFeatureConfig {
                    system_prompt: "You are a content strategist for service businesses. Based on the provided topic and page type, generate a detailed content outline with sections, key points, and suggested word counts. Return as a JSON object with keys: title, sections (array of {heading, key_points (array), word_count}).".to_string(),
                    max_input_len: 2000,
                    credit_cost: 2,
                    escalation_credit_cost: 1,
                    result_parser: |s| {
                        let trimmed = s.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                        serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {e}"))
                    },
                });
                reg.insert("content_ai_template".into(), AiFeatureConfig {
                    system_prompt: "You are a prompt engineering expert for content generation. Based on the provided description, create a Handlebars prompt template that an AI can use to generate content. Include {{company_name}}, {{industry}}, {{location}}, and other relevant variables. Return just the template text with Handlebars placeholders.".to_string(),
                    max_input_len: 2000,
                    credit_cost: 3,
                    escalation_credit_cost: 1,
                    result_parser: |s| Ok(serde_json::Value::String(s.trim().to_string())),
                });
            });
        }

        let router = Router::new()
            // Content Jobs
            .route("/api/modules/content-pipeline/jobs", get(list_jobs))
            .route("/api/modules/content-pipeline/jobs", post(create_job))
            .route("/api/modules/content-pipeline/jobs/{id}", get(get_job))
            .route("/api/modules/content-pipeline/jobs/{id}", put(update_job))
            .route(
                "/api/modules/content-pipeline/jobs/{id}",
                delete(delete_job_handler),
            )
            // Templates
            .route(
                "/api/modules/content-pipeline/templates",
                get(list_templates),
            )
            .route(
                "/api/modules/content-pipeline/templates",
                post(create_template),
            )
            .route(
                "/api/modules/content-pipeline/templates/{id}",
                put(update_template),
            )
            .route(
                "/api/modules/content-pipeline/templates/{id}",
                delete(delete_template_handler),
            )
            // SEO Guidelines
            .route(
                "/api/modules/content-pipeline/seo-guidelines",
                get(list_guidelines),
            )
            .route(
                "/api/modules/content-pipeline/seo-guidelines",
                post(create_guideline),
            )
            .route(
                "/api/modules/content-pipeline/seo-guidelines/{id}",
                put(update_guideline),
            )
            .route(
                "/api/modules/content-pipeline/seo-guidelines/{id}",
                delete(delete_guideline_handler),
            )
            // Fact Packs
            .route(
                "/api/modules/content-pipeline/fact-packs",
                get(list_fact_packs),
            )
            .route(
                "/api/modules/content-pipeline/fact-packs",
                post(create_fact_pack),
            )
            .route(
                "/api/modules/content-pipeline/fact-packs/{id}",
                put(update_fact_pack),
            )
            .route(
                "/api/modules/content-pipeline/fact-packs/{id}",
                delete(delete_fact_pack_handler),
            )
            // Generation
            .route(
                "/api/modules/content-pipeline/generate",
                post(generate_single),
            )
            .route(
                "/api/modules/content-pipeline/generate-site",
                post(generate_site),
            )
            // Stats
            .route("/api/modules/content-pipeline/stats", get(pipeline_stats))
            .route("/api/modules/content-pipeline/import-native", post(import_native_pages))
            .with_state(state);

        // Seed default templates on first load
        {
            let j = ctx.journal.clone();
            tokio::spawn(async move {
                let mut journal = j.lock().await;
                match templates::seed_default_templates(&mut journal) {
                    Ok(n) if n > 0 => println!("Content Pipeline: seeded {} default templates", n),
                    Ok(_) => {}
                    Err(e) => eprintln!("Content Pipeline: template seed error: {}", e),
                }
            });
        }

        Some(router)
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![
            AdminView {
                id: "content-pipeline-generator".into(),
                label: "Content Generator".into(),
                section: "Content".into(),
            },
            AdminView {
                id: "content-pipeline-jobs".into(),
                label: "Content Jobs".into(),
                section: "Content".into(),
            },
            AdminView {
                id: "content-pipeline-templates".into(),
                label: "Content Templates".into(),
                section: "Content".into(),
            },
            AdminView {
                id: "content-pipeline-seo".into(),
                label: "SEO Data".into(),
                section: "Content".into(),
            },
        ]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::ADMIN_JS.to_string())
    }

    fn admin_css(&self) -> Option<String> {
        Some(admin_js::ADMIN_CSS.to_string())
    }
}

fn reference_data_managed_on_this_node(state: &PipelineState) -> bool {
    matches!(state.nexus_role.as_str(), "" | "central")
}

fn reference_data_forbidden_response() -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error": "SEO guides and fact packs are managed on Central and are not editable from client sites."
        })),
    )
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — Content Jobs
// ══════════════════════════════════════════════════════════════════════

async fn list_jobs(State(state): State<PipelineState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let all = jobs::load_all_jobs(&j);
    Json(serde_json::json!({ "jobs": all }))
}

#[derive(Deserialize)]
struct CreateJobReq {
    page_type: String,
    target_slug: String,
    #[serde(default = "default_quality")]
    quality_level: String,
}

fn default_quality() -> String {
    "quick_draft".to_string()
}

async fn create_job(
    State(state): State<PipelineState>,
    Json(body): Json<CreateJobReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let id = ulid::Ulid::new().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let job = jobs::ContentJob {
        id: id.clone(),
        page_type: body.page_type,
        target_slug: body.target_slug,
        quality_level: body.quality_level,
        status: "pending".to_string(),
        created_at: now,
        ..Default::default()
    };

    let mut j = state.journal.lock().await;
    match jobs::persist_job(&mut j, &job) {
        Ok(()) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!(job)),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn get_job(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let j = state.journal.lock().await;
    match jobs::load_job(&j, &id) {
        Some(job) => (axum::http::StatusCode::OK, Json(serde_json::json!(job))),
        None => (
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Job not found"})),
        ),
    }
}

#[derive(Deserialize)]
struct UpdateJobReq {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    generated_content: Option<String>,
    #[serde(default)]
    published_at: Option<u64>,
    #[serde(default)]
    reviewed_at: Option<u64>,
}

async fn update_job(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateJobReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let mut j = state.journal.lock().await;
    let job = match jobs::load_job(&j, &id) {
        Some(j) => j,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Job not found"})),
            );
        }
    };

    let mut updated = job;
    if let Some(s) = body.status {
        updated.status = s;
    }
    if let Some(c) = body.generated_content {
        updated.generated_content = c;
    }
    if let Some(t) = body.published_at {
        updated.published_at = t;
    }
    if let Some(t) = body.reviewed_at {
        updated.reviewed_at = t;
    }

    match jobs::persist_job(&mut j, &updated) {
        Ok(()) => (axum::http::StatusCode::OK, Json(serde_json::json!(updated))),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn delete_job_handler(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let mut j = state.journal.lock().await;
    match jobs::delete_job(&mut j, &id) {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({"deleted": true})),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — Templates
// ══════════════════════════════════════════════════════════════════════

async fn list_templates(State(state): State<PipelineState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let all = templates::load_all_templates(&j);
    Json(serde_json::json!({ "templates": all }))
}

#[derive(Deserialize)]
struct CreateTemplateReq {
    #[serde(default)]
    page_type: String,
    #[serde(default)]
    industry_slug: String,
    #[serde(default)]
    prompt_template: String,
    #[serde(default)]
    section_prompts: Vec<String>,
}

async fn create_template(
    State(state): State<PipelineState>,
    Json(body): Json<CreateTemplateReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let id = ulid::Ulid::new().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let tpl = templates::ContentTemplate {
        id: id.clone(),
        page_type: body.page_type,
        industry_slug: body.industry_slug,
        prompt_template: body.prompt_template,
        section_prompts: body.section_prompts,
        active: true,
        created_at: now,
    };

    let mut j = state.journal.lock().await;
    match templates::persist_template(&mut j, &tpl) {
        Ok(()) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!(tpl)),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
struct UpdateTemplateReq {
    #[serde(default)]
    prompt_template: Option<String>,
    #[serde(default)]
    section_prompts: Option<Vec<String>>,
    #[serde(default)]
    active: Option<bool>,
}

async fn update_template(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateTemplateReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let mut j = state.journal.lock().await;
    let tpl = match templates::load_template(&j, &id) {
        Some(t) => t,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Template not found"})),
            );
        }
    };

    let mut updated = tpl;
    if let Some(pt) = body.prompt_template {
        updated.prompt_template = pt;
    }
    if let Some(sp) = body.section_prompts {
        updated.section_prompts = sp;
    }
    if let Some(a) = body.active {
        updated.active = a;
    }

    match templates::persist_template(&mut j, &updated) {
        Ok(()) => (axum::http::StatusCode::OK, Json(serde_json::json!(updated))),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn delete_template_handler(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    let mut j = state.journal.lock().await;
    match templates::delete_template(&mut j, &id) {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({"deleted": true})),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — SEO Guidelines
// ══════════════════════════════════════════════════════════════════════

async fn list_guidelines(State(state): State<PipelineState>) -> Json<serde_json::Value> {
    if !reference_data_managed_on_this_node(&state) {
        return Json(serde_json::json!({
            "error": "SEO guides are managed on Central.",
            "central_only": true,
        }));
    }
    let j = state.journal.lock().await;
    let all = seo_data::load_all_guidelines(&j);
    Json(serde_json::json!({ "guidelines": all }))
}

async fn create_guideline(
    State(state): State<PipelineState>,
    Json(mut body): Json<seo_data::SeoGuideline>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    if body.id.is_empty() {
        body.id = ulid::Ulid::new().to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if body.created_at == 0 {
        body.created_at = now;
    }
    body.updated_at = now;

    let mut j = state.journal.lock().await;
    match seo_data::persist_guideline(&mut j, &body) {
        Ok(()) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!(body)),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
struct UpdateGuidelineReq {
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    scope_type: Option<String>,
    #[serde(default)]
    industry_slugs: Option<Vec<String>>,
    #[serde(default)]
    content_structure: Option<seo_data::ContentStructure>,
    #[serde(default)]
    term_frequencies: Option<Vec<seo_data::TermFrequency>>,
    #[serde(default)]
    fact_groups: Option<Vec<seo_data::FactGroup>>,
    #[serde(default)]
    active: Option<bool>,
}

async fn update_guideline(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateGuidelineReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    let mut j = state.journal.lock().await;
    let guide = match seo_data::load_guideline(&j, &id) {
        Some(g) => g,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Guideline not found"})),
            );
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut updated = guide;
    if let Some(s) = body.scope {
        updated.scope = s;
    }
    if let Some(st) = body.scope_type {
        updated.scope_type = st;
    }
    if let Some(is) = body.industry_slugs {
        updated.industry_slugs = is;
    }
    if let Some(cs) = body.content_structure {
        updated.content_structure = cs;
    }
    if let Some(tf) = body.term_frequencies {
        updated.term_frequencies = tf;
    }
    if let Some(fg) = body.fact_groups {
        updated.fact_groups = fg;
    }
    if let Some(a) = body.active {
        updated.active = a;
    }
    updated.updated_at = now;

    match seo_data::persist_guideline(&mut j, &updated) {
        Ok(()) => (axum::http::StatusCode::OK, Json(serde_json::json!(updated))),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn delete_guideline_handler(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    let mut j = state.journal.lock().await;
    match seo_data::delete_guideline(&mut j, &id) {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({"deleted": true})),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — Fact Packs
// ══════════════════════════════════════════════════════════════════════

async fn list_fact_packs(State(state): State<PipelineState>) -> Json<serde_json::Value> {
    if !reference_data_managed_on_this_node(&state) {
        return Json(serde_json::json!({
            "error": "Fact packs are managed on Central.",
            "central_only": true,
        }));
    }
    let j = state.journal.lock().await;
    let all = seo_data::load_all_fact_packs(&j);
    Json(serde_json::json!({ "fact_packs": all }))
}

async fn create_fact_pack(
    State(state): State<PipelineState>,
    Json(mut body): Json<seo_data::FactPack>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    if body.id.is_empty() {
        body.id = ulid::Ulid::new().to_string();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if body.created_at == 0 {
        body.created_at = now;
    }
    body.updated_at = now;

    let mut j = state.journal.lock().await;
    match seo_data::persist_fact_pack(&mut j, &body) {
        Ok(()) => (
            axum::http::StatusCode::CREATED,
            Json(serde_json::json!(body)),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

#[derive(Deserialize)]
struct UpdateFactPackReq {
    #[serde(default)]
    subject_slug: Option<String>,
    #[serde(default)]
    subject_type: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    industry_slugs: Option<Vec<String>>,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    sources: Option<Vec<seo_data::FactSource>>,
    #[serde(default)]
    active: Option<bool>,
}

async fn update_fact_pack(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateFactPackReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    let mut j = state.journal.lock().await;
    let fp = match seo_data::load_fact_pack(&j, &id) {
        Some(f) => f,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Fact pack not found"})),
            );
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut updated = fp;
    if let Some(s) = body.subject_slug {
        updated.subject_slug = s;
    }
    if let Some(st) = body.subject_type {
        updated.subject_type = st;
    }
    if let Some(t) = body.title {
        updated.title = t;
    }
    if let Some(is) = body.industry_slugs {
        updated.industry_slugs = is;
    }
    if let Some(d) = body.data {
        updated.data = d;
    }
    if let Some(sr) = body.sources {
        updated.sources = sr;
    }
    if let Some(a) = body.active {
        updated.active = a;
    }
    updated.updated_at = now;

    match seo_data::persist_fact_pack(&mut j, &updated) {
        Ok(()) => (axum::http::StatusCode::OK, Json(serde_json::json!(updated))),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn delete_fact_pack_handler(
    State(state): State<PipelineState>,
    Path(id): Path<String>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    if !reference_data_managed_on_this_node(&state) {
        return reference_data_forbidden_response();
    }
    let mut j = state.journal.lock().await;
    match seo_data::delete_fact_pack(&mut j, &id) {
        Ok(()) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({"deleted": true})),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — Generation
// ══════════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct GenerateReq {
    page_type: String,
    target_slug: String,
    #[serde(default = "default_quality")]
    quality_level: String,
}

async fn generate_single(
    State(state): State<PipelineState>,
    Json(body): Json<GenerateReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    // Select AI client
    let ai_ref = match models::select_client_from_opt(&state.ai_client, &body.quality_level) {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({"error": e})),
            );
        }
    };
    let ai_client = ai_ref.as_ref();

    // Assemble context (lock journal, clone data out, release lock)
    let (ctx_result, tpl) = {
        let j = state.journal.lock().await;
        let ctx = context::assemble_context(&j, &body.page_type, &body.target_slug);
        let tpl = templates::find_template(&j, &body.page_type, "");
        (ctx, tpl)
    };

    let ctx = match ctx_result {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e})),
            );
        }
    };

    let tpl = match tpl {
        Some(t) => t,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": format!("No template found for page type: {}", body.page_type)}),
                ),
            );
        }
    };

    // Generate content (no lock held during AI call)
    let content = match generator::generate_page(ai_client, &tpl, &ctx, &body.quality_level).await {
        Ok(c) => c,
        Err(e) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e})),
            );
        }
    };

    // Create a job record
    let job_id = ulid::Ulid::new().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let job = jobs::ContentJob {
        id: job_id.clone(),
        page_type: body.page_type.clone(),
        target_slug: body.target_slug.clone(),
        quality_level: body.quality_level.clone(),
        model_used: content.model_used.clone(),
        status: "review".to_string(),
        prompt_context_json: serde_json::to_string(&ctx).unwrap_or_default(),
        generated_content: content.html.clone(),
        token_count: content.token_count,
        generation_time_ms: content.generation_time_ms,
        created_at: now,
        ..Default::default()
    };

    // Persist the job
    {
        let mut j = state.journal.lock().await;
        let _ = jobs::persist_job(&mut j, &job);
    }

    (
        axum::http::StatusCode::OK,
        Json(serde_json::json!({
            "job_id": job_id,
            "generated_content": content.html,
            "token_count": content.token_count,
            "generation_time_ms": content.generation_time_ms,
            "model_used": content.model_used,
            "sections": content.sections.len(),
        })),
    )
}

#[derive(Deserialize)]
struct GenerateSiteReq {
    #[serde(default = "default_quality")]
    quality_level: String,
}

async fn generate_site(
    State(state): State<PipelineState>,
    Json(body): Json<GenerateSiteReq>,
) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    // Validate AI client availability
    let ai_client = match &state.ai_client {
        Some(c) => Arc::clone(c),
        None => {
            // Try cloud client
            match models::cloud_client() {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    return (
                        axum::http::StatusCode::SERVICE_UNAVAILABLE,
                        Json(
                            serde_json::json!({"error": format!("No AI client available: {}", e)}),
                        ),
                    );
                }
            }
        }
    };

    let journal = state.journal.clone();
    let quality = body.quality_level.clone();

    // Spawn batch generation as a background task
    let job_ids =
        tokio::spawn(
            async move { batch::generate_entire_site(journal, &ai_client, &quality).await },
        );

    // Return immediately with the promise of jobs
    match job_ids.await {
        Ok(ids) => (
            axum::http::StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "message": "Site generation started",
                "job_ids": ids,
                "total_pages": ids.len(),
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Batch generation failed: {}", e)})),
        ),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Route handlers — Stats
// ══════════════════════════════════════════════════════════════════════

async fn pipeline_stats(State(state): State<PipelineState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let all_jobs = jobs::load_all_jobs(&j);
    let all_templates = templates::load_all_templates(&j);
    let all_guidelines = seo_data::load_all_guidelines(&j);
    let all_packs = seo_data::load_all_fact_packs(&j);

    let total_tokens: u64 = all_jobs.iter().map(|j| j.token_count as u64).sum();
    let total_time_ms: u64 = all_jobs.iter().map(|j| j.generation_time_ms).sum();

    let mut status_counts = std::collections::HashMap::new();
    for job in &all_jobs {
        *status_counts.entry(job.status.clone()).or_insert(0u32) += 1;
    }

    Json(serde_json::json!({
        "jobs": {
            "total": all_jobs.len(),
            "by_status": status_counts,
            "total_tokens": total_tokens,
            "total_generation_time_ms": total_time_ms,
        },
        "templates": {
            "total": all_templates.len(),
            "active": all_templates.iter().filter(|t| t.active).count(),
        },
        "seo_guidelines": {
            "total": all_guidelines.len(),
            "active": all_guidelines.iter().filter(|g| g.active).count(),
        },
        "fact_packs": {
            "total": all_packs.len(),
            "active": all_packs.iter().filter(|f| f.active).count(),
        },
    }))
}

// ── Native content import (apex migration) ──────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct NativeImportPage {
    pub content_type: String,
    pub title: String,
    pub slug: String,
    pub body_json: String,
    #[serde(default)]
    pub excerpt: String,
    #[serde(default)]
    pub seo_title: String,
    #[serde(default)]
    pub seo_description: String,
    #[serde(default)]
    pub focus_keyword: String,
    #[serde(default)]
    pub og_image: String,
    #[serde(default)]
    pub canonical_url: String,
    #[serde(default)]
    pub schema_json: String,
    #[serde(default)]
    pub robots: String,
}

#[derive(serde::Serialize)]
pub(crate) struct ImportResult {
    ok: bool,
    message: String,
    imported: usize,
    skipped: usize,
}

pub(crate) async fn import_native_pages(
    State(state): State<PipelineState>,
    Json(pages): Json<Vec<NativeImportPage>>,
) -> Json<ImportResult> {
    use luperiq_forge::{ApexEvent, ForgeContent, ForgeContentManager};

    let mut j = state.journal.lock().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let mut imported = 0usize;
    let mut skipped = 0usize;

    let existing_slugs: std::collections::HashSet<String> = {
        let mgr = ForgeContentManager::new(&mut j);
        mgr.list_content(None, None, None, 10_000, 0, None, None)
            .unwrap_or_default()
            .0
            .into_iter()
            .map(|c| c.slug)
            .collect()
    };

    for page in pages {
        if page.title.is_empty() || page.slug.is_empty() {
            skipped += 1;
            continue;
        }
        if existing_slugs.contains(&page.slug) {
            skipped += 1;
            continue;
        }

        let content = ForgeContent {
            content_id: String::new(),
            content_type: if page.content_type.is_empty() { "page".into() } else { page.content_type.clone() },
            title: page.title.clone(),
            slug: page.slug.clone(),
            body_json: page.body_json.clone(),
            excerpt: if page.excerpt.is_empty() { None } else { Some(page.excerpt.clone()) },
            author_id: "migrate".into(),
            status: "published".into(),
            created_at: now,
            updated_at: now,
            published_at: Some(now),
        };

        let mut mgr = ForgeContentManager::new(&mut j);
        let content_id = match mgr.create_content(&content) {
            Ok(id) => id,
            Err(e) => {
                eprintln!("import-native: create_content failed for {}: {e}", page.slug);
                skipped += 1;
                continue;
            }
        };

        if !page.seo_title.is_empty() || !page.seo_description.is_empty() || !page.canonical_url.is_empty() {
            let seo = serde_json::json!({
                "content_id": content_id,
                "title": page.seo_title,
                "description": page.seo_description,
                "focus_keyword": page.focus_keyword,
                "og_image": page.og_image,
                "canonical_url": page.canonical_url,
                "schema_json": page.schema_json,
                "robots": page.robots,
                "seo_score": 0u8,
            });
            if let Ok(payload) = serde_json::to_vec(&seo) {
                let event = ApexEvent::new("SeoMeta", &content_id, payload);
                let _ = j.append(event);
            }
        }

        imported += 1;
    }

    Json(ImportResult {
        ok: true,
        message: format!("imported {imported}, skipped {skipped}"),
        imported,
        skipped,
    })
}
