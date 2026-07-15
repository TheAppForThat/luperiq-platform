//! The `ReportSource` seam: each commerce engine implements it to feed its WAL
//! events as normalized `LedgerRow`s. The reporting engine owns no data — it
//! only reads through this trait. Real impls (in the workspace): CommerceProjection
//! (orders), invoicing (Invoice/Estimate), promotions (Promo:Redemption),
//! creator-hub (commissions), counter-pos (transactions).

use crate::types::LedgerRow;

pub trait ReportSource: Send + Sync {
    fn rows(&self) -> Vec<LedgerRow>;
}

/// In-memory source for tests / standalone use.
pub struct MockSource(pub Vec<LedgerRow>);

impl ReportSource for MockSource {
    fn rows(&self) -> Vec<LedgerRow> {
        self.0.clone()
    }
}

/// Merge many sources into one normalized stream for the report functions.
pub fn collect(sources: &[&dyn ReportSource]) -> Vec<LedgerRow> {
    sources.iter().flat_map(|s| s.rows()).collect()
}
