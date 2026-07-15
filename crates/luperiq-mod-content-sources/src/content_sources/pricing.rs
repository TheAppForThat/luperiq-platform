
//! Centralized pricing configuration for page generation and content sourcing.
//!
//! Stored as a ForgeJournal aggregate. If no aggregate exists (fresh install),
//! hardcoded defaults are used. The aggregate is created on first admin
//! modification of pricing.

use serde::{Deserialize, Serialize};

pub const AGG_PAGE_GEN_PRICING: &str = "PageGenPricing";

/// Runtime-adjustable pricing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageGenPricing {
    // Per-page generation
    pub credits_per_page: u32,
    pub credits_per_seo: u32,

    // Commissioned fact sheets
    pub credits_ai_verified: u32,
    pub credits_expert_reviewed: u32,

    // Sharing refund percentages
    pub refund_trusted_source_pct: u32,
    pub refund_anonymized_pct: u32,

    // Active discounts / promo codes
    #[serde(default)]
    pub active_discounts: Vec<Discount>,

    // ── Contributor quality tier thresholds (Layer 2) ──
    #[serde(default = "default_tier_gold")]
    pub quality_tier_gold: f32,
    #[serde(default = "default_tier_silver")]
    pub quality_tier_silver: f32,
    #[serde(default = "default_tier_minimum")]
    pub quality_tier_minimum: f32,

    // ── Base payout rates (credits per validated submission) ──
    #[serde(default = "default_payout_bronze")]
    pub payout_base_bronze: u32,
    #[serde(default = "default_payout_silver")]
    pub payout_base_silver: u32,
    #[serde(default = "default_payout_gold")]
    pub payout_base_gold: u32,

    // ── Royalty rates (credits per page generation use) ──
    #[serde(default = "default_royalty_bronze")]
    pub royalty_rate_bronze: f32,
    #[serde(default = "default_royalty_silver")]
    pub royalty_rate_silver: f32,
    #[serde(default = "default_royalty_gold")]
    pub royalty_rate_gold: f32,

    // ── Royalty per search click ──
    #[serde(default = "default_royalty_click")]
    pub royalty_per_click: f32,
    // LAYER 3: Credit marketplace fees
    // marketplace_fee_pct: u32,
}

fn default_tier_gold() -> f32 {
    85.0
}
fn default_tier_silver() -> f32 {
    70.0
}
fn default_tier_minimum() -> f32 {
    50.0
}
fn default_payout_bronze() -> u32 {
    5
}
fn default_payout_silver() -> u32 {
    10
}
fn default_payout_gold() -> u32 {
    20
}
fn default_royalty_bronze() -> f32 {
    0.5
}
fn default_royalty_silver() -> f32 {
    1.0
}
fn default_royalty_gold() -> f32 {
    2.0
}
fn default_royalty_click() -> f32 {
    0.1
}

impl Default for PageGenPricing {
    fn default() -> Self {
        Self {
            credits_per_page: 15,
            credits_per_seo: 3,
            credits_ai_verified: 50,
            credits_expert_reviewed: 200,
            refund_trusted_source_pct: 50,
            refund_anonymized_pct: 25,
            active_discounts: vec![],
            quality_tier_gold: default_tier_gold(),
            quality_tier_silver: default_tier_silver(),
            quality_tier_minimum: default_tier_minimum(),
            payout_base_bronze: default_payout_bronze(),
            payout_base_silver: default_payout_silver(),
            payout_base_gold: default_payout_gold(),
            royalty_rate_bronze: default_royalty_bronze(),
            royalty_rate_silver: default_royalty_silver(),
            royalty_rate_gold: default_royalty_gold(),
            royalty_per_click: default_royalty_click(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Discount {
    pub code: String,
    pub discount_pct: u32,
    pub applies_to: DiscountScope,
    pub valid_until: Option<u64>,
    pub max_uses: Option<u32>,
    #[serde(default)]
    pub uses: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiscountScope {
    All,
    PageGeneration,
    Commissions,
}

/// Load pricing config from journal, falling back to defaults if none exists.
pub fn load_pricing(journal: &luperiq_forge::ForgeJournal) -> PageGenPricing {
    let events = journal.latest_by_aggregate_type(AGG_PAGE_GEN_PRICING);
    if let Some(event) = events.first() {
        if let Ok(pricing) = serde_json::from_slice::<PageGenPricing>(&event.payload) {
            return pricing;
        }
    }
    PageGenPricing::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_pricing_values() {
        let p = PageGenPricing::default();
        assert_eq!(p.credits_per_page, 15);
        assert_eq!(p.credits_per_seo, 3);
        assert_eq!(p.credits_ai_verified, 50);
        assert_eq!(p.credits_expert_reviewed, 200);
        assert_eq!(p.refund_trusted_source_pct, 50);
        assert_eq!(p.refund_anonymized_pct, 25);
        assert!(p.active_discounts.is_empty());
    }

    #[test]
    fn pricing_round_trip_serde() {
        let p = PageGenPricing::default();
        let json = serde_json::to_vec(&p).unwrap();
        let p2: PageGenPricing = serde_json::from_slice(&json).unwrap();
        assert_eq!(p2.credits_per_page, p.credits_per_page);
        assert_eq!(p2.credits_per_seo, p.credits_per_seo);
    }
}
