//! Shared industry definition type.
//!
//! Lives in `luperiq-module-api` to break the circular dependency between
//! `theme_studio` and `site_blueprint`, both of which need an industry list
//! but neither should own it.

/// Shared industry definition — used by theme_studio, site_blueprint,
/// and industry modules. Lives here to break the circular dependency
/// between theme_studio and site_blueprint.
#[derive(Debug, Clone)]
pub struct IndustryDefinition {
    pub slug: &'static str,
    pub name: &'static str,
    pub emoji: &'static str,
    pub accent_color: &'static str,
    pub default_modules: &'static [&'static str],
}
