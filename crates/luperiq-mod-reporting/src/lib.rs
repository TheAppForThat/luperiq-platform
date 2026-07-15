//! # luperiq-mod-reporting
//!
//! The ONE reporting engine for LuperIQ commerce. It owns **no data**: each
//! commerce engine implements [`ReportSource`] to feed its events as normalized
//! [`LedgerRow`]s, and the pure report functions in [`reports`] compute
//! per-site-type profiles over that stream:
//!
//! - [`sales_summary`] — gross/discount/tax/fees/refunds/net/AOV (every store)
//! - [`daily_sales`] — by-day totals (food truck / daily close)
//! - [`coupon_roi`] — per-code redemptions + discount + attributed gross
//! - [`mrr`] — subscription MRR + active count
//! - [`outstanding_invoices`] / [`ar_aging`] — wholesale + Drs/attorneys
//! - [`tickets_sold`] — per-event (ball games)
//!
//! Built standalone on the siteRustTemplate2 conventions (compiles + `cargo
//! test`s green with no platform crates — the report math is fully testable).
//! In the workspace, one `impl ReportSource` per engine feeds the same
//! functions and a thin CmsModule exposes admin dashboards.

pub mod reports;
pub mod source;
pub mod types;

pub use reports::{
    affiliate_roi, ar_aging, coupon_roi, daily_sales, mrr, outstanding_invoices, sales_summary,
    tickets_sold,
};
pub use source::{collect, MockSource, ReportSource};
pub use types::{
    AffiliateStat, ArAging, CouponStat, DailyBucket, LedgerKind, LedgerRow, MrrReport,
    OutstandingInvoices, SalesSummary, TicketStat,
};
