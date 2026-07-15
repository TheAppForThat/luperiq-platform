/// Durability mode for WAL writes.
///
/// - [`Sync`] — every `append()` calls `fsync` before returning.
/// - [`Async`] — `append()` returns immediately; the OS flushes later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityMode {
    Sync,
    Async,
}
