//! Verified Content System — page content hashing, drift detection, and WAL persistence.
//!
//! Stores a blake3 hash of page content at verification time. Detects when content
//! has drifted from the verified state. Tracks internal link structure over time.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use luperiq_forge::{ApexEvent, ForgeJournal};

use super::TOMBSTONE;

// ── Aggregate type constants ──────────────────────────────────────────────────

pub const AGG_VERIFIED: &str = "Page:Verified";

// ── Core types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageVerified {
    pub content_id: String,
    /// blake3 hex hash of body_json at verification time.
    pub content_hash: String,
    /// Title of the page when it was verified.
    pub title_at_verify: String,
    pub word_count_at_verify: u64,
    /// Unix timestamp of the first verification.
    pub first_verified_at: u64,
    /// Unix timestamp of the most recent verification.
    pub latest_verified_at: u64,
    /// Email address of the user who performed the verification.
    pub verified_by: String,
    /// Slugs this page links to (outgoing internal links).
    pub internal_links_out: Vec<String>,
    /// Slugs that link to this page (populated separately via backlink pass).
    pub internal_links_in: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriftReport {
    pub content_id: String,
    /// `true` if a verification record exists for this page.
    pub is_verified: bool,
    /// `true` if the current content hash differs from the verified hash.
    pub has_drifted: bool,
    pub verified_hash: Option<String>,
    pub current_hash: String,
    /// Unix timestamp of the last verification, if any.
    pub verified_at: Option<u64>,
    /// Positive means content grew; negative means it shrank.
    pub word_count_change: Option<i64>,
    pub links_added: Vec<String>,
    pub links_removed: Vec<String>,
}

// ── Hash and link extraction ──────────────────────────────────────────────────

/// Compute a blake3 hex hash of a content string.
pub fn compute_content_hash(body_json: &str) -> String {
    let hash = blake3::hash(body_json.as_bytes());
    hash.to_hex().to_string()
}

/// Extract internal link slugs from a body JSON string.
///
/// Searches for `href="/<slug>"` patterns and returns the slug portion.
/// Only matches paths starting with `/` followed by at least one non-slash character.
pub fn extract_internal_links(body_json: &str) -> Vec<String> {
    let mut slugs: Vec<String> = Vec::new();
    let needle = "href=\"/";

    let mut search = body_json;
    while let Some(pos) = search.find(needle) {
        let after = &search[pos + needle.len()..];
        // Find the closing quote.
        if let Some(end) = after.find('"') {
            let slug_candidate = &after[..end];
            // Skip empty paths, anchors, absolute URLs slipping through, and
            // paths that start with another slash (e.g. `//cdn.example.com`).
            if !slug_candidate.is_empty()
                && !slug_candidate.starts_with('/')
                && !slug_candidate.starts_with('#')
            {
                // Strip any trailing query/fragment for storage.
                let slug = slug_candidate
                    .split('?')
                    .next()
                    .unwrap_or(slug_candidate)
                    .split('#')
                    .next()
                    .unwrap_or(slug_candidate)
                    .to_string();
                if !slug.is_empty() && !slugs.contains(&slug) {
                    slugs.push(slug);
                }
            }
        }
        // Advance past this occurrence.
        search = &search[pos + needle.len()..];
    }
    slugs
}

