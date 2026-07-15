//! Closes the last engine-to-engine seam: a sale that flows through
//! catalog (price) → promotions (discount) → payments-core (checkout intent)
//! must, on completion, normalize into `LedgerRow`s that RECONCILE in the
//! reporting engine's numbers. This is the storefront spine's
//! `impl ReportSource` proven on real types across all four crates.

use std::collections::HashMap;

use luperiq_mod_catalog::{
    billing_options, price_cart, price_selection, LineSelection, Product, ProductKind,
    SubscriptionPlan, TicketInfo,
};
use luperiq_mod_payments_core::{build_intent, CheckoutIntent, Interval, Mode};
use luperiq_mod_promotions::{
    evaluate, AppliedPromotions, AppliesTo, CartContext, CartLine, DiscountKind, Promotion,
};
use luperiq_mod_reporting::{
    coupon_roi, mrr, sales_summary, tickets_sold, collect, LedgerKind, LedgerRow, MockSource,
};

// ---- catalog builders ------------------------------------------------------

fn simple(id: &str, name: &str, cents: i64, cats: &[&str]) -> Product {
    Product {
        id: id.into(),
        slug: id.into(),
        name: name.into(),
        kind: ProductKind::Simple,
        base_price_cents: cents,
        currency: "USD".into(),
        active: true,
        categories: cats.iter().map(|s| s.to_string()).collect(),
        variants: vec![],
        option_groups: vec![],
        volume_tiers: vec![],
        group_prices: vec![],
        stock: None,
        ticket: None,
        subscription: None,
    }
}

fn sel(id: &str, qty: u32) -> LineSelection {
    LineSelection {
        product_id: id.into(),
        variant_id: None,
        modifier_ids: vec![],
        quantity: qty,
        customer_group: None,
    }
}

fn category_promo(code: &str, pct: u32, cats: &[&str]) -> Promotion {
    Promotion {
        id: code.into(),
        code: code.into(),
        kind: DiscountKind::Percent { pct },
        applies_to: AppliesTo::Categories { ids: cats.iter().map(|s| s.to_string()).collect() },
        min_order_cents: 0,
        max_uses: 0,
        uses: 0,
        max_uses_per_customer: 0,
        first_order_only: false,
        stackable: true,
        starts_at: 0,
        expires_at: 0,
        channels: vec![],
        active: true,
        created_at: 0,
        updated_at: 0,
    }
}

// ---- the storefront spine's ledger glue (what a real ReportSource emits) ----

/// On a completed one-time/subscription order, derive the normalized ledger
/// rows: the sale itself + one coupon-redemption row per applied promotion.
fn ledger_for_completed_sale(
    intent: &CheckoutIntent,
    applied: &AppliedPromotions,
    tenant: &str,
    customer: &str,
    order_ref: &str,
    ts: u64,
) -> Vec<LedgerRow> {
    let kind = if intent.mode() == Mode::Subscription {
        LedgerKind::SubscriptionRenewal
    } else {
        LedgerKind::Sale
    };
    let mut rows = vec![LedgerRow {
        tenant: tenant.into(),
        ts,
        kind,
        gross_cents: intent.subtotal_cents(),
        discount_cents: intent.discount_cents,
        tax_cents: 0,
        fee_cents: 0,
        currency: intent.currency.clone(),
        source: "commerce".into(),
        reference: order_ref.into(),
        customer: customer.into(),
        meta: HashMap::new(),
    }];
    for ap in &applied.applied {
        rows.push(LedgerRow {
            tenant: tenant.into(),
            ts,
            kind: LedgerKind::CouponRedemption,
            gross_cents: intent.subtotal_cents(),
            discount_cents: ap.discount_cents,
            tax_cents: 0,
            fee_cents: 0,
            currency: intent.currency.clone(),
            source: "promotions".into(),
            reference: ap.code.clone(),
            customer: customer.into(),
            meta: HashMap::new(),
        });
    }
    rows
}

// ---- tests -----------------------------------------------------------------

