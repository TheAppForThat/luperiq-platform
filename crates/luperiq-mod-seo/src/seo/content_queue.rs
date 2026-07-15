//! AI content work queue — per-page phase tracking stored in the WAL.
//!
//! Pages move through phases: Pending → ContentAiInProgress → ContentAiDone →
//! ReviewAiInProgress → ReviewAiDone → Published (or Error at any step).
//!
//! `claim_next` selects the highest-priority unclaimed page for a given phase,
//! with automatic stale-lock reclaim after `STALE_LOCK_SECS`.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use luperiq_forge::{ApexEvent, ForgeJournal};

use super::TOMBSTONE;

// ── Constants ─────────────────────────────────────────────────────────────────

pub const AGG_QUEUE_ITEM: &str = "AI:QueueItem";
/// Seconds before an in-progress lock is considered stale and can be reclaimed.
const STALE_LOCK_SECS: u64 = 600;

// ── Core types ────────────────────────────────────────────────────────────────

/// Processing phase for a queued page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueuePhase {
    Pending,
    ContentAiInProgress,
    ContentAiDone,
    ReviewAiInProgress,
    ReviewAiDone,
    Published,
    Error,
}

/// Per-page queue entry stored in the WAL under `AI:QueueItem`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub content_id: String,
    pub queue_batch: String,
    pub slug: String,
    /// Derived page type: "hub", "how-to", "feature", "topic", or "landing".
    pub page_type: String,
    /// 1 = critical, 2 = high, 3 = medium, 4 = low; 5 = won't be queued.
    pub priority: u8,
    pub reason: String,
    pub seo_score: u8,
    pub surfer_score: u8,
    pub word_count: u64,
    pub phase: QueuePhase,
    pub needs_human_review: bool,
    pub content_ai_started_at: Option<u64>,
    pub content_ai_completed_at: Option<u64>,
    pub review_ai_started_at: Option<u64>,
    pub review_ai_completed_at: Option<u64>,
    pub published_at: Option<u64>,
    pub error: Option<String>,
    pub notes: String,
}

// ── WAL operations ────────────────────────────────────────────────────────────

/// Persist a `QueueItem` to the journal (keyed by `content_id`).
pub fn save_item(journal: &mut ForgeJournal, item: &QueueItem) -> Result<(), String> {
    let payload = serde_json::to_vec(item).map_err(|e| format!("Serialize QueueItem: {e}"))?;
    let event = ApexEvent::new(AGG_QUEUE_ITEM, &item.content_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append QueueItem: {e}"))?;
    Ok(())
}

/// Write a tombstone event for the given `content_id`, logically deleting it.
pub fn delete_item(journal: &mut ForgeJournal, content_id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_QUEUE_ITEM, content_id, TOMBSTONE.to_vec());
    journal
        .append(event)
        .map_err(|e| format!("Journal append tombstone: {e}"))?;
    Ok(())
}

/// Load all non-deleted `QueueItem` values from the journal.
pub fn load_all_items(journal: &ForgeJournal) -> Vec<QueueItem> {
    journal
        .latest_by_aggregate_type(AGG_QUEUE_ITEM)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .collect()
}

/// Load a single `QueueItem` by its `content_id`. Returns `None` if not found
/// or if the latest event is a tombstone.
pub fn load_item(journal: &ForgeJournal, content_id: &str) -> Option<QueueItem> {
    let event = journal.get_latest(AGG_QUEUE_ITEM, content_id)?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

// ── Page-type detection ───────────────────────────────────────────────────────

/// Derive a page type from its slug.
///
/// Rules (evaluated in order):
/// 1. Ends with `"-hub"` → `"hub"`
/// 2. Starts with `"how-to-"` → `"how-to"`
/// 3. Contains a known feature keyword → `"feature"`
/// 4. Is `"/"`, `"home"`, or has no hyphens → `"landing"`
/// 5. Everything else → `"topic"`
pub fn detect_page_type(slug: &str) -> &'static str {
    const FEATURE_KEYWORDS: &[&str] = &[
        "scheduling",
        "invoicing",
        "marketing",
        "customer-portal",
        "service-area",
        "technician",
        "chemical",
        "ai-content",
        "website-design",
        "seo",
    ];

    if slug.ends_with("-hub") {
        return "hub";
    }
    if slug.starts_with("how-to-") {
        return "how-to";
    }
    for kw in FEATURE_KEYWORDS {
        if slug.contains(kw) {
            return "feature";
        }
    }
    if slug == "/" || slug == "home" || !slug.contains('-') {
        return "landing";
    }
    "topic"
}

