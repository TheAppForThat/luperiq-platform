//! HTTP handlers for Surfer SEO sheets, page mapping, AI work queue,
//! and page intelligence.
//!
//! All handlers follow the shared `SeoState` / `ApiResult` pattern used in
//! `handlers.rs`. Routes are wired in `mod.rs` via `seo_router()`.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

use luperiq_forge::ForgeContentManager;

use super::content_queue::{
    claim_next, compute_priority, detect_page_type, load_all_items, load_item, save_item,
    QueueItem, QueuePhase,
};
use super::intelligence::assemble;
use super::surfer::{
    delete_sheet, import_directory, load_all_sheets, load_sheet, parse_surfer_txt, save_sheet,
};
use super::surfer_map::{
    auto_map_all, load_all_maps, load_map, save_map, suggest_sheets, PageSurferMap,
};
use super::surfer_scoring::score_against_sheet;
use super::{ApiResult, SeoState, AGG_SEO_META, TOMBSTONE};

// ── Helper ────────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn strip_html_words(html: &str) -> u64 {
    let mut plain = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            plain.push(' ');
        } else if !in_tag {
            plain.push(ch);
        }
    }
    plain.split_whitespace().count() as u64
}

fn surfer_managed_on_this_node(state: &SeoState) -> bool {
    matches!(
        state
            .nexus_config
            .as_ref()
            .and_then(|cfg| cfg.role.as_deref()),
        None | Some("") | Some("central")
    )
}

fn surfer_central_only_result() -> Json<ApiResult> {
    Json(ApiResult {
        ok: false,
        message: "Surfer sheets, mappings, and queue tools are managed on Central.".into(),
        data: Some(serde_json::json!({ "central_only": true })),
    })
}

macro_rules! require_surfer_central {
    ($state:expr) => {
        if !surfer_managed_on_this_node($state) {
            return surfer_central_only_result();
        }
    };
}

// ── Query / payload types ─────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub(crate) struct SheetFilterQuery {
    #[serde(default)]
    pub(crate) industry: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct UploadSheetPayload {
    pub(crate) filename: String,
    pub(crate) content: String,
}

#[derive(Deserialize)]
pub(crate) struct SetMapPayload {
    pub(crate) sheet_ids: Vec<String>,
    pub(crate) primary_sheet_id: String,
}

#[derive(Deserialize, Default)]
pub(crate) struct QueueFilterQuery {
    #[serde(default)]
    pub(crate) phase: Option<String>,
    #[serde(default)]
    pub(crate) priority: Option<u8>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateQueueStatusPayload {
    pub(crate) phase: String,
    #[serde(default)]
    pub(crate) notes: Option<String>,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

// ── Sheet CRUD ────────────────────────────────────────────────────────────────

/// GET /api/modules/seo/surfer/sheets — list all sheets, optionally filtered by industry.
pub(crate) async fn list_sheets(
    State(state): State<SeoState>,
    Query(params): Query<SheetFilterQuery>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let j = state.journal.lock().await;
    let mut sheets = load_all_sheets(&j);

    if let Some(ref ind) = params.industry {
        if !ind.is_empty() {
            sheets.retain(|s| &s.industry == ind);
        }
    }

    let data: Vec<serde_json::Value> = sheets
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "sheet_id": s.sheet_id,
                "topic": s.topic,
                "source_file": s.source_file,
                "source_date": s.source_date,
                "industry": s.industry,
                "term_count": s.terms.len(),
                "fact_group_count": s.facts.len(),
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} sheets", data.len()),
        data: Some(serde_json::json!(data)),
    })
}

