use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::collections::HashSet;

use luperiq_forge::{ApexEvent, ForgeContentManager, ForgeSlugManager};

use super::scoring::calculate_seo_score;
// Foreign aggregate keys — local aliases until owning crates export them as pub const.
const AGG_SVC_SERVICE: &str = "SvcService";
const AGG_LOC_PROFILE: &str = "LocProf:Profile";
const AGG_COMP_PROFILE: &str = "CompProf:Profile";

use super::{
    ApiResult, BulkImportPayload, SeoMeta, SeoMetaPayload, SeoPublicState, SeoState, AGG_SEO_META,
    TOMBSTONE,
};

// ── AI payload types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SeoAiRequest {
    pub(crate) content_id: String,
}

#[derive(Deserialize)]
pub(crate) struct SeoBulkAiRequest {
    pub(crate) content_ids: Vec<String>,
}

// ── Slug change payload types ────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SlugChangePayload {
    pub(crate) new_slug: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct SlugCheckQuery {
    #[serde(default)]
    pub(crate) new_slug: Option<String>,
}

// ── A/B experiment payload types ────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct AbCreatePayload {
    pub(crate) content_id: String,
    pub(crate) field: super::ab_seo::SeoAbField,
    pub(crate) variant_b: String,
    #[serde(default)]
    pub(crate) duration_days: Option<u32>,
}

#[derive(Deserialize)]
pub(crate) struct AbCompletePayload {
    pub(crate) action: String, // "apply_winner" or "keep_control"
    #[serde(default)]
    pub(crate) winner: Option<String>, // "a" or "b" — required when action is "apply_winner"
}

#[derive(Deserialize, Default)]
pub(crate) struct AbListQuery {
    #[serde(default)]
    pub(crate) status: Option<String>,
}

// ── Timeline query types ────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct TimelineQuery {
    #[serde(default)]
    pub(crate) content_id: Option<String>,
    #[serde(default)]
    pub(crate) change_type: Option<String>,
    #[serde(default)]
    pub(crate) from: Option<String>,
    #[serde(default)]
    pub(crate) to: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<u32>,
}

// ── AI Timeline analysis types ──────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct TimelineAnalysisPayload {
    #[serde(default)]
    pub(crate) content_id: Option<String>,
}

// ── API handlers ──────────────────────────────────────────────────────

/// GET /api/modules/seo/meta — list all SEO meta entries.
pub(crate) async fn list_meta(State(state): State<SeoState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let events = j.latest_by_aggregate_type(AGG_SEO_META);

    let items: Vec<serde_json::Value> = events
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| {
            let meta: SeoMeta = serde_json::from_slice(&e.payload).ok()?;
            Some(serde_json::json!({
                "content_id": meta.content_id,
                "title": meta.title,
                "description": meta.description,
                "og_image": meta.og_image,
                "canonical_url": meta.canonical_url,
                "robots": meta.robots,
                "schema_json": meta.schema_json,
                "focus_keyword": meta.focus_keyword,
                "seo_score": meta.seo_score,
            }))
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} entries", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

/// GET /api/modules/seo/meta/:content_id — get SEO meta for a content item.
pub(crate) async fn get_meta(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    match j.get_latest(AGG_SEO_META, &content_id) {
        Some(event) if event.payload != TOMBSTONE => {
            match serde_json::from_slice::<SeoMeta>(&event.payload) {
                Ok(meta) => Json(ApiResult {
                    ok: true,
                    message: "SEO meta found".into(),
                    data: Some(serde_json::json!({
                        "content_id": meta.content_id,
                        "title": meta.title,
                        "description": meta.description,
                        "og_image": meta.og_image,
                        "canonical_url": meta.canonical_url,
                        "robots": meta.robots,
                        "schema_json": meta.schema_json,
                        "focus_keyword": meta.focus_keyword,
                        "seo_score": meta.seo_score,
                    })),
                }),
                Err(_) => Json(ApiResult {
                    ok: false,
                    message: "Failed to parse SEO meta".into(),
                    data: None,
                }),
            }
        }
        _ => Json(ApiResult {
            ok: false,
            message: "No SEO meta for this content".into(),
            data: None,
        }),
    }
}

/// PUT /api/modules/seo/meta/:content_id — set/update SEO meta.
pub(crate) async fn set_meta(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
    axum::extract::Json(payload): axum::extract::Json<SeoMetaPayload>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Look up content to calculate score
    let mgr = ForgeContentManager::new(&mut j);
    let (content_title, content_body) = mgr
        .get_content(&content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json))
        .unwrap_or_default();

    let mut meta = SeoMeta {
        content_id: content_id.clone(),
        title: payload.title,
        description: payload.description,
        og_image: payload.og_image,
        canonical_url: payload.canonical_url,
        robots: payload.robots,
        schema_json: payload.schema_json,
        focus_keyword: payload.focus_keyword,
        seo_score: 0,
    };
    meta.seo_score = calculate_seo_score(&meta, &content_title, &content_body, None);

    let payload_bytes = match serde_json::to_vec(&meta) {
        Ok(b) => b,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Serialization error: {e}"),
                data: None,
            });
        }
    };

    // ── Change tracking: detect field changes and record SeoChange events ──
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(old_event) = j.get_latest(AGG_SEO_META, &content_id) {
        if old_event.payload != TOMBSTONE {
            if let Ok(old_meta) = serde_json::from_slice::<super::SeoMeta>(&old_event.payload) {
                let changes = [
                    (
                        super::tracker::SeoChangeType::TitleChange,
                        &old_meta.title,
                        &meta.title,
                    ),
                    (
                        super::tracker::SeoChangeType::DescriptionChange,
                        &old_meta.description,
                        &meta.description,
                    ),
                    (
                        super::tracker::SeoChangeType::KeywordChange,
                        &old_meta.focus_keyword,
                        &meta.focus_keyword,
                    ),
                    (
                        super::tracker::SeoChangeType::SchemaChange,
                        &old_meta.schema_json,
                        &meta.schema_json,
                    ),
                ];
                for (change_type, old_val, new_val) in changes {
                    if old_val != new_val && !old_val.is_empty() {
                        let change = super::tracker::SeoChange {
                            content_id: content_id.clone(),
                            change_type,
                            old_value: old_val.clone(),
                            new_value: new_val.clone(),
                            snapshot_before: None,
                            ai_warning: None,
                            timestamp: now,
                        };
                        if let Err(e) = super::tracker::record_seo_change(&mut j, &change) {
                            eprintln!("[seo] Failed to record change: {e}");
                        }
                    }
                }
            }
        }
    }

    let event = ApexEvent::new(AGG_SEO_META, &content_id, payload_bytes);
    match j.append(event) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: format!(
                "SEO meta saved for {content_id} (score: {})",
                meta.seo_score
            ),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("Failed to save: {e}"),
            data: None,
        }),
    }
}

