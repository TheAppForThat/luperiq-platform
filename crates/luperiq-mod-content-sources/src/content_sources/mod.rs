//! Content Sourcing module — manages multiple content sources for page generation.
//!
//! Customers can use LuperIQ fact sheets, upload their own content, scrape their
//! existing sites, or commission new fact sheets at two quality tiers.
//!
//! ## Architecture
//!
//! This is Layer 1 of a three-layer vision:
//! - Layer 1 (this module): Content sourcing, conflict detection, pricing config
//! - Layer 2 (roadmap): Contributor network — fact-checker accounts, quality scoring
//! - Layer 3 (roadmap): Credit marketplace — peer-to-peer credit transfers
//!
//! ## Future Integration Points
//!
//! - LAYER 2: content_type_tag on ContentSource supports "story", "expansion", "correction"
//! - LAYER 2: quality_score populated by QualityReview aggregate
//! - LAYER 2: contributor_id links to ContributorProfile aggregate
//! - LAYER 3: transferable + credit_value enable marketplace listings

pub mod admin_css;
pub mod admin_js;
pub mod conflict;
pub mod pricing;
pub mod query;
pub mod types;
pub mod upload;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    routing::{delete, get, post, put},
    Json, Router,
};

use luperiq_forge::ApexEvent;
use luperiq_module_api::{AdminView, AppContext, CmsModule, NexusNetworkConfig, SharedJournal};

pub use conflict::*;
pub use pricing::PageGenPricing;
pub use types::*;

// ── Provider trait for cross-module hooks ────────────────────────────

/// Optional hooks into content-review and content-validation pipelines.
/// The CMS glue file wires in the real implementations; the extracted
/// crate defaults to no-ops.
pub trait ContentSourcesHooks: Send + Sync + 'static {
    /// Fire an audit trail entry (content-review pipeline).
    fn audit_source_created(
        &self,
        _journal: &mut luperiq_forge::ForgeJournal,
        _source_id: &str,
        _contributor: &str,
        _metadata: serde_json::Value,
    ) {
    }

    /// Submit conflict feedback to Central (content-validation pipeline).
    fn submit_conflict_feedback(
        &self,
        _nexus: &NexusNetworkConfig,
        _luperiq_source_id: &str,
        _field_name: &str,
        _customer_value: &str,
    ) {
    }

    /// Submit source for validation (content-validation pipeline).
    fn submit_for_validation(
        &self,
        _nexus: &NexusNetworkConfig,
        _source: &ContentSource,
        _instance_url: &str,
    ) {
    }
}

/// Default no-op hooks.
pub struct NoopContentSourcesHooks;
impl ContentSourcesHooks for NoopContentSourcesHooks {}

// ── Module definition ────────────────────────────────────────────────

pub struct ContentSourcesModule {
    pub hooks: Arc<dyn ContentSourcesHooks>,
}

impl Default for ContentSourcesModule {
    fn default() -> Self {
        Self {
            hooks: Arc::new(NoopContentSourcesHooks),
        }
    }
}

impl CmsModule for ContentSourcesModule {
    fn slug(&self) -> &str {
        "content-sources"
    }
    fn name(&self) -> &str {
        "Content Sources"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Manage content sources for AI page generation — uploads, scrapes, fact sheets"
    }
    fn category(&self) -> &str {
        "Content"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        let state = ContentSourcesState {
            journal: ctx.journal.clone(),
            nexus_config: ctx.nexus_config.clone(),
            hooks: self.hooks.clone(),
        };

        let router = Router::new()
            .route("/api/modules/content-sources/sources", get(list_sources))
            .route("/api/modules/content-sources/sources", post(create_source))
            .route(
                "/api/modules/content-sources/sources/{source_id}",
                get(get_source),
            )
            .route(
                "/api/modules/content-sources/sources/{source_id}",
                put(update_source),
            )
            .route(
                "/api/modules/content-sources/sources/{source_id}",
                delete(delete_source),
            )
            .route("/api/modules/content-sources/pricing", get(get_pricing))
            .route("/api/modules/content-sources/pricing", put(update_pricing))
            .route(
                "/api/modules/content-sources/for-topic",
                get(sources_for_topic),
            )
            .route("/api/modules/content-sources/upload", post(upload_file))
            .route(
                "/api/modules/content-sources/commission",
                post(commission_source),
            )
            .route(
                "/api/modules/content-sources/conflicts",
                get(list_conflicts),
            )
            .route(
                "/api/modules/content-sources/conflicts/{conflict_id}",
                put(resolve_conflict),
            )
            .route(
                "/api/modules/content-sources/conflicts/{conflict_id}",
                delete(delete_conflict),
            )
            .route(
                "/api/modules/content-sources/sources/{source_id}/sharing",
                put(update_sharing),
            )
            .with_state(state);

        Some(router)
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![AdminView {
            id: "content-library".to_string(),
            label: "Content Library".to_string(),
            section: "Content".to_string(),
        }]
    }

