//! IndustryProfile aggregate — centrally maintained data sheets per industry.
//!
//! Stores terminology, keywords, compliance, services, equipment, materials,
//! SEO keywords, content guidelines, seasonal patterns, pricing norms, and more.
//!
//! Each profile is keyed by a URL-safe slug (e.g. "hvac", "pest-control") and
//! persisted via the ForgeJournal event-sourcing pattern.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for industry profiles in the ForgeJournal.
pub const AGG_INDUSTRY: &str = "IndProf:Profile";

/// Valid category values for an [`IndustryProfile`].
///
/// Used by the create and update handlers to validate the `category` field.
/// Centralised here so there is a single authoritative list.
pub const VALID_CATEGORIES: [&str; 5] =
    ["field_service", "professional", "retail", "ecommerce", "other"];

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Primary aggregate ────────────────────────────────────────────────

/// A comprehensive industry data sheet that drives content generation,
/// SEO optimization, service catalog defaults, and compliance checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryProfile {
    pub id: String,
    /// URL-safe slug, e.g. "hvac", "pest-control", "law-office".
    pub slug: String,
    /// Display name, e.g. "HVAC", "Pest Control".
    pub name: String,
    /// Short description of the industry.
    pub description: String,
    /// Category: "field_service", "professional", "retail", "ecommerce", "other".
    pub category: String,
    /// Industry-specific vocabulary.
    pub terminology: Vec<IndustryTerm>,
    /// Regulatory / compliance requirements.
    pub compliance_requirements: Vec<ComplianceReq>,
    /// Standard services offered by businesses in this industry.
    pub common_services: Vec<CommonService>,
    /// Equipment categories and their items.
    pub equipment_categories: Vec<EquipmentCategory>,
    /// Material categories and their items.
    pub material_categories: Vec<MaterialCategory>,
    /// SEO keywords with optional search metrics.
    pub seo_keywords: Vec<SeoKeyword>,
    /// Content guidelines per page type.
    pub content_guidelines: Vec<ContentGuideline>,
    /// Seasonal demand patterns.
    pub seasonal_patterns: Vec<SeasonalPattern>,
    /// Typical pricing norms for the industry.
    pub pricing_norms: PricingNorms,
    /// Common customer pain points (used for content & marketing copy).
    pub customer_pain_points: Vec<String>,
    /// Trust signals that matter to customers.
    pub trust_factors: Vec<String>,
    /// Terms competitors might use (for competitive SEO).
    pub competitor_terms: Vec<String>,
    /// Schema.org types relevant to this industry.
    pub schema_org_types: Vec<String>,
    /// Whether the profile is active and usable.
    pub active: bool,
}

// ── Sub-types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryTerm {
    /// The term, e.g. "SEER Rating".
    pub term: String,
    /// Definition / explanation.
    pub definition: String,
    /// When to use this term, e.g. "customer-facing", "technician notes".
    pub usage_context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReq {
    /// Requirement name, e.g. "EPA Section 608 Certification".
    pub name: String,
    /// What it covers.
    pub description: String,
    /// Whether this is legally mandatory vs. optional best practice.
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonService {
    /// Service name, e.g. "AC Installation".
    pub name: String,
    /// URL-safe slug, e.g. "ac-installation".
    pub slug: String,
    /// Brief description.
    pub description: String,
    /// Typical price range string, e.g. "$3,000 - $7,000".
    pub price_range: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentCategory {
    /// Category name, e.g. "Cooling Systems".
    pub name: String,
    /// Specific items, e.g. ["Central AC", "Mini-Split", "Heat Pump"].
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialCategory {
    /// Category name, e.g. "Refrigerants".
    pub name: String,
    /// Specific items, e.g. ["R-410A", "R-32", "R-454B"].
    pub items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoKeyword {
    /// The keyword phrase.
    pub keyword: String,
    /// Estimated monthly search volume (if known).
    pub search_volume: Option<u32>,
    /// Keyword difficulty score 0-100 (if known).
    pub difficulty: Option<u32>,
    /// Search intent: "informational", "commercial", "transactional", "navigational".
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGuideline {
    /// Page type, e.g. "service-page", "homepage", "about", "blog-post".
    pub page_type: String,
    /// Recommended H2 sections for this page type.
    pub recommended_sections: Vec<String>,
    /// Minimum recommended word count.
    pub word_count_min: u32,
    /// Maximum recommended word count.
    pub word_count_max: u32,
    /// Tone and voice notes for content generation.
    pub tone_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    /// Month numbers (1-12) when this pattern applies.
    pub months: Vec<u32>,
    /// Description of the seasonal demand.
    pub description: String,
    /// Demand level: "high", "medium", "low".
    pub demand_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PricingNorms {
    /// Typical hourly rate range, e.g. "$75 - $150".
    pub hourly_rate_range: String,
    /// Service call / dispatch fee range, e.g. "$50 - $100".
    pub service_call_fee_range: String,
    /// Emergency / after-hours markup percentage.
    pub emergency_markup_pct: Option<f64>,
    /// Weekend / holiday markup percentage.
    pub weekend_markup_pct: Option<f64>,
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load all non-deleted industry profiles from the journal.
pub fn load_all_profiles(j: &luperiq_forge::ForgeJournal) -> Vec<IndustryProfile> {
    j.latest_by_aggregate_type(AGG_INDUSTRY)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<IndustryProfile>(&e.payload).ok())
        .collect()
}

/// Load a single profile by its slug.
///
/// Because profiles are stored with the slug as the aggregate_id,
/// this is a direct key lookup rather than a scan.
pub fn load_profile_by_slug(
    j: &luperiq_forge::ForgeJournal,
    slug: &str,
) -> Option<IndustryProfile> {
    j.get_latest(AGG_INDUSTRY, slug)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<IndustryProfile>(&e.payload).ok())
}

/// Persist an industry profile to the journal.
///
/// The profile's `slug` is used as the aggregate_id for direct lookups.
pub fn persist_profile(
    j: &mut luperiq_forge::ForgeJournal,
    profile: &IndustryProfile,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(profile).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_INDUSTRY, &profile.slug, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete an industry profile by slug.
pub fn delete_profile(j: &mut luperiq_forge::ForgeJournal, slug: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_INDUSTRY, slug, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