#[test]
fn one_time_sale_with_coupon_reconciles_in_reporting() {
    // pipeline: price → discount → intent
    let products = vec![
        simple("burger", "Burger", 1000, &["food"]),
        simple("tee", "Tee", 2000, &["apparel"]),
    ];
    let priced = price_cart(&products, &[sel("burger", 2), sel("tee", 1)]).unwrap();
    let cart_lines: Vec<CartLine> = priced
        .iter()
        .enumerate()
        .map(|(i, pl)| CartLine {
            line_id: format!("L{i}"),
            product_id: pl.product_id.clone(),
            category_ids: pl.categories.clone(),
            unit_price_cents: pl.unit_price_cents,
            quantity: pl.quantity,
        })
        .collect();
    let ctx = CartContext { lines: cart_lines, customer_id: None, channel: String::new(), is_first_order: false };
    let applied = evaluate(&[category_promo("FOOD10", 10, &["food"])], &ctx, &["FOOD10".into()], &HashMap::new(), 0);
    assert_eq!(applied.total_discount_cents, 200);

    let lines = priced
        .iter()
        .map(|pl| luperiq_mod_payments_core::IntentLine {
            product_id: pl.product_id.clone(),
            name: pl.name.clone(),
            unit_amount_cents: pl.unit_price_cents,
            quantity: pl.quantity,
            recurring: None,
        })
        .collect();
    let intent = build_intent(
        lines,
        "USD",
        applied.total_discount_cents,
        applied.free_shipping,
        None,
        "https://s",
        "https://c",
        Some("LIQ-1".into()),
    )
    .unwrap();

    // spine → ledger → reporting
    let rows = ledger_for_completed_sale(&intent, &applied, "crew", "buyer@x.co", "LIQ-1", 1_000);
    let src = MockSource(rows);
    let all = collect(&[&src]);

    let ss = sales_summary(&all);
    assert_eq!(ss.gross_cents, 4000);
    assert_eq!(ss.discount_cents, 200);
    assert_eq!(ss.sale_count, 1, "coupon-redemption row is not itself a sale");
    // reporting's net must equal payments-core's order total — the seam reconciles.
    assert_eq!(ss.net_cents, intent.total_cents());
    assert_eq!(ss.net_cents, 3800);
    assert_eq!(ss.aov_cents, 3800);

    let roi = coupon_roi(&all);
    assert_eq!(roi.len(), 1);
    assert_eq!(roi[0].code, "FOOD10");
    assert_eq!(roi[0].redemptions, 1);
    assert_eq!(roi[0].discount_total_cents, 200);
    assert_eq!(roi[0].gross_attributed_cents, 4000);
}

#[test]
fn subscription_renewal_feeds_mrr() {
    // catalog's monthly toggle drives a recurring intent...
    let mut club = simple("coffee-club", "Coffee Club", 0, &["club"]);
    club.kind = ProductKind::Subscription { interval: "month".into() };
    club.subscription = Some(SubscriptionPlan { monthly_cents: 1500, annual_free_months: 2 });
    let monthly = &billing_options(&club)[0];
    assert_eq!(monthly.interval, "month");

    let line = luperiq_mod_payments_core::IntentLine {
        product_id: club.id.clone(),
        name: club.name.clone(),
        unit_amount_cents: monthly.unit_price_cents,
        quantity: 1,
        recurring: Interval::parse(&monthly.interval),
    };
    let intent =
        build_intent(vec![line], "USD", 0, false, Some("subber@x.co".into()), "https://s", "https://c", Some("SUB-1".into())).unwrap();
    assert_eq!(intent.mode(), Mode::Subscription);

    // ...and a renewal normalizes into an MRR-counting ledger row.
    let rows = ledger_for_completed_sale(&intent, &AppliedPromotions::default(), "crew", "subber@x.co", "SUB-1", 5_000);
    let src = MockSource(rows);
    let all = collect(&[&src]);

    let report = mrr(&all, 10_000, 30);
    assert_eq!(report.active_subscriptions, 1);
    assert_eq!(report.mrr_cents, 1500);
    // a subscription renewal also counts as revenue in the sales rollup.
    assert_eq!(sales_summary(&all).gross_cents, 1500);
}

#[test]
fn ticket_sale_feeds_tickets_sold() {
    // catalog prices a ball-game ticket (capacity-checked)...
    let mut game = simple("ga", "General Admission", 2500, &["tickets"]);
    game.kind = ProductKind::Ticket { event: "Rivalry Game".into() };
    game.ticket = Some(TicketInfo { capacity: 100, sold: 0 });
    let line = price_selection(&game, &sel("ga", 3)).unwrap();
    assert_eq!(line.unit_price_cents, 2500);

    // ...the spine emits one TicketSold row per seat (count = quantity).
    let event = match &game.kind {
        ProductKind::Ticket { event } => event.clone(),
        _ => unreachable!(),
    };
    let mut meta = HashMap::new();
    meta.insert("event".to_string(), event.clone());
    let rows: Vec<LedgerRow> = (0..line.quantity)
        .map(|i| LedgerRow {
            tenant: "stadium".into(),
            ts: 1_000 + i as u64,
            kind: LedgerKind::TicketSold,
            gross_cents: line.unit_price_cents,
            discount_cents: 0,
            tax_cents: 0,
            fee_cents: 0,
            currency: "USD".into(),
            source: "commerce".into(),
            reference: "LIQ-T1".into(),
            customer: "fan@x.co".into(),
            meta: meta.clone(),
        })
        .collect();
    let src = MockSource(rows);
    let all = collect(&[&src]);

    let stats = tickets_sold(&all);
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].event, "Rivalry Game");
    assert_eq!(stats[0].count, 3);
    assert_eq!(stats[0].gross_cents, 7500);
}
