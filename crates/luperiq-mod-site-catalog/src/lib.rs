//! `luperiq-mod-site-catalog` — central registry for all LuperIQ industry/site
//! types, plus the hosted fleet control plane (Central-only).
//!
//! Provides `SiteTypeDefinition` WAL persistence, theme presets, module and page
//! defaults, onboarding wizard configuration, and the Site Fleet admin API that
//! shells out to fleet-management scripts.

pub mod site_catalog;

// Re-export taxonomy types for external consumers.
pub use site_catalog::taxonomy;
