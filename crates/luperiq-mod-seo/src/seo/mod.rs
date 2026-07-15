//! LuperIQ SEO Insights Module — per-page SEO meta, bulk import, and sitemap.
//!
//! Stores SEO metadata (title, description) per content ID in ForgeJournal
//! via the `SeoMeta` aggregate type. Provides REST API endpoints for CRUD,
//! bulk import (resolving slugs to content IDs), and an XML sitemap.
//!
//! Security notes:
//! - Admin UI uses DOM methods (no innerHTML) for XSS safety
//! - Sitemap only includes published pages

pub mod ab_seo;
pub mod admin_js;
pub mod content_queue;
pub mod crawl_summary;
pub mod crawl_tracker;
pub mod google;
pub mod handlers;
pub mod intelligence;
pub mod keyword_gate;
pub mod link_checker;
pub mod photo_library;
pub mod photo_library_js;
pub mod scoring;
pub mod surfer;
pub mod surfer_admin_js;
pub mod surfer_handlers;
pub mod surfer_map;
pub mod surfer_scoring;
pub mod timeline_js;
pub mod tracker;
pub mod verified;
pub mod verified_admin_js;
pub mod verified_handlers;
pub mod structured_data;

use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use luperiq_module_api::{
    AdminView, AiFeatureConfig, AppContext, CmsModule, NexusNetworkConfig, SharedJournal,
};

use admin_js::SEO_ADMIN_JS;

// ---------------------------------------------------------------------------
// AI provider abstraction
// ---------------------------------------------------------------------------

/// Response from an AI generation call.
pub struct SeoAiResponse {
    pub content: String,
}

/// Trait for AI text generation, abstracting the CMS's `AiClient`.
///
/// The CMS wires in the real implementation; without one, AI endpoints return
/// "AI not configured".
pub trait SeoAiProvider: Send + Sync + 'static {
    fn generate(
        &self,
        system: &str,
        user_message: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SeoAiResponse, String>> + Send + '_>,
    >;
}

/// Shared handle to an optional AI provider.
pub type OptSeoAiProvider = Option<Arc<dyn SeoAiProvider>>;

// ---------------------------------------------------------------------------
// Sitemap extension provider
// ---------------------------------------------------------------------------

/// Trait for providing custom sitemap entries from modules outside this crate.
///
/// The CMS wires in an implementation that knows about salon, brooke_grace,
/// KNOWN_MODULES, etc. Without one, sitemaps contain only pages/posts.
pub trait SitemapExtProvider: Send + Sync + 'static {
    /// Return extra sitemap entries for a given host.
    /// Each entry is (path, changefreq, priority).
    fn custom_entries_for_host(
        &self,
        host: &str,
        journal: &luperiq_forge::ForgeJournal,
    ) -> Option<Vec<(String, String, String)>>;

    /// Return marketing-specific sitemap entries (modules, AI workflows).
    fn marketing_entries(&self) -> Vec<(String, String, String)>;
}

/// Shared handle to an optional sitemap extension provider.
pub type OptSitemapExtProvider = Option<Arc<dyn SitemapExtProvider>>;

// ── Shared state for all SEO handlers ────────────────────────────────

#[derive(Clone)]
pub(crate) struct SeoState {
    pub(crate) journal: SharedJournal,
    pub(crate) ai_provider: OptSeoAiProvider,
    pub(crate) nexus_config: Option<NexusNetworkConfig>,
    /// JWT secret used by the photo-review routes (and any future SEO route
    /// that does its own per-capability check inside the handler — see
    /// `photo_library::resolve_seo_reviewer`).
    pub(crate) jwt_secret: String,
}

#[derive(Clone)]
pub(crate) struct SeoPublicState {
    pub(crate) journal: SharedJournal,
    pub(crate) site_type: String,
    pub(crate) sitemap_ext: OptSitemapExtProvider,
}

// ── Aggregate type for SEO meta in ForgeJournal ──────────────────────

pub const AGG_SEO_META: &str = "SeoMeta";

/// Tombstone value used when SEO meta is deleted.
pub const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Module definition ─────────────────────────────────────────────────

pub struct SeoModule {
    pub ai_provider: OptSeoAiProvider,
    pub sitemap_ext: OptSitemapExtProvider,
}