/// GET /api/modules/seo/surfer/sheets/:id — get a single sheet by id.
pub(crate) async fn get_sheet(
    State(state): State<SeoState>,
    Path(id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let j = state.journal.lock().await;
    match load_sheet(&j, &id) {
        Some(sheet) => Json(ApiResult {
            ok: true,
            message: "sheet found".into(),
            data: Some(serde_json::to_value(&sheet).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: format!("sheet not found: {id}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/surfer/sheets/upload — parse and store a .txt file.
/// JSON body: `{ filename, content }`
pub(crate) async fn upload_sheet(
    State(state): State<SeoState>,
    Json(payload): Json<UploadSheetPayload>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    match parse_surfer_txt(&payload.content, &payload.filename) {
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("parse error: {e}"),
            data: None,
        }),
        Ok(sheet) => {
            let mut j = state.journal.lock().await;
            let sheet_id = sheet.sheet_id.clone();
            match save_sheet(&mut j, &sheet) {
                Ok(()) => Json(ApiResult {
                    ok: true,
                    message: format!("sheet saved: {sheet_id}"),
                    data: Some(serde_json::json!({ "sheet_id": sheet_id })),
                }),
                Err(e) => Json(ApiResult {
                    ok: false,
                    message: format!("save error: {e}"),
                    data: None,
                }),
            }
        }
    }
}

/// DELETE /api/modules/seo/surfer/sheets/:id — tombstone-delete a sheet.
pub(crate) async fn delete_sheet_handler(
    State(state): State<SeoState>,
    Path(id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;
    match delete_sheet(&mut j, &id) {
        Ok(()) => Json(ApiResult {
            ok: true,
            message: format!("sheet deleted: {id}"),
            data: None,
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("delete error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/surfer/import-dir — import all .txt files from the
/// `surfer/` directory relative to the binary's working directory.
pub(crate) async fn import_dir(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let dir = std::path::Path::new("surfer");
    let mut j = state.journal.lock().await;
    let (imported, errors) = import_directory(&mut j, dir);

    let error_list: Vec<serde_json::Value> = errors
        .iter()
        .map(|(f, e)| serde_json::json!({ "file": f, "error": e }))
        .collect();

    Json(ApiResult {
        ok: errors.is_empty(),
        message: format!("imported {imported}, {} errors", errors.len()),
        data: Some(serde_json::json!({
            "imported": imported,
            "errors": error_list,
        })),
    })
}

// ── Mapping ───────────────────────────────────────────────────────────────────

/// GET /api/modules/seo/surfer/map/unmapped — list pages with no surfer mapping.
pub(crate) async fn unmapped_pages(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    let mapped_ids: std::collections::HashSet<String> = load_all_maps(&j)
        .into_iter()
        .map(|m| m.content_id)
        .collect();

    let mgr = ForgeContentManager::new(&mut j);
    let (pages, _) = mgr
        .list_content(None, Some("published"), None, 500, 0, None, None)
        .unwrap_or_default();

    let unmapped: Vec<serde_json::Value> = pages
        .into_iter()
        .filter(|p| !mapped_ids.contains(&p.content_id))
        .map(|p| {
            serde_json::json!({
                "content_id": p.content_id,
                "slug": p.slug,
                "title": p.title,
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} unmapped pages", unmapped.len()),
        data: Some(serde_json::json!(unmapped)),
    })
}

/// GET /api/modules/seo/surfer/map/suggest/:content_id — auto-suggest sheets.
pub(crate) async fn suggest_map(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    // Load the page to get its slug.
    let (slug, industry) = {
        let mgr = ForgeContentManager::new(&mut j);
        match mgr.get_content(&content_id) {
            Ok(Some(page)) => {
                let ind = super::surfer::derive_industry(&page.slug);
                (page.slug, ind)
            }
            _ => (content_id.clone(), String::new()),
        }
    };

    let suggestions = suggest_sheets(&j, &content_id, &slug, &industry);
    let data: Vec<serde_json::Value> = suggestions
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "sheet_id": s.sheet_id,
                "topic": s.topic,
                "confidence": s.confidence,
                "match_reason": s.match_reason,
            })
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} suggestions", data.len()),
        data: Some(serde_json::json!(data)),
    })
}

/// GET /api/modules/seo/surfer/map/:content_id — get sheet mappings for a page.
pub(crate) async fn get_map(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let j = state.journal.lock().await;
    match load_map(&j, &content_id) {
        Some(map) => Json(ApiResult {
            ok: true,
            message: "mapping found".into(),
            data: Some(serde_json::to_value(&map).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: format!("no mapping for: {content_id}"),
            data: None,
        }),
    }
}

/// PUT /api/modules/seo/surfer/map/:content_id — set mappings for a page.
/// JSON body: `{ sheet_ids, primary_sheet_id }`
pub(crate) async fn set_map(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
    Json(payload): Json<SetMapPayload>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    let map = PageSurferMap {
        content_id: content_id.clone(),
        sheet_ids: payload.sheet_ids,
        primary_sheet_id: payload.primary_sheet_id,
        auto_suggested: false,
        confirmed: true,
        mapped_at: now_secs(),
    };

    match save_map(&mut j, &map) {
        Ok(()) => Json(ApiResult {
            ok: true,
            message: format!("mapping saved: {content_id}"),
            data: Some(serde_json::to_value(&map).unwrap_or_default()),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("save error: {e}"),
            data: None,
        }),
    }
}

/// POST /api/modules/seo/surfer/auto-map — bulk auto-map all unmapped pages.
pub(crate) async fn auto_map(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    // Gather all published pages.
    let pages: Vec<(String, String, String)> = {
        let mgr = ForgeContentManager::new(&mut j);
        let (all_pages, _) = mgr
            .list_content(None, Some("published"), None, 500, 0, None, None)
            .unwrap_or_default();
        all_pages
            .into_iter()
            .map(|p| {
                let industry = super::surfer::derive_industry(&p.slug);
                (p.content_id, p.slug, industry)
            })
            .collect()
    };

    let (mapped, skipped) = auto_map_all(&mut j, &pages, 0.3);

    Json(ApiResult {
        ok: true,
        message: format!("auto-mapped {mapped}, skipped {skipped}"),
        data: Some(serde_json::json!({ "mapped": mapped, "skipped": skipped })),
    })
}

// ── Queue ─────────────────────────────────────────────────────────────────────

/// GET /api/modules/seo/surfer/queue — list queue items, filterable by phase and priority.
pub(crate) async fn list_queue(
    State(state): State<SeoState>,
    Query(params): Query<QueueFilterQuery>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let j = state.journal.lock().await;
    let mut items = load_all_items(&j);

    if let Some(ref phase_str) = params.phase {
        if let Some(phase) = parse_queue_phase(phase_str) {
            items.retain(|i| i.phase == phase);
        }
    }
    if let Some(pri) = params.priority {
        items.retain(|i| i.priority == pri);
    }

    let data: Vec<serde_json::Value> = items
        .into_iter()
        .map(|i| serde_json::to_value(&i).unwrap_or_default())
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} items", data.len()),
        data: Some(serde_json::json!(data)),
    })
}

/// POST /api/modules/seo/surfer/queue/generate — scan all published pages,
/// score each, and create/update queue items.
pub(crate) async fn generate_queue(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;
    let batch = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Load all published pages.
    let pages = {
        let mgr = ForgeContentManager::new(&mut j);
        let (pages, _) = mgr
            .list_content(None, Some("published"), None, 1000, 0, None, None)
            .unwrap_or_default();
        pages
    };

    // Track "first 2 per page_type" for needs_human_review.
    let mut type_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    let mut created = 0usize;

    for page in pages {
        let body = &page.body_json;
        let word_count = strip_html_words(body);

        // Check if surfer map exists and load the score if so.
        let surfer_map = load_map(&j, &page.content_id);
        let has_surfer_sheet = surfer_map.is_some();

        let surfer_score: u8 = match &surfer_map {
            Some(map) => match load_sheet(&j, &map.primary_sheet_id) {
                Some(sheet) => score_against_sheet(body, &sheet).overall_surfer_score,
                None => 0,
            },
            None => 0,
        };

        // Load SEO score.
        let seo_score: u8 = {
            let event = j.get_latest(AGG_SEO_META, &page.content_id);
            match event {
                Some(e) if e.payload != TOMBSTONE => {
                    serde_json::from_slice::<super::SeoMeta>(&e.payload)
                        .map(|m| m.seo_score)
                        .unwrap_or(0)
                }
                _ => 0,
            }
        };

        // Compute priority.
        let (priority, reason) =
            compute_priority(word_count, surfer_score, seo_score, has_surfer_sheet, 0);

        // Skip pages that won't benefit (priority 5).
        if priority > 4 {
            continue;
        }

        let page_type = detect_page_type(&page.slug).to_string();
        let count = type_counts.entry(page_type.clone()).or_insert(0);
        let needs_human_review = *count < 2;
        *count += 1;

        let item = QueueItem {
            content_id: page.content_id.clone(),
            queue_batch: batch.clone(),
            slug: page.slug.clone(),
            page_type,
            priority,
            reason,
            seo_score,
            surfer_score,
            word_count,
            phase: QueuePhase::Pending,
            needs_human_review,
            content_ai_started_at: None,
            content_ai_completed_at: None,
            review_ai_started_at: None,
            review_ai_completed_at: None,
            published_at: None,
            error: None,
            notes: String::new(),
        };

        if save_item(&mut j, &item).is_ok() {
            created += 1;
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("queue generated: {created} items"),
        data: Some(serde_json::json!({ "created": created, "batch": batch })),
    })
}

/// GET /api/modules/seo/surfer/queue/next — claim the next page for content AI.
pub(crate) async fn queue_next(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    let content_id = match claim_next(&j, &QueuePhase::Pending) {
        Some(id) => id,
        None => {
            return Json(ApiResult {
                ok: true,
                message: "no items available".into(),
                data: Some(serde_json::json!({ "content_id": null })),
            });
        }
    };

    // Update the item's phase and started_at.
    if let Some(mut item) = load_item(&j, &content_id) {
        item.phase = QueuePhase::ContentAiInProgress;
        item.content_ai_started_at = Some(now_secs());
        let _ = save_item(&mut j, &item);
    }

    Json(ApiResult {
        ok: true,
        message: format!("claimed: {content_id}"),
        data: Some(serde_json::json!({ "content_id": content_id })),
    })
}

/// GET /api/modules/seo/surfer/queue/stats — summary counts by phase.
pub(crate) async fn queue_stats(State(state): State<SeoState>) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let j = state.journal.lock().await;
    let items = load_all_items(&j);

    let mut stats: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for item in &items {
        let phase_str = phase_to_str(&item.phase).to_string();
        *stats.entry(phase_str).or_insert(0) += 1;
    }

    let human_review = items.iter().filter(|i| i.needs_human_review).count();

    Json(ApiResult {
        ok: true,
        message: format!("{} total items", items.len()),
        data: Some(serde_json::json!({
            "total": items.len(),
            "by_phase": stats,
            "needs_human_review": human_review,
        })),
    })
}

/// PUT /api/modules/seo/surfer/queue/:content_id/status — update phase.
/// JSON body: `{ phase, notes?, error? }`
pub(crate) async fn update_queue_status(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
    Json(payload): Json<UpdateQueueStatusPayload>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;

    let mut item = match load_item(&j, &content_id) {
        Some(i) => i,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("queue item not found: {content_id}"),
                data: None,
            });
        }
    };

    let new_phase = match parse_queue_phase(&payload.phase) {
        Some(p) => p,
        None => {
            return Json(ApiResult {
                ok: false,
                message: format!("unknown phase: {}", payload.phase),
                data: None,
            });
        }
    };

    let now = now_secs();

    // Update the timestamp that corresponds to the new phase.
    match &new_phase {
        QueuePhase::ContentAiInProgress => {
            item.content_ai_started_at = Some(now);
        }
        QueuePhase::ContentAiDone => {
            item.content_ai_completed_at = Some(now);
        }
        QueuePhase::ReviewAiInProgress => {
            item.review_ai_started_at = Some(now);
        }
        QueuePhase::ReviewAiDone => {
            item.review_ai_completed_at = Some(now);
        }
        QueuePhase::Published => {
            item.published_at = Some(now);
        }
        _ => {}
    }

    item.phase = new_phase;

    if let Some(notes) = payload.notes {
        if !notes.is_empty() {
            item.notes = notes;
        }
    }
    if let Some(err) = payload.error {
        if !err.is_empty() {
            item.error = Some(err);
        }
    }

    match save_item(&mut j, &item) {
        Ok(()) => Json(ApiResult {
            ok: true,
            message: format!("status updated: {content_id}"),
            data: Some(serde_json::to_value(&item).unwrap_or_default()),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("save error: {e}"),
            data: None,
        }),
    }
}

// ── Intelligence ──────────────────────────────────────────────────────────────

/// GET /api/modules/seo/intelligence/:content_id — full page intelligence.
pub(crate) async fn page_intelligence(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;
    match assemble(&mut j, &content_id) {
        Ok(intel) => Json(ApiResult {
            ok: true,
            message: "intelligence assembled".into(),
            data: Some(serde_json::to_value(&intel).unwrap_or_default()),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        }),
    }
}

// ── Phase helpers ─────────────────────────────────────────────────────────────

fn parse_queue_phase(s: &str) -> Option<QueuePhase> {
    match s {
        "pending" => Some(QueuePhase::Pending),
        "content_ai_in_progress" => Some(QueuePhase::ContentAiInProgress),
        "content_ai_done" => Some(QueuePhase::ContentAiDone),
        "review_ai_in_progress" => Some(QueuePhase::ReviewAiInProgress),
        "review_ai_done" => Some(QueuePhase::ReviewAiDone),
        "published" => Some(QueuePhase::Published),
        "error" => Some(QueuePhase::Error),
        _ => None,
    }
}

/// POST /api/modules/seo/surfer/queue/:content_id/approve — clear human review flag.
pub(crate) async fn approve_queue_item(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    require_surfer_central!(&state);
    let mut j = state.journal.lock().await;
    match super::content_queue::load_item(&j, &content_id) {
        Some(mut item) => {
            item.needs_human_review = false;
            match super::content_queue::save_item(&mut j, &item) {
                Ok(_) => Json(ApiResult {
                    ok: true,
                    message: "Approved".into(),
                    data: None,
                }),
                Err(e) => Json(ApiResult {
                    ok: false,
                    message: e,
                    data: None,
                }),
            }
        }
        None => Json(ApiResult {
            ok: false,
            message: "Not in queue".into(),
            data: None,
        }),
    }
}

fn phase_to_str(phase: &QueuePhase) -> &'static str {
    match phase {
        QueuePhase::Pending => "pending",
        QueuePhase::ContentAiInProgress => "content_ai_in_progress",
        QueuePhase::ContentAiDone => "content_ai_done",
        QueuePhase::ReviewAiInProgress => "review_ai_in_progress",
        QueuePhase::ReviewAiDone => "review_ai_done",
        QueuePhase::Published => "published",
        QueuePhase::Error => "error",
    }
}
