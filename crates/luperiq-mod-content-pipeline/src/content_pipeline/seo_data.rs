//! SeoGuideline + FactPack aggregates — invisible server-side reference data.
//!
//! SeoGuideline stores Surfer-style content optimization rules per topic:
//! word count targets, term frequency requirements, and fact groups.
//!
//! FactPack stores deep reference data per subject (pest species, location data,
//! equipment specs, etc.) with university-grade citations.
//!
//! These are maintained on Central (luperiq.com) and distributed to client sites.
//! Customers never see this data — it feeds the AI content generation pipeline.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

// ── Aggregate type constants ─────────────────────────────────────────

/// Aggregate type for SEO guidelines in the ForgeJournal.
pub const AGG_SEO_GUIDE: &str = "CntPipe:SeoGuide";

/// Aggregate type for fact packs in the ForgeJournal.
pub const AGG_FACT_PACK: &str = "CntPipe:FactPack";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── SeoGuideline aggregate ──────────────────────────────────────────

/// Surfer-style content optimization rules for a specific scope.
///
/// Scopes can be topic-level ("topic:german-cockroaches"), location-level
/// ("location:fort-worth-tx"), or page-type-level ("page_type:homepage").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoGuideline {
    pub id: String,
    /// Scope identifier: "topic:german-cockroaches", "location:fort-worth-tx", "page_type:homepage".
    #[serde(default)]
    pub scope: String,
    /// Scope type: "topic", "location", "page_type".
    #[serde(default)]
    pub scope_type: String,
    /// Related industry slugs. Empty = universal (applies to all industries).
    #[serde(default)]
    pub industry_slugs: Vec<String>,
    /// Content structure targets (word count, headings, paragraphs, images).
    #[serde(default)]
    pub content_structure: ContentStructure,
    /// Term frequency requirements: which terms to use and how often.
    #[serde(default)]
    pub term_frequencies: Vec<TermFrequency>,
    /// Grouped facts to include in the content (from Surfer's "Facts to Include").
    #[serde(default)]
    pub fact_groups: Vec<FactGroup>,
    /// Whether this guideline is active and available for use.
    #[serde(default = "default_true")]
    pub active: bool,
    /// Unix timestamp when created.
    #[serde(default)]
    pub created_at: u64,
    /// Unix timestamp when last updated.
    #[serde(default)]
    pub updated_at: u64,
}

/// Content structure targets for a page.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentStructure {
    #[serde(default)]
    pub word_count_min: u32,
    #[serde(default)]
    pub word_count_max: u32,
    #[serde(default)]
    pub heading_count_min: u32,
    #[serde(default)]
    pub heading_count_max: u32,
    #[serde(default)]
    pub paragraph_count_min: u32,
    #[serde(default)]
    pub image_count_min: u32,
    #[serde(default)]
    pub image_count_max: u32,
}

/// A single term frequency requirement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermFrequency {
    /// The term or phrase.
    #[serde(default)]
    pub term: String,
    /// Minimum number of times this term should appear.
    #[serde(default)]
    pub min_count: u32,
    /// Maximum number of times this term should appear.
    #[serde(default)]
    pub max_count: u32,
}

/// A group of related facts to include in content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactGroup {
    /// Topic heading for this fact group.
    #[serde(default)]
    pub topic: String,
    /// Individual facts within the group.
    #[serde(default)]
    pub facts: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Default for SeoGuideline {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: String::new(),
            scope: String::new(),
            scope_type: String::new(),
            industry_slugs: Vec::new(),
            content_structure: ContentStructure::default(),
            term_frequencies: Vec::new(),
            fact_groups: Vec::new(),
            active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── SeoGuideline journal helpers ────────────────────────────────────

/// Load all non-deleted SEO guidelines from the journal.
pub fn load_all_guidelines(j: &luperiq_forge::ForgeJournal) -> Vec<SeoGuideline> {
    j.latest_by_aggregate_type(AGG_SEO_GUIDE)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<SeoGuideline>(&e.payload).ok())
        .collect()
}

/// Load a single SEO guideline by ID.
pub fn load_guideline(j: &luperiq_forge::ForgeJournal, id: &str) -> Option<SeoGuideline> {
    j.get_latest(AGG_SEO_GUIDE, id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<SeoGuideline>(&e.payload).ok())
}