impl CmsModule for SeoModule {
    fn slug(&self) -> &str {
        "seo"
    }
    fn name(&self) -> &str {
        "SEO Insights"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Per-page SEO meta management, bulk import, and XML sitemap generation."
    }
    fn category(&self) -> &str {
        "Content"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(seo_router(
            ctx,
            self.ai_provider.clone(),
            self.sitemap_ext.clone(),
        ))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![
            AdminView {
                id: "seo-dashboard".into(),
                label: "SEO Dashboard".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-meta".into(),
                label: "Page Editor".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-health".into(),
                label: "Site Health".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-redirects".into(),
                label: "Redirect Manager".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-bulk".into(),
                label: "Bulk Optimizer".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-google".into(),
                label: "Google Overview".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-ga4".into(),
                label: "Analytics (GA4)".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-gsc".into(),
                label: "Search Console".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-ads".into(),
                label: "Google Ads".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-google-settings".into(),
                label: "Google Settings".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-sitemap".into(),
                label: "Sitemap Manager".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-link-checker".into(),
                label: "Link Checker".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-timeline".into(),
                label: "Change Timeline".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-crawl-tracker".into(),
                label: "Crawl Tracker".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-surfer".into(),
                label: "Surfer Sheets".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-mapping".into(),
                label: "Page Mapping".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-queue".into(),
                label: "AI Queue".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-verified".into(),
                label: "Verified Content".into(),
                section: "SEO".into(),
            },
            AdminView {
                id: "seo-photo-review".into(),
                label: "Photo Review".into(),
                section: "SEO".into(),
            },
        ]
    }

    fn admin_js(&self) -> Option<String> {
        Some(format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}",
            SEO_ADMIN_JS,
            google::admin_js::GOOGLE_ADMIN_JS,
            timeline_js::SEO_TIMELINE_JS,
            crawl_tracker::CRAWL_TRACKER_ADMIN_JS,
            surfer_admin_js::SURFER_ADMIN_JS,
            verified_admin_js::VERIFIED_ADMIN_JS,
            photo_library_js::PHOTO_REVIEW_ADMIN_JS,
        ))
    }
}

// ── Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoMeta {
    pub content_id: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub og_image: String,
    #[serde(default)]
    pub canonical_url: String,
    #[serde(default)]
    pub robots: String,
    #[serde(default)]
    pub schema_json: String,
    #[serde(default)]
    pub focus_keyword: String,
    #[serde(default)]
    pub seo_score: u8,
}

#[derive(Serialize)]
pub(crate) struct ApiResult {
    pub(crate) ok: bool,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub(crate) struct SeoMetaPayload {
    pub(crate) title: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) og_image: String,
    #[serde(default)]
    pub(crate) canonical_url: String,
    #[serde(default)]
    pub(crate) robots: String,
    #[serde(default)]
    pub(crate) schema_json: String,
    #[serde(default)]
    pub(crate) focus_keyword: String,
}

#[derive(Deserialize)]
pub(crate) struct BulkImportPayload {
    pub(crate) items: Vec<BulkImportItem>,
}

#[derive(Deserialize)]
pub(crate) struct BulkImportItem {
    pub(crate) slug: String,
    #[serde(default)]
    pub(crate) new_slug: Option<String>,
    pub(crate) title: String,
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) focus_keyword: String,
}

// ── Router ────────────────────────────────────────────────────────────

