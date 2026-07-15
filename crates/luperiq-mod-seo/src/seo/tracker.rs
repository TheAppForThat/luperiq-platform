//! SEO change tracking — records every SEO-relevant change with before/after
//! values and optional GSC performance snapshots.

use serde::{Deserialize, Serialize};

// ── Aggregate type constants ─────────────────────────────────────────

pub const AGG_SEO_CHANGE: &str = "SeoChange";
pub const AGG_SEO_SNAPSHOT: &str = "SeoSnapshot";
pub const AGG_SEO_AI_INSIGHT: &str = "SeoAiInsight";

// ── SeoChange ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeoChangeType {
    SlugChange,
    KeywordChange,
    TitleChange,
    DescriptionChange,
    SchemaChange,
    AbVariantStart,
    AbVariantEnd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoChange {
    pub content_id: String,
    pub change_type: SeoChangeType,
    pub old_value: String,
    pub new_value: String,
    #[serde(default)]
    pub snapshot_before: Option<SeoSnapshot>,
    #[serde(default)]
    pub ai_warning: Option<String>,
    pub timestamp: u64,
}

// ── SeoSnapshot ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoSnapshot {
    pub content_id: String,
    pub date: String,
    pub url: String,
    pub impressions: u64,
    pub clicks: u64,
    pub avg_position: f64,
    pub ctr: f64,
    #[serde(default)]
    pub focus_keyword: String,
    #[serde(default)]
    pub keyword_position: Option<f64>,
}

