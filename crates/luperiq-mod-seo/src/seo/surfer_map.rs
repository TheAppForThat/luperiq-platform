//! Page-to-Surfer-Sheet mapping with auto-suggest.
//!
//! Stores `PageSurferMap` aggregates in the ForgeJournal under the
//! `Page:SurferMap` aggregate type, keyed by `content_id`.
//!
//! Auto-suggest scores each available sheet against a page using three signals:
//! - **Focus keyword match** (weight 0.5): sheet topic appears in the page's
//!   `SeoMeta.focus_keyword`.
//! - **Industry match** (weight 0.3): sheet industry matches the page's
//!   industry slug.
//! - **Slug token overlap** (weight 0.2): fraction of slug tokens (split on
//!   `-`) that appear in the sheet topic tokens.

use serde::{Deserialize, Serialize};

use luperiq_forge::{ApexEvent, ForgeJournal};

use super::surfer::load_all_sheets;
use super::SeoMeta;
use super::{AGG_SEO_META, TOMBSTONE as SEO_TOMBSTONE};

// ── Aggregate type constants ──────────────────────────────────────────────────

pub const AGG_SURFER_MAP: &str = "Page:SurferMap";

// ── Core types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSurferMap {
    pub content_id: String,
    pub sheet_ids: Vec<String>,
    pub primary_sheet_id: String,
    pub auto_suggested: bool,
    pub confirmed: bool,
    pub mapped_at: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SheetSuggestion {
    pub sheet_id: String,
    pub topic: String,
    pub confidence: f64, // 0.0 - 1.0
    pub match_reason: String,
}

// ── WAL operations ────────────────────────────────────────────────────────────

/// Persist a `PageSurferMap` to the journal.
///
/// Uses `map.content_id` as the aggregate ID so subsequent saves replace the
/// same aggregate (latest-event semantics).
pub fn save_map(journal: &mut ForgeJournal, map: &PageSurferMap) -> Result<(), String> {
    let payload = serde_json::to_vec(map).map_err(|e| format!("Serialize PageSurferMap: {e}"))?;
    let event = ApexEvent::new(AGG_SURFER_MAP, &map.content_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append PageSurferMap: {e}"))?;
    Ok(())
}

/// Write a tombstone event for the given `content_id`, logically deleting it.
pub fn delete_map(journal: &mut ForgeJournal, content_id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_SURFER_MAP, content_id, SEO_TOMBSTONE.to_vec());
    journal
        .append(event)
        .map_err(|e| format!("Journal append map tombstone: {e}"))?;
    Ok(())
}