    fn admin_js(&self) -> Option<String> {
        Some(admin_js::CONTENT_SOURCES_ADMIN_JS.to_string())
    }

    fn admin_css(&self) -> Option<String> {
        Some(admin_css::CONTENT_SOURCES_ADMIN_CSS.to_string())
    }
}

// ── State ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ContentSourcesState {
    journal: SharedJournal,
    nexus_config: Option<NexusNetworkConfig>,
    hooks: Arc<dyn ContentSourcesHooks>,
}

// ── API response helper ──────────────────────────────────────────────

#[derive(serde::Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

// ── Handlers ─────────────────────────────────────────────────────────

/// GET /api/modules/content-sources/sources?industry=slug&topic=slug
async fn list_sources(
    State(state): State<ContentSourcesState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_CONTENT_SOURCE);

    let sources: Vec<ContentSource> = events
        .into_iter()
        .filter(|e| e.payload != CONTENT_SOURCE_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentSource>(&e.payload).ok())
        .filter(|s| {
            let industry_match = params
                .get("industry")
                .map_or(true, |i| s.industry_slug == *i);
            let topic_match = params.get("topic").map_or(true, |t| s.topic_slug == *t);
            industry_match && topic_match
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} content sources", sources.len()),
        data: Some(serde_json::json!(sources)),
    })
}

/// POST /api/modules/content-sources/sources
async fn create_source(
    State(state): State<ContentSourcesState>,
    axum::extract::Json(source): axum::extract::Json<ContentSource>,
) -> Json<ApiResult> {
    let payload = match serde_json::to_vec(&source) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            })
        }
    };

    let source_id = source.source_id.clone();
    let contributor = source.contributor_id.clone().unwrap_or_default();
    let event = ApexEvent::new(AGG_CONTENT_SOURCE, &source_id, payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => {
            // Fire audit trail entry for content review pipeline
            state.hooks.audit_source_created(
                &mut j,
                &source_id,
                &contributor,
                serde_json::json!({
                    "industry_slug": source.industry_slug,
                    "content_type_tag": source.content_type_tag,
                }),
            );
            Json(ApiResult {
                ok: true,
                message: "Content source created".into(),
                data: Some(serde_json::json!({ "source_id": source_id })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        }),
    }
}

/// GET /api/modules/content-sources/sources/{source_id}
async fn get_source(
    State(state): State<ContentSourcesState>,
    axum::extract::Path(source_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_CONTENT_SOURCE);

    let source = events
        .into_iter()
        .filter(|e| e.aggregate_id == source_id && e.payload != CONTENT_SOURCE_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentSource>(&e.payload).ok())
        .next();

    match source {
        Some(s) => Json(ApiResult {
            ok: true,
            message: "Found".into(),
            data: Some(serde_json::to_value(&s).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: "Content source not found".into(),
            data: None,
        }),
    }
}

/// PUT /api/modules/content-sources/sources/{source_id}
async fn update_source(
    State(state): State<ContentSourcesState>,
    axum::extract::Path(source_id): axum::extract::Path<String>,
    axum::extract::Json(mut source): axum::extract::Json<ContentSource>,
) -> Json<ApiResult> {
    source.source_id = source_id.clone();
    let payload = match serde_json::to_vec(&source) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            })
        }
    };

    let event = ApexEvent::new(AGG_CONTENT_SOURCE, &source_id, payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: "Content source updated".into(),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to update: {e}"),
            data: None,
        }),
    }
}

