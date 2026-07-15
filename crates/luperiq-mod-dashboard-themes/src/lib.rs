//! Admin panel visual theme system for LuperIQ.
//!
//! Provides 14 named color presets, a custom theme builder, AI theme generation,
//! and injects the theme-system JavaScript bundle into every admin page via
//! [`DashboardThemesModule::admin_js`]. Theme state is persisted client-side in
//! `localStorage`; the selected theme ID is also persisted server-side via the
//! companion dashboard module's `/api/modules/dashboard/theme` route.
//!
//! This crate controls **the admin panel's own appearance** only — it is
//! completely separate from `theme-studio`, which owns public-site design tokens.
pub mod dashboard_themes;
pub use dashboard_themes::DashboardThemesModule;
pub use dashboard_themes::{get_theme, theme_css, themes_json, DashboardTheme, ThemeInfo, THEMES};
