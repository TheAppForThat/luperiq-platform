
// Items here are used by the main server binary but appear dead when compiled
// by bin/ tools that include this file via #[path] with a narrower scope.
#![allow(dead_code)]
//! Core types for the Content Sourcing system.
//!
//! ## Future Integration Points
//!
//! - LAYER 2: content_type_tag supports "story", "expansion", "correction"
//! - LAYER 2: quality_score populated by QualityReview aggregate
//! - LAYER 2: contributor_id links to ContributorProfile aggregate
//! - LAYER 3: transferable + credit_value enable marketplace listings

use serde::{Deserialize, Serialize};

// ── Aggregate type constants ─────────────────────────────────────────

pub const AGG_CONTENT_SOURCE: &str = "ContentSource";

// Reserved for future layers (documented, not created):
// pub const AGG_CONTRIBUTOR_PROFILE: &str = "ContributorProfile";   // LAYER 2
// pub const AGG_CONTRIBUTOR_PAYOUT: &str = "ContributorPayout";     // LAYER 2
// pub const AGG_QUALITY_REVIEW: &str = "QualityReview";             // LAYER 2
// pub const AGG_CREDIT_TRANSFER: &str = "CreditTransfer";           // LAYER 3
// pub const AGG_MARKETPLACE_LISTING: &str = "MarketplaceListing";   // LAYER 3

// ── FactEntry ────────────────────────────────────────────────────────

/// Individual addressable fact within a ContentSource.
/// Enables field-level conflict detection and structured AI prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FactEntry {
    /// Fact key, e.g. "peak_months", "treatment_approach", "severity"
    pub key: String,
    /// Fact value, e.g. "June, July, August"
    pub value: String,
    /// How this fact was established
    pub confidence: FactConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FactConfidence {
    Verified,
    AiGenerated,
    CustomerStated,
}

// ── ContentSourceType ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentSourceType {
    LuperiqFactSheet,
    CustomerUpload,
    SiteScrape,
    CommissionedAiVerified,
    CommissionedExpertReview,
    // LAYER 2: future types
    // ContributorFact,
    // ContributorStory,
    // ContributorExpansion,
    // ContributorCorrection,
}

// ── SharingTier ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SharingTier {
    NeverShare,
    ShareAnonymized,
    ShareAsTrustedSource,
}

impl Default for SharingTier {
    fn default() -> Self {
        Self::NeverShare
    }
}

// ── ValidationStatus ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Pending,
    InReview,
    Validated,
    Rejected,
    NotApplicable,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        Self::NotApplicable
    }
}

// ── PayoutStatus ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PayoutStatus {
    NotApplicable,
    // LAYER 2: future variants
    // Pending,
    // Paid,
    // Held,
    // Rejected,
}

impl Default for PayoutStatus {
    fn default() -> Self {
        Self::NotApplicable
    }
}

// ── ContentSource ────────────────────────────────────────────────────

/// A content source for page generation.
///
/// Stored in ForgeJournal with aggregate type "ContentSource".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSource {
    pub source_id: String,
    pub source_type: ContentSourceType,
    pub industry_slug: String,
    pub topic_slug: String,
    pub title: String,

    // Content
    pub structured_facts: Vec<FactEntry>,
    #[serde(default)]
    pub raw_content: String,

    // Sharing & licensing
    #[serde(default)]
    pub sharing_tier: SharingTier,
    #[serde(default)]
    pub sharing_discount_applied: bool,
    #[serde(default)]
    pub validation_status: ValidationStatus,

    // Metadata
    pub owner_license_key: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub file_format: String,

    // LAYER 2: Contributor Network — reserved fields, default to None/empty
    #[serde(default)]
    pub contributor_id: Option<String>,
    #[serde(default)]
    pub contributor_payout_status: PayoutStatus,
    #[serde(default)]
    pub quality_score: Option<f32>,
    #[serde(default = "default_content_type_tag")]
    pub content_type_tag: String,
    #[serde(default)]
    pub parent_source_id: Option<String>,

    // LAYER 3: Credit Marketplace — reserved fields
    #[serde(default)]
    pub transferable: bool,
    #[serde(default)]
    pub credit_value: Option<u32>,
}

fn default_content_type_tag() -> String {
    "fact_sheet".to_string()
}

/// Tombstone marker for deleted ContentSource aggregates.
pub const CONTENT_SOURCE_TOMBSTONE: &[u8] = b"__CONTENT_SOURCE_DELETED__";
