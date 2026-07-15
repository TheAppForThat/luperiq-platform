//! Crawl Summary — aggregated bot crawl activity for the analytics dashboard.
//!
//! Reads SeoCrawl events from the WAL and returns a summary with totals,
//! per-bot breakdowns, and 404 error counts over a configurable time window.

use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;

use super::crawl_tracker::{load_all_events, CrawlEvent};

// ── Query params ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct SummaryQuery {
    /// Number of days to look back (default 7).
    pub days: Option<u32>,
}

// ── Handler ──────────────────────────────────────────────────────────

pub(crate) async fn crawl_summary(
    State(state): State<super::SeoState>,
    Query(q): Query<SummaryQuery>,
) -> Json<serde_json::Value> {
    let period_days = q.days.unwrap_or(7);
    let cutoff = Utc::now() - chrono::Duration::days(i64::from(period_days));
    let cutoff_str = cutoff.format("%Y-%m-%dT%H:%M:%S").to_string();

    let j = state.journal.lock().await;
    let events = load_all_events(&j);
    drop(j); // release lock early

    // Filter to events within the requested window
    let recent: Vec<&CrawlEvent> = events
        .iter()
        .filter(|e| e.timestamp.as_str() >= cutoff_str.as_str())
        .collect();

    let total_crawls = recent.len();

    // Group by bot name
    let mut by_bot: HashMap<&str, usize> = HashMap::new();
    let mut errors_404: usize = 0;

    for ev in &recent {
        *by_bot.entry(ev.bot_name.as_str()).or_insert(0) += 1;
        if ev.status_code == 404 {
            errors_404 += 1;
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "total_crawls": total_crawls,
        "by_bot": by_bot,
        "errors_404": errors_404,
        "period_days": period_days,
    }))
}