// ── Priority computation ──────────────────────────────────────────────────────

/// Compute priority (1–5) and a human-readable reason string.
///
/// | Condition | Priority | Reason |
/// |-----------|----------|--------|
/// | word_count < 300 AND has_surfer_sheet | 1 | `"thin_content + has_surfer_sheet"` |
/// | surfer_score < 30 OR seo_score < 40 | 2 | (score labels) |
/// | surfer_score < 60 OR keyword_gate_passed < 5 | 3 | (labels) |
/// | surfer_score < 80 | 4 | `"surfer_score < 80"` |
/// | else | 5 | `"no_priority"` (won't be queued) |
pub fn compute_priority(
    word_count: u64,
    surfer_score: u8,
    seo_score: u8,
    has_surfer_sheet: bool,
    keyword_gate_passed: u8,
) -> (u8, String) {
    if word_count < 300 && has_surfer_sheet {
        return (1, "thin_content + has_surfer_sheet".to_string());
    }
    if surfer_score < 30 || seo_score < 40 {
        let mut parts = Vec::new();
        if surfer_score < 30 {
            parts.push("surfer_score < 30");
        }
        if seo_score < 40 {
            parts.push("seo_score < 40");
        }
        return (2, parts.join(" + "));
    }
    if surfer_score < 60 || keyword_gate_passed < 5 {
        let mut parts = Vec::new();
        if surfer_score < 60 {
            parts.push("surfer_score < 60");
        }
        if keyword_gate_passed < 5 {
            parts.push("keyword_gate_passed < 5");
        }
        return (3, parts.join(" + "));
    }
    if surfer_score < 80 {
        return (4, "surfer_score < 80".to_string());
    }
    (5, "no_priority".to_string())
}

