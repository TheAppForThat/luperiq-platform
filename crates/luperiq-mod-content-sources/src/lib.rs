//! Content Sourcing module — Layer 1 of 3 of the LuperIQ content pipeline.
//!
//! Manages the full lifecycle of content used for AI page generation:
//! customer uploads, commissioned fact sheets, scrape intents, LuperIQ platform
//! fact sheets, and conflict detection. See `content_sources::mod` for the
//! full architecture description.
pub mod content_sources;
pub use content_sources::ContentSourcesModule;
