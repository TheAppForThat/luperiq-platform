
//! Conflict detection types for the Content Sourcing system.
//!
//! When a customer uploads content that contradicts LuperIQ fact sheets,
//! a ConflictRecord is created so the customer can review and decide.

use serde::{Deserialize, Serialize};

pub const AGG_CONTENT_CONFLICT: &str = "ContentConflict";

/// A detected conflict between customer content and LuperIQ fact sheets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRecord {
    pub conflict_id: String,
    pub source_id: String,
    pub luperiq_source_id: String,
    pub conflicting_fields: Vec<ConflictField>,
    #[serde(default)]
    pub customer_notes: String,
    #[serde(default)]
    pub resolution: ConflictResolution,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
}

/// A single field-level conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictField {
    pub field_name: String,
    pub luperiq_value_summary: String,
    pub customer_value: String,
}

/// How a conflict was resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    Pending,
    CustomerProceeded,
    CustomerDeferred,
    LuperiqUpdated,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::Pending
    }
}

pub const CONFLICT_TOMBSTONE: &[u8] = b"__CONFLICT_DELETED__";

use super::types::ContentSource;

/// Compare customer source facts against a LuperIQ fact sheet for the same topic.
/// Returns a ConflictRecord if any contradictions are found, None otherwise.
///
/// V1 uses direct key matching: if both sources have a fact with the same key
/// but different values, it's flagged. Future versions will use AI comparison
/// for semantic conflict detection.
pub fn detect_conflicts(
    customer_source: &ContentSource,
    luperiq_source: &ContentSource,
) -> Option<ConflictRecord> {
    let mut conflicts = Vec::new();

    for customer_fact in &customer_source.structured_facts {
        for luperiq_fact in &luperiq_source.structured_facts {
            // Same key, different value = potential conflict
            if customer_fact.key.to_lowercase() == luperiq_fact.key.to_lowercase()
                && customer_fact.value.to_lowercase() != luperiq_fact.value.to_lowercase()
            {
                conflicts.push(ConflictField {
                    field_name: customer_fact.key.clone(),
                    luperiq_value_summary: truncate(&luperiq_fact.value, 200),
                    customer_value: customer_fact.value.clone(),
                });
            }
        }
    }

    if conflicts.is_empty() {
        return None;
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let now = ts.as_secs();

    Some(ConflictRecord {
        conflict_id: format!("conflict-{}-{}", now, ts.subsec_nanos()),
        source_id: customer_source.source_id.clone(),
        luperiq_source_id: luperiq_source.source_id.clone(),
        conflicting_fields: conflicts,
        customer_notes: String::new(),
        resolution: ConflictResolution::Pending,
        created_at: now,
        resolved_at: None,
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let boundary = s
            .char_indices()
            .take_while(|(i, _)| *i < max)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...", &s[..boundary])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{ContentSourceType, FactEntry};

    fn make_source(source_type: ContentSourceType, facts: Vec<(&str, &str)>) -> ContentSource {
        ContentSource {
            source_id: "test".into(),
            source_type,
            industry_slug: "pest-control".into(),
            topic_slug: "termites".into(),
            title: "Test".into(),
            structured_facts: facts
                .into_iter()
                .map(|(k, v)| FactEntry {
                    key: k.to_string(),
                    value: v.to_string(),
                    confidence: super::super::types::FactConfidence::Verified,
                })
                .collect(),
            raw_content: String::new(),
            sharing_tier: super::super::types::SharingTier::NeverShare,
            sharing_discount_applied: false,
            validation_status: super::super::types::ValidationStatus::NotApplicable,
            owner_license_key: String::new(),
            created_at: 0,
            updated_at: 0,
            file_format: String::new(),
            contributor_id: None,
            contributor_payout_status: super::super::types::PayoutStatus::NotApplicable,
            quality_score: None,
            content_type_tag: "fact_sheet".into(),
            parent_source_id: None,
            transferable: false,
            credit_value: None,
        }
    }

    #[test]
    fn no_conflicts_when_facts_match() {
        let customer = make_source(
            ContentSourceType::CustomerUpload,
            vec![("Peak Season", "June through September")],
        );
        let luperiq = make_source(
            ContentSourceType::LuperiqFactSheet,
            vec![("Peak Season", "June through September")],
        );
        assert!(detect_conflicts(&customer, &luperiq).is_none());
    }

    #[test]
    fn detects_value_mismatch() {
        let customer = make_source(
            ContentSourceType::CustomerUpload,
            vec![("Peak Season", "March through May")],
        );
        let luperiq = make_source(
            ContentSourceType::LuperiqFactSheet,
            vec![("Peak Season", "June through September")],
        );
        let result = detect_conflicts(&customer, &luperiq);
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.conflicting_fields.len(), 1);
        assert_eq!(conflict.conflicting_fields[0].field_name, "Peak Season");
    }

    #[test]
    fn ignores_extra_customer_facts() {
        let customer = make_source(
            ContentSourceType::CustomerUpload,
            vec![
                ("Peak Season", "June through September"),
                ("Our Specialty", "We focus on eco-friendly treatments"),
            ],
        );
        let luperiq = make_source(
            ContentSourceType::LuperiqFactSheet,
            vec![("Peak Season", "June through September")],
        );
        assert!(detect_conflicts(&customer, &luperiq).is_none());
    }

    #[test]
    fn case_insensitive_key_matching() {
        let customer = make_source(
            ContentSourceType::CustomerUpload,
            vec![("peak season", "March through May")],
        );
        let luperiq = make_source(
            ContentSourceType::LuperiqFactSheet,
            vec![("Peak Season", "June through September")],
        );
        let result = detect_conflicts(&customer, &luperiq);
        assert!(result.is_some());
    }
}
