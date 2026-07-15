//! Affiliate-program ROI report (the previously-unaggregated AffiliateCommission
//! ledger kind). Per-affiliate referrals + attributed gross + commission earned.

use std::collections::HashMap;

use luperiq_mod_reporting::{affiliate_roi, LedgerKind, LedgerRow};

fn commission_row(affiliate: &str, order_gross: i64, commission: i64) -> LedgerRow {
    let mut meta = HashMap::new();
    meta.insert("commission_cents".to_string(), commission.to_string());
    LedgerRow {
        tenant: "shop".into(),
        ts: 1_000,
        kind: LedgerKind::AffiliateCommission,
        gross_cents: order_gross,
        discount_cents: 0,
        tax_cents: 0,
        fee_cents: 0,
        currency: "USD".into(),
        source: "creator-hub".into(),
        reference: affiliate.into(),
        customer: "buyer".into(),
        meta,
    }
}

fn sale_row(gross: i64) -> LedgerRow {
    LedgerRow {
        tenant: "shop".into(),
        ts: 1_000,
        kind: LedgerKind::Sale,
        gross_cents: gross,
        discount_cents: 0,
        tax_cents: 0,
        fee_cents: 0,
        currency: "USD".into(),
        source: "commerce".into(),
        reference: "O9".into(),
        customer: "c".into(),
        meta: HashMap::new(),
    }
}

#[test]
fn affiliate_roi_aggregates_by_affiliate_sorted() {
    let rows = vec![
        commission_row("alice", 10_000, 1_000),
        commission_row("alice", 5_000, 500),
        commission_row("bob", 20_000, 2_000),
        sale_row(9_999), // non-affiliate row ignored
    ];
    let roi = affiliate_roi(&rows);
    assert_eq!(roi.len(), 2);

    // bob first (higher attributed gross)
    assert_eq!(roi[0].affiliate, "bob");
    assert_eq!(roi[0].referrals, 1);
    assert_eq!(roi[0].attributed_gross_cents, 20_000);
    assert_eq!(roi[0].commission_cents, 2_000);

    // alice aggregated across two orders
    assert_eq!(roi[1].affiliate, "alice");
    assert_eq!(roi[1].referrals, 2);
    assert_eq!(roi[1].attributed_gross_cents, 15_000);
    assert_eq!(roi[1].commission_cents, 1_500);
}

#[test]
fn affiliate_roi_empty_when_no_commissions() {
    assert!(affiliate_roi(&[sale_row(100)]).is_empty());
}

#[test]
fn affiliate_roi_counts_people_not_rows_when_meta_present() {
    // A referral-reward row carries the true referred-party count in
    // meta["referrals"]; affiliate_roi must sum that, not count rows.
    let mut row = commission_row("alice", 100_000, 50_000);
    row.meta.insert("referrals".to_string(), "2".to_string());
    let roi = affiliate_roi(&[row]);
    assert_eq!(roi.len(), 1);
    assert_eq!(roi[0].referrals, 2, "one grant row, but it represents 2 referred people");
    assert_eq!(roi[0].commission_cents, 50_000);
}