/// DELETE /api/modules/content-sources/sources/{source_id}
async fn delete_source(
    State(state): State<ContentSourcesState>,
    axum::extract::Path(source_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let event = ApexEvent::new(
        AGG_CONTENT_SOURCE,
        &source_id,
        CONTENT_SOURCE_TOMBSTONE.to_vec(),
    );
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: "Content source deleted".into(),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to delete: {e}"),
            data: None,
        }),
    }
}

/// GET /api/modules/content-sources/pricing
async fn get_pricing(State(state): State<ContentSourcesState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let pricing = pricing::load_pricing(&j);
    Json(ApiResult {
        ok: true,
        message: "Pricing config".into(),
        data: Some(serde_json::to_value(&pricing).unwrap_or_default()),
    })
}

/// PUT /api/modules/content-sources/pricing
async fn update_pricing(
    State(state): State<ContentSourcesState>,
    axum::extract::Json(pricing): axum::extract::Json<PageGenPricing>,
) -> Json<ApiResult> {
    let payload = match serde_json::to_vec(&pricing) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            })
        }
    };

    let event = ApexEvent::new(pricing::AGG_PAGE_GEN_PRICING, "default", payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: "Pricing updated".into(),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to update pricing: {e}"),
            data: None,
        }),
    }
}

/// GET /api/modules/content-sources/for-topic?industry=slug&topic=slug
async fn sources_for_topic(
    State(state): State<ContentSourcesState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let industry = params.get("industry").map(|s| s.as_str()).unwrap_or("");
    let topic = params.get("topic").map(|s| s.as_str()).unwrap_or("");

    if industry.is_empty() || topic.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Both industry and topic params required".into(),
            data: None,
        });
    }

    let j = state.journal.lock().await;
    let sources = query::get_sources_for_topic(&j, industry, topic);

    Json(ApiResult {
        ok: true,
        message: format!("{} sources for {}/{}", sources.len(), industry, topic),
        data: Some(serde_json::json!(sources)),
    })
}

