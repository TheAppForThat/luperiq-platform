//! Pure report computations over a `&[LedgerRow]` stream. Each function is a
//! per-site-type "report profile" — deterministic, no I/O, fully testable.

use std::collections::{HashMap, HashSet};

use crate::types::*;

const DAY: u64 = 86_400;

/// Sales kinds that count as revenue.
fn is_sale(k: LedgerKind) -> bool {
    matches!(
        k,
        LedgerKind::Sale
            | LedgerKind::SubscriptionRenewal
            | LedgerKind::InvoicePaid
            | LedgerKind::TicketSold
    )
}

/// Overall revenue rollup (gross/discount/tax/fees/refunds/net/AOV).
pub fn sales_summary(rows: &[LedgerRow]) -> SalesSummary {
    let mut s = SalesSummary::default();
    for r in rows {
        if is_sale(r.kind) {
            s.gross_cents += r.gross_cents;
            s.discount_cents += r.discount_cents;
            s.tax_cents += r.tax_cents;
            s.fee_cents += r.fee_cents;
            s.sale_count += 1;
        } else if r.kind == LedgerKind::Refund {
            s.refunds_cents += r.gross_cents;
        }
    }
    s.net_cents = s.gross_cents - s.discount_cents - s.refunds_cents - s.fee_cents;
    s.aov_cents = if s.sale_count > 0 {
        (s.gross_cents - s.discount_cents) / s.sale_count as i64
    } else {
        0
    };
    s
}

/// Sales grouped by UTC day (ascending). Net of discount.
pub fn daily_sales(rows: &[LedgerRow]) -> Vec<DailyBucket> {
    let mut m: HashMap<u64, (i64, u64)> = HashMap::new();
    for r in rows {
        if is_sale(r.kind) {
            let day = r.ts - (r.ts % DAY);
            let e = m.entry(day).or_insert((0, 0));
            e.0 += r.net_cents();
            e.1 += 1;
        }
    }
    let mut out: Vec<DailyBucket> = m
        .into_iter()
        .map(|(d, (g, c))| DailyBucket { day_epoch: d, gross_cents: g, count: c })
        .collect();
    out.sort_by_key(|b| b.day_epoch);
    out
}

/// Per-coupon ROI from `CouponRedemption` rows (reference = code, discount_cents
/// = discount granted, gross_cents = order gross the coupon was applied to).
pub fn coupon_roi(rows: &[LedgerRow]) -> Vec<CouponStat> {
    let mut m: HashMap<String, (u64, i64, i64)> = HashMap::new();
    for r in rows {
        if r.kind == LedgerKind::CouponRedemption {
            let e = m.entry(r.reference.clone()).or_insert((0, 0, 0));
            e.0 += 1;
            e.1 += r.discount_cents;
            e.2 += r.gross_cents;
        }
    }
    let mut out: Vec<CouponStat> = m
        .into_iter()
        .map(|(code, (n, d, g))| CouponStat {
            code,
            redemptions: n,
            discount_total_cents: d,
            gross_attributed_cents: g,
        })
        .collect();
    out.sort_by(|a, b| b.gross_attributed_cents.cmp(&a.gross_attributed_cents));
    out
}

/// Per-affiliate ROI from `AffiliateCommission` rows (reference = affiliate id/
/// code, gross_cents = order gross attributed to the referral, meta["commission_cents"]
/// = commission earned). Sorted by attributed gross desc.
pub fn affiliate_roi(rows: &[LedgerRow]) -> Vec<AffiliateStat> {
    let mut m: HashMap<String, (u64, i64, i64)> = HashMap::new();
    for r in rows {
        if r.kind == LedgerKind::AffiliateCommission {
            let e = m.entry(r.reference.clone()).or_insert((0, 0, 0));
            // Count PEOPLE referred, not ledger rows: a referral-reward row
            // carries the true count in meta["referrals"]; a plain cash-affiliate
            // row (one row per sale) falls back to 1.
            e.0 += r
                .meta
                .get("referrals")
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1);
            e.1 += r.gross_cents;
            e.2 += r
                .meta
                .get("commission_cents")
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
        }
    }
    let mut out: Vec<AffiliateStat> = m
        .into_iter()
        .map(|(affiliate, (n, g, c))| AffiliateStat {
            affiliate,
            referrals: n,
            attributed_gross_cents: g,
            commission_cents: c,
        })
        .collect();
    out.sort_by(|a, b| b.attributed_gross_cents.cmp(&a.attributed_gross_cents));
    out
}

