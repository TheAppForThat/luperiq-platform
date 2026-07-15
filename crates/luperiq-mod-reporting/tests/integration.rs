//! Standalone tests for every report profile (no platform crates).

use std::collections::HashMap;

use luperiq_mod_reporting::{
    ar_aging, collect, coupon_roi, daily_sales, mrr, outstanding_invoices, sales_summary,
    tickets_sold, LedgerKind, LedgerRow, MockSource,
};

const NOW: u64 = 1_780_000_000;
const DAY: u64 = 86_400;

fn row(kind: LedgerKind, ts: u64, gross: i64) -> LedgerRow {
    LedgerRow {
        tenant: "t".into(),
        ts,
        kind,
        gross_cents: gross,
        discount_cents: 0,
        tax_cents: 0,
        fee_cents: 0,
        currency: "USD".into(),
        source: "commerce".into(),
        reference: String::new(),
        customer: String::new(),
        meta: HashMap::new(),
    }
}

#[test]
fn sales_summary_nets_discounts_refunds_fees() {
    let mut s1 = row(LedgerKind::Sale, NOW, 2000);
    s1.tax_cents = 100;
    s1.fee_cents = 50;
    let mut s2 = row(LedgerKind::Sale, NOW - DAY, 3000);
    s2.discount_cents = 300;
    s2.fee_cents = 90;
    let refund = row(LedgerKind::Refund, NOW, 500);
    let rows = vec![s1, s2, refund];

    let s = sales_summary(&rows);
    assert_eq!(s.gross_cents, 5000);
    assert_eq!(s.discount_cents, 300);
    assert_eq!(s.refunds_cents, 500);
    assert_eq!(s.fee_cents, 140);
    assert_eq!(s.sale_count, 2);
    assert_eq!(s.net_cents, 5000 - 300 - 500 - 140); // 4060
    assert_eq!(s.aov_cents, (5000 - 300) / 2); // 2350
}

#[test]
fn daily_sales_buckets_by_day_sorted() {
    let s1 = row(LedgerKind::Sale, NOW, 2000);
    let mut s2 = row(LedgerKind::Sale, NOW - DAY, 3000);
    s2.discount_cents = 300;
    let buckets = daily_sales(&[s1, s2]);
    assert_eq!(buckets.len(), 2);
    // ascending by day
    assert!(buckets[0].day_epoch < buckets[1].day_epoch);
    assert_eq!(buckets[0].gross_cents, 2700); // 3000 - 300 (prev day)
    assert_eq!(buckets[1].gross_cents, 2000); // today
}

#[test]
fn coupon_roi_aggregates_by_code() {
    let mut c1 = row(LedgerKind::CouponRedemption, NOW, 2000);
    c1.reference = "SAVE20".into();
    c1.discount_cents = 400;
    let mut c2 = row(LedgerKind::CouponRedemption, NOW, 1000);
    c2.reference = "SAVE20".into();
    c2.discount_cents = 200;
    let mut other = row(LedgerKind::CouponRedemption, NOW, 5000);
    other.reference = "WELCOME".into();
    other.discount_cents = 750;

    let stats = coupon_roi(&[c1, c2, other]);
    assert_eq!(stats.len(), 2);
    // sorted by attributed gross desc -> WELCOME (5000) first
    assert_eq!(stats[0].code, "WELCOME");
    let save20 = stats.iter().find(|s| s.code == "SAVE20").unwrap();
    assert_eq!(save20.redemptions, 2);
    assert_eq!(save20.discount_total_cents, 600);
    assert_eq!(save20.gross_attributed_cents, 3000);
}

#[test]
fn mrr_sums_window_and_counts_distinct_customers() {
    let mut a = row(LedgerKind::SubscriptionRenewal, NOW, 1500);
    a.customer = "c1".into();
    let mut b = row(LedgerKind::SubscriptionRenewal, NOW - 5 * DAY, 1500);
    b.customer = "c2".into();
    let mut old = row(LedgerKind::SubscriptionRenewal, NOW - 60 * DAY, 999);
    old.customer = "c3".into();

    let m = mrr(&[a, b, old], NOW, 30);
    assert_eq!(m.active_subscriptions, 2); // c1, c2 (c3 outside window)
    assert_eq!(m.mrr_cents, 3000);
}

#[test]
fn outstanding_and_ar_aging() {
    let mut inv1 = row(LedgerKind::InvoiceIssued, NOW - 5 * DAY, 10000);
    inv1.reference = "INV-1".into();
    inv1.meta.insert("due_ts".into(), (NOW - 5 * DAY).to_string()); // current (<=30d)
    let mut inv2 = row(LedgerKind::InvoiceIssued, NOW - 45 * DAY, 20000);
    inv2.reference = "INV-2".into();
    inv2.meta.insert("due_ts".into(), (NOW - 45 * DAY).to_string()); // 31-60d
    let mut paid1 = row(LedgerKind::InvoicePaid, NOW, 10000);
    paid1.reference = "INV-1".into();

    let rows = vec![inv1, inv2, paid1];
    let o = outstanding_invoices(&rows);
    assert_eq!(o.count, 1); // INV-1 paid, INV-2 outstanding
    assert_eq!(o.total_cents, 20000);

    let a = ar_aging(&rows, NOW);
    assert_eq!(a.current_cents, 0); // INV-1 is paid
    assert_eq!(a.d31_60_cents, 20000); // INV-2 aged 45d
    assert_eq!(a.d61_90_cents, 0);
    assert_eq!(a.d90_plus_cents, 0);
}

#[test]
fn tickets_sold_per_event() {
    let mut t1 = row(LedgerKind::TicketSold, NOW, 2500);
    t1.meta.insert("event".into(), "Game1".into());
    let mut t2 = row(LedgerKind::TicketSold, NOW, 2500);
    t2.meta.insert("event".into(), "Game1".into());
    let mut t3 = row(LedgerKind::TicketSold, NOW, 3000);
    t3.meta.insert("event".into(), "Game2".into());

    let stats = tickets_sold(&[t1, t2, t3]);
    assert_eq!(stats.len(), 2);
    let g1 = stats.iter().find(|s| s.event == "Game1").unwrap();
    assert_eq!(g1.count, 2);
    assert_eq!(g1.gross_cents, 5000);
    let g2 = stats.iter().find(|s| s.event == "Game2").unwrap();
    assert_eq!(g2.count, 1);
    assert_eq!(g2.gross_cents, 3000);
}

#[test]
fn collect_merges_sources() {
    let a = MockSource(vec![row(LedgerKind::Sale, NOW, 1000)]);
    let b = MockSource(vec![row(LedgerKind::Sale, NOW, 2000), row(LedgerKind::Refund, NOW, 100)]);
    let merged = collect(&[&a, &b]);
    assert_eq!(merged.len(), 3);
    assert_eq!(sales_summary(&merged).gross_cents, 3000);
}
