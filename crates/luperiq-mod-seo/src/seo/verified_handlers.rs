//! HTTP handlers for the Verified Content System.
//!
//! All handlers follow the shared `SeoState` / `ApiResult` pattern. Routes are
//! wired in `mod.rs` via `seo_router()`.

use axum::extract::{Path, State};
use axum::Json;

use luperiq_forge::ForgeContentManager;

use super::verified::{check_drift, count_words, load_all_verified, load_verified, verify_page};
use super::{ApiResult, SeoState};

// ── POST /api/modules/seo/verified/:content_id/verify ────────────────────────

/// Verify a page. Reads current content from `ForgeContentManager`, computes
/// a blake3 hash, and saves (or updates) the verification record.
pub(crate) async fn verify_handler(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Load the current page content.
    let (body_json, title, word_count) = {
        let mgr = ForgeContentManager::new(&mut j);
        match mgr.get_content(&content_id) {
            Ok(Some(page)) => {
                let wc = count_words(&page.body_json);
                (page.body_json, page.title, wc)
            }
            Ok(None) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("page not found: {content_id}"),
                    data: None,
                });
            }
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("content load error: {e}"),
                    data: None,
                });
            }
        }
    };

    match verify_page(
        &mut j,
        &content_id,
        &body_json,
        &title,
        word_count,
        // No user auth info is threaded into SeoState; use a placeholder.
        "admin",
    ) {
        Ok(record) => Json(ApiResult {
            ok: true,
            message: "page verified".into(),
            data: Some(serde_json::to_value(&record).unwrap_or_default()),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: format!("verify failed: {e}"),
            data: None,
        }),
    }
}

// ── GET /api/modules/seo/verified/:content_id ────────────────────────────────

/// Get the verification record for a single page.
pub(crate) async fn get_verified(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    match load_verified(&j, &content_id) {
        Some(record) => Json(ApiResult {
            ok: true,
            message: "verification record found".into(),
            data: Some(serde_json::to_value(&record).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: format!("no verification record for: {content_id}"),
            data: None,
        }),
    }
}

// ── GET /api/modules/seo/verified/:content_id/drift ──────────────────────────

/// Check whether the current page content has drifted from its verified state.
pub(crate) async fn drift_handler(
    State(state): State<SeoState>,
    Path(content_id): Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;

    // Load current content to compute live hash.
    let body_json = {
        let mgr = ForgeContentManager::new(&mut j);
        match mgr.get_content(&content_id) {
            Ok(Some(page)) => page.body_json,
            Ok(None) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("page not found: {content_id}"),
                    data: None,
                });
            }
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("content load error: {e}"),
                    data: None,
                });
            }
        }
    };

    let report = check_drift(&j, &content_id, &body_json);
    Json(ApiResult {
        ok: true,
        message: if report.has_drifted {
            "content has drifted".into()
        } else {
            "content is current".into()
        },
        data: Some(serde_json::to_value(&report).unwrap_or_default()),
    })
}

// ── GET /api/modules/seo/verified ────────────────────────────────────────────

/// List all verification records.
pub(crate) async fn list_verified(State(state): State<SeoState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let all = load_all_verified(&j);
    let data: Vec<serde_json::Value> = all
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "content_id":           r.content_id,
                "title_at_verify":      r.title_at_verify,
                "content_hash":         r.content_hash,
                "word_count_at_verify": r.word_count_at_verify,
                "first_verified_at":    r.first_verified_at,
                "latest_verified_at":   r.latest_verified_at,
                "verified_by":          r.verified_by,
                "links_out_count":      r.internal_links_out.len(),
                "links_in_count":       r.internal_links_in.len(),
            })
        })
        .collect();
    Json(ApiResult {
        ok: true,
        message: format!("{} verified pages", data.len()),
        data: Some(serde_json::json!(data)),
    })
}

// ── POST /api/modules/seo/verified/bulk-drift ─────────────────────────────────

/// Check drift for all verified pages at once.
///
/// Returns a list of drift reports, filtered to only those that have drifted
/// (or all, if none have drifted, for completeness).
pub(crate) async fn bulk_drift(State(state): State<SeoState>) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let all_records = load_all_verified(&j);

    let mut reports: Vec<serde_json::Value> = Vec::new();

    for record in &all_records {
        // Attempt to load current content for each verified page.
        let body_json_opt = {
            let mgr = ForgeContentManager::new(&mut j);
            mgr.get_content(&record.content_id)
                .ok()
                .flatten()
                .map(|p| p.body_json)
        };

        let report = match body_json_opt {
            Some(body) => check_drift(&j, &record.content_id, &body),
            None => {
                // Page has been deleted — report as drifted with no live hash.
                super::verified::DriftReport {
                    content_id: record.content_id.clone(),
                    is_verified: true,
                    has_drifted: true,
                    verified_hash: Some(record.content_hash.clone()),
                    current_hash: String::new(),
                    verified_at: Some(record.latest_verified_at),
                    word_count_change: None,
                    links_added: Vec::new(),
                    links_removed: Vec::new(),
                }
            }
        };

        reports.push(serde_json::json!({
            "content_id":        report.content_id,
            "is_verified":       report.is_verified,
            "has_drifted":       report.has_drifted,
            "verified_hash":     report.verified_hash,
            "current_hash":      report.current_hash,
            "verified_at":       report.verified_at,
            "word_count_change": report.word_count_change,
            "links_added":       report.links_added,
            "links_removed":     report.links_removed,
        }));
    }

    let drifted_count = reports
        .iter()
        .filter(|r| r["has_drifted"].as_bool().unwrap_or(false))
        .count();

    Json(ApiResult {
        ok: true,
        message: format!("{} pages checked, {} drifted", reports.len(), drifted_count),
        data: Some(serde_json::json!({
            "total":   reports.len(),
            "drifted": drifted_count,
            "reports": reports,
        })),
    })
}