// ── Timestamp helper ──────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Count words in a body_json string by stripping HTML tags and splitting on
/// whitespace. Returns 0 if the string is empty.
pub fn count_words(body_json: &str) -> u64 {
    let mut plain = String::with_capacity(body_json.len());
    let mut in_tag = false;
    for ch in body_json.chars() {
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

// ── WAL operations ────────────────────────────────────────────────────────────

/// Save a `PageVerified` record to the journal.
///
/// Uses `record.content_id` as the aggregate ID so successive saves replace
/// the previous record (latest-event semantics).
pub fn save_verified(journal: &mut ForgeJournal, record: &PageVerified) -> Result<(), String> {
    let payload = serde_json::to_vec(record).map_err(|e| format!("Serialize PageVerified: {e}"))?;
    let event = ApexEvent::new(AGG_VERIFIED, &record.content_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append PageVerified: {e}"))?;
    Ok(())
}

/// Write a tombstone event for the given `content_id`, logically deleting it.
pub fn delete_verified(journal: &mut ForgeJournal, content_id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_VERIFIED, content_id, TOMBSTONE.to_vec());
    journal
        .append(event)
        .map_err(|e| format!("Journal append tombstone: {e}"))?;
    Ok(())
}

/// Load a single `PageVerified` record by `content_id`.
///
/// Returns `None` if no record exists or the latest event is a tombstone.
pub fn load_verified(journal: &ForgeJournal, content_id: &str) -> Option<PageVerified> {
    let event = journal.get_latest(AGG_VERIFIED, content_id)?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

/// Load all non-deleted `PageVerified` records from the journal.
pub fn load_all_verified(journal: &ForgeJournal) -> Vec<PageVerified> {
    journal
        .latest_by_aggregate_type(AGG_VERIFIED)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .collect()
}

// ── High-level operations ─────────────────────────────────────────────────────

/// Create or update a verification record for a page.
///
/// - If a record already exists, `first_verified_at` is preserved and only
///   `content_hash`, `title_at_verify`, `word_count_at_verify`,
///   `latest_verified_at`, `verified_by`, and `internal_links_out` are updated.
/// - If no record exists, both timestamps are set to now.
pub fn verify_page(
    journal: &mut ForgeJournal,
    content_id: &str,
    body_json: &str,
    title: &str,
    word_count: u64,
    verified_by: &str,
) -> Result<PageVerified, String> {
    let now = now_secs();
    let content_hash = compute_content_hash(body_json);
    let internal_links_out = extract_internal_links(body_json);

    let (first_verified_at, links_in) = match load_verified(journal, content_id) {
        Some(existing) => (existing.first_verified_at, existing.internal_links_in),
        None => (now, Vec::new()),
    };

    let record = PageVerified {
        content_id: content_id.to_string(),
        content_hash,
        title_at_verify: title.to_string(),
        word_count_at_verify: word_count,
        first_verified_at,
        latest_verified_at: now,
        verified_by: verified_by.to_string(),
        internal_links_out,
        internal_links_in: links_in,
        notes: String::new(),
    };

    save_verified(journal, &record)?;
    Ok(record)
}

/// Check whether a page's current content has drifted from its verified state.
pub fn check_drift(
    journal: &ForgeJournal,
    content_id: &str,
    current_body_json: &str,
) -> DriftReport {
    let current_hash = compute_content_hash(current_body_json);
    let current_links = extract_internal_links(current_body_json);
    let current_wc = count_words(current_body_json) as i64;

    match load_verified(journal, content_id) {
        None => DriftReport {
            content_id: content_id.to_string(),
            is_verified: false,
            has_drifted: false,
            verified_hash: None,
            current_hash,
            verified_at: None,
            word_count_change: None,
            links_added: Vec::new(),
            links_removed: Vec::new(),
        },
        Some(record) => {
            let has_drifted = record.content_hash != current_hash;
            let word_count_change = Some(current_wc - record.word_count_at_verify as i64);

            let verified_set: std::collections::HashSet<String> =
                record.internal_links_out.iter().cloned().collect();
            let current_set: std::collections::HashSet<String> =
                current_links.iter().cloned().collect();

            let links_added: Vec<String> = current_set.difference(&verified_set).cloned().collect();
            let links_removed: Vec<String> =
                verified_set.difference(&current_set).cloned().collect();

            DriftReport {
                content_id: content_id.to_string(),
                is_verified: true,
                has_drifted,
                verified_hash: Some(record.content_hash),
                current_hash,
                verified_at: Some(record.latest_verified_at),
                word_count_change,
                links_added,
                links_removed,
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use luperiq_forge::{DurabilityMode, ForgeJournal};
    use tempfile::TempDir;

    fn make_journal(tmp: &TempDir) -> ForgeJournal {
        let wal = tmp.path().join("events.wal");
        let snap = tmp.path().join("snapshot.bin");
        ForgeJournal::open(
            wal.to_str().unwrap(),
            snap.to_str().unwrap(),
            DurabilityMode::Sync,
        )
        .unwrap()
    }

    // ── compute_content_hash ─────────────────────────────────────────────────

    #[test]
    fn test_hash_is_deterministic() {
        let h1 = compute_content_hash("hello world");
        let h2 = compute_content_hash("hello world");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_is_64_hex_chars() {
        let h = compute_content_hash("some body content");
        assert_eq!(h.len(), 64, "blake3 hex output must be 64 chars");
    }

    #[test]
    fn test_different_content_different_hash() {
        let h1 = compute_content_hash("foo");
        let h2 = compute_content_hash("bar");
        assert_ne!(h1, h2);
    }

    // ── extract_internal_links ───────────────────────────────────────────────

    #[test]
    fn test_extract_links_basic() {
        let body = r#"<a href="/about">About</a> and <a href="/contact">Contact</a>"#;
        let links = extract_internal_links(body);
        assert!(links.contains(&"about".to_string()));
        assert!(links.contains(&"contact".to_string()));
    }

    #[test]
    fn test_extract_links_no_duplicates() {
        let body = r#"<a href="/about">A</a><a href="/about">B</a>"#;
        let links = extract_internal_links(body);
        assert_eq!(links.iter().filter(|l| l.as_str() == "about").count(), 1);
    }

    #[test]
    fn test_extract_links_ignores_external() {
        let body = r#"<a href="https://example.com">Ext</a>"#;
        let links = extract_internal_links(body);
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_links_strips_query() {
        let body = r#"<a href="/services?tab=all">Services</a>"#;
        let links = extract_internal_links(body);
        assert_eq!(links, vec!["services".to_string()]);
    }

    #[test]
    fn test_extract_links_empty() {
        let links = extract_internal_links("");
        assert!(links.is_empty());
    }

    // ── WAL round-trip ───────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_verified() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let record = PageVerified {
            content_id: "page-01".to_string(),
            content_hash: compute_content_hash("some content"),
            title_at_verify: "My Page".to_string(),
            word_count_at_verify: 42,
            first_verified_at: 1000,
            latest_verified_at: 2000,
            verified_by: "admin@example.com".to_string(),
            internal_links_out: vec!["about".to_string()],
            internal_links_in: vec![],
            notes: String::new(),
        };

        save_verified(&mut j, &record).unwrap();
        let loaded = load_verified(&j, "page-01").unwrap();
        assert_eq!(loaded.content_id, "page-01");
        assert_eq!(loaded.title_at_verify, "My Page");
        assert_eq!(loaded.word_count_at_verify, 42);
        assert_eq!(loaded.internal_links_out, vec!["about".to_string()]);
    }

    #[test]
    fn test_load_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        let j = make_journal(&tmp);
        assert!(load_verified(&j, "does-not-exist").is_none());
    }

    #[test]
    fn test_tombstone_hides_record() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let record = PageVerified {
            content_id: "page-02".to_string(),
            content_hash: compute_content_hash("x"),
            title_at_verify: "T".to_string(),
            word_count_at_verify: 1,
            first_verified_at: 1,
            latest_verified_at: 1,
            verified_by: "u".to_string(),
            internal_links_out: vec![],
            internal_links_in: vec![],
            notes: String::new(),
        };
        save_verified(&mut j, &record).unwrap();
        delete_verified(&mut j, "page-02").unwrap();
        assert!(load_verified(&j, "page-02").is_none());
    }

    #[test]
    fn test_load_all_verified_excludes_tombstones() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        for id in &["p1", "p2", "p3"] {
            let r = PageVerified {
                content_id: id.to_string(),
                content_hash: compute_content_hash(id),
                title_at_verify: id.to_string(),
                word_count_at_verify: 10,
                first_verified_at: 1,
                latest_verified_at: 1,
                verified_by: "u".to_string(),
                internal_links_out: vec![],
                internal_links_in: vec![],
                notes: String::new(),
            };
            save_verified(&mut j, &r).unwrap();
        }
        delete_verified(&mut j, "p2").unwrap();

        let all = load_all_verified(&j);
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|r| r.content_id != "p2"));
    }

    // ── verify_page ──────────────────────────────────────────────────────────

    #[test]
    fn test_verify_page_creates_record() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let body = r#"<p>Hello <a href="/about">world</a></p>"#;
        let rec = verify_page(&mut j, "home", body, "Home", 2, "admin@x.com").unwrap();
        assert_eq!(rec.content_id, "home");
        assert_eq!(rec.content_hash, compute_content_hash(body));
        assert_eq!(rec.verified_by, "admin@x.com");
        assert!(rec.internal_links_out.contains(&"about".to_string()));
    }

    #[test]
    fn test_verify_page_preserves_first_verified_at() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let body1 = "initial content";
        let r1 = verify_page(&mut j, "pg", body1, "T", 2, "a@b.com").unwrap();
        let first_ts = r1.first_verified_at;

        // Small sleep is avoided — timestamps may collide in fast tests; we just
        // check the field is preserved rather than changed.
        let body2 = "updated content";
        let r2 = verify_page(&mut j, "pg", body2, "T2", 3, "a@b.com").unwrap();
        assert_eq!(
            r2.first_verified_at, first_ts,
            "first_verified_at must not change on re-verify"
        );
        assert_eq!(r2.content_hash, compute_content_hash(body2));
    }

    // ── check_drift ──────────────────────────────────────────────────────────

    #[test]
    fn test_drift_unverified_page() {
        let tmp = TempDir::new().unwrap();
        let j = make_journal(&tmp);
        let report = check_drift(&j, "unknown", "some content");
        assert!(!report.is_verified);
        assert!(!report.has_drifted);
        assert!(report.verified_hash.is_none());
    }

    #[test]
    fn test_no_drift_same_content() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let body = "unchanged content";
        verify_page(&mut j, "p", body, "T", 2, "u").unwrap();
        let report = check_drift(&j, "p", body);
        assert!(report.is_verified);
        assert!(!report.has_drifted);
        assert!(report.links_added.is_empty());
        assert!(report.links_removed.is_empty());
    }

    #[test]
    fn test_drift_detected_on_change() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        let original = r#"<p>Original <a href="/about">link</a></p>"#;
        verify_page(&mut j, "p", original, "T", 5, "u").unwrap();

        let updated = r#"<p>Updated content <a href="/contact">new link</a></p>"#;
        let report = check_drift(&j, "p", updated);
        assert!(report.is_verified);
        assert!(report.has_drifted);
        assert!(report.links_added.contains(&"contact".to_string()));
        assert!(report.links_removed.contains(&"about".to_string()));
    }

    #[test]
    fn test_word_count_change_positive() {
        let tmp = TempDir::new().unwrap();
        let mut j = make_journal(&tmp);

        verify_page(&mut j, "pg", "one two three", "T", 3, "u").unwrap();
        let report = check_drift(&j, "pg", "one two three four five six seven");
        assert_eq!(report.word_count_change, Some(4));
    }
}
