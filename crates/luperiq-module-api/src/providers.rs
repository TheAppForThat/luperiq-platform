//! Provider traits for decoupling module interactions.
//!
//! These traits allow modules like `site_pages` and `theme_studio` to consume
//! functionality from industry modules without directly importing them.
//! Phase 4 registers concrete implementations into `AppContext` at startup;
//! consumers downcast via `AppContext::service()`.

use crate::context::AppContext;
use axum::Router;

/// Trait for modules that provide homepage sections for industry sites.
///
/// Used by `site_pages` to render industry-specific content without
/// directly importing each industry module (breaks coupling).
pub trait IndustryHomepageProvider: Send + Sync {
    fn industry_slug(&self) -> &str;
    fn homepage_section(&self, ctx: &AppContext) -> Option<String>;
}

/// Trait for modules that provide customer portal functionality.
pub trait CustomerPortalProvider: Send + Sync {
    fn portal_routes(&self, ctx: &AppContext) -> Option<Router>;
    fn portal_name(&self) -> &str;
}