/// POST /api/modules/content-sources/upload (multipart)
async fn upload_file(
    State(state): State<ContentSourcesState>,
    mut multipart: Multipart,
) -> Json<ApiResult> {
    let mut industry_slug = String::new();
    let mut topic_slug = String::new();
    let mut title = String::new();
    let mut file_content = String::new();
    let mut filename = String::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "industry_slug" => {
                industry_slug = field.text().await.unwrap_or_default();
            }
            "topic_slug" => {
                topic_slug = field.text().await.unwrap_or_default();
            }
            "title" => {
                title = field.text().await.unwrap_or_default();
            }
            "file" => {
                filename = field.file_name().unwrap_or("upload.txt").to_string();
                match field.bytes().await {
                    Ok(bytes) => {
                        if bytes.len() > 1_048_576 {
                            return Json(ApiResult {
                                ok: false,
                                message: "File too large (max 1 MB)".into(),
                                data: None,
                            });
                        }
                        file_content = String::from_utf8_lossy(&bytes).to_string();
                    }
                    Err(e) => {
                        return Json(ApiResult {
                            ok: false,
                            message: format!("Failed to read file: {e}"),
                            data: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if industry_slug.is_empty() || topic_slug.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "industry_slug and topic_slug are required".into(),
            data: None,
        });
    }

    if file_content.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "No file content received".into(),
            data: None,
        });
    }

    if title.is_empty() {
        title = filename.clone();
    }

    let format = upload::detect_format(&filename);
    let (facts, raw) = match format {
        "csv" => upload::parse_csv(&file_content),
        _ => upload::parse_text(&file_content),
    };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now = ts.as_secs();

    let source = ContentSource {
        source_id: format!("upload-{}-{}", now, ts.subsec_nanos()),
        source_type: ContentSourceType::CustomerUpload,
        industry_slug,
        topic_slug,
        title,
        structured_facts: facts,
        raw_content: raw,
        sharing_tier: SharingTier::NeverShare,
        sharing_discount_applied: false,
        validation_status: ValidationStatus::NotApplicable,
        owner_license_key: String::new(),
        created_at: now,
        updated_at: now,
        file_format: format.to_string(),
        contributor_id: None,
        contributor_payout_status: PayoutStatus::NotApplicable,
        quality_score: None,
        content_type_tag: "fact_sheet".to_string(),
        parent_source_id: None,
        transferable: false,
        credit_value: None,
    };

    let payload = match serde_json::to_vec(&source) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            });
        }
    };

    let event = ApexEvent::new(AGG_CONTENT_SOURCE, &source.source_id, payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => {
            let luperiq_sources: Vec<ContentSource> = j
                .latest_by_aggregate_type(AGG_CONTENT_SOURCE)
                .into_iter()
                .filter(|e| e.payload != CONTENT_SOURCE_TOMBSTONE)
                .filter_map(|e| serde_json::from_slice::<ContentSource>(&e.payload).ok())
                .filter(|s| {
                    s.source_type == ContentSourceType::LuperiqFactSheet
                        && s.industry_slug == source.industry_slug
                        && s.topic_slug == source.topic_slug
                })
                .collect();

            let mut conflict_count = 0;
            for liq_source in &luperiq_sources {
                if let Some(cr) = conflict::detect_conflicts(&source, liq_source) {
                    if let Ok(cp) = serde_json::to_vec(&cr) {
                        let ce = ApexEvent::new(AGG_CONTENT_CONFLICT, &cr.conflict_id, cp);
                        let _ = j.append(ce);
                        conflict_count += 1;
                    }
                }
            }

            Json(ApiResult {
                ok: true,
                message: if conflict_count > 0 {
                    format!(
                        "Content uploaded: {} facts extracted, {} conflicts detected",
                        source.structured_facts.len(),
                        conflict_count
                    )
                } else {
                    format!(
                        "Content uploaded: {} facts extracted",
                        source.structured_facts.len()
                    )
                },
                data: Some(serde_json::json!({
                    "source_id": source.source_id,
                    "facts_count": source.structured_facts.len(),
                    "conflicts_detected": conflict_count,
                })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        }),
    }
}

/// Commission request from the admin UI.
#[derive(serde::Deserialize)]
struct CommissionRequest {
    industry_slug: String,
    topic_slug: String,
    tier: String,
}

/// POST /api/modules/content-sources/commission
async fn commission_source(
    State(state): State<ContentSourcesState>,
    Json(req): Json<CommissionRequest>,
) -> Json<ApiResult> {
    if req.industry_slug.is_empty() || req.topic_slug.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "industry_slug and topic_slug are required".into(),
            data: None,
        });
    }

    let pricing = pricing::load_pricing(&*state.journal.lock().await);
    let (source_type, credits, validation) = match req.tier.as_str() {
        "ai_verified" => (
            ContentSourceType::CommissionedAiVerified,
            pricing.credits_ai_verified,
            ValidationStatus::InReview,
        ),
        "expert_reviewed" => (
            ContentSourceType::CommissionedExpertReview,
            pricing.credits_expert_reviewed,
            ValidationStatus::Pending,
        ),
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "tier must be 'ai_verified' or 'expert_reviewed'".into(),
                data: None,
            });
        }
    };

    if let Some(ref nexus) = state.nexus_config {
        let role = nexus.role.as_deref().unwrap_or("");
        if role == "client" {
            let central_url = match nexus.central_url.as_deref() {
                Some(u) => u,
                None => {
                    return Json(ApiResult {
                        ok: false,
                        message: "No central_url configured".into(),
                        data: None,
                    });
                }
            };
            let license_key = match nexus.license_key.as_deref() {
                Some(k) => k,
                None => {
                    return Json(ApiResult {
                        ok: false,
                        message: "No license_key configured".into(),
                        data: None,
                    });
                }
            };

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("reqwest Client builder failed");

            let resp = client
                .post(format!("{central_url}/api/modules/nexus/credits/deduct"))
                .json(&serde_json::json!({
                    "license_key": license_key,
                    "operation": format!("content_commission_{}", req.tier),
                    "amount": credits,
                    "module_key": "content-sources",
                }))
                .send()
                .await;

            match resp {
                Ok(r) => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        if body.get("ok").and_then(|v| v.as_bool()) != Some(true) {
                            let msg = body
                                .get("message")
                                .and_then(|v| v.as_str())
                                .unwrap_or("Insufficient credits");
                            return Json(ApiResult {
                                ok: false,
                                message: format!("Credit deduction failed: {msg}"),
                                data: None,
                            });
                        }
                    }
                }
                Err(e) => {
                    return Json(ApiResult {
                        ok: false,
                        message: format!("Credit service unavailable: {e}"),
                        data: None,
                    });
                }
            }
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now = ts.as_secs();

    let source = ContentSource {
        source_id: format!("comm-{}-{}", now, ts.subsec_nanos()),
        source_type,
        industry_slug: req.industry_slug,
        topic_slug: req.topic_slug,
        title: format!(
            "{} Fact Sheet",
            if req.tier == "ai_verified" {
                "AI Verified"
            } else {
                "Expert Reviewed"
            }
        ),
        structured_facts: vec![],
        raw_content: String::new(),
        sharing_tier: SharingTier::NeverShare,
        sharing_discount_applied: false,
        validation_status: validation,
        owner_license_key: String::new(),
        created_at: now,
        updated_at: now,
        file_format: "markdown".to_string(),
        contributor_id: None,
        contributor_payout_status: PayoutStatus::NotApplicable,
        quality_score: None,
        content_type_tag: "fact_sheet".to_string(),
        parent_source_id: None,
        transferable: false,
        credit_value: None,
    };

    let payload = match serde_json::to_vec(&source) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            });
        }
    };

    let event = ApexEvent::new(AGG_CONTENT_SOURCE, &source.source_id, payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: format!("Commissioned {} fact sheet ({} credits)", req.tier, credits),
            data: Some(serde_json::json!({
                "source_id": source.source_id,
                "credits_deducted": credits,
            })),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        }),
    }
}