/// GET /api/modules/seo/export — export all pages/posts with slug, title, SEO meta.
pub(crate) async fn seo_export(State(state): State<SeoState>) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Get all SEO metas keyed by content_id
    let seo_events = j.latest_by_aggregate_type(AGG_SEO_META);
    let seo_map: std::collections::HashMap<String, SeoMeta> = seo_events
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<SeoMeta>(&e.payload).ok())
        .map(|m| (m.content_id.clone(), m))
        .collect();

    // Get all published pages, posts, and static pages
    let mgr = ForgeContentManager::new(&mut j);
    let mut items = Vec::new();
    for content_type in &["page", "post", "static_page"] {
        let (entries, _) = mgr
            .list_content(
                Some(content_type),
                Some("published"),
                None,
                1000,
                0,
                None,
                None,
            )
            .unwrap_or_default();
        for entry in entries {
            let seo = seo_map.get(&entry.content_id);
            items.push(serde_json::json!({
                "content_id": entry.content_id,
                "content_type": entry.content_type,
                "slug": entry.slug,
                "page_title": entry.title,
                "seo_title": seo.map(|s| s.title.as_str()).unwrap_or(""),
                "seo_description": seo.map(|s| s.description.as_str()).unwrap_or(""),
                "focus_keyword": seo.map(|s| s.focus_keyword.as_str()).unwrap_or(""),
                "seo_score": seo.map(|s| s.seo_score).unwrap_or(0),
                "og_image": seo.map(|s| s.og_image.as_str()).unwrap_or(""),
                "has_schema": seo.map(|s| !s.schema_json.is_empty()).unwrap_or(false),
                "robots": seo.map(|s| s.robots.as_str()).unwrap_or(""),
                "canonical_url": seo.map(|s| s.canonical_url.as_str()).unwrap_or(""),
                "schema_json": seo.map(|s| s.schema_json.as_str()).unwrap_or(""),
                "body_json": entry.body_json,
                "excerpt": entry.excerpt.unwrap_or_default(),
            }));
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("{} items exported", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

/// POST /api/modules/seo/import — bulk import SEO meta, resolving slugs to content IDs.
/// Supports optional `new_slug` (renames page slug, auto-creates 301 redirect)
/// and `focus_keyword` fields.
pub(crate) async fn bulk_import(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<BulkImportPayload>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Build a slug -> content_id map from all published pages AND posts
    let mut slug_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    {
        let mgr = ForgeContentManager::new(&mut j);
        for content_type in &["page", "post"] {
            if let Ok((entries, _)) = mgr.list_content(
                Some(content_type),
                Some("published"),
                None,
                1000,
                0,
                None,
                None,
            ) {
                for p in entries {
                    slug_map.insert(p.slug.clone(), p.content_id.clone());
                }
            }
        }
    }

    let mut imported = 0u32;
    let mut renamed = 0u32;
    let mut skipped = Vec::new();

    for item in &payload.items {
        let content_id = match slug_map.get(&item.slug) {
            Some(id) => id.clone(),
            None => {
                skipped.push(item.slug.clone());
                continue;
            }
        };

        // If new_slug is provided, rename the content slug (auto-creates 301 redirect)
        if let Some(ref new_slug) = item.new_slug {
            if !new_slug.is_empty() && *new_slug != item.slug {
                let mut mgr = ForgeContentManager::new(&mut j);
                match mgr.update_content(&content_id, None, Some(new_slug), None, None) {
                    Ok(_) => renamed += 1,
                    Err(_) => {
                        skipped.push(format!("rename:{}", item.slug));
                        continue;
                    }
                }
            }
        }

        // Look up content for SEO scoring
        let mgr = ForgeContentManager::new(&mut j);
        let (content_title, content_body) = mgr
            .get_content(&content_id)
            .ok()
            .flatten()
            .map(|c| (c.title, c.body_json))
            .unwrap_or_default();

        let mut meta = SeoMeta {
            content_id: content_id.clone(),
            title: item.title.clone(),
            description: item.description.clone(),
            og_image: String::new(),
            canonical_url: String::new(),
            robots: String::new(),
            schema_json: String::new(),
            focus_keyword: item.focus_keyword.clone(),
            seo_score: 0,
        };
        meta.seo_score = calculate_seo_score(&meta, &content_title, &content_body, None);

        let payload_bytes = match serde_json::to_vec(&meta) {
            Ok(b) => b,
            Err(_) => {
                skipped.push(item.slug.clone());
                continue;
            }
        };

        let event = ApexEvent::new(AGG_SEO_META, &content_id, payload_bytes);
        match j.append(event) {
            Ok(_) => imported += 1,
            Err(_) => skipped.push(item.slug.clone()),
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!(
            "{imported} imported, {renamed} renamed, {} skipped",
            skipped.len()
        ),
        data: Some(serde_json::json!({
            "imported": imported,
            "renamed": renamed,
            "skipped": skipped,
        })),
    })
}

/// GET /api/modules/seo/redirects — list all redirects.
pub(crate) async fn list_redirects(State(state): State<SeoState>) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mgr = luperiq_forge::ForgeRedirectManager::new(&mut j);
    match mgr.list_redirects(1000, 0, false) {
        Ok((items, total)) => {
            let data: Vec<serde_json::Value> = items
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "redirect_id": r.redirect_id,
                        "source": r.source_pattern,
                        "target": r.target_url,
                        "status_code": r.redirect_type,
                        "pattern_type": r.pattern_type,
                        "is_active": r.is_active,
                        "hit_count": r.hit_count,
                    })
                })
                .collect();
            Json(ApiResult {
                ok: true,
                message: format!("{total} redirects"),
                data: Some(serde_json::json!(data)),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/redirects — create a redirect.
pub(crate) async fn create_redirect(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> Json<ApiResult> {
    let source = payload["source"].as_str().unwrap_or_default();
    let target = payload["target"].as_str().unwrap_or_default();
    let status_code = payload["status_code"].as_u64().unwrap_or(301) as u16;
    let match_type = payload["match_type"].as_str().unwrap_or("exact");

    if source.is_empty() || target.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "source and target are required".into(),
            data: None,
        });
    }

    let mut j = state.journal.lock().await;
    let mut mgr = luperiq_forge::ForgeRedirectManager::new(&mut j);
    match mgr.create_redirect(source, target, status_code, match_type) {
        Ok(id) => Json(ApiResult {
            ok: true,
            message: format!("redirect created: {id}"),
            data: Some(serde_json::json!({ "redirect_id": id })),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

/// DELETE /api/modules/seo/redirects/:redirect_id — delete a redirect.
pub(crate) async fn delete_redirect(
    State(state): State<SeoState>,
    axum::extract::Path(redirect_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mut mgr = luperiq_forge::ForgeRedirectManager::new(&mut j);
    match mgr.delete_redirect(&redirect_id) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: format!("Redirect {redirect_id} deleted"),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

/// PUT /api/modules/seo/redirects/:redirect_id — update a redirect.
pub(crate) async fn update_redirect(
    State(state): State<SeoState>,
    axum::extract::Path(redirect_id): axum::extract::Path<String>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> Json<ApiResult> {
    let source = payload.get("source").and_then(|v| v.as_str());
    let target = payload.get("target").and_then(|v| v.as_str());
    let status_code = payload
        .get("status_code")
        .and_then(|v| v.as_u64())
        .map(|v| v as u16);
    let pattern_type = payload.get("pattern_type").and_then(|v| v.as_str());
    let is_active = payload.get("is_active").and_then(|v| v.as_bool());

    let mut j = state.journal.lock().await;
    let mut mgr = luperiq_forge::ForgeRedirectManager::new(&mut j);
    match mgr.update_redirect(
        &redirect_id,
        source,
        target,
        status_code,
        pattern_type,
        is_active,
    ) {
        Ok(_) => Json(ApiResult {
            ok: true,
            message: format!("Redirect {redirect_id} updated"),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

/// GET /api/modules/seo/score/:content_id — calculate SEO score on demand.
pub(crate) async fn score_content(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Get SEO meta
    let meta: SeoMeta = match j.get_latest(AGG_SEO_META, &content_id) {
        Some(event) if event.payload != TOMBSTONE => match serde_json::from_slice(&event.payload) {
            Ok(m) => m,
            Err(_) => {
                return Json(ApiResult {
                    ok: false,
                    message: "Failed to parse SEO meta".into(),
                    data: None,
                })
            }
        },
        _ => SeoMeta {
            content_id: content_id.clone(),
            title: String::new(),
            description: String::new(),
            og_image: String::new(),
            canonical_url: String::new(),
            robots: String::new(),
            schema_json: String::new(),
            focus_keyword: String::new(),
            seo_score: 0,
        },
    };

    // Get content for scoring
    let mgr = ForgeContentManager::new(&mut j);
    let (content_title, content_body) = mgr
        .get_content(&content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json))
        .unwrap_or_default();

    let score = calculate_seo_score(&meta, &content_title, &content_body, None);

    Json(ApiResult {
        ok: true,
        message: format!("Score: {score}"),
        data: Some(serde_json::json!({
            "content_id": content_id,
            "score": score,
            "has_title": !meta.title.is_empty(),
            "has_description": !meta.description.is_empty(),
            "has_og_image": !meta.og_image.is_empty(),
            "has_canonical": !meta.canonical_url.is_empty(),
            "has_schema": !meta.schema_json.is_empty(),
            "has_robots": !meta.robots.is_empty(),
            "has_focus_keyword": !meta.focus_keyword.is_empty(),
        })),
    })
}

/// GET /api/modules/seo/health — aggregate SEO health stats.
pub(crate) async fn site_health(State(state): State<SeoState>) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Get all SEO metas
    let seo_events = j.latest_by_aggregate_type(AGG_SEO_META);
    let metas: Vec<SeoMeta> = seo_events
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .collect();

    // Get all published pages
    let mgr = ForgeContentManager::new(&mut j);
    let total_pages = mgr
        .list_content(Some("page"), Some("published"), None, 1000, 0, None, None)
        .map(|(_, total)| total)
        .unwrap_or(0);

    let total_with_meta = metas.len();
    let total_score: u32 = metas.iter().map(|m| m.seo_score as u32).sum();
    let avg_score = if total_with_meta > 0 {
        total_score / total_with_meta as u32
    } else {
        0
    };
    let missing_title = metas.iter().filter(|m| m.title.is_empty()).count();
    let missing_desc = metas.iter().filter(|m| m.description.is_empty()).count();
    let missing_og = metas.iter().filter(|m| m.og_image.is_empty()).count();
    let missing_schema = metas.iter().filter(|m| m.schema_json.is_empty()).count();
    let missing_robots = metas.iter().filter(|m| m.robots.is_empty()).count();
    let pages_without_meta = if total_pages > total_with_meta {
        total_pages - total_with_meta
    } else {
        0
    };

    Json(ApiResult {
        ok: true,
        message: format!("SEO health for {total_pages} pages"),
        data: Some(serde_json::json!({
            "total_pages": total_pages,
            "pages_with_meta": total_with_meta,
            "pages_without_meta": pages_without_meta,
            "average_score": avg_score,
            "missing_title": missing_title,
            "missing_description": missing_desc,
            "missing_og_image": missing_og,
            "missing_schema": missing_schema,
            "missing_robots": missing_robots,
        })),
    })
}

// ── Sitemap handler ───────────────────────────────────────────────────

/// Slugs to exclude from sitemap (utility, test, internal pages).
const SITEMAP_EXCLUDE_PREFIXES: &[&str] = &["demo-", "example-", "partner-", "orbit/", "orbit"];
const SITEMAP_EXCLUDE_EXACT: &[&str] = &[
    "checkout",
    "register",
    "customer-login",
    "my-account",
    "home-2",
    "orbit",
];

/// Old Orbit demo pages that only need to be hidden on the central marketing site.
const SITEMAP_EXCLUDE_MARKETING_DEMO_EXACT: &[&str] =
    &["menu", "about", "reservations", "contact", "events"];

/// Central marketing pages that redirect because the old generated slug used
/// the wrong vocabulary for the site type.
const SITEMAP_EXCLUDE_REDIRECT_EXACT: &[&str] = &[
    "blog-scheduling",
    "blog-invoicing",
    "blog-customer-portal",
    "blog-service-area-pages",
    "blog-technician-management",
    "how-to-set-up-blog-scheduling",
    "how-to-create-blog-invoices",
    "how-to-set-up-blog-customer-portal",
    "how-to-create-blog-service-area-pages",
    "how-to-manage-blog-technicians",
    "creator-scheduling",
    "creator-invoicing",
    "creator-customer-portal",
    "creator-service-area-pages",
    "creator-technician-management",
    "how-to-set-up-creator-scheduling",
    "how-to-create-creator-invoices",
    "how-to-set-up-creator-customer-portal",
    "how-to-create-creator-service-area-pages",
    "how-to-manage-creator-technicians",
    "app-publisher-scheduling",
    "app-publisher-invoicing",
    "app-publisher-customer-portal",
    "app-publisher-service-area-pages",
    "app-publisher-technician-management",
    "how-to-set-up-app-publisher-scheduling",
    "how-to-create-app-publisher-invoices",
    "how-to-set-up-app-publisher-customer-portal",
    "how-to-create-app-publisher-service-area-pages",
    "how-to-manage-app-publisher-technicians",
    "medical-office-service-area-pages",
    "medical-office-technician-management",
    "how-to-create-medical-office-service-area-pages",
    "how-to-manage-medical-office-technicians",
    "restaurant-service-area-pages",
    "how-to-create-restaurant-service-area-pages",
    "restaurant-technician-management",
    "how-to-manage-restaurant-technicians",
    "bakery-service-area-pages",
    "how-to-create-bakery-service-area-pages",
    "bakery-technician-management",
    "how-to-manage-bakery-technicians",
    "coffee-shop-service-area-pages",
    "how-to-create-coffee-shop-service-area-pages",
    "coffee-shop-technician-management",
    "how-to-manage-coffee-shop-technicians",
    "salon-service-area-pages",
    "how-to-create-salon-service-area-pages",
    "salon-technician-management",
    "how-to-manage-salon-technicians",
    "artisan-market-service-area-pages",
    "how-to-create-artisan-market-service-area-pages",
    "artisan-market-technician-management",
    "how-to-manage-artisan-market-technicians",
];

/// GET /sitemap.xml — standard XML sitemap of all published pages and blog posts.
pub(crate) async fn sitemap_handler(
    State(state): State<SeoPublicState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let base_url = effective_public_base_url(&headers);
    let host = request_host(&headers);

    // bleed-stop: pestcontroller (vertical host) does not serve/sitemap shared
    // platform Page-Studio pages. Omit the ForgeContent page/post entries and
    // the platform marketing-special entries; keep the host-appropriate home `/`.
    // Compare case-insensitively to match the canonical host_is_pestcontroller helper.
    let host_lc = host.to_ascii_lowercase();
    let is_pestcontroller_host =
        host_lc == "pestcontroller.org" || host_lc.ends_with(".pestcontroller.org");

    if is_preview_host(&host) {
        return xml_response(empty_sitemap_xml());
    }

    let mut j = state.journal.lock().await;
    if let Some(ref ext) = state.sitemap_ext {
        if let Some(custom_entries) = ext.custom_entries_for_host(&host, &j) {
            return xml_response(render_sitemap_owned(&base_url, custom_entries));
        }
    }

    let mgr = ForgeContentManager::new(&mut j);

    let pages = mgr
        .list_content(Some("page"), Some("published"), None, 1000, 0, None, None)
        .map(|(v, _)| v)
        .unwrap_or_default();

    let posts = mgr
        .list_content(Some("post"), Some("published"), None, 1000, 0, None, None)
        .map(|(v, _)| v)
        .unwrap_or_default();

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let mut seen_locs = HashSet::new();

    // Build a set of all page slugs so we can detect duplicate-counter slugs
    // (e.g. when a site has been re-provisioned, "home" + "home-3" + "home-4"
    // can co-exist; only "home" should be in the sitemap).
    let canonical_slugs: HashSet<String> =
        pages.iter().map(|p| p.slug.clone()).collect();

    // Helper: check if a slug should be excluded
    let should_exclude = |slug: &str| -> bool {
        SITEMAP_EXCLUDE_EXACT.contains(&slug)
            || (state.site_type == "marketing"
                && SITEMAP_EXCLUDE_MARKETING_DEMO_EXACT.contains(&slug))
            || (state.site_type == "marketing" && SITEMAP_EXCLUDE_REDIRECT_EXACT.contains(&slug))
            || SITEMAP_EXCLUDE_PREFIXES.iter().any(|p| slug.starts_with(p))
            || is_duplicate_counter_slug(slug, &canonical_slugs)
            || is_duplicate_doubled_word_slug(slug, &canonical_slugs)
    };

    // Key pages get higher priority
    let high_priority_slugs = &[
        "home",
        "modules",
        "wordpress-business-plugins",
        "themes",
        "wordpress-business-themes",
        "pricing",
        "luperiq-pricing-plans",
        "get-started",
        "get-started-with-luperiq",
    ];

    for page in &pages {
        // bleed-stop: no shared Page-Studio ForgeContent pages on pestcontroller.
        if is_pestcontroller_host {
            break;
        }
        if should_exclude(&page.slug) {
            continue;
        }

        // Content-quality-weighted sitemap priority
        let word_count = page.body_json.split_whitespace().count();
        let loc = if page.slug == "home" {
            format!("{}/", base_url.trim_end_matches('/'))
        } else {
            format!("{}/{}/", base_url.trim_end_matches('/'), page.slug)
        };
        let priority = if page.slug == "home" {
            "1.0"
        } else if high_priority_slugs.contains(&page.slug.as_str()) {
            "0.9"
        } else if word_count >= 1200 {
            "0.8" // Rich content pages
        } else if word_count >= 500 {
            "0.7" // Decent content
        } else if word_count >= 200 {
            "0.5" // Thin but present
        } else {
            "0.3" // Stub pages — lowest priority
        };
        if !seen_locs.insert(sitemap_seen_key(&loc)) {
            continue;
        }

        let lastmod = chrono::DateTime::from_timestamp(page.updated_at as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", super::xml_escape(&loc)));
        if !lastmod.is_empty() {
            xml.push_str(&format!("    <lastmod>{lastmod}</lastmod>\n"));
        }
        xml.push_str(&format!("    <changefreq>weekly</changefreq>\n"));
        xml.push_str(&format!("    <priority>{priority}</priority>\n"));
        xml.push_str("  </url>\n");
    }

    // Blog posts
    // bleed-stop: no shared Page-Studio ForgeContent posts on pestcontroller.
    if !posts.is_empty() && !is_pestcontroller_host {
        // Blog index page
        let blog_loc = format!("{}/blog", base_url.trim_end_matches('/'));
        seen_locs.insert(sitemap_seen_key(&blog_loc));
        xml.push_str("  <url>\n");
        xml.push_str(&format!(
            "    <loc>{}/blog</loc>\n",
            super::xml_escape(base_url.trim_end_matches('/'))
        ));
        xml.push_str("    <changefreq>daily</changefreq>\n");
        xml.push_str("    <priority>0.7</priority>\n");
        xml.push_str("  </url>\n");

        for post in &posts {
            let loc = format!("{}/blog/{}/", base_url.trim_end_matches('/'), post.slug);
            if !seen_locs.insert(sitemap_seen_key(&loc)) {
                continue;
            }
            let lastmod = chrono::DateTime::from_timestamp(post.updated_at as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_default();

            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}</loc>\n", super::xml_escape(&loc)));
            if !lastmod.is_empty() {
                xml.push_str(&format!("    <lastmod>{lastmod}</lastmod>\n"));
            }
            xml.push_str("    <changefreq>monthly</changefreq>\n");
            xml.push_str("    <priority>0.5</priority>\n");
            xml.push_str("  </url>\n");
        }
    }

    if state.site_type == "marketing" {
        append_marketing_home_entry(&mut xml, &mut seen_locs, &base_url);
        // bleed-stop: the platform marketing-special routes (modules, industry
        // `-website` hubs, migrate-from, group landing pages, AI-workflows) are
        // luperiq-platform content — omit them on the pestcontroller vertical host.
        if !is_pestcontroller_host {
            append_marketing_special_entries(
                &mut xml,
                &mut seen_locs,
                &base_url,
                &state.sitemap_ext,
            );
        }
    } else {
        append_site_pages_entries(&mut xml, &mut seen_locs, &j, &base_url);
    }

    xml.push_str("</urlset>\n");

    xml_response(xml)
}

fn is_group_family_slug(slug: &str) -> bool {
    matches!(
        slug,
        "family" | "roommates" | "elder-care" | "pet-owners" | "church" | "small-group"
            | "life-group" | "bible-study" | "mission-team" | "classroom" | "homeschool"
            | "homeschool-coop" | "sports-team" | "club" | "hobby" | "band" | "book-club"
            | "nonprofit" | "volunteer" | "neighborhood" | "travel" | "travel-group"
            | "wedding" | "reunion" | "memorial" | "scouts" | "fitness" | "farm"
            | "support-group" | "maker-space" | "business-team" | "business"
    )
}

fn append_site_pages_entries(
    xml: &mut String,
    seen_locs: &mut HashSet<String>,
    journal: &luperiq_forge::ForgeJournal,
    base_url: &str,
) {
    let industry_slug = site_industry_slug(journal);
    let is_restaurant = industry_slug == "restaurant";
    let is_group = is_group_family_slug(&industry_slug);
    let mut entries: Vec<(String, &'static str, &'static str)> = if is_restaurant {
        vec![
            ("/".to_string(), "weekly", "1.0"),
            ("/menu".to_string(), "weekly", "0.9"),
            ("/reservations".to_string(), "weekly", "0.8"),
            ("/cart".to_string(), "weekly", "0.7"),
            ("/catering".to_string(), "monthly", "0.7"),
            ("/merch".to_string(), "monthly", "0.6"),
        ]
    } else if is_group {
        // Group/family/community sites: service-catalog, service-areas, and
        // financing pages are not public-facing routes on these site types.
        // Only the homepage is guaranteed; about/contact come from content pages.
        vec![("/".to_string(), "weekly", "1.0")]
    } else {
        vec![
            ("/".to_string(), "weekly", "1.0"),
            ("/services".to_string(), "weekly", "0.9"),
            ("/service-areas".to_string(), "monthly", "0.8"),
            ("/financing".to_string(), "monthly", "0.6"),
        ]
    };

    if !is_restaurant && !is_group {
        let svc_events = journal.latest_by_aggregate_type(AGG_SVC_SERVICE);
        for event in &svc_events {
            if event.payload == TOMBSTONE {
                continue;
            }
            if let Ok(svc) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                if let Some(slug) = svc.get("slug").and_then(|v| v.as_str()) {
                    if svc.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                        entries.push((format!("/services/{slug}"), "weekly", "0.8"));
                    }
                }
            }
        }

        let loc_events = journal.latest_by_aggregate_type(AGG_LOC_PROFILE);
        for event in &loc_events {
            if event.payload == TOMBSTONE {
                continue;
            }
            if let Ok(loc) = serde_json::from_slice::<serde_json::Value>(&event.payload) {
                if let Some(slug) = loc.get("slug").and_then(|v| v.as_str()) {
                    if loc.get("active").and_then(|v| v.as_bool()).unwrap_or(false) {
                        entries.push((format!("/service-areas/{slug}"), "monthly", "0.7"));
                    }
                }
            }
        }
    }

    for (loc_path, changefreq, priority) in entries {
        let loc = if loc_path == "/" {
            format!("{}/", base_url.trim_end_matches('/'))
        } else {
            format!("{}{}", base_url.trim_end_matches('/'), loc_path)
        };
        if !seen_locs.insert(sitemap_seen_key(&loc)) {
            continue;
        }
        xml.push_str("  <url>\n");
        xml.push_str(&format!("    <loc>{}</loc>\n", super::xml_escape(&loc)));
        xml.push_str(&format!("    <changefreq>{changefreq}</changefreq>\n"));
        xml.push_str(&format!("    <priority>{priority}</priority>\n"));
        xml.push_str("  </url>\n");
    }
}

fn site_industry_slug(journal: &luperiq_forge::ForgeJournal) -> String {
    journal
        .get_latest(AGG_COMP_PROFILE, "global")
        .and_then(|event| serde_json::from_slice::<serde_json::Value>(&event.payload).ok())
        .and_then(|value| {
            value
                .get("industry_slug")
                .and_then(|slug| slug.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

// ── robots.txt handler ───────────────────────────────────────────────

/// GET /robots.txt — standard robots.txt for search engine crawlers.
pub(crate) async fn robots_handler(headers: HeaderMap) -> impl IntoResponse {
    let base_url = effective_public_base_url(&headers);
    let host = request_host(&headers);
    if is_preview_host(&host) {
        return (
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )],
            "User-agent: *\nDisallow: /\n".to_string(),
        );
    }
    let robots = format!(
        "\
User-agent: *\n\
Allow: /\n\
\n\
# Block utility and demo pages\n\
Disallow: /checkout\n\
Disallow: /register\n\
Disallow: /customer-login\n\
Disallow: /my-account\n\
Disallow: /partner-activation\n\
Disallow: /partner-portal\n\
Disallow: /partner-resources\n\
Disallow: /admin\n\
Disallow: /api/\n\
Disallow: /demo-*\n\
Disallow: /example-*\n\
Disallow: /orbit/\n\
Disallow: /orbit\n\
\n\
# Sitemaps\n\
Sitemap: {base}/sitemap.xml\n\
Sitemap: {base}/directory/sitemap.xml\n\
Sitemap: {base}/quiz2/sitemap.xml\n\
Sitemap: {base}/chemicals/sitemap.xml\n\
Sitemap: {base}/pest-news/sitemap.xml\n\
",
        base = base_url.trim_end_matches('/')
    );

    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        robots,
    )
}



/// GET /sitemap_index.xml — index of all sitemaps for pestcontroller.org.
pub(crate) async fn sitemap_index_handler(headers: HeaderMap) -> impl IntoResponse {
    let base_url = effective_public_base_url(&headers);
    let host = request_host(&headers);
    let base = base_url.trim_end_matches('/');

    if host == "pestcontroller.org" || host == "www.pestcontroller.org" || is_preview_host(&host) {
        let xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n            <sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n            <sitemap><loc>{}/sitemap.xml</loc></sitemap>\n            <sitemap><loc>{}/chemicals/sitemap.xml</loc></sitemap>\n            <sitemap><loc>{}/directory/sitemap.xml</loc></sitemap>\n            <sitemap><loc>{}/quiz2/sitemap.xml</loc></sitemap>\n            <sitemap><loc>{}/pest-news/sitemap.xml</loc></sitemap>\n            </sitemapindex>\n",
            base, base, base, base, base
        );
        return (
            [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
            xml,
        );
    }

    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n        <sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n        <sitemap><loc>{}/sitemap.xml</loc></sitemap>\n        </sitemapindex>\n",
        base
    );
    (
        [(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
}

fn append_marketing_home_entry(xml: &mut String, seen_locs: &mut HashSet<String>, base_url: &str) {
    let loc = format!("{}/", base_url.trim_end_matches('/'));
    if !seen_locs.insert(sitemap_seen_key(&loc)) {
        return;
    }
    xml.push_str("  <url>\n");
    xml.push_str(&format!("    <loc>{}</loc>\n", super::xml_escape(&loc)));
    xml.push_str("    <changefreq>weekly</changefreq>\n");
    xml.push_str("    <priority>1.0</priority>\n");
    xml.push_str("  </url>\n");
}

pub fn effective_public_base_url(headers: &HeaderMap) -> String {
    let host = request_host(headers);

    if host.is_empty() {
        return "https://luperiq.com".to_string();
    }

    if matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1") {
        return format!("http://{host}");
    }

    format!("https://{host}")
}

fn request_host(headers: &HeaderMap) -> String {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .trim();

    if host.is_empty() {
        return String::new();
    }

    host.strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host)
        .split(':')
        .next()
        .unwrap_or(host)
        .to_string()
}

fn is_preview_host(host: &str) -> bool {
    host == "preview.luperiq.com" || host.ends_with(".preview.luperiq.com")
}

fn xml_response(xml: String) -> ([(axum::http::header::HeaderName, &'static str); 1], String) {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "application/xml; charset=utf-8",
        )],
        xml,
    )
}

fn empty_sitemap_xml() -> String {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n</urlset>\n".to_string()
}

fn render_sitemap_owned(base_url: &str, entries: Vec<(String, String, String)>) -> String {
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    let mut seen_locs = HashSet::new();
    for (path, changefreq, priority) in entries {
        push_sitemap_entry(
            &mut xml,
            &mut seen_locs,
            base_url,
            &path,
            &changefreq,
            &priority,
        );
    }
    xml.push_str("</urlset>\n");
    xml
}

fn push_sitemap_entry(
    xml: &mut String,
    seen_locs: &mut HashSet<String>,
    base_url: &str,
    path: &str,
    changefreq: &str,
    priority: &str,
) {
    let loc = if path == "/" {
        format!("{}/", base_url.trim_end_matches('/'))
    } else {
        format!("{}{}", base_url.trim_end_matches('/'), path)
    };
    if !seen_locs.insert(sitemap_seen_key(&loc)) {
        return;
    }
    xml.push_str("  <url>\n");
    xml.push_str(&format!("    <loc>{}</loc>\n", super::xml_escape(&loc)));
    xml.push_str(&format!("    <changefreq>{changefreq}</changefreq>\n"));
    xml.push_str(&format!("    <priority>{priority}</priority>\n"));
    xml.push_str("  </url>\n");
}

/// Detect duplicate-counter slugs like "home-3", "services-4", "about-2" —
/// the artifact left over when a page was re-provisioned and the existing one
/// was not deleted first. We strip the trailing "-<digits>" and check if the
/// base slug exists too. If it does, this is a stale duplicate and should be
/// kept out of the sitemap.
fn is_duplicate_counter_slug(slug: &str, all_slugs: &HashSet<String>) -> bool {
    // Need at least one trailing digit after a hyphen.
    let bytes = slug.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    // Scan back from the end for digits.
    let mut i = bytes.len();
    while i > 0 && bytes[i - 1].is_ascii_digit() {
        i -= 1;
    }
    if i == bytes.len() || i == 0 {
        return false; // no trailing digits, or all digits (no base)
    }
    if bytes[i - 1] != b'-' {
        return false; // digits must come after a hyphen
    }
    let base = &slug[..i - 1];
    if base.is_empty() {
        return false;
    }
    all_slugs.contains(base)
}

/// Detect "verb-doubled" slugs left over from an earlier page-generator
/// build (e.g. "urgent-care-care", "urgent-care-care-fort-worth",
/// "ac-repair-repair-dallas"): the slug contains two adjacent identical
/// hyphen-separated tokens AND collapsing them yields a slug that exists.
/// Skips the dup so only the canonical clean slug shows up in the sitemap.
fn is_duplicate_doubled_word_slug(slug: &str, all_slugs: &HashSet<String>) -> bool {
    let tokens: Vec<&str> = slug.split('-').collect();
    for i in 0..tokens.len().saturating_sub(1) {
        if tokens[i].is_empty() || tokens[i] != tokens[i + 1] {
            continue;
        }
        let mut deduped: Vec<&str> = Vec::with_capacity(tokens.len() - 1);
        for (j, t) in tokens.iter().enumerate() {
            if j == i + 1 {
                continue;
            }
            deduped.push(t);
        }
        let deduped_slug = deduped.join("-");
        if all_slugs.contains(&deduped_slug) {
            return true;
        }
    }
    false
}

fn sitemap_seen_key(loc: &str) -> String {
    let trimmed = loc.trim_end_matches('/');
    let origin_only = trimmed
        .split_once("://")
        .map(|(_, rest)| !rest.contains('/'))
        .unwrap_or(trimmed.is_empty());
    if origin_only {
        format!("{trimmed}/")
    } else {
        trimmed.to_string()
    }
}

fn append_marketing_special_entries(
    xml: &mut String,
    seen_locs: &mut HashSet<String>,
    base_url: &str,
    sitemap_ext: &super::OptSitemapExtProvider,
) {
    for (path, changefreq, priority) in [
        ("/get-started", "weekly", "0.9"),
        ("/verified-source/proof", "monthly", "0.6"),
        ("/modules", "weekly", "0.9"),
        ("/ai-workflows", "weekly", "0.9"),
        ("/pricing/", "monthly", "0.8"),
        ("/start-trial/", "monthly", "0.8"),
        ("/migrate/", "monthly", "0.7"),
        ("/need-help/", "monthly", "0.5"),
        ("/onboarding/", "monthly", "0.5"),
    ] {
        push_sitemap_entry(xml, seen_locs, base_url, path, changefreq, priority);
    }

    // Delegate module/AI-workflow entries to the extension provider
    if let Some(ext) = sitemap_ext {
        for (path, changefreq, priority) in ext.marketing_entries() {
            push_sitemap_entry(xml, seen_locs, base_url, &path, &changefreq, &priority);
        }
    }

    for rate in luperiq_forge::nexus::CREDIT_RATES {
        push_sitemap_entry(
            xml,
            seen_locs,
            base_url,
            &format!("/ai-workflows/{}/", rate.operation),
            "monthly",
            "0.7",
        );
    }
}

// ── Slug change handlers ─────────────────────────────────────────────

/// GET /api/modules/seo/meta/{content_id}/slug-check?new_slug=...
/// Phase 1: Pre-check slug availability and fetch GSC performance data.
pub(crate) async fn slug_check(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
    Query(q): Query<SlugCheckQuery>,
) -> Json<ApiResult> {
    let new_slug = match q.new_slug {
        Some(s) if !s.is_empty() => s,
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "new_slug query parameter is required".into(),
                data: None,
            })
        }
    };

    // Read current slug and content_type from journal (then release lock)
    let (current_slug, content_type, gsc_site_url, gsc_authenticated) = {
        let mut j = state.journal.lock().await;

        // Scope the mutable borrow for ForgeContentManager
        let (slug, ct) = {
            let mgr = ForgeContentManager::new(&mut j);
            let content = mgr.get_content(&content_id).ok().flatten();
            content
                .map(|c| (c.slug, c.content_type))
                .unwrap_or_default()
        };

        let config = super::google::load_google_config(&j);
        (slug, ct, config.gsc_site_url.clone(), config.authenticated)
    };

    if content_type.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Content not found".into(),
            data: None,
        });
    }

    // Check slug availability
    let slug_available = {
        let mut j = state.journal.lock().await;
        let mgr = ForgeSlugManager::new(&mut j);
        mgr.resolve_slug(&new_slug, Some(&content_type))
            .ok()
            .flatten()
            .is_none()
    };

    // Build response with optional GSC data
    let mut data = serde_json::json!({
        "current_slug": current_slug,
        "new_slug_available": slug_available,
    });

    // If GSC is authenticated, fetch performance data for the current URL
    if gsc_authenticated && !gsc_site_url.is_empty() && !current_slug.is_empty() {
        // AI warning based on traffic data could be added here
        // For now, return the GSC status
        data["gsc_authenticated"] = serde_json::json!(true);
    }

    Json(ApiResult {
        ok: true,
        message: "Slug check complete".into(),
        data: Some(data),
    })
}

/// PUT /api/modules/seo/meta/{content_id}/slug — execute slug change.
/// Phase 2: Change the slug, creating 301 redirect automatically.
pub(crate) async fn slug_change(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
    axum::extract::Json(payload): axum::extract::Json<SlugChangePayload>,
) -> Json<ApiResult> {
    if payload.new_slug.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "new_slug is required".into(),
            data: None,
        });
    }

    let mut j = state.journal.lock().await;

    // Look up content_type from content (scope ForgeContentManager so borrow is released)
    let (old_slug, content_type) = {
        let mgr = ForgeContentManager::new(&mut j);
        match mgr.get_content(&content_id).ok().flatten() {
            Some(c) => (c.slug.clone(), c.content_type.clone()),
            None => {
                return Json(ApiResult {
                    ok: false,
                    message: "Content not found".into(),
                    data: None,
                })
            }
        }
    };

    if old_slug == payload.new_slug {
        return Json(ApiResult {
            ok: false,
            message: "New slug is the same as current slug".into(),
            data: None,
        });
    }

    // Execute slug change — ForgeSlugManager auto-creates 301 redirect
    {
        let mut slug_mgr = ForgeSlugManager::new(&mut j);
        match slug_mgr.set_slug(&content_id, &payload.new_slug, &content_type) {
            Ok(_slug_id) => {}
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Slug change failed: {e}"),
                    data: None,
                })
            }
        }
    }

    // Record SeoChange event
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let change = super::tracker::SeoChange {
        content_id: content_id.clone(),
        change_type: super::tracker::SeoChangeType::SlugChange,
        old_value: old_slug.clone(),
        new_value: payload.new_slug.clone(),
        snapshot_before: None,
        ai_warning: None,
        timestamp: now,
    };

    if let Err(e) = super::tracker::record_seo_change(&mut j, &change) {
        eprintln!("[seo] Failed to record slug change: {e}");
    }

    Json(ApiResult {
        ok: true,
        message: format!("URL changed from /{old_slug} to /{}", payload.new_slug),
        data: Some(serde_json::json!({
            "old_slug": old_slug,
            "new_slug": payload.new_slug,
            "redirect_created": true,
        })),
    })
}