/// Load a single `PageSurferMap` by `content_id`. Returns `None` if not found
/// or if the latest event is a tombstone.
pub fn load_map(journal: &ForgeJournal, content_id: &str) -> Option<PageSurferMap> {
    let event = journal.get_latest(AGG_SURFER_MAP, content_id)?;
    if event.payload == SEO_TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

/// Load all non-deleted `PageSurferMap` values from the journal.
pub fn load_all_maps(journal: &ForgeJournal) -> Vec<PageSurferMap> {
    journal
        .latest_by_aggregate_type(AGG_SURFER_MAP)
        .into_iter()
        .filter(|e| e.payload != SEO_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .collect()
}

// ── Auto-suggest ──────────────────────────────────────────────────────────────

/// Score all available surfer sheets against a page and return suggestions.
///
/// Scoring signals (weights sum to 1.0):
/// - Focus keyword match (0.5): does the sheet's topic appear (case-insensitive)
///   in the `SeoMeta.focus_keyword` for `content_id`?
/// - Industry match (0.3): does the sheet's `industry` field equal `industry_slug`?
/// - Slug token overlap (0.2): fraction of slug tokens (split on `-`) that
///   appear in the sheet topic's tokens.
///
/// Results are sorted by confidence descending and filtered to > 0.2.
pub fn suggest_sheets(
    journal: &ForgeJournal,
    content_id: &str,
    slug: &str,
    industry_slug: &str,
) -> Vec<SheetSuggestion> {
    // Load the focus keyword for this page from SeoMeta if available.
    let focus_keyword = load_focus_keyword(journal, content_id);

    let slug_tokens: Vec<String> = slug
        .split('-')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_lowercase())
        .collect();

    let sheets = load_all_sheets(journal);
    let mut suggestions: Vec<SheetSuggestion> = sheets
        .into_iter()
        .map(|sheet| {
            let topic_lower = sheet.topic.to_lowercase();
            let topic_tokens: Vec<&str> = topic_lower.split_whitespace().collect();

            // Signal 1: focus keyword match (0.5)
            let kw_score = if !focus_keyword.is_empty()
                && topic_lower.contains(&focus_keyword.to_lowercase())
            {
                0.5_f64
            } else {
                0.0_f64
            };

            // Signal 2: industry match (0.3)
            let industry_score = if !industry_slug.is_empty() && sheet.industry == industry_slug {
                0.3_f64
            } else {
                0.0_f64
            };

            // Signal 3: slug token overlap (0.2)
            let overlap_score = if slug_tokens.is_empty() {
                0.0_f64
            } else {
                let matches = slug_tokens
                    .iter()
                    .filter(|tok| topic_tokens.contains(&tok.as_str()))
                    .count();
                0.2_f64 * (matches as f64 / slug_tokens.len() as f64)
            };

            let confidence = kw_score + industry_score + overlap_score;

            // Build a human-readable explanation.
            let mut reasons: Vec<&str> = Vec::new();
            if kw_score > 0.0 {
                reasons.push("focus keyword");
            }
            if industry_score > 0.0 {
                reasons.push("industry");
            }
            if overlap_score > 0.0 {
                reasons.push("slug tokens");
            }
            let match_reason = if reasons.is_empty() {
                "no match".to_string()
            } else {
                reasons.join(", ")
            };

            SheetSuggestion {
                sheet_id: sheet.sheet_id,
                topic: sheet.topic,
                confidence,
                match_reason,
            }
        })
        .filter(|s| s.confidence > 0.2)
        .collect();

    suggestions.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suggestions
}

/// Read the `focus_keyword` field of `SeoMeta` for `content_id` from the
/// journal. Returns an empty string if not found or tombstoned.
fn load_focus_keyword(journal: &ForgeJournal, content_id: &str) -> String {
    let event = match journal.get_latest(AGG_SEO_META, content_id) {
        Some(e) => e,
        None => return String::new(),
    };
    if event.payload == SEO_TOMBSTONE {
        return String::new();
    }
    let meta: SeoMeta = match serde_json::from_slice(&event.payload) {
        Ok(m) => m,
        Err(_) => return String::new(),
    };
    meta.focus_keyword
}

// ── Bulk auto-map ─────────────────────────────────────────────────────────────

/// For each unmapped page in `pages`, run `suggest_sheets` and apply the best
/// suggestion if its confidence meets or exceeds `threshold`.
///
/// `pages` is a slice of `(content_id, slug, industry_slug)` tuples.
///
/// Returns `(mapped_count, skipped_count)`:
/// - `mapped_count`  — pages that were newly mapped
/// - `skipped_count` — pages that were already mapped or had no suggestion
///   above threshold
pub fn auto_map_all(
    journal: &mut ForgeJournal,
    pages: &[(String, String, String)],
    threshold: f64,
) -> (usize, usize) {
    let mut mapped = 0usize;
    let mut skipped = 0usize;

    // Snapshot which content_ids already have a live map so we don't re-map them.
    let already_mapped: std::collections::HashSet<String> = load_all_maps(journal)
        .into_iter()
        .map(|m| m.content_id)
        .collect();

    // Collect pages to process before we start mutating the journal.
    // Pages already mapped are counted as skipped immediately.
    let pages_to_process: Vec<(String, String, String)> = pages
        .iter()
        .filter(|(id, _, _)| {
            if already_mapped.contains(id) {
                false // will be counted below via skipped increment
            } else {
                true
            }
        })
        .cloned()
        .collect();

    // Count pre-mapped pages as skipped.
    skipped += pages.len() - pages_to_process.len();

    for (content_id, slug, industry_slug) in pages_to_process {
        let suggestions = suggest_sheets(journal, &content_id, &slug, &industry_slug);
        let best = suggestions.into_iter().next(); // already sorted desc

        match best {
            Some(s) if s.confidence >= threshold => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let map = PageSurferMap {
                    content_id: content_id.clone(),
                    sheet_ids: vec![s.sheet_id.clone()],
                    primary_sheet_id: s.sheet_id,
                    auto_suggested: true,
                    confirmed: false,
                    mapped_at: now,
                };
                if save_map(journal, &map).is_ok() {
                    mapped += 1;
                } else {
                    skipped += 1;
                }
            }
            _ => {
                skipped += 1;
            }
        }
    }

    (mapped, skipped)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seo::surfer::{save_sheet, StructureRange, StructureTargets, SurferSheet};
    use luperiq_forge::DurabilityMode;

    fn make_test_journal() -> (ForgeJournal, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let journal = ForgeJournal::open(wal, snap, DurabilityMode::Sync).unwrap();
        (journal, dir)
    }

    fn empty_structure() -> StructureTargets {
        StructureTargets {
            images: StructureRange {
                min: None,
                max: None,
            },
            headings: StructureRange {
                min: None,
                max: None,
            },
            words: StructureRange {
                min: None,
                max: None,
            },
            paragraphs: StructureRange {
                min: None,
                max: None,
            },
            characters: StructureRange {
                min: None,
                max: None,
            },
        }
    }

    fn pest_sheet() -> SurferSheet {
        SurferSheet {
            sheet_id: "pest-control-website".to_string(),
            topic: "pest control website".to_string(),
            source_file: "surfer-guidelines-pest control website-16-03-2026.txt".to_string(),
            source_date: "2026-03-16".to_string(),
            industry: "pest-control".to_string(),
            structure: empty_structure(),
            terms: vec![],
            facts: vec![],
        }
    }

    fn hvac_sheet() -> SurferSheet {
        SurferSheet {
            sheet_id: "hvac-seo".to_string(),
            topic: "hvac seo".to_string(),
            source_file: "surfer-guidelines-hvac seo-30-03-2026.txt".to_string(),
            source_date: "2026-03-30".to_string(),
            industry: "hvac".to_string(),
            structure: empty_structure(),
            terms: vec![],
            facts: vec![],
        }
    }

    // ── WAL save/load ─────────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_map() {
        let (mut journal, _dir) = make_test_journal();

        let map = PageSurferMap {
            content_id: "page-abc".to_string(),
            sheet_ids: vec!["pest-control-website".to_string()],
            primary_sheet_id: "pest-control-website".to_string(),
            auto_suggested: false,
            confirmed: true,
            mapped_at: 1000,
        };

        save_map(&mut journal, &map).unwrap();
        let loaded = load_map(&journal, "page-abc").unwrap();
        assert_eq!(loaded.content_id, "page-abc");
        assert_eq!(loaded.primary_sheet_id, "pest-control-website");
        assert!(loaded.confirmed);
    }

    #[test]
    fn test_load_missing_map_returns_none() {
        let (journal, _dir) = make_test_journal();
        assert!(load_map(&journal, "does-not-exist").is_none());
    }

    #[test]
    fn test_delete_map_tombstone() {
        let (mut journal, _dir) = make_test_journal();

        let map = PageSurferMap {
            content_id: "page-del".to_string(),
            sheet_ids: vec!["sheet-1".to_string()],
            primary_sheet_id: "sheet-1".to_string(),
            auto_suggested: false,
            confirmed: false,
            mapped_at: 0,
        };

        save_map(&mut journal, &map).unwrap();
        assert!(load_map(&journal, "page-del").is_some());

        delete_map(&mut journal, "page-del").unwrap();
        assert!(load_map(&journal, "page-del").is_none());
    }

    #[test]
    fn test_load_all_maps_excludes_tombstoned() {
        let (mut journal, _dir) = make_test_journal();

        for id in &["a", "b", "c"] {
            let map = PageSurferMap {
                content_id: id.to_string(),
                sheet_ids: vec![],
                primary_sheet_id: String::new(),
                auto_suggested: false,
                confirmed: false,
                mapped_at: 0,
            };
            save_map(&mut journal, &map).unwrap();
        }
        delete_map(&mut journal, "b").unwrap();

        let all = load_all_maps(&journal);
        assert_eq!(all.len(), 2);
        let ids: Vec<&str> = all.iter().map(|m| m.content_id.as_str()).collect();
        assert!(ids.contains(&"a"));
        assert!(ids.contains(&"c"));
        assert!(!ids.contains(&"b"));
    }

    #[test]
    fn test_overwrite_same_content_id() {
        let (mut journal, _dir) = make_test_journal();

        let mut map = PageSurferMap {
            content_id: "page-x".to_string(),
            sheet_ids: vec!["sheet-old".to_string()],
            primary_sheet_id: "sheet-old".to_string(),
            auto_suggested: true,
            confirmed: false,
            mapped_at: 1,
        };
        save_map(&mut journal, &map).unwrap();

        map.primary_sheet_id = "sheet-new".to_string();
        map.confirmed = true;
        save_map(&mut journal, &map).unwrap();

        let loaded = load_map(&journal, "page-x").unwrap();
        assert_eq!(loaded.primary_sheet_id, "sheet-new");
        assert!(loaded.confirmed);

        // Latest-event semantics: load_all should return one entry.
        assert_eq!(load_all_maps(&journal).len(), 1);
    }

    // ── suggest_sheets ────────────────────────────────────────────────────────

    #[test]
    fn test_suggest_sheets_industry_match() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();
        save_sheet(&mut journal, &hvac_sheet()).unwrap();

        // Industry matches pest-control; no SeoMeta focus keyword
        let suggestions = suggest_sheets(&journal, "page-1", "our-services", "pest-control");

        // Pest sheet should score 0.3 (industry only) — above 0.2 threshold
        assert!(!suggestions.is_empty());
        let top = &suggestions[0];
        assert_eq!(top.sheet_id, "pest-control-website");
        assert!((top.confidence - 0.3).abs() < 0.01);
        assert!(top.match_reason.contains("industry"));
    }

    #[test]
    fn test_suggest_sheets_slug_overlap() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        // Slug "pest-control" has two tokens that both appear in "pest control website"
        let suggestions = suggest_sheets(&journal, "page-2", "pest-control", "");

        // 2/2 tokens overlap → slug score = 0.2 * 1.0 = 0.2, which is NOT > 0.2
        // so the filter removes it.  Let's verify the logic with a slug that gives > 0.2.
        // "pest-control-website" → 3 tokens, all in topic → 0.2 * 1.0 = 0.2 (still == 0.2)
        // The test verifies the filter boundary: confidence must be STRICTLY > 0.2.
        // All three tokens match → still exactly 0.2, so filtered out.
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggest_sheets_combined_score() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        // Inject SeoMeta with focus_keyword matching the sheet topic
        let meta = SeoMeta {
            content_id: "page-3".to_string(),
            title: "Pest Control Services".to_string(),
            description: String::new(),
            og_image: String::new(),
            canonical_url: String::new(),
            robots: String::new(),
            schema_json: String::new(),
            focus_keyword: "pest control website".to_string(),
            seo_score: 0,
        };
        let payload = serde_json::to_vec(&meta).unwrap();
        let event = luperiq_forge::ApexEvent::new(AGG_SEO_META, "page-3", payload);
        // We need a mutable journal borrow — use a helper approach
        // Since journal is already mutably borrowed in tests, inject via append
        journal.append(event).unwrap();

        // industry + focus keyword → 0.5 + 0.3 = 0.8
        let suggestions = suggest_sheets(&journal, "page-3", "pest-control-page", "pest-control");
        assert!(!suggestions.is_empty());
        let top = &suggestions[0];
        assert_eq!(top.sheet_id, "pest-control-website");
        assert!(
            top.confidence > 0.79,
            "expected >= 0.8, got {}",
            top.confidence
        );
        assert!(top.match_reason.contains("focus keyword"));
        assert!(top.match_reason.contains("industry"));
    }

    #[test]
    fn test_suggest_sheets_no_sheets_returns_empty() {
        let (journal, _dir) = make_test_journal();
        let suggestions = suggest_sheets(&journal, "page-4", "some-slug", "pest-control");
        assert!(suggestions.is_empty());
    }

    #[test]
    fn test_suggest_sheets_sorted_desc() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();
        save_sheet(&mut journal, &hvac_sheet()).unwrap();

        // Both sheets in journal; "pest-control" industry only matches pest sheet
        let suggestions = suggest_sheets(&journal, "page-5", "services", "pest-control");
        // Only the pest sheet should appear (hvac scores 0.0)
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].sheet_id, "pest-control-website");
    }

    // ── auto_map_all ──────────────────────────────────────────────────────────

    #[test]
    fn test_auto_map_all_maps_matching_pages() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        let pages = vec![
            (
                "page-pest".to_string(),
                "pest-control-services".to_string(),
                "pest-control".to_string(),
            ),
            (
                "page-hvac".to_string(),
                "hvac-maintenance".to_string(),
                "hvac".to_string(),
            ),
        ];

        // threshold = 0.3 → pest page (industry match 0.3) should be mapped;
        // hvac page has no hvac sheet → skipped
        let (mapped, skipped) = auto_map_all(&mut journal, &pages, 0.3);
        assert_eq!(mapped, 1, "expected 1 mapped");
        assert_eq!(skipped, 1, "expected 1 skipped");

        let m = load_map(&journal, "page-pest").unwrap();
        assert_eq!(m.primary_sheet_id, "pest-control-website");
        assert!(m.auto_suggested);
        assert!(!m.confirmed);
    }

    #[test]
    fn test_auto_map_all_skips_already_mapped() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        // Pre-map page-pest manually.
        let existing = PageSurferMap {
            content_id: "page-pest".to_string(),
            sheet_ids: vec!["pest-control-website".to_string()],
            primary_sheet_id: "pest-control-website".to_string(),
            auto_suggested: false,
            confirmed: true,
            mapped_at: 999,
        };
        save_map(&mut journal, &existing).unwrap();

        let pages = vec![(
            "page-pest".to_string(),
            "pest-control-services".to_string(),
            "pest-control".to_string(),
        )];

        let (mapped, skipped) = auto_map_all(&mut journal, &pages, 0.3);
        assert_eq!(mapped, 0);
        assert_eq!(skipped, 1);

        // Original map should be untouched (confirmed still true).
        let m = load_map(&journal, "page-pest").unwrap();
        assert!(m.confirmed);
        assert_eq!(m.mapped_at, 999);
    }

    #[test]
    fn test_auto_map_all_threshold_filters() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        let pages = vec![(
            "page-p".to_string(),
            "pest-control".to_string(),
            "pest-control".to_string(),
        )];

        // Industry score 0.3 exactly meets a 0.3 threshold → should map.
        let (mapped, _) = auto_map_all(&mut journal, &pages, 0.3);
        assert_eq!(mapped, 1);
    }

    #[test]
    fn test_auto_map_all_high_threshold_skips() {
        let (mut journal, _dir) = make_test_journal();
        save_sheet(&mut journal, &pest_sheet()).unwrap();

        let pages = vec![(
            "page-q".to_string(),
            "pest-control".to_string(),
            "pest-control".to_string(),
        )];

        // Industry score is only 0.3, but threshold is 0.8 → should skip.
        let (mapped, skipped) = auto_map_all(&mut journal, &pages, 0.8);
        assert_eq!(mapped, 0);
        assert_eq!(skipped, 1);
    }

    // ── Serialization round-trip ──────────────────────────────────────────────

    #[test]
    fn test_page_surfer_map_serialization() {
        let map = PageSurferMap {
            content_id: "page-ser".to_string(),
            sheet_ids: vec!["sheet-a".to_string(), "sheet-b".to_string()],
            primary_sheet_id: "sheet-a".to_string(),
            auto_suggested: true,
            confirmed: false,
            mapped_at: 12345678,
        };
        let json = serde_json::to_string(&map).unwrap();
        let back: PageSurferMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back.content_id, map.content_id);
        assert_eq!(back.sheet_ids, map.sheet_ids);
        assert_eq!(back.primary_sheet_id, map.primary_sheet_id);
        assert_eq!(back.auto_suggested, map.auto_suggested);
        assert_eq!(back.confirmed, map.confirmed);
        assert_eq!(back.mapped_at, map.mapped_at);
    }
}