fn seo_router(
    ctx: &AppContext,
    ai_provider: OptSeoAiProvider,
    _sitemap_ext: OptSitemapExtProvider,
) -> Router {
    // Register AI features in the shared registry
    {
        let features = ctx.ai_features.clone();
        tokio::task::spawn(async move {
            let mut reg = features.lock().await;
            reg.insert("seo_ai_meta".into(), AiFeatureConfig {
                system_prompt: "You are an SEO expert. Based on the provided page content and focus keyword, generate an optimized SEO title (50-60 chars) and meta description (150-160 chars). Return as JSON: {\"title\": \"...\", \"description\": \"...\"}".to_string(),
                max_input_len: 4000,
                credit_cost: 2,
                escalation_credit_cost: 1,
                result_parser: |s| {
                    let trimmed = s.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {e}"))
                },
            });
            reg.insert("seo_ai_schema".into(), AiFeatureConfig {
                system_prompt: "You are a structured data expert. Based on the provided page content and type, generate valid schema.org JSON-LD markup. Include @context, @type, and all relevant properties. Return only the JSON-LD object (no markdown fences).".to_string(),
                max_input_len: 4000,
                credit_cost: 3,
                escalation_credit_cost: 1,
                result_parser: |s| {
                    let trimmed = s.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {e}"))
                },
            });
        });
    }

    let state = SeoState {
        journal: ctx.journal.clone(),
        ai_provider,
        nexus_config: ctx.nexus_config.clone(),
        jwt_secret: ctx.jwt_secret.clone(),
    };

    let google_state = google::oauth::GoogleState {
        journal: ctx.journal.clone(),
        nexus_config: ctx.nexus_config.clone(),
    };

    let seo_routes = Router::new()
        .route("/api/modules/seo/meta", get(handlers::list_meta))
        .route(
            "/api/modules/seo/meta/{content_id}",
            get(handlers::get_meta).put(handlers::set_meta),
        )
        .route(
            "/api/modules/seo/meta/{content_id}/slug-check",
            get(handlers::slug_check),
        )
        .route(
            "/api/modules/seo/meta/{content_id}/slug",
            axum::routing::put(handlers::slug_change),
        )
        .route(
            "/api/modules/seo/score/{content_id}",
            get(handlers::score_content),
        )
        .route("/api/modules/seo/health", get(handlers::site_health))
        .route("/api/modules/seo/import", post(handlers::bulk_import))
        .route("/api/modules/seo/export", get(handlers::seo_export))
        .route(
            "/api/modules/seo/redirects",
            get(handlers::list_redirects).post(handlers::create_redirect),
        )
        .route(
            "/api/modules/seo/redirects/{redirect_id}",
            axum::routing::delete(handlers::delete_redirect).put(handlers::update_redirect),
        )
        // AI endpoints (Pro)
        .route(
            "/api/modules/seo/ai/title",
            post(handlers::ai_generate_title),
        )
        .route(
            "/api/modules/seo/ai/description",
            post(handlers::ai_generate_description),
        )
        .route(
            "/api/modules/seo/ai/schema",
            post(handlers::ai_generate_schema),
        )
        .route(
            "/api/modules/seo/ai/keywords",
            post(handlers::ai_suggest_keywords),
        )
        .route("/api/modules/seo/ai/bulk", post(handlers::ai_bulk_optimize))
        .route(
            "/api/modules/seo/ai/brief",
            post(handlers::ai_generate_brief),
        )
        .route(
            "/api/modules/seo/insights",
            post(handlers::generate_insights),
        )
        .route(
            "/api/modules/seo/link-check",
            post(link_checker::check_links),
        )
        .route("/api/modules/seo/timeline", get(handlers::seo_timeline))
        .route(
            "/api/modules/seo/ai/timeline-analysis",
            post(handlers::ai_timeline_analysis),
        )
        // Keyword check
        .route(
            "/api/modules/seo/keyword-check/{content_id}",
            get(handlers::keyword_check),
        )
        // A/B experiment CRUD
        .route("/api/modules/seo/ab/create", post(handlers::ab_create))
        .route("/api/modules/seo/ab/list", get(handlers::ab_list))
        .route(
            "/api/modules/seo/ab/{experiment_id}",
            get(handlers::ab_detail),
        )
        .route(
            "/api/modules/seo/ab/{experiment_id}/complete",
            post(handlers::ab_complete),
        )
        .route(
            "/api/modules/seo/ab/{experiment_id}/cancel",
            post(handlers::ab_cancel),
        )
        // Crawl tracker endpoints
        .route(
            "/api/modules/seo/crawl-summary",
            get(crawl_summary::crawl_summary),
        )
        .route(
            "/api/modules/seo/crawl-stats",
            get(crawl_tracker::crawl_stats),
        )
        .route("/api/modules/seo/crawl-log", get(crawl_tracker::crawl_log))
        .route(
            "/api/modules/seo/crawl-events",
            get(crawl_tracker::crawl_events),
        )
        .route(
            "/api/modules/seo/missing-pages",
            get(crawl_tracker::missing_pages),
        )
        .route(
            "/api/modules/seo/page-discovery",
            get(crawl_tracker::page_discovery),
        )
        // ── Surfer sheet CRUD ──────────────────────────────────────────
        .route(
            "/api/modules/seo/surfer/sheets",
            get(surfer_handlers::list_sheets),
        )
        // Static segment "upload" MUST come before the ":id" catch-all.
        .route(
            "/api/modules/seo/surfer/sheets/upload",
            post(surfer_handlers::upload_sheet),
        )
        .route(
            "/api/modules/seo/surfer/sheets/{id}",
            get(surfer_handlers::get_sheet).delete(surfer_handlers::delete_sheet_handler),
        )
        .route(
            "/api/modules/seo/surfer/import-dir",
            post(surfer_handlers::import_dir),
        )
        // ── Surfer mapping ─────────────────────────────────────────────
        // Static segments must precede the ":content_id" catch-all.
        .route(
            "/api/modules/seo/surfer/map/unmapped",
            get(surfer_handlers::unmapped_pages),
        )
        .route(
            "/api/modules/seo/surfer/map/suggest/{content_id}",
            get(surfer_handlers::suggest_map),
        )
        .route(
            "/api/modules/seo/surfer/map/{content_id}",
            get(surfer_handlers::get_map).put(surfer_handlers::set_map),
        )
        .route(
            "/api/modules/seo/surfer/auto-map",
            post(surfer_handlers::auto_map),
        )
        // ── AI work queue ──────────────────────────────────────────────
        .route(
            "/api/modules/seo/surfer/queue",
            get(surfer_handlers::list_queue),
        )
        // Static segments must precede the ":content_id" catch-all.
        .route(
            "/api/modules/seo/surfer/queue/generate",
            post(surfer_handlers::generate_queue),
        )
        .route(
            "/api/modules/seo/surfer/queue/next",
            get(surfer_handlers::queue_next),
        )
        .route(
            "/api/modules/seo/surfer/queue/stats",
            get(surfer_handlers::queue_stats),
        )
        .route(
            "/api/modules/seo/surfer/queue/{content_id}/status",
            axum::routing::put(surfer_handlers::update_queue_status),
        )
        .route(
            "/api/modules/seo/surfer/queue/{content_id}/approve",
            post(surfer_handlers::approve_queue_item),
        )
        // ── Page intelligence ──────────────────────────────────────────
        .route(
            "/api/modules/seo/intelligence/{content_id}",
            get(surfer_handlers::page_intelligence),
        )
        // ── Verified content ───────────────────────────────────────────
        // Static segment "bulk-drift" MUST come before the ":content_id" catch-all.
        .route(
            "/api/modules/seo/verified/bulk-drift",
            post(verified_handlers::bulk_drift),
        )
        .route(
            "/api/modules/seo/verified/{content_id}/verify",
            post(verified_handlers::verify_handler),
        )
        .route(
            "/api/modules/seo/verified/{content_id}/drift",
            get(verified_handlers::drift_handler),
        )
        .route(
            "/api/modules/seo/verified/{content_id}",
            get(verified_handlers::get_verified),
        )
        .route(
            "/api/modules/seo/verified",
            get(verified_handlers::list_verified),
        )
        // ── Photo Library (Phase 7 / 2026-05-27) ───────────────────────
        // Per-handler capability gate (`tenant.seo.review`) lives in
        // `photo_library::resolve_seo_reviewer`.
        .route(
            "/api/modules/seo/photo-review",
            get(photo_library::list_for_review),
        )
        .route(
            "/api/modules/seo/photo-library",
            get(photo_library::list_library),
        )
        .route(
            "/api/modules/seo/photo-review/{photo_id}/approve",
            post(photo_library::approve_handler),
        )
        .route(
            "/api/modules/seo/photo-review/{photo_id}/reject",
            post(photo_library::reject_handler),
        )
        .route(
            "/admin/seo/photo-review",
            get(photo_library::admin_page),
        )
        .with_state(state);

    let google_routes = google::oauth::google_router(google_state);

    seo_routes.merge(google_routes)
}