/// MRR proxy: sum of net subscription renewals within the trailing window;
/// active subscriptions = distinct renewing customers in that window.
pub fn mrr(rows: &[LedgerRow], now: u64, window_days: u64) -> MrrReport {
    let cutoff = now.saturating_sub(window_days * DAY);
    let mut total = 0i64;
    let mut subs: HashSet<&str> = HashSet::new();
    for r in rows {
        if r.kind == LedgerKind::SubscriptionRenewal && r.ts >= cutoff {
            total += r.net_cents();
            subs.insert(r.customer.as_str());
        }
    }
    MrrReport { active_subscriptions: subs.len() as u64, mrr_cents: total }
}

/// Issued-but-unpaid invoices (matched by reference).
pub fn outstanding_invoices(rows: &[LedgerRow]) -> OutstandingInvoices {
    let (issued, paid) = invoice_state(rows);
    let mut o = OutstandingInvoices::default();
    for (reference, (amt, _due)) in &issued {
        if !paid.contains(reference) {
            o.count += 1;
            o.total_cents += amt;
        }
    }
    o
}

/// AR aging buckets for unpaid invoices, aged by `meta["due_ts"]` (else ts).
pub fn ar_aging(rows: &[LedgerRow], now: u64) -> ArAging {
    let (issued, paid) = invoice_state(rows);
    let mut a = ArAging::default();
    for (reference, (amt, due)) in &issued {
        if paid.contains(reference) {
            continue;
        }
        let age_days = now.saturating_sub(*due) / DAY;
        if age_days <= 30 {
            a.current_cents += amt;
        } else if age_days <= 60 {
            a.d31_60_cents += amt;
        } else if age_days <= 90 {
            a.d61_90_cents += amt;
        } else {
            a.d90_plus_cents += amt;
        }
    }
    a
}

/// Shared: (reference -> (amount, due_ts)) for issued, set of paid references.
fn invoice_state(rows: &[LedgerRow]) -> (HashMap<String, (i64, u64)>, HashSet<String>) {
    let mut issued: HashMap<String, (i64, u64)> = HashMap::new();
    let mut paid: HashSet<String> = HashSet::new();
    for r in rows {
        match r.kind {
            LedgerKind::InvoiceIssued => {
                let due = r
                    .meta
                    .get("due_ts")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(r.ts);
                issued.insert(r.reference.clone(), (r.gross_cents, due));
            }
            LedgerKind::InvoicePaid => {
                paid.insert(r.reference.clone());
            }
            _ => {}
        }
    }
    (issued, paid)
}

/// Tickets sold per event (`meta["event"]`, else reference). Gross desc.
pub fn tickets_sold(rows: &[LedgerRow]) -> Vec<TicketStat> {
    let mut m: HashMap<String, (u64, i64)> = HashMap::new();
    for r in rows {
        if r.kind == LedgerKind::TicketSold {
            let event = r
                .meta
                .get("event")
                .cloned()
                .unwrap_or_else(|| r.reference.clone());
            let e = m.entry(event).or_insert((0, 0));
            e.0 += 1;
            e.1 += r.gross_cents;
        }
    }
    let mut out: Vec<TicketStat> = m
        .into_iter()
        .map(|(event, (c, g))| TicketStat { event, count: c, gross_cents: g })
        .collect();
    out.sort_by(|a, b| b.gross_cents.cmp(&a.gross_cents));
    out
}