// ── Conflict CRUD Handlers ────────────────────────────────────────────

/// GET /api/modules/content-sources/conflicts?source_id=xxx
async fn list_conflicts(
    State(state): State<ContentSourcesState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_CONTENT_CONFLICT);

    let conflicts: Vec<ConflictRecord> = events
        .into_iter()
        .filter(|e| e.payload != CONFLICT_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ConflictRecord>(&e.payload).ok())
        .filter(|c| {
            params
                .get("source_id")
                .map_or(true, |sid| c.source_id == *sid)
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} conflicts", conflicts.len()),
        data: Some(serde_json::json!(conflicts)),
    })
}

#[derive(serde::Deserialize)]
struct ResolveConflictRequest {
    resolution: String,
    #[serde(default)]
    customer_notes: String,
}

/// PUT /api/modules/content-sources/conflicts/{conflict_id}
async fn resolve_conflict(
    State(state): State<ContentSourcesState>,
    axum::extract::Path(conflict_id): axum::extract::Path<String>,
    Json(req): Json<ResolveConflictRequest>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_CONTENT_CONFLICT);

    let existing = events
        .into_iter()
        .filter(|e| e.aggregate_id == conflict_id && e.payload != CONFLICT_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ConflictRecord>(&e.payload).ok())
        .next();

    let mut conflict = match existing {
        Some(c) => c,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Conflict not found".into(),
                data: None,
            });
        }
    };

    conflict.resolution = match req.resolution.as_str() {
        "customer_proceeded" => ConflictResolution::CustomerProceeded,
        "customer_deferred" => ConflictResolution::CustomerDeferred,
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "resolution must be 'customer_proceeded' or 'customer_deferred'".into(),
                data: None,
            });
        }
    };
    conflict.customer_notes = req.customer_notes;
    conflict.resolved_at = Some(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );

    let payload = match serde_json::to_vec(&conflict) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            });
        }
    };

    drop(j);
    let event = ApexEvent::new(AGG_CONTENT_CONFLICT, &conflict_id, payload);
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => {
            // If the customer chose to proceed with their data, submit feedback to Central
            if conflict.resolution == ConflictResolution::CustomerProceeded {
                if let Some(ref nx) = state.nexus_config {
                    for field in &conflict.conflicting_fields {
                        state.hooks.submit_conflict_feedback(
                            nx,
                            &conflict.luperiq_source_id,
                            &field.field_name,
                            &field.customer_value,
                        );
                    }
                }
            }

            Json(ApiResult {
                ok: true,
                message: "Conflict resolved".into(),
                data: None,
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to resolve: {e}"),
            data: None,
        }),
    }
}