/// Build the sitemap + robots.txt router (mounted at root level in main.rs).
pub fn sitemap_router(
    journal: SharedJournal,
    site_type: String,
    sitemap_ext: OptSitemapExtProvider,
) -> Router {
    let state = SeoPublicState {
        journal,
        site_type,
        sitemap_ext,
    };
    Router::new()
        .route("/sitemap.xml", get(handlers::sitemap_handler))
        .route("/sitemap_index.xml", get(handlers::sitemap_index_handler))
        .route("/robots.txt", get(handlers::robots_handler))
        .with_state(state)
}

// ── Public helper for page rendering ──────────────────────────────────

/// Look up SEO meta for a given content_id. Used by pages.rs to inject into templates.
/// Checks for active A/B experiments and substitutes the appropriate variant.
pub fn lookup_seo_meta(journal: &luperiq_forge::ForgeJournal, content_id: &str) -> Option<SeoMeta> {
    let mut meta: SeoMeta = journal
        .get_latest(AGG_SEO_META, content_id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice(&e.payload).ok())?;

    // Check for active A/B experiments and override the tested field
    let experiments = ab_seo::running_experiments(journal);
    if !experiments.is_empty() {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

        if let Some(val) = ab_seo::get_experiment_override(
            &experiments,
            content_id,
            &ab_seo::SeoAbField::Title,
            &today,
        ) {
            meta.title = val;
        }
        if let Some(val) = ab_seo::get_experiment_override(
            &experiments,
            content_id,
            &ab_seo::SeoAbField::Description,
            &today,
        ) {
            meta.description = val;
        }
        if let Some(val) = ab_seo::get_experiment_override(
            &experiments,
            content_id,
            &ab_seo::SeoAbField::FocusKeyword,
            &today,
        ) {
            meta.focus_keyword = val;
        }
        if let Some(val) = ab_seo::get_experiment_override(
            &experiments,
            content_id,
            &ab_seo::SeoAbField::Schema,
            &today,
        ) {
            meta.schema_json = val;
        }
    }

    Some(meta)
}

// ── Helpers ───────────────────────────────────────────────────────────

pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
