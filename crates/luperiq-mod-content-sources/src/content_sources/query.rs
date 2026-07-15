
//! Query functions for assembling content sources for the page generator.
//!
//! The page generator calls `get_sources_for_topic()` to fetch all active
//! ContentSources for a given industry + topic. It calls
//! `render_facts_for_prompt()` to convert a Vec<FactEntry> into the
//! "Key: Value" line format expected by ai_user_prompt().

use super::types::*;

/// All sources found for a topic, separated by type for the prompt builder.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PlannedPageSources {
    /// Rendered from IndustryItem.fact_sheet (existing LuperIQ data).
    pub luperiq_facts: String,
    /// Rendered from customer ContentSource structured_facts.
    pub customer_facts: String,
    /// Excerpt of customer raw_content (first 2000 chars).
    pub raw_reference: String,
}

impl PlannedPageSources {
    /// Returns true if all source sections are empty.
    pub fn is_empty(&self) -> bool {
        self.luperiq_facts.is_empty()
            && self.customer_facts.is_empty()
            && self.raw_reference.is_empty()
    }
}

/// Query all active ContentSources for a given industry + topic from the journal.
pub fn get_sources_for_topic(
    journal: &luperiq_forge::ForgeJournal,
    industry_slug: &str,
    topic_slug: &str,
) -> Vec<ContentSource> {
    let events = journal.latest_by_aggregate_type(AGG_CONTENT_SOURCE);
    events
        .into_iter()
        .filter(|e| e.payload != CONTENT_SOURCE_TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentSource>(&e.payload).ok())
        .filter(|s| s.industry_slug == industry_slug && s.topic_slug == topic_slug)
        .collect()
}

/// Assemble all sources for a topic into the prompt-ready PlannedPageSources struct.
///
/// `luperiq_fact_sheet` is the existing IndustryItem.fact_sheet string (from the
/// provider's build_*_fact_sheet() function). Customer sources come from the journal.
pub fn assemble_sources(
    luperiq_fact_sheet: &str,
    customer_sources: &[ContentSource],
) -> PlannedPageSources {
    // Combine all customer structured facts
    let mut customer_lines: Vec<String> = Vec::new();
    let mut raw_parts: Vec<String> = Vec::new();

    for source in customer_sources {
        // Skip LuperIQ fact sheets in customer sources list (they're handled separately)
        if source.source_type == ContentSourceType::LuperiqFactSheet {
            continue;
        }

        for fact in &source.structured_facts {
            customer_lines.push(format!("{}: {}", fact.key, fact.value));
        }

        if !source.raw_content.is_empty() {
            raw_parts.push(source.raw_content.clone());
        }
    }

    // Truncate raw reference to ~2000 chars (safe UTF-8 boundary)
    let raw_combined = raw_parts.join("\n\n---\n\n");
    let raw_reference = if raw_combined.len() > 2000 {
        // Use char_indices to find a safe UTF-8 boundary near 2000 bytes
        let safe_end = raw_combined
            .char_indices()
            .take_while(|&(i, _)| i < 2000)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        let mut truncated = raw_combined[..safe_end].to_string();
        truncated.push_str("...");
        truncated
    } else {
        raw_combined
    };

    PlannedPageSources {
        luperiq_facts: luperiq_fact_sheet.to_string(),
        customer_facts: customer_lines.join("\n"),
        raw_reference,
    }
}

/// Render a Vec<FactEntry> into the "Key: Value" line format used by
/// existing build_pest_fact_sheet() and ai_user_prompt().
pub fn render_facts_for_prompt(facts: &[FactEntry]) -> String {
    facts
        .iter()
        .map(|f| format!("{}: {}", f.key, f.value))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::*;

    fn make_source(
        source_type: ContentSourceType,
        facts: Vec<FactEntry>,
        raw: &str,
    ) -> ContentSource {
        ContentSource {
            source_id: "test-1".into(),
            source_type,
            industry_slug: "pest-control".into(),
            topic_slug: "termites".into(),
            title: "Test Source".into(),
            structured_facts: facts,
            raw_content: raw.to_string(),
            sharing_tier: SharingTier::NeverShare,
            sharing_discount_applied: false,
            validation_status: ValidationStatus::NotApplicable,
            owner_license_key: "test".into(),
            created_at: 0,
            updated_at: 0,
            file_format: "text".into(),
            contributor_id: None,
            contributor_payout_status: PayoutStatus::NotApplicable,
            quality_score: None,
            content_type_tag: "fact_sheet".into(),
            parent_source_id: None,
            transferable: false,
            credit_value: None,
        }
    }

    #[test]
    fn assemble_sources_empty() {
        let result = assemble_sources("", &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn assemble_sources_luperiq_only() {
        let result = assemble_sources("Severity: High\nCategory: Insects", &[]);
        assert_eq!(result.luperiq_facts, "Severity: High\nCategory: Insects");
        assert!(result.customer_facts.is_empty());
        assert!(result.raw_reference.is_empty());
    }

    #[test]
    fn assemble_sources_customer_facts() {
        let facts = vec![FactEntry {
            key: "Treatment".into(),
            value: "Bait stations".into(),
            confidence: FactConfidence::CustomerStated,
        }];
        let source = make_source(ContentSourceType::CustomerUpload, facts, "");
        let result = assemble_sources("", &[source]);
        assert_eq!(result.customer_facts, "Treatment: Bait stations");
    }

    #[test]
    fn assemble_sources_skips_luperiq_type_in_customer_list() {
        let source = make_source(
            ContentSourceType::LuperiqFactSheet,
            vec![],
            "should be skipped",
        );
        let result = assemble_sources("", &[source]);
        assert!(result.raw_reference.is_empty());
    }

    #[test]
    fn render_facts_for_prompt_works() {
        let facts = vec![
            FactEntry {
                key: "Severity".into(),
                value: "High".into(),
                confidence: FactConfidence::Verified,
            },
            FactEntry {
                key: "Category".into(),
                value: "Insects".into(),
                confidence: FactConfidence::Verified,
            },
        ];
        let rendered = render_facts_for_prompt(&facts);
        assert_eq!(rendered, "Severity: High\nCategory: Insects");
    }
}