/// DELETE /api/modules/content-sources/conflicts/{conflict_id}
async fn delete_conflict(
    State(state): State<ContentSourcesState>,
    axum::extract::Path(conflict_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let event = ApexEvent::new(
        AGG_CONTENT_CONFLICT,
        &conflict_id,
        CONFLICT_TOMBSTONE.to_vec(),
    );
    let mut j = state.journal.lock().await;
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: "Conflict deleted".into(),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to delete: {e}"),
            data: None,
        }),
    }
}

// ── Sharing Preference Handler ────────────────────────────────────────

#[derive(serde::Deserialize)]
struct UpdateSharingRequest {
    sharing_tier: String,
}

/// PUT /api/modules/content-sources/sources/{source_id}/sharing
async fn update_sharing(
    headers: axum::http::HeaderMap,
    State(state): State<ContentSourcesState>,
    axum::extract::Path(source_id): axum::extract::Path<String>,
    Json(req): Json<UpdateSharingRequest>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_CONTENT_SOURCE);

    let existing = events
        .into_iter()
        .filter(|e| e.aggregate_id == source_id && e.payload != CONTENT_SOURCE_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentSource>(&e.payload).ok())
        .next();

    let mut source = match existing {
        Some(s) => s,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Content source not found".into(),
                data: None,
            });
        }
    };

    source.sharing_tier = match req.sharing_tier.as_str() {
        "never_share" => SharingTier::NeverShare,
        "share_anonymized" => SharingTier::ShareAnonymized,
        "share_as_trusted_source" => SharingTier::ShareAsTrustedSource,
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "sharing_tier must be 'never_share', 'share_anonymized', or 'share_as_trusted_source'".into(),
                data: None,
            });
        }
    };

    source.sharing_discount_applied = source.sharing_tier != SharingTier::NeverShare;
    // Set validation_status in the same payload so both state changes land in one WAL event.
    if source.sharing_tier != SharingTier::NeverShare {
        source.validation_status = ValidationStatus::Pending;
    }

    let payload = match serde_json::to_vec(&source) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            });
        }
    };

    let event = ApexEvent::new(AGG_CONTENT_SOURCE, &source_id, payload);
    match j.append(event) {
        Ok(_) => {

            let msg = match source.sharing_tier {
                SharingTier::ShareAsTrustedSource => "Trusted Source preference saved. Your content is being reviewed — credits will be applied within 24 hours.",
                SharingTier::ShareAnonymized => "Anonymous sharing preference saved. Your content is being reviewed — credits will be applied within 24 hours.",
                SharingTier::NeverShare => "Sharing preference saved. Your content remains private.",
            };
            let response_data = serde_json::json!({
                "sharing_tier": req.sharing_tier,
                "validation_status": format!("{:?}", source.validation_status),
            });

            drop(j);

            // Submit for validation in background
            if source.sharing_tier != SharingTier::NeverShare {
                if let Some(ref nx) = state.nexus_config {
                    let instance_url = headers
                        .get("host")
                        .and_then(|h| h.to_str().ok())
                        .map(|h| format!("https://{h}"))
                        .unwrap_or_else(|| "http://localhost:3000".to_string());
                    state
                        .hooks
                        .submit_for_validation(nx, &source, &instance_url);
                }
            }

            Json(ApiResult {
                ok: true,
                message: msg.into(),
                data: Some(response_data),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to update: {e}"),
            data: None,
        }),
    }
}