// ── AI credit deduction ───────────────────────────────────────────────

pub(crate) async fn deduct_seo_credits(
    nexus_config: &Option<luperiq_module_api::NexusNetworkConfig>,
    operation: &str,
    amount: u32,
) -> Result<(), String> {
    let cfg = match nexus_config {
        Some(c) => c,
        None => return Ok(()), // standalone = free
    };
    let central_url = match &cfg.central_url {
        Some(u) => u,
        None => return Ok(()), // central node = free
    };
    let key = match &cfg.license_key {
        Some(k) => k,
        None => return Err("No license key configured".into()),
    };
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let resp = client
        .post(format!("{central_url}/api/modules/nexus/credits/deduct"))
        .json(&serde_json::json!({
            "license_key": key,
            "operation": operation,
            "amount": amount,
            "module_key": "seo",
        }))
        .send()
        .await
        .map_err(|e| format!("Credit deduction failed: {e}"))?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["ok"].as_bool() != Some(true) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Credit deduction failed")
            .to_string());
    }
    Ok(())
}

// ── AI handlers ───────────────────────────────────────────────────────

/// POST /api/modules/seo/ai/title — AI-generate an SEO title for a page.
pub(crate) async fn ai_generate_title(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_title", 1).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    // Read page content
    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let (title, body) = mgr
        .get_content(&payload.content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json))
        .unwrap_or_default();
    drop(j);

    let system = "You are an SEO expert. Return ONLY the requested text, nothing else.";
    let user_msg = format!(
        "Generate an SEO-optimized title tag for this web page. The title should be 30-60 characters, \
         include the primary keyword, and be compelling for search results.\n\n\
         Page title: {}\nPage content (first 500 chars): {}",
        title, &body[..body.len().min(500)]
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => Json(ApiResult {
            ok: true,
            message: "AI title generated".into(),
            data: Some(serde_json::json!({
                "content_id": payload.content_id,
                "suggested_title": result.content.trim().trim_matches('"'),
            })),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/ai/description — AI-generate a meta description.
pub(crate) async fn ai_generate_description(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_description", 1).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let (title, body) = mgr
        .get_content(&payload.content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json))
        .unwrap_or_default();
    drop(j);

    let system = "You are an SEO expert. Return ONLY the requested text, nothing else.";
    let user_msg = format!(
        "Generate an SEO-optimized meta description for this web page. The description should be \
         120-160 characters, include the primary keyword, and be compelling for search results.\n\n\
         Page title: {}\nPage content (first 500 chars): {}",
        title,
        &body[..body.len().min(500)]
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => Json(ApiResult {
            ok: true,
            message: "AI description generated".into(),
            data: Some(serde_json::json!({
                "content_id": payload.content_id,
                "suggested_description": result.content.trim().trim_matches('"'),
            })),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/ai/schema — AI-generate JSON-LD structured data.
pub(crate) async fn ai_generate_schema(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_schema", 2).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let (title, body, slug) = mgr
        .get_content(&payload.content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json, c.slug))
        .unwrap_or_default();
    drop(j);

    let system = "You are an SEO and structured data expert. Return ONLY valid JSON-LD, no markdown formatting.";
    let user_msg = format!(
        "Generate valid JSON-LD structured data (schema.org) for this web page. Choose the most \
         appropriate schema type (WebPage, Article, FAQPage, Service, etc.) based on the content.\n\n\
         Page title: {}\nPage slug: {}\nPage content (first 800 chars): {}",
        title, slug, &body[..body.len().min(800)]
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => {
            let cleaned = result
                .content
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            Json(ApiResult {
                ok: true,
                message: "AI schema generated".into(),
                data: Some(serde_json::json!({
                    "content_id": payload.content_id,
                    "suggested_schema": cleaned,
                })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/ai/keywords — AI-suggest focus keywords.
pub(crate) async fn ai_suggest_keywords(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_keywords", 1).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let (title, body) = mgr
        .get_content(&payload.content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json))
        .unwrap_or_default();
    drop(j);

    let system = "You are an SEO keyword research expert. Return ONLY a JSON array of strings.";
    let user_msg = format!(
        "Suggest 5-8 SEO focus keywords for this web page. Choose keywords that are specific, \
         have search intent, and are relevant to the content.\n\n\
         Page title: {}\nPage content (first 500 chars): {}",
        title,
        &body[..body.len().min(500)]
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => {
            let cleaned = result
                .content
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            let keywords: Vec<String> = serde_json::from_str(cleaned).unwrap_or_default();
            Json(ApiResult {
                ok: true,
                message: format!("{} keywords suggested", keywords.len()),
                data: Some(serde_json::json!({
                    "content_id": payload.content_id,
                    "keywords": keywords,
                })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/ai/bulk — AI-optimize SEO for multiple pages (max 10).
pub(crate) async fn ai_bulk_optimize(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoBulkAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    let ids: Vec<String> = payload.content_ids.iter().take(10).cloned().collect();
    if ids.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "No content IDs provided".into(),
            data: None,
        });
    }

    let count = ids.len() as u32;
    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_bulk", count).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    let mut results = Vec::new();
    let mut errors = Vec::new();

    for id in &ids {
        let mut j = state.journal.lock().await;
        let mgr = ForgeContentManager::new(&mut j);
        let content = mgr.get_content(id).ok().flatten();
        let (title, body, slug) = content
            .map(|c| (c.title, c.body_json, c.slug))
            .unwrap_or_default();
        drop(j);

        // Read existing SEO meta before overwriting
        let existing_meta = {
            let j2 = state.journal.lock().await;
            j2.get_latest(AGG_SEO_META, id)
                .filter(|e| e.payload != TOMBSTONE)
                .and_then(|e| serde_json::from_slice::<SeoMeta>(&e.payload).ok())
        };
        let (old_title, old_desc, old_kw, old_score) = existing_meta
            .as_ref()
            .map(|m| {
                (
                    m.title.clone(),
                    m.description.clone(),
                    m.focus_keyword.clone(),
                    m.seo_score,
                )
            })
            .unwrap_or_default();

        if title.is_empty() && body.is_empty() {
            errors.push(format!("{id}: no content found"));
            continue;
        }

        let system = "You are an SEO expert. Return ONLY a JSON object with the requested fields.";
        let user_msg = format!(
            "Generate SEO metadata for this web page. Return a JSON object with exactly these fields:\n\
             - \"title\": SEO title (30-60 chars)\n\
             - \"description\": meta description (120-160 chars)\n\
             - \"focus_keyword\": primary keyword\n\n\
             Page title: {}\nPage slug: {}\nContent (first 400 chars): {}",
            title, slug, &body[..body.len().min(400)]
        );

        match ai.generate(system, &user_msg).await {
            Ok(result) => {
                let cleaned = result
                    .content
                    .trim()
                    .trim_start_matches("```json")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(cleaned) {
                    let ai_title = parsed["title"].as_str().unwrap_or("").to_string();
                    let ai_desc = parsed["description"].as_str().unwrap_or("").to_string();
                    let ai_kw = parsed["focus_keyword"].as_str().unwrap_or("").to_string();

                    // Auto-save the generated meta
                    let mut j = state.journal.lock().await;
                    let mgr = ForgeContentManager::new(&mut j);
                    let (ct, cb) = mgr
                        .get_content(id)
                        .ok()
                        .flatten()
                        .map(|c| (c.title, c.body_json))
                        .unwrap_or_default();

                    let mut meta = SeoMeta {
                        content_id: id.clone(),
                        title: ai_title.clone(),
                        description: ai_desc.clone(),
                        og_image: existing_meta
                            .as_ref()
                            .map(|m| m.og_image.clone())
                            .unwrap_or_default(),
                        canonical_url: existing_meta
                            .as_ref()
                            .map(|m| m.canonical_url.clone())
                            .unwrap_or_default(),
                        robots: existing_meta
                            .as_ref()
                            .map(|m| m.robots.clone())
                            .unwrap_or_default(),
                        schema_json: existing_meta
                            .as_ref()
                            .map(|m| m.schema_json.clone())
                            .unwrap_or_default(),
                        focus_keyword: ai_kw.clone(),
                        seo_score: 0,
                    };
                    meta.seo_score = calculate_seo_score(&meta, &ct, &cb, None);

                    if let Ok(bytes) = serde_json::to_vec(&meta) {
                        let event = ApexEvent::new(AGG_SEO_META, id, bytes);
                        let _ = j.append(event);
                    }
                    drop(j);

                    results.push(serde_json::json!({
                        "content_id": id,
                        "title": ai_title,
                        "description": ai_desc,
                        "focus_keyword": ai_kw,
                        "score": meta.seo_score,
                        "before": {
                            "title": old_title,
                            "description": old_desc,
                            "focus_keyword": old_kw,
                            "score": old_score,
                        },
                    }));
                } else {
                    errors.push(format!("{id}: failed to parse AI response"));
                }
            }
            Err(e) => errors.push(format!("{id}: {e}")),
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("{} optimized, {} errors", results.len(), errors.len()),
        data: Some(serde_json::json!({
            "results": results,
            "errors": errors,
        })),
    })
}

// ── AI Content Brief ─────────────────────────────────────────────────

/// POST /api/modules/seo/ai/brief — AI-generate a content optimization brief.
pub(crate) async fn ai_generate_brief(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<SeoAiRequest>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    if let Err(e) = deduct_seo_credits(&state.nexus_config, "seo_ai_brief", 3).await {
        return Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        });
    }

    // Read page content and SEO meta
    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let (title, body, slug) = mgr
        .get_content(&payload.content_id)
        .ok()
        .flatten()
        .map(|c| (c.title, c.body_json, c.slug))
        .unwrap_or_default();
    let seo_meta = super::lookup_seo_meta(&j, &payload.content_id);
    drop(j);

    let focus_kw = seo_meta
        .as_ref()
        .map(|m| m.focus_keyword.as_str())
        .unwrap_or("");
    let current_title = seo_meta
        .as_ref()
        .map(|m| m.title.as_str())
        .unwrap_or(&title);
    let current_desc = seo_meta
        .as_ref()
        .map(|m| m.description.as_str())
        .unwrap_or("");

    let word_count = body.split_whitespace().count();

    let system =
        "You are an SEO content strategist. Return ONLY a JSON object with the requested fields.";
    let user_msg = format!(
        "Generate a content optimization brief for this web page.\n\n\
         Page title: {current_title}\n\
         Page slug: {slug}\n\
         Current focus keyword: {focus_kw}\n\
         Current word count: {word_count}\n\
         Current meta description: {current_desc}\n\
         Content (first 800 chars): {}\n\n\
         Return a JSON object with:\n\
         - \"target_word_count\": recommended word count (number)\n\
         - \"heading_structure\": recommended H2/H3 headings (array of strings)\n\
         - \"related_topics\": related topics to cover (array of strings, 5-8 items)\n\
         - \"faq_suggestions\": suggested FAQ questions (array of strings, 3-5 items)\n\
         - \"content_gaps\": what's missing from the current content (array of strings)\n\
         - \"competitor_angles\": angles competitors likely cover (array of strings, 3-5 items)\n\
         - \"summary\": 2-3 sentence optimization summary",
        &body[..body.len().min(800)]
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => {
            let cleaned = result
                .content
                .trim()
                .trim_start_matches("```json")
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim();
            let parsed: serde_json::Value = serde_json::from_str(cleaned).unwrap_or_else(|_| {
                serde_json::json!({
                    "summary": cleaned,
                    "error": "Could not parse structured brief"
                })
            });
            Json(ApiResult {
                ok: true,
                message: "Content brief generated".into(),
                data: Some(serde_json::json!({
                    "content_id": payload.content_id,
                    "current_word_count": word_count,
                    "brief": parsed,
                })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI error: {e}"),
            data: None,
        }),
    }
}

// ── Insights endpoint (rule-based, zero credits) ─────────────────────

#[derive(Deserialize)]
pub(crate) struct InsightsRequest {
    /// JSON array of query metrics or page metrics
    #[serde(default)]
    pub(crate) queries: Vec<super::google::insights::QueryMetrics>,
    #[serde(default)]
    pub(crate) pages: Vec<super::google::insights::PageMetrics>,
}

/// POST /api/modules/seo/insights — run rule-based insights on provided metrics.
pub(crate) async fn generate_insights(
    axum::extract::Json(payload): axum::extract::Json<InsightsRequest>,
) -> Json<ApiResult> {
    use super::google::insights::InsightsEngine;

    let mut all_insights = Vec::new();

    if !payload.queries.is_empty() {
        all_insights.extend(InsightsEngine::analyze_queries(&payload.queries));
    }
    if !payload.pages.is_empty() {
        all_insights.extend(InsightsEngine::analyze_pages(&payload.pages));
    }

    Json(ApiResult {
        ok: true,
        message: format!("{} insights generated", all_insights.len()),
        data: Some(serde_json::json!({
            "insights": all_insights,
            "query_count": payload.queries.len(),
            "page_count": payload.pages.len(),
        })),
    })
}

// ── Keyword check ────────────────────────────────────────────────────

/// GET /api/modules/seo/keyword-check/{content_id}
/// Returns the 7-point keyword consistency checklist.
pub(crate) async fn keyword_check(
    State(state): State<SeoState>,
    axum::extract::Path(content_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Get SEO meta
    let meta = match j.get_latest(AGG_SEO_META, &content_id) {
        Some(event) if event.payload != TOMBSTONE => {
            match serde_json::from_slice::<super::SeoMeta>(&event.payload) {
                Ok(m) => m,
                Err(_) => {
                    return Json(ApiResult {
                        ok: false,
                        message: "Failed to parse SEO meta".into(),
                        data: None,
                    })
                }
            }
        }
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "No SEO meta for this content — set a focus keyword first".into(),
                data: None,
            })
        }
    };

    // Get content for slug and body
    let mgr = ForgeContentManager::new(&mut j);
    let content = mgr.get_content(&content_id).ok().flatten();
    let (slug, body) = content.map(|c| (c.slug, c.body_json)).unwrap_or_default();

    let checks = super::keyword_gate::keyword_consistency_check(
        &meta.focus_keyword,
        &slug,
        &meta.title,
        &meta.description,
        &body,
    );

    let (passed, total) = super::keyword_gate::keyword_score(&checks);

    Json(ApiResult {
        ok: true,
        message: format!("Keyword check: {passed}/{total} passed"),
        data: Some(serde_json::json!({
            "focus_keyword": meta.focus_keyword,
            "score": passed,
            "total": total,
            "checks": checks,
        })),
    })
}

// ── A/B experiment CRUD ──────────────────────────────────────────────

/// POST /api/modules/seo/ab/create — create a new A/B experiment.
pub(crate) async fn ab_create(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<AbCreatePayload>,
) -> Json<ApiResult> {
    if payload.variant_b.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "variant_b is required".into(),
            data: None,
        });
    }

    let mut j = state.journal.lock().await;

    // Check no existing running experiment for this content + field
    let existing = super::ab_seo::running_experiments(&j);
    if existing
        .iter()
        .any(|e| e.content_id == payload.content_id && e.field == payload.field)
    {
        return Json(ApiResult {
            ok: false,
            message: "An experiment is already running for this field".into(),
            data: None,
        });
    }

    // Get current value (variant A) from SEO meta
    let variant_a = match j.get_latest(AGG_SEO_META, &payload.content_id) {
        Some(event) if event.payload != TOMBSTONE => {
            match serde_json::from_slice::<super::SeoMeta>(&event.payload) {
                Ok(meta) => match payload.field {
                    super::ab_seo::SeoAbField::Title => meta.title,
                    super::ab_seo::SeoAbField::Description => meta.description,
                    super::ab_seo::SeoAbField::FocusKeyword => meta.focus_keyword,
                    super::ab_seo::SeoAbField::Schema => meta.schema_json,
                },
                Err(_) => {
                    return Json(ApiResult {
                        ok: false,
                        message: "Failed to parse current SEO meta".into(),
                        data: None,
                    })
                }
            }
        }
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "No SEO meta — set meta first before A/B testing".into(),
                data: None,
            })
        }
    };

    let duration = payload.duration_days.unwrap_or(14);
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let end_date = super::ab_seo::calculate_end_date(&today, duration);
    let experiment_id = ulid::Ulid::new().to_string();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let experiment = super::ab_seo::SeoAbExperiment {
        experiment_id: experiment_id.clone(),
        content_id: payload.content_id.clone(),
        field: payload.field.clone(),
        variant_a,
        variant_b: payload.variant_b.clone(),
        status: super::ab_seo::SeoAbStatus::Running,
        schedule: super::ab_seo::SeoAbSchedule {
            period_days: duration / 2,
        },
        start_date: today,
        end_date,
        winner: None,
        winner_confidence: None,
        created_at: now,
    };

    if let Err(e) = super::ab_seo::save_experiment(&mut j, &experiment) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to save experiment: {e}"),
            data: None,
        });
    }

    // Record SeoChange event for experiment start
    let change = super::tracker::SeoChange {
        content_id: payload.content_id.clone(),
        change_type: super::tracker::SeoChangeType::AbVariantStart,
        old_value: experiment.variant_a.clone(),
        new_value: experiment.variant_b.clone(),
        snapshot_before: None,
        ai_warning: None,
        timestamp: now,
    };
    let _ = super::tracker::record_seo_change(&mut j, &change);

    Json(ApiResult {
        ok: true,
        message: format!("A/B experiment started: {experiment_id}"),
        data: Some(serde_json::json!({
            "experiment_id": experiment_id,
            "duration_days": duration,
            "end_date": experiment.end_date,
        })),
    })
}

/// GET /api/modules/seo/ab/list — list all experiments.
pub(crate) async fn ab_list(
    State(state): State<SeoState>,
    Query(q): Query<AbListQuery>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;

    let status_filter = q.status.as_deref().and_then(|s| match s {
        "running" => Some(super::ab_seo::SeoAbStatus::Running),
        "awaiting_decision" => Some(super::ab_seo::SeoAbStatus::AwaitingDecision),
        "completed" => Some(super::ab_seo::SeoAbStatus::Completed),
        "cancelled" => Some(super::ab_seo::SeoAbStatus::Cancelled),
        _ => None,
    });

    let experiments = super::ab_seo::list_experiments(&j, status_filter.as_ref());

    let items: Vec<serde_json::Value> = experiments
        .iter()
        .map(|e| {
            serde_json::json!({
                "experiment_id": e.experiment_id,
                "content_id": e.content_id,
                "field": e.field,
                "variant_a": e.variant_a,
                "variant_b": e.variant_b,
                "status": e.status,
                "start_date": e.start_date,
                "end_date": e.end_date,
                "winner": e.winner,
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} experiments", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

/// GET /api/modules/seo/ab/{experiment_id} — get experiment detail.
pub(crate) async fn ab_detail(
    State(state): State<SeoState>,
    axum::extract::Path(experiment_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;

    match super::ab_seo::load_experiment(&j, &experiment_id) {
        Some(exp) => {
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let current_variant = super::ab_seo::active_variant(&exp, &today);

            Json(ApiResult {
                ok: true,
                message: "Experiment found".into(),
                data: Some(serde_json::json!({
                    "experiment_id": exp.experiment_id,
                    "content_id": exp.content_id,
                    "field": exp.field,
                    "variant_a": exp.variant_a,
                    "variant_b": exp.variant_b,
                    "status": exp.status,
                    "schedule": exp.schedule,
                    "start_date": exp.start_date,
                    "end_date": exp.end_date,
                    "current_variant": current_variant,
                    "winner": exp.winner,
                    "winner_confidence": exp.winner_confidence,
                })),
            })
        }
        None => Json(ApiResult {
            ok: false,
            message: "Experiment not found".into(),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/ab/{experiment_id}/complete — complete experiment.
pub(crate) async fn ab_complete(
    State(state): State<SeoState>,
    axum::extract::Path(experiment_id): axum::extract::Path<String>,
    axum::extract::Json(payload): axum::extract::Json<AbCompletePayload>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    let mut exp = match super::ab_seo::load_experiment(&j, &experiment_id) {
        Some(e) => e,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Experiment not found".into(),
                data: None,
            })
        }
    };

    if exp.status != super::ab_seo::SeoAbStatus::Running
        && exp.status != super::ab_seo::SeoAbStatus::AwaitingDecision
    {
        return Json(ApiResult {
            ok: false,
            message: "Experiment is not running or awaiting decision".into(),
            data: None,
        });
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match payload.action.as_str() {
        "apply_winner" => {
            // Apply the chosen winner variant as the permanent value
            let winner_variant = payload.winner.as_deref().unwrap_or("b");
            let winner_value = if winner_variant == "a" {
                exp.variant_a.clone()
            } else {
                exp.variant_b.clone()
            };

            // Update SEO meta with the winning variant
            if let Some(event) = j.get_latest(AGG_SEO_META, &exp.content_id) {
                if event.payload != TOMBSTONE {
                    if let Ok(mut meta) = serde_json::from_slice::<super::SeoMeta>(&event.payload) {
                        match exp.field {
                            super::ab_seo::SeoAbField::Title => meta.title = winner_value.clone(),
                            super::ab_seo::SeoAbField::Description => {
                                meta.description = winner_value.clone()
                            }
                            super::ab_seo::SeoAbField::FocusKeyword => {
                                meta.focus_keyword = winner_value.clone()
                            }
                            super::ab_seo::SeoAbField::Schema => {
                                meta.schema_json = winner_value.clone()
                            }
                        }
                        if let Ok(bytes) = serde_json::to_vec(&meta) {
                            let evt = ApexEvent::new(AGG_SEO_META, &exp.content_id, bytes);
                            let _ = j.append(evt);
                        }
                    }
                }
            }

            exp.status = super::ab_seo::SeoAbStatus::Completed;
            exp.winner = Some(winner_variant.to_string());

            // Record change event
            let change = super::tracker::SeoChange {
                content_id: exp.content_id.clone(),
                change_type: super::tracker::SeoChangeType::AbVariantEnd,
                old_value: exp.variant_a.clone(),
                new_value: winner_value,
                snapshot_before: None,
                ai_warning: None,
                timestamp: now,
            };
            let _ = super::tracker::record_seo_change(&mut j, &change);
        }
        "keep_control" => {
            exp.status = super::ab_seo::SeoAbStatus::Completed;
            exp.winner = Some("a".into());
        }
        other => {
            return Json(ApiResult {
                ok: false,
                message: format!("Unknown action: {other}. Use 'apply_winner' or 'keep_control'"),
                data: None,
            });
        }
    }

    if let Err(e) = super::ab_seo::save_experiment(&mut j, &exp) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to update experiment: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!(
            "Experiment {experiment_id} completed: winner = {:?}",
            exp.winner
        ),
        data: Some(serde_json::json!({
            "experiment_id": experiment_id,
            "winner": exp.winner,
            "status": exp.status,
        })),
    })
}

/// POST /api/modules/seo/ab/{experiment_id}/cancel — cancel experiment.
pub(crate) async fn ab_cancel(
    State(state): State<SeoState>,
    axum::extract::Path(experiment_id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    let mut exp = match super::ab_seo::load_experiment(&j, &experiment_id) {
        Some(e) => e,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Experiment not found".into(),
                data: None,
            })
        }
    };

    if exp.status != super::ab_seo::SeoAbStatus::Running
        && exp.status != super::ab_seo::SeoAbStatus::AwaitingDecision
    {
        return Json(ApiResult {
            ok: false,
            message: "Only running or awaiting-decision experiments can be cancelled".into(),
            data: None,
        });
    }

    exp.status = super::ab_seo::SeoAbStatus::Cancelled;

    if let Err(e) = super::ab_seo::save_experiment(&mut j, &exp) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to cancel experiment: {e}"),
            data: None,
        });
    }

    Json(ApiResult {
        ok: true,
        message: format!("Experiment {experiment_id} cancelled"),
        data: None,
    })
}

// ── Change Timeline ──────────────────────────────────────────────────

/// GET /api/modules/seo/timeline — list SEO changes with optional filtering.
pub(crate) async fn seo_timeline(
    State(state): State<SeoState>,
    Query(q): Query<TimelineQuery>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;

    let content_filter = q.content_id.as_deref().filter(|s| !s.is_empty());
    let changes = super::tracker::list_seo_changes(&j, content_filter);

    // Apply optional filters: change_type, date range
    let change_type_filter = q.change_type.as_deref().filter(|s| !s.is_empty());
    let from_ts = q
        .from
        .as_deref()
        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc().timestamp() as u64);
    let to_ts =
        q.to.as_deref()
            .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|d| d.and_hms_opt(23, 59, 59).unwrap_or_default().and_utc().timestamp() as u64);

    let items: Vec<serde_json::Value> = changes
        .iter()
        .filter(|(_id, change)| {
            if let Some(ct) = change_type_filter {
                let type_str = serde_json::to_string(&change.change_type).unwrap_or_default();
                if !type_str.contains(ct) {
                    return false;
                }
            }
            if let Some(from) = from_ts {
                if change.timestamp < from {
                    return false;
                }
            }
            if let Some(to) = to_ts {
                if change.timestamp > to {
                    return false;
                }
            }
            true
        })
        .take(q.limit.unwrap_or(50) as usize)
        .map(|(id, change)| {
            let after_snapshot =
                super::tracker::latest_snapshot_for_content(&j, &change.content_id);

            serde_json::json!({
                "change_id": id,
                "content_id": change.content_id,
                "change_type": change.change_type,
                "old_value": change.old_value,
                "new_value": change.new_value,
                "timestamp": change.timestamp,
                "ai_warning": change.ai_warning,
                "snapshot_before": change.snapshot_before,
                "snapshot_after": after_snapshot,
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} changes", items.len()),
        data: Some(serde_json::json!(items)),
    })
}

// ── AI Timeline Analysis ─────────────────────────────────────────────

/// POST /api/modules/seo/ai/timeline-analysis
/// Gathers recent SeoChange events and SeoSnapshot data, sends to AI for
/// impact analysis, and stores the result as an SeoAiInsight.
pub(crate) async fn ai_timeline_analysis(
    State(state): State<SeoState>,
    axum::extract::Json(payload): axum::extract::Json<TimelineAnalysisPayload>,
) -> Json<ApiResult> {
    let ai = match &state.ai_provider {
        Some(a) => a.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "AI not configured".into(),
                data: None,
            })
        }
    };

    // Gather changes and snapshots from journal (then release lock)
    let (changes_summary, snapshots_summary) = {
        let j = state.journal.lock().await;
        let content_filter = payload.content_id.as_deref();
        let changes = super::tracker::list_seo_changes(&j, content_filter);

        let changes_text: Vec<String> = changes
            .iter()
            .take(20)
            .map(|(_id, c)| {
                format!(
                    "- {} [{}]: '{}' → '{}' (content: {})",
                    chrono::DateTime::from_timestamp(c.timestamp as i64, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_default(),
                    serde_json::to_string(&c.change_type).unwrap_or_default(),
                    c.old_value.chars().take(60).collect::<String>(),
                    c.new_value.chars().take(60).collect::<String>(),
                    c.content_id,
                )
            })
            .collect();

        let change_ids: Vec<String> = changes.iter().take(20).map(|(id, _)| id.clone()).collect();

        // Get recent snapshots for affected content
        let mut snapshot_text = Vec::new();
        let affected_ids: HashSet<&str> =
            changes.iter().map(|(_, c)| c.content_id.as_str()).collect();
        for cid in affected_ids.iter().take(10) {
            if let Some(snap) = super::tracker::latest_snapshot_for_content(&j, cid) {
                snapshot_text.push(format!(
                    "- {} ({}): {} imp, {} clicks, pos {:.1}, CTR {:.1}%",
                    cid, snap.date, snap.impressions, snap.clicks, snap.avg_position, snap.ctr
                ));
            }
        }

        (
            (changes_text.join("\n"), change_ids),
            snapshot_text.join("\n"),
        )
    };

    if changes_summary.0.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "No recent SEO changes to analyze".into(),
            data: None,
        });
    }

    let system = "You are an SEO analyst. Analyze the following SEO changes and their performance impact. Provide: 1) Impact assessment per change, 2) Correlations (which changes helped/hurt), 3) Actionable recommendations, 4) Risk flags. Be specific and data-driven.";

    let scope = if let Some(ref cid) = payload.content_id {
        format!("Page: {cid}")
    } else {
        "Site-wide (last 30 days)".into()
    };

    let user_msg = format!(
        "Scope: {scope}\n\nRecent SEO Changes:\n{}\n\nLatest Performance Snapshots:\n{}",
        changes_summary.0,
        if snapshots_summary.is_empty() {
            "No GSC data available yet".into()
        } else {
            snapshots_summary
        },
    );

    match ai.generate(system, &user_msg).await {
        Ok(result) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let insight = super::tracker::SeoAiInsight {
                content_id: payload.content_id.clone(),
                analysis: result.content.clone(),
                changes_analyzed: changes_summary.1,
                timestamp: now,
            };

            // Store insight in journal
            let mut j = state.journal.lock().await;
            let _ = super::tracker::record_seo_ai_insight(&mut j, &insight);

            Json(ApiResult {
                ok: true,
                message: "AI timeline analysis complete".into(),
                data: Some(serde_json::json!({
                    "analysis": result.content,
                    "changes_analyzed": insight.changes_analyzed.len(),
                    "scope": scope,
                })),
            })
        }
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("AI analysis failed: {e}"),
            data: None,
        }),
    }
}
