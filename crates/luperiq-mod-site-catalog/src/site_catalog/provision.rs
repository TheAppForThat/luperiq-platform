//! Provision data extractor — converts a `SiteTypeDefinition` into a flat
//! `ProvisionPayload` that shell scripts can consume via jq.
//!
//! Endpoint: GET /api/modules/site-catalog/provision/{slug}
//! Auth: none (called by the provisioning shell script without cookies)

use serde::Serialize;
use serde_json::Value;

use super::types::{BrandColors, DefaultPage, NavItem, OnboardingStep, SiteTypeDefinition};

/// Flat, JSON-friendly representation of everything needed to provision a new
/// CMS site of a given type.  Shell scripts consume this via `jq`.
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionPayload {
    // ── Identity ──────────────────────────────────────────────────────
    pub slug: String,
    pub name: String,
    pub emoji: String,
    /// "free" or "business"
    pub category: String,
    pub description: String,
    pub default_tagline: String,

    // ── Modules ────────────────────────────────────────────────────────
    /// Module slugs to enable on the new site
    pub enabled_modules: Vec<String>,

    // ── Theme ──────────────────────────────────────────────────────────
    /// Full ThemeStudio profile JSON (first preset's profile, or legacy fallback)
    pub theme_profile: Option<Value>,
    /// Nav style from the first preset ("card_grid_mega", "horizontal", etc.)
    pub nav_style: String,
    /// Human-readable name of the chosen preset
    pub theme_preset_name: String,

    // ── Brand ──────────────────────────────────────────────────────────
    pub brand_primary: String,
    pub brand_secondary: String,
    pub brand_accent: String,
    pub default_tone: String,

    // ── Navigation ─────────────────────────────────────────────────────
    /// Flat nav items (children preserved inside each item's own `children` field)
    pub default_nav_items: Vec<NavItem>,

    // ── Pages ──────────────────────────────────────────────────────────
    pub default_pages: Vec<DefaultPage>,

    // ── Homepage ───────────────────────────────────────────────────────
    pub homepage_blocks: Option<Value>,

    // ── Pricing / Tier ─────────────────────────────────────────────────
    pub default_tier: String,
    pub always_free: bool,
    /// Price override in cents (0 = use standard tier pricing)
    pub price_override_cents: i64,

    // ── SEO Defaults ───────────────────────────────────────────────────
    pub seo_title_template: String,
    pub seo_description_template: String,

    // ── Onboarding ─────────────────────────────────────────────────────
    pub onboarding_steps: Vec<OnboardingStep>,
}

/// Convert a `SiteTypeDefinition` into a `ProvisionPayload`.
///
/// Theme resolution order:
/// 1. First `ThemePreset` in `theme_presets` (display_order == 1 preferred,
///    otherwise whichever comes first).
/// 2. Legacy `theme_profile` field.
/// 3. `None` (theme will be set to platform default at provision time).
pub fn extract(def: &SiteTypeDefinition) -> ProvisionPayload {
    // Pick the best preset: prefer display_order == 1, else take first entry.
    let best_preset = def
        .theme_presets
        .iter()
        .min_by_key(|p| p.display_order)
        .or_else(|| def.theme_presets.first());

    let theme_profile: Option<Value> = best_preset
        .map(|p| p.profile.clone())
        .or_else(|| def.theme_profile.clone());

    let nav_style = best_preset.map(|p| p.nav_style.clone()).unwrap_or_default();

    let theme_preset_name = best_preset.map(|p| p.name.clone()).unwrap_or_default();

    let BrandColors {
        primary,
        secondary,
        accent,
    } = def.default_brand_colors.clone();

    ProvisionPayload {
        slug: def.slug.clone(),
        name: def.name.clone(),
        emoji: def.emoji.clone(),
        category: def.category.clone(),
        description: def.description.clone(),
        default_tagline: def.default_tagline.clone(),
        enabled_modules: def.enabled_modules.clone(),
        theme_profile,
        nav_style,
        theme_preset_name,
        brand_primary: primary,
        brand_secondary: secondary,
        brand_accent: accent,
        default_tone: def.default_tone.clone(),
        default_nav_items: def.default_nav_items.clone(),
        default_pages: def.default_pages.clone(),
        homepage_blocks: def.homepage_blocks.clone(),
        default_tier: def.default_tier.clone(),
        always_free: def.always_free,
        price_override_cents: def.price_override_cents,
        seo_title_template: def.seo_title_template.clone(),
        seo_description_template: def.seo_description_template.clone(),
        onboarding_steps: def.onboarding_steps.clone(),
    }
}