/// Find the best matching guideline for a given scope and industry.
///
/// Priority:
/// 1. `scope == "topic:{target_slug}"` with matching industry
/// 2. `scope == "topic:{target_slug}"` universal (no industry restriction)
/// 3. `scope == "page_type:{page_type}"` with matching industry
/// 4. `scope == "page_type:{page_type}"` universal
pub fn find_guideline(
    j: &luperiq_forge::ForgeJournal,
    page_type: &str,
    target_slug: &str,
    industry_slug: &str,
) -> Option<SeoGuideline> {
    let all = load_all_guidelines(j);
    let active: Vec<&SeoGuideline> = all.iter().filter(|g| g.active).collect();

    let topic_scope = format!("topic:{}", target_slug);
    let location_scope = format!("location:{}", target_slug);
    let page_scope = format!("page_type:{}", page_type);

    // 1. Topic-specific with matching industry
    if !industry_slug.is_empty() {
        if let Some(g) = active
            .iter()
            .find(|g| g.scope == topic_scope && g.industry_slugs.iter().any(|s| s == industry_slug))
        {
            return Some((*g).clone());
        }
        if let Some(g) = active.iter().find(|g| {
            g.scope == location_scope && g.industry_slugs.iter().any(|s| s == industry_slug)
        }) {
            return Some((*g).clone());
        }
    }

    // 2. Topic-specific universal
    if let Some(g) = active
        .iter()
        .find(|g| g.scope == topic_scope && g.industry_slugs.is_empty())
    {
        return Some((*g).clone());
    }
    if let Some(g) = active
        .iter()
        .find(|g| g.scope == location_scope && g.industry_slugs.is_empty())
    {
        return Some((*g).clone());
    }

    // 3. Page-type with matching industry
    if !industry_slug.is_empty() {
        if let Some(g) = active
            .iter()
            .find(|g| g.scope == page_scope && g.industry_slugs.iter().any(|s| s == industry_slug))
        {
            return Some((*g).clone());
        }
    }

    // 4. Page-type universal
    active
        .iter()
        .find(|g| g.scope == page_scope && g.industry_slugs.is_empty())
        .map(|g| (*g).clone())
}

/// Persist an SEO guideline to the journal.
pub fn persist_guideline(
    j: &mut luperiq_forge::ForgeJournal,
    guideline: &SeoGuideline,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(guideline).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_SEO_GUIDE, &guideline.id, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete an SEO guideline by ID.
pub fn delete_guideline(j: &mut luperiq_forge::ForgeJournal, id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_SEO_GUIDE, id, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

// ══════════════════════════════════════════════════════════════════════
// FactPack aggregate
// ══════════════════════════════════════════════════════════════════════

/// Deep reference data per subject, stored as structured JSON.
///
/// Fact packs provide authoritative, citation-backed data that enriches
/// AI-generated content with specific facts, statistics, and technical details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactPack {
    pub id: String,
    /// Subject slug: "german-cockroach", "fire-ant", "fort-worth-tx", "central-ac".
    #[serde(default)]
    pub subject_slug: String,
    /// Subject type: "pest", "location", "equipment", "service", "material".
    #[serde(default)]
    pub subject_type: String,
    /// Human-readable title.
    #[serde(default)]
    pub title: String,
    /// Related industry slugs.
    #[serde(default)]
    pub industry_slugs: Vec<String>,
    /// The structured fact data as a JSON value (flexible schema per subject type).
    #[serde(default)]
    pub data: serde_json::Value,
    /// Source citations for authority.
    #[serde(default)]
    pub sources: Vec<FactSource>,
    /// Whether this fact pack is active and available for use.
    #[serde(default = "default_true")]
    pub active: bool,
    /// Unix timestamp when created.
    #[serde(default)]
    pub created_at: u64,
    /// Unix timestamp when last updated.
    #[serde(default)]
    pub updated_at: u64,
}

/// A source citation for a fact pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactSource {
    /// Short code, e.g. "RUTGERS_FS1322", "UF_IFAS_2024".
    #[serde(default)]
    pub code: String,
    /// Full citation text.
    #[serde(default)]
    pub citation: String,
    /// URL (optional).
    #[serde(default)]
    pub url: Option<String>,
}

impl Default for FactPack {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: String::new(),
            subject_slug: String::new(),
            subject_type: String::new(),
            title: String::new(),
            industry_slugs: Vec::new(),
            data: serde_json::Value::Null,
            sources: Vec::new(),
            active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

// ── FactPack journal helpers ────────────────────────────────────────

/// Load all non-deleted fact packs from the journal.
pub fn load_all_fact_packs(j: &luperiq_forge::ForgeJournal) -> Vec<FactPack> {
    j.latest_by_aggregate_type(AGG_FACT_PACK)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<FactPack>(&e.payload).ok())
        .collect()
}

/// Load a single fact pack by ID.
pub fn load_fact_pack(j: &luperiq_forge::ForgeJournal, id: &str) -> Option<FactPack> {
    j.get_latest(AGG_FACT_PACK, id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<FactPack>(&e.payload).ok())
}

/// Find the best matching fact pack for a given subject.
///
/// Tries exact subject_slug match first, then looks for related fact packs
/// matching the industry.
pub fn find_fact_pack(
    j: &luperiq_forge::ForgeJournal,
    subject_slug: &str,
    industry_slug: &str,
) -> Option<FactPack> {
    let all = load_all_fact_packs(j);
    let active: Vec<&FactPack> = all.iter().filter(|f| f.active).collect();

    // Exact subject match with matching industry
    if !industry_slug.is_empty() {
        if let Some(f) = active.iter().find(|f| {
            f.subject_slug == subject_slug && f.industry_slugs.iter().any(|s| s == industry_slug)
        }) {
            return Some((*f).clone());
        }
    }

    // Exact subject match (any industry)
    if let Some(f) = active.iter().find(|f| f.subject_slug == subject_slug) {
        return Some((*f).clone());
    }

    None
}

/// Persist a fact pack to the journal.
pub fn persist_fact_pack(
    j: &mut luperiq_forge::ForgeJournal,
    fact_pack: &FactPack,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(fact_pack).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_FACT_PACK, &fact_pack.id, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete a fact pack by ID.
pub fn delete_fact_pack(j: &mut luperiq_forge::ForgeJournal, id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_FACT_PACK, id, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