// ── Claim next ────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Find and return the `content_id` of the next available page for a given
/// `target_phase`, or `None` if nothing is available.
///
/// Selection rules:
/// 1. Include items whose `phase == target_phase`.
/// 2. Also include stale-locked items: phase is the *in-progress* variant for
///    `target_phase` and the relevant `started_at` timestamp is older than
///    `STALE_LOCK_SECS`.
/// 3. Exclude items with `needs_human_review = true`.
/// 4. Sort by `priority` ascending then `surfer_score` ascending (worst first).
/// 5. Return the first match's `content_id`.
///
/// Stale-lock mapping:
/// - `Pending` → stale `ContentAiInProgress` (content_ai_started_at)
/// - `ContentAiDone` → stale `ReviewAiInProgress` (review_ai_started_at)
pub fn claim_next(journal: &ForgeJournal, target_phase: &QueuePhase) -> Option<String> {
    let now = now_secs();

    let mut candidates: Vec<QueueItem> = load_all_items(journal)
        .into_iter()
        .filter(|item| {
            if item.needs_human_review {
                return false;
            }
            // Direct match on the target phase
            if &item.phase == target_phase {
                return true;
            }
            // Stale lock reclaim
            match target_phase {
                QueuePhase::Pending => {
                    if item.phase == QueuePhase::ContentAiInProgress {
                        if let Some(started) = item.content_ai_started_at {
                            return now.saturating_sub(started) > STALE_LOCK_SECS;
                        }
                    }
                    false
                }
                QueuePhase::ContentAiDone => {
                    if item.phase == QueuePhase::ReviewAiInProgress {
                        if let Some(started) = item.review_ai_started_at {
                            return now.saturating_sub(started) > STALE_LOCK_SECS;
                        }
                    }
                    false
                }
                _ => false,
            }
        })
        .collect();

    // Sort: priority asc, then surfer_score asc (worst first = needs help most)
    candidates.sort_by(|a, b| {
        a.priority
            .cmp(&b.priority)
            .then(a.surfer_score.cmp(&b.surfer_score))
    });

    candidates.into_iter().next().map(|item| item.content_id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use luperiq_forge::{DurabilityMode, ForgeJournal};
    use tempfile::TempDir;

    fn make_journal() -> (ForgeJournal, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let journal = ForgeJournal::open(wal, snap, DurabilityMode::Sync).expect("journal open");
        (journal, dir)
    }

    fn make_item(content_id: &str, phase: QueuePhase, priority: u8, surfer_score: u8) -> QueueItem {
        QueueItem {
            content_id: content_id.to_string(),
            queue_batch: "batch-1".to_string(),
            slug: content_id.to_string(),
            page_type: "topic".to_string(),
            priority,
            reason: "test".to_string(),
            seo_score: 50,
            surfer_score,
            word_count: 500,
            phase,
            needs_human_review: false,
            content_ai_started_at: None,
            content_ai_completed_at: None,
            review_ai_started_at: None,
            review_ai_completed_at: None,
            published_at: None,
            error: None,
            notes: String::new(),
        }
    }

    // ── WAL round-trip ────────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_item() {
        let (mut journal, _dir) = make_journal();
        let item = make_item("page-001", QueuePhase::Pending, 2, 25);
        save_item(&mut journal, &item).unwrap();

        let loaded = load_item(&journal, "page-001").unwrap();
        assert_eq!(loaded.content_id, "page-001");
        assert_eq!(loaded.phase, QueuePhase::Pending);
        assert_eq!(loaded.priority, 2);
    }

    #[test]
    fn test_load_item_not_found() {
        let (journal, _dir) = make_journal();
        assert!(load_item(&journal, "nonexistent").is_none());
    }

    #[test]
    fn test_tombstone_hides_item() {
        let (mut journal, _dir) = make_journal();
        let item = make_item("page-002", QueuePhase::Pending, 3, 40);
        save_item(&mut journal, &item).unwrap();
        delete_item(&mut journal, "page-002").unwrap();
        assert!(load_item(&journal, "page-002").is_none());
    }

    #[test]
    fn test_load_all_items_excludes_tombstoned() {
        let (mut journal, _dir) = make_journal();
        save_item(&mut journal, &make_item("a", QueuePhase::Pending, 1, 10)).unwrap();
        save_item(&mut journal, &make_item("b", QueuePhase::Pending, 2, 20)).unwrap();
        delete_item(&mut journal, "a").unwrap();

        let items = load_all_items(&journal);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content_id, "b");
    }

    #[test]
    fn test_overwrite_updates_phase() {
        let (mut journal, _dir) = make_journal();
        let item = make_item("page-003", QueuePhase::Pending, 2, 30);
        save_item(&mut journal, &item).unwrap();

        let mut updated = item.clone();
        updated.phase = QueuePhase::ContentAiInProgress;
        save_item(&mut journal, &updated).unwrap();

        let loaded = load_item(&journal, "page-003").unwrap();
        assert_eq!(loaded.phase, QueuePhase::ContentAiInProgress);
    }

    // ── detect_page_type ──────────────────────────────────────────────────────

    #[test]
    fn test_detect_hub() {
        assert_eq!(detect_page_type("pest-control-hub"), "hub");
        assert_eq!(detect_page_type("landscaping-hub"), "hub");
    }

    #[test]
    fn test_detect_how_to() {
        assert_eq!(detect_page_type("how-to-get-rid-of-ants"), "how-to");
        assert_eq!(detect_page_type("how-to-invoice-clients"), "how-to");
    }

    #[test]
    fn test_detect_feature() {
        assert_eq!(detect_page_type("online-scheduling-software"), "feature");
        assert_eq!(detect_page_type("pest-control-invoicing"), "feature");
        assert_eq!(detect_page_type("marketing-automation"), "feature");
        assert_eq!(detect_page_type("customer-portal-login"), "feature");
        assert_eq!(detect_page_type("service-area-management"), "feature");
        assert_eq!(detect_page_type("technician-app"), "feature");
        assert_eq!(detect_page_type("chemical-tracking"), "feature");
        assert_eq!(detect_page_type("ai-content-writer"), "feature");
        assert_eq!(detect_page_type("website-design-services"), "feature");
        assert_eq!(detect_page_type("seo-for-pest-control"), "feature");
    }

    #[test]
    fn test_detect_landing() {
        assert_eq!(detect_page_type("/"), "landing");
        assert_eq!(detect_page_type("home"), "landing");
        assert_eq!(detect_page_type("about"), "landing"); // no hyphens
        assert_eq!(detect_page_type("pricing"), "landing");
    }

    #[test]
    fn test_detect_topic() {
        assert_eq!(detect_page_type("bed-bug-treatment"), "topic");
        assert_eq!(detect_page_type("termite-prevention-tips"), "topic");
    }

    #[test]
    fn test_detect_hub_wins_over_feature() {
        // "-hub" suffix checked before feature keywords
        assert_eq!(detect_page_type("seo-hub"), "hub");
    }

    // ── compute_priority ─────────────────────────────────────────────────────

    #[test]
    fn test_priority_1_thin_content_with_sheet() {
        let (p, r) = compute_priority(200, 70, 70, true, 10);
        assert_eq!(p, 1);
        assert_eq!(r, "thin_content + has_surfer_sheet");
    }

    #[test]
    fn test_priority_1_requires_sheet() {
        // thin content without sheet should fall through to priority 2 check
        let (p, _r) = compute_priority(200, 20, 30, false, 0);
        assert_eq!(p, 2); // triggered by surfer_score < 30
    }

    #[test]
    fn test_priority_2_low_surfer() {
        let (p, r) = compute_priority(500, 20, 70, false, 10);
        assert_eq!(p, 2);
        assert!(r.contains("surfer_score < 30"));
    }

    #[test]
    fn test_priority_2_low_seo() {
        let (p, r) = compute_priority(500, 35, 35, false, 10);
        assert_eq!(p, 2);
        assert!(r.contains("seo_score < 40"));
    }

    #[test]
    fn test_priority_2_both_flags() {
        let (p, r) = compute_priority(500, 20, 35, false, 10);
        assert_eq!(p, 2);
        assert!(r.contains("surfer_score < 30"));
        assert!(r.contains("seo_score < 40"));
    }

    #[test]
    fn test_priority_3_surfer() {
        let (p, r) = compute_priority(500, 50, 60, false, 10);
        assert_eq!(p, 3);
        assert!(r.contains("surfer_score < 60"));
    }

    #[test]
    fn test_priority_3_keyword_gate() {
        let (p, r) = compute_priority(500, 65, 60, false, 3);
        assert_eq!(p, 3);
        assert!(r.contains("keyword_gate_passed < 5"));
    }

    #[test]
    fn test_priority_4() {
        let (p, r) = compute_priority(500, 70, 60, false, 10);
        assert_eq!(p, 4);
        assert_eq!(r, "surfer_score < 80");
    }

    #[test]
    fn test_priority_5_no_action() {
        let (p, r) = compute_priority(1000, 85, 80, false, 10);
        assert_eq!(p, 5);
        assert_eq!(r, "no_priority");
    }

    // ── claim_next ────────────────────────────────────────────────────────────

    #[test]
    fn test_claim_next_returns_highest_priority() {
        let (mut journal, _dir) = make_journal();
        save_item(
            &mut journal,
            &make_item("low-pri", QueuePhase::Pending, 3, 40),
        )
        .unwrap();
        save_item(
            &mut journal,
            &make_item("high-pri", QueuePhase::Pending, 1, 10),
        )
        .unwrap();
        save_item(
            &mut journal,
            &make_item("mid-pri", QueuePhase::Pending, 2, 25),
        )
        .unwrap();

        let claimed = claim_next(&journal, &QueuePhase::Pending).unwrap();
        assert_eq!(claimed, "high-pri");
    }

    #[test]
    fn test_claim_next_tiebreak_by_surfer_score() {
        let (mut journal, _dir) = make_journal();
        save_item(
            &mut journal,
            &make_item("worse-surfer", QueuePhase::Pending, 2, 15),
        )
        .unwrap();
        save_item(
            &mut journal,
            &make_item("better-surfer", QueuePhase::Pending, 2, 25),
        )
        .unwrap();

        let claimed = claim_next(&journal, &QueuePhase::Pending).unwrap();
        assert_eq!(claimed, "worse-surfer");
    }

    #[test]
    fn test_claim_next_skips_human_review() {
        let (mut journal, _dir) = make_journal();
        let mut item = make_item("needs-review", QueuePhase::Pending, 1, 5);
        item.needs_human_review = true;
        save_item(&mut journal, &item).unwrap();
        save_item(
            &mut journal,
            &make_item("normal", QueuePhase::Pending, 3, 50),
        )
        .unwrap();

        let claimed = claim_next(&journal, &QueuePhase::Pending).unwrap();
        assert_eq!(claimed, "normal");
    }

    #[test]
    fn test_claim_next_returns_none_when_empty() {
        let (journal, _dir) = make_journal();
        assert!(claim_next(&journal, &QueuePhase::Pending).is_none());
    }

    #[test]
    fn test_claim_next_stale_content_ai_lock() {
        let (mut journal, _dir) = make_journal();
        // Simulate a stale ContentAiInProgress lock (started 700 seconds ago)
        let stale_started = now_secs().saturating_sub(700);
        let mut stale = make_item("stale-content", QueuePhase::ContentAiInProgress, 2, 20);
        stale.content_ai_started_at = Some(stale_started);
        save_item(&mut journal, &stale).unwrap();

        // claim_next for Pending should reclaim the stale lock
        let claimed = claim_next(&journal, &QueuePhase::Pending).unwrap();
        assert_eq!(claimed, "stale-content");
    }

    #[test]
    fn test_claim_next_fresh_lock_not_reclaimed() {
        let (mut journal, _dir) = make_journal();
        // Fresh lock started 30 seconds ago — not stale
        let fresh_started = now_secs().saturating_sub(30);
        let mut fresh = make_item("fresh-content", QueuePhase::ContentAiInProgress, 2, 20);
        fresh.content_ai_started_at = Some(fresh_started);
        save_item(&mut journal, &fresh).unwrap();

        // No Pending items, and the ContentAiInProgress is not stale
        assert!(claim_next(&journal, &QueuePhase::Pending).is_none());
    }

    #[test]
    fn test_claim_next_stale_review_ai_lock() {
        let (mut journal, _dir) = make_journal();
        let stale_started = now_secs().saturating_sub(700);
        let mut stale = make_item("stale-review", QueuePhase::ReviewAiInProgress, 3, 30);
        stale.review_ai_started_at = Some(stale_started);
        save_item(&mut journal, &stale).unwrap();

        let claimed = claim_next(&journal, &QueuePhase::ContentAiDone).unwrap();
        assert_eq!(claimed, "stale-review");
    }

    #[test]
    fn test_claim_next_wrong_phase_not_returned() {
        let (mut journal, _dir) = make_journal();
        save_item(
            &mut journal,
            &make_item("published", QueuePhase::Published, 1, 5),
        )
        .unwrap();

        assert!(claim_next(&journal, &QueuePhase::Pending).is_none());
    }

    #[test]
    fn test_full_phase_progression() {
        let (mut journal, _dir) = make_journal();
        let mut item = make_item("workflow-page", QueuePhase::Pending, 2, 25);
        save_item(&mut journal, &item).unwrap();

        // Claim for content AI
        let id = claim_next(&journal, &QueuePhase::Pending).unwrap();
        assert_eq!(id, "workflow-page");

        item.phase = QueuePhase::ContentAiInProgress;
        item.content_ai_started_at = Some(now_secs());
        save_item(&mut journal, &item).unwrap();

        // Should not appear again for Pending (lock is fresh)
        assert!(claim_next(&journal, &QueuePhase::Pending).is_none());

        // Advance to ContentAiDone
        item.phase = QueuePhase::ContentAiDone;
        item.content_ai_completed_at = Some(now_secs());
        save_item(&mut journal, &item).unwrap();

        // Now available for review AI
        let id2 = claim_next(&journal, &QueuePhase::ContentAiDone).unwrap();
        assert_eq!(id2, "workflow-page");
    }
}
