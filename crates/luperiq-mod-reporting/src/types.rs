//! Normalized reporting types. Every commerce engine maps its events into a
//! `LedgerRow`; all report profiles compute over `&[LedgerRow]` so the engine
//! owns no data and is industry-agnostic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// What a ledger row represents. Stable serde discriminators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerKind {
    Sale,
    Refund,
    SubscriptionRenewal,
    CouponRedemption,
    AffiliateCommission,
    InvoiceIssued,
    InvoicePaid,
    TicketSold,
}

fn default_usd() -> String {
    "USD".to_string()
}

/// One normalized money event, fed by a `ReportSource`. `meta` carries
/// per-kind extras (e.g. `due_ts` for invoices, `event` for tickets, `plan`
/// for subscriptions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRow {
    pub tenant: String,
    /// Epoch seconds.
    pub ts: u64,
    pub kind: LedgerKind,
    #[serde(default)]
    pub gross_cents: i64,
    #[serde(default)]
    pub discount_cents: i64,
    #[serde(default)]
    pub tax_cents: i64,
    /// Processor fee (Stripe/Square), when known.
    #[serde(default)]
    pub fee_cents: i64,
    #[serde(default = "default_usd")]
    pub currency: String,
    /// Origin engine: "commerce" | "pos" | "invoicing" | "promotions" | …
    #[serde(default)]
    pub source: String,
    /// Order number / invoice id / coupon code / event id, depending on kind.
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub customer: String,
    #[serde(default)]
    pub meta: HashMap<String, String>,
}

impl LedgerRow {
    /// Net of discount (not of refunds/fees).
    pub fn net_cents(&self) -> i64 {
        self.gross_cents - self.discount_cents
    }
}

/// Revenue rollup. `net_cents` = gross − discount − refunds − fees.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesSummary {
    pub gross_cents: i64,
    pub discount_cents: i64,
    pub tax_cents: i64,
    pub fee_cents: i64,
    pub refunds_cents: i64,
    pub net_cents: i64,
    pub sale_count: u64,
    pub aov_cents: i64,
}

/// One day's sales (food-truck / daily-close view).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DailyBucket {
    pub day_epoch: u64,
    pub gross_cents: i64,
    pub count: u64,
}

/// Per-coupon ROI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CouponStat {
    pub code: String,
    pub redemptions: u64,
    pub discount_total_cents: i64,
    pub gross_attributed_cents: i64,
}

/// Per-affiliate ROI (the affiliate / referral program). Mirrors `CouponStat`:
/// `attributed_gross_cents` = order gross the referral drove, `commission_cents`
/// = what the affiliate earned on it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffiliateStat {
    pub affiliate: String,
    pub referrals: u64,
    pub attributed_gross_cents: i64,
    pub commission_cents: i64,
}

/// Subscription MRR snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MrrReport {
    pub active_subscriptions: u64,
    pub mrr_cents: i64,
}

/// AR aging buckets (wholesale / professional services).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArAging {
    pub current_cents: i64,
    pub d31_60_cents: i64,
    pub d61_90_cents: i64,
    pub d90_plus_cents: i64,
}

/// Outstanding (issued-but-unpaid) invoices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutstandingInvoices {
    pub count: u64,
    pub total_cents: i64,
}

/// Tickets sold per event (ball games).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketStat {
    pub event: String,
    pub count: u64,
    pub gross_cents: i64,
}