// ── SeoAiInsight ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoAiInsight {
    #[serde(default)]
    pub content_id: Option<String>,
    pub analysis: String,
    #[serde(default)]
    pub changes_analyzed: Vec<String>,
    pub timestamp: u64,
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Record an SeoChange event in the journal. Each change gets a unique ULID
/// as the aggregate_id (not content_id) so the full history is preserved.
pub(crate) fn record_seo_change(
    journal: &mut luperiq_forge::ForgeJournal,
    change: &SeoChange,
) -> Result<String, String> {
    let id = ulid::Ulid::new().to_string();
    let payload = serde_json::to_vec(change).map_err(|e| format!("Serialize SeoChange: {e}"))?;
    let event = luperiq_forge::ApexEvent::new(AGG_SEO_CHANGE, &id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append: {e}"))?;
    Ok(id)
}

/// Record an SeoSnapshot in the journal.
/// Aggregate ID format: "{content_id}:{date}" — one snapshot per page per day.
pub fn record_seo_snapshot(
    journal: &mut luperiq_forge::ForgeJournal,
    snapshot: &SeoSnapshot,
) -> Result<(), String> {
    let agg_id = format!("{}:{}", snapshot.content_id, snapshot.date);
    let payload =
        serde_json::to_vec(snapshot).map_err(|e| format!("Serialize SeoSnapshot: {e}"))?;
    let event = luperiq_forge::ApexEvent::new(AGG_SEO_SNAPSHOT, &agg_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append: {e}"))?;
    Ok(())
}

/// Record an SeoAiInsight in the journal.
pub(crate) fn record_seo_ai_insight(
    journal: &mut luperiq_forge::ForgeJournal,
    insight: &SeoAiInsight,
) -> Result<(), String> {
    let id = ulid::Ulid::new().to_string();
    let payload =
        serde_json::to_vec(insight).map_err(|e| format!("Serialize SeoAiInsight: {e}"))?;
    let event = luperiq_forge::ApexEvent::new(AGG_SEO_AI_INSIGHT, &id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append: {e}"))?;
    Ok(())
}

/// Get all SeoChange events, optionally filtered by content_id.
/// Returns newest-first (sorted by timestamp descending).
pub(crate) fn list_seo_changes(
    journal: &luperiq_forge::ForgeJournal,
    content_id_filter: Option<&str>,
) -> Vec<(String, SeoChange)> {
    let events = journal.latest_by_aggregate_type(AGG_SEO_CHANGE);
    let mut changes: Vec<(String, SeoChange)> = events
        .into_iter()
        .filter_map(|e| {
            let change: SeoChange = serde_json::from_slice(&e.payload).ok()?;
            if let Some(filter) = content_id_filter {
                if change.content_id != filter {
                    return None;
                }
            }
            Some((e.aggregate_id.clone(), change))
        })
        .collect();
    changes.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
    changes
}

/// Get the latest SeoSnapshot for a content_id.
pub fn latest_snapshot_for_content(
    journal: &luperiq_forge::ForgeJournal,
    content_id: &str,
) -> Option<SeoSnapshot> {
    let events = journal.latest_by_aggregate_type(AGG_SEO_SNAPSHOT);
    let mut snapshots: Vec<SeoSnapshot> = events
        .into_iter()
        .filter_map(|e| {
            if !e.aggregate_id.starts_with(&format!("{content_id}:")) {
                return None;
            }
            serde_json::from_slice(&e.payload).ok()
        })
        .collect();
    snapshots.sort_by(|a, b| b.date.cmp(&a.date));
    snapshots.into_iter().next()
}

/// Get all SeoSnapshots for a content_id within a date range.
/// Used by A/B experiment analysis to compare performance across periods.
pub fn snapshots_in_range(
    journal: &luperiq_forge::ForgeJournal,
    content_id: &str,
    start_date: &str,
    end_date: &str,
) -> Vec<SeoSnapshot> {
    let events = journal.latest_by_aggregate_type(AGG_SEO_SNAPSHOT);
    let mut snapshots: Vec<SeoSnapshot> = events
        .into_iter()
        .filter_map(|e| {
            if !e.aggregate_id.starts_with(&format!("{content_id}:")) {
                return None;
            }
            let snap: SeoSnapshot = serde_json::from_slice(&e.payload).ok()?;
            if snap.date >= start_date.to_string() && snap.date <= end_date.to_string() {
                Some(snap)
            } else {
                None
            }
        })
        .collect();
    snapshots.sort_by(|a, b| a.date.cmp(&b.date));
    snapshots
}

/// Get content_ids that have had SeoChange events in the last N days.
pub fn recently_changed_content_ids(
    journal: &luperiq_forge::ForgeJournal,
    days: u64,
) -> Vec<String> {
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(days * 86400);

    let events = journal.latest_by_aggregate_type(AGG_SEO_CHANGE);
    let mut ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for e in events {
        if let Ok(change) = serde_json::from_slice::<SeoChange>(&e.payload) {
            if change.timestamp >= cutoff {
                ids.insert(change.content_id);
            }
        }
    }
    ids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seo_change_serialization_roundtrip() {
        let change = SeoChange {
            content_id: "page-123".into(),
            change_type: SeoChangeType::TitleChange,
            old_value: "Old Title".into(),
            new_value: "New Title".into(),
            snapshot_before: None,
            ai_warning: None,
            timestamp: 1710000000,
        };
        let bytes = serde_json::to_vec(&change).unwrap();
        let decoded: SeoChange = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.content_id, "page-123");
        assert_eq!(decoded.change_type, SeoChangeType::TitleChange);
        assert_eq!(decoded.old_value, "Old Title");
        assert_eq!(decoded.new_value, "New Title");
    }

    #[test]
    fn test_seo_snapshot_serialization_roundtrip() {
        let snap = SeoSnapshot {
            content_id: "page-123".into(),
            date: "2026-03-13".into(),
            url: "/pest-control".into(),
            impressions: 340,
            clicks: 15,
            avg_position: 12.3,
            ctr: 4.4,
            focus_keyword: "pest control".into(),
            keyword_position: Some(8.5),
        };
        let bytes = serde_json::to_vec(&snap).unwrap();
        let decoded: SeoSnapshot = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.impressions, 340);
        assert_eq!(decoded.clicks, 15);
        assert!((decoded.avg_position - 12.3).abs() < 0.01);
    }

    #[test]
    fn test_seo_change_type_variants() {
        let types = vec![
            SeoChangeType::SlugChange,
            SeoChangeType::KeywordChange,
            SeoChangeType::TitleChange,
            SeoChangeType::DescriptionChange,
            SeoChangeType::SchemaChange,
            SeoChangeType::AbVariantStart,
            SeoChangeType::AbVariantEnd,
        ];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let decoded: SeoChangeType = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, t);
        }
    }

    #[test]
    fn test_seo_ai_insight_optional_content_id() {
        let insight = SeoAiInsight {
            content_id: None,
            analysis: "Site-wide analysis".into(),
            changes_analyzed: vec!["change-1".into(), "change-2".into()],
            timestamp: 1710000000,
        };
        let bytes = serde_json::to_vec(&insight).unwrap();
        let decoded: SeoAiInsight = serde_json::from_slice(&bytes).unwrap();
        assert!(decoded.content_id.is_none());
        assert_eq!(decoded.changes_analyzed.len(), 2);
    }

    #[test]
    fn test_snapshot_aggregate_id_format() {
        let snap = SeoSnapshot {
            content_id: "page-abc".into(),
            date: "2026-03-13".into(),
            url: "/test".into(),
            impressions: 0,
            clicks: 0,
            avg_position: 0.0,
            ctr: 0.0,
            focus_keyword: String::new(),
            keyword_position: None,
        };
        let agg_id = format!("{}:{}", snap.content_id, snap.date);
        assert_eq!(agg_id, "page-abc:2026-03-13");
    }
}
