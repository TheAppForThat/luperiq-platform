//! Theme preset generator — 8 visual styles applied to any industry color palette.
//!
//! Style 1-2 use Card Grid Mega nav (best for desktop + mobile).
//! Styles 3-8 use other nav layouts for variety.

use super::types::ThemePreset;

/// Industry color palette — the raw materials for generating presets.
pub struct Palette {
    pub primary: &'static str,
    pub accent: &'static str,
    pub dark: &'static str,
    pub light_bg: &'static str,
    pub light_text: &'static str,
    pub dark_text: &'static str,
}

/// Map an industry slug to its content-hashed hero image path.
pub fn hero_image_for(slug: &str) -> Option<&'static str> {
    match slug {
        "pest-control" => Some("/static/img/heroes/hero_pest-control_a3f6871d.png"),
        "hvac" => Some("/static/img/heroes/hero_hvac_4638e37a.png"),
        "plumbing" => Some("/static/img/heroes/hero_plumbing_0030deb2.png"),
        "electrical" => Some("/static/img/heroes/hero_electrical_69eef758.png"),
        "landscaping" => Some("/static/img/heroes/hero_landscaping_4b2a403c.png"),
        "restaurant" => Some("/static/img/heroes/hero_restaurant_bdb413ee.png"),
        "bakery" => Some("/static/img/heroes/hero_bakery_5324799f.png"),
        "coffee-shop" => Some("/static/img/heroes/hero_coffee-shop_c5c06a2d.png"),
        "salon" => Some("/static/img/heroes/hero_salon_82e18bb2.png"),
        "artisan-market" => Some("/static/img/heroes/hero_artisan-market_a2644cdb.png"),
        "cell-phone-repair" => Some("/static/img/heroes/hero_cell-phone-repair_6df6e802.png"),
        "electronics-repair" => Some("/static/img/heroes/hero_electronics-repair_b56e5bf8.png"),
        "auto-repair" => Some("/static/img/heroes/hero_auto-repair_5e493552.png"),
        "medical-office" | "medical" => Some("/static/img/heroes/hero_medical-office_a0d6bb9f.png"),
        "app-publisher" => Some("/static/img/heroes/hero_app-publisher_dd04a01b.png"),
        "church" => Some("/static/img/heroes/hero_church_4ee6ad68.png"),
        "small-group" => Some("/static/img/heroes/hero_small-group_7ef13db2.png"),
        "mission-team" => Some("/static/img/heroes/hero_mission-team_13ffbd2e.png"),
        "homeschool-coop" => Some("/static/img/heroes/hero_homeschool-coop_53eb2fb7.png"),
        "business" | "business-team" => Some("/static/img/heroes/hero_business_f3099eed.png"),
        "reunion" => Some("/static/img/heroes/hero_reunion_267f5fe6.png"),
        "memorial" => Some("/static/img/heroes/hero_memorial_89b15d87.png"),
        _ => None,
    }
}

/// Build a full theme profile with all playground-controllable fields.
fn profile(
    primary: &str,
    accent: &str,
    hdr_bg: &str,
    hdr_txt: &str,
    bg: &str,
    txt: &str,
    font: &str,
    radius: u32,
    sticky: bool,
    nav_link_style: &str,
    header_style: &str,
    footer_style: &str,
    nav_layout: &str,
    hero_style: &str,
    hero_image: Option<&str>,
) -> serde_json::Value {
    let hero = if let Some(img) = hero_image {
        serde_json::json!({ "style": hero_style, "image": img })
    } else {
        serde_json::json!({ "style": hero_style })
    };
    serde_json::json!({
        "tokens": {
            "primary": primary, "accent": accent, "link": primary,
            "button_text": "#ffffff", "header_bg": hdr_bg, "header_text": hdr_txt,
            "background": bg, "surface": "#ffffff", "text": txt,
            "radius": radius, "container": 1100, "brand_size": 36, "nav_size": 15,
            "body_size": 16, "body_font": font, "full_bleed": true
        },
        "header": { "enabled": true, "sticky": sticky, "style": header_style },
        "footer": { "enabled": true, "style": footer_style },
        "nav_link_style": nav_link_style,
        "nav_layout": nav_layout,
        "hero": hero
    })
}

/// Generate 8 theme presets from a color palette and industry slug.
///
/// Each preset controls: colors, fonts, radius, header/footer style,
/// nav link style, navigation layout, and hero config — everything in the Design Playground.
///
/// Hero style distribution:
///   Presets 1-3 (display_order 1-3): "image-bottom-fade" with hero image
///   Presets 4-5 (display_order 4-5): "image-brand-tint" with hero image
///   Presets 6-7 (display_order 6-7): "gradient" with no image
///   Preset  8   (display_order 8):   "solid" with no image
pub fn generate_presets(p: &Palette, industry: &str) -> Vec<ThemePreset> {
    let img = hero_image_for(industry);
    //                                  primary    accent     hdr_bg    hdr_txt    bg         txt       font                    rad  sticky nav_links  hdr_style  ftr_style  nav_layout           hero_style           hero_image
    vec![
        // 1. Card Grid Mega — Clean & Professional (DEFAULT)
        ThemePreset {
            id: "card-grid-clean".into(),
            name: "Card Grid — Clean".into(),
            description: "Card Grid Mega navigation with a clean, professional look. Great on desktop and mobile.".into(),
            profile: profile(p.primary, p.accent, "#ffffff",  p.dark_text, "#f8fafc", p.dark_text, "Geometric", 12, true,  "flat",      "trust-forward",  "trust-forward",  "card_grid_mega", "image-bottom-fade", img),
            nav_style: "card_grid_mega".into(),
            display_order: 1,
        },
        // 2. Card Grid Mega — Dark Header
        ThemePreset {
            id: "card-grid-dark".into(),
            name: "Card Grid — Bold".into(),
            description: "Card Grid Mega with a dark header and bold accent colors.".into(),
            profile: profile(p.primary, p.accent, p.dark,     p.light_text, p.light_bg, p.dark_text, "System",     8,  true,  "flat",      "modern-edge",    "modern-edge",    "card_grid_mega", "image-bottom-fade", img),
            nav_style: "card_grid_mega".into(),
            display_order: 2,
        },
        // 3. Horizontal — Modern Minimal
        ThemePreset {
            id: "modern-minimal".into(),
            name: "Modern Minimal".into(),
            description: "Clean horizontal nav with lots of white space and subtle colors.".into(),
            profile: profile(p.primary, p.accent, "#ffffff",  p.dark_text, "#ffffff",  p.dark_text, "System",     4,  false, "underline", "clean-slate",    "clean-slate",    "horizontal",     "image-bottom-fade", img),
            nav_style: "horizontal".into(),
            display_order: 3,
        },
        // 4. Horizontal — Warm & Friendly
        ThemePreset {
            id: "warm-friendly".into(),
            name: "Warm & Friendly".into(),
            description: "Inviting warm tones with rounded corners and friendly typography.".into(),
            profile: profile(p.primary, p.accent, p.light_bg, p.dark_text, p.light_bg, p.dark_text, "Humanist",          20, true,  "pill",      "friendly-local", "friendly-local", "horizontal",     "image-brand-tint",  img),
            nav_style: "horizontal".into(),
            display_order: 4,
        },
        // 5. Full-Width Mega — Elegant
        ThemePreset {
            id: "elegant".into(),
            name: "Elegant".into(),
            description: "Sophisticated serif typography with full-width mega menu.".into(),
            profile: profile(p.primary, p.accent, p.dark,     p.light_text, "#fafaf9",  p.dark_text, "OldStyle",            2,  false, "flat",      "earth-guard",    "earth-guard",    "full_width_mega", "image-brand-tint", img),
            nav_style: "full_width_mega".into(),
            display_order: 5,
        },
        // 6. Side Drawer — Dark Mode
        ThemePreset {
            id: "dark-mode".into(),
            name: "Dark Mode".into(),
            description: "Dark background with slide-in drawer navigation and glowing accents.".into(),
            profile: profile(p.accent,  p.primary, "#0f172a",  "#e2e8f0",   "#0f172a",  "#e2e8f0",  "System",     10, true,  "flat",      "modern-edge",    "modern-edge",    "side_drawer",    "gradient",          None),
            nav_style: "side_drawer".into(),
            display_order: 6,
        },
        // 7. Two-Row — Compact Pro
        ThemePreset {
            id: "two-row-compact".into(),
            name: "Two-Row Nav".into(),
            description: "Category tabs on top, sub-items below. Compact and information-dense.".into(),
            profile: profile(p.primary, p.accent, p.dark,     p.light_text, "#f1f5f9",  p.dark_text, "Geometric", 8, true,  "flat",      "trust-forward",  "trust-forward",  "two_row",        "gradient",          None),
            nav_style: "two_row".into(),
            display_order: 7,
        },
        // 8. Card Grid Mega — Industry Branded
        ThemePreset {
            id: "card-grid-branded".into(),
            name: "Card Grid — Branded".into(),
            description: "Full brand color saturation with Card Grid Mega navigation.".into(),
            profile: profile(p.primary, p.accent, p.primary, "#ffffff", p.light_bg, p.dark_text, "Transitional", 12, true, "pill", "friendly-local", "friendly-local", "card_grid_mega", "solid",           None),
            nav_style: "card_grid_mega".into(),
            display_order: 8,
        },
    ]
}

// ── Industry Palettes ─────────────────────────────────────────────

pub fn palette_for(slug: &str) -> Palette {
    match slug {
        "family" => Palette {
            primary: "#16a34a",
            accent: "#f59e0b",
            dark: "#14532d",
            light_bg: "#fffbeb",
            light_text: "#ffffff",
            dark_text: "#422006",
        },
        "creator" => Palette {
            primary: "#8b5cf6",
            accent: "#f472b6",
            dark: "#3b0764",
            light_bg: "#faf5ff",
            light_text: "#f5f3ff",
            dark_text: "#1e1b4b",
        },
        "blog" => Palette {
            primary: "#0f172a",
            accent: "#3b82f6",
            dark: "#020617",
            light_bg: "#f8fafc",
            light_text: "#e2e8f0",
            dark_text: "#0f172a",
        },
        "pest-control" => Palette {
            primary: "#16a34a",
            accent: "#22c55e",
            dark: "#14532d",
            light_bg: "#f0fdf4",
            light_text: "#ffffff",
            dark_text: "#14532d",
        },
        "hvac" => Palette {
            primary: "#0284c7",
            accent: "#38bdf8",
            dark: "#0c4a6e",
            light_bg: "#f0f9ff",
            light_text: "#ffffff",
            dark_text: "#0c4a6e",
        },
        "plumbing" => Palette {
            primary: "#2563eb",
            accent: "#60a5fa",
            dark: "#1e3a5f",
            light_bg: "#eff6ff",
            light_text: "#ffffff",
            dark_text: "#1e3a5f",
        },
        "electrical" => Palette {
            primary: "#ca8a04",
            accent: "#facc15",
            dark: "#1c1917",
            light_bg: "#fffbeb",
            light_text: "#fef3c7",
            dark_text: "#1c1917",
        },
        "landscaping" => Palette {
            primary: "#15803d",
            accent: "#4ade80",
            dark: "#052e16",
            light_bg: "#f0fdf4",
            light_text: "#dcfce7",
            dark_text: "#052e16",
        },
        "restaurant" => Palette {
            primary: "#b91c1c",
            accent: "#f87171",
            dark: "#1a0a0a",
            light_bg: "#fef2f2",
            light_text: "#fecaca",
            dark_text: "#1a0a0a",
        },
        "bakery" => Palette {
            primary: "#b45309",
            accent: "#fbbf24",
            dark: "#451a03",
            light_bg: "#fffbeb",
            light_text: "#fef3c7",
            dark_text: "#451a03",
        },
        "coffee-shop" => Palette {
            primary: "#78350f",
            accent: "#d97706",
            dark: "#1c0f05",
            light_bg: "#fefce8",
            light_text: "#fde68a",
            dark_text: "#1c0f05",
        },
        "salon" => Palette {
            primary: "#be185d",
            accent: "#f472b6",
            dark: "#500724",
            light_bg: "#fdf2f8",
            light_text: "#fce7f3",
            dark_text: "#500724",
        },
        "cell-phone-repair" => Palette {
            primary: "#6d28d9",
            accent: "#a78bfa",
            dark: "#1e1b4b",
            light_bg: "#eef2ff",
            light_text: "#e0e7ff",
            dark_text: "#1e1b4b",
        },
        "artisan-market" => Palette {
            primary: "#0d9488",
            accent: "#2dd4bf",
            dark: "#042f2e",
            light_bg: "#f0fdfa",
            light_text: "#ccfbf1",
            dark_text: "#042f2e",
        },
        "electronics-repair" => Palette {
            primary: "#059669",
            accent: "#34d399",
            dark: "#022c22",
            light_bg: "#ecfdf5",
            light_text: "#d1fae5",
            dark_text: "#022c22",
        },
        "auto-repair" => Palette {
            primary: "#4f46e5",
            accent: "#818cf8",
            dark: "#1e1b4b",
            light_bg: "#eef2ff",
            light_text: "#e0e7ff",
            dark_text: "#1e1b4b",
        },
        "app-publisher" => Palette {
            primary: "#4338ca",
            accent: "#a5b4fc",
            dark: "#0f0d2e",
            light_bg: "#eef2ff",
            light_text: "#e0e7ff",
            dark_text: "#0f0d2e",
        },
        "medical-office" | "medical" => Palette {
            primary: "#0e7490",
            accent: "#22d3ee",
            dark: "#083344",
            light_bg: "#f0fdfa",
            light_text: "#ecfeff",
            dark_text: "#083344",
        },
        "church" => Palette {
            primary: "#1e3a5f",
            accent: "#b45309",
            dark: "#0f1d30",
            light_bg: "#fef3c7",
            light_text: "#fef3c7",
            dark_text: "#1e3a5f",
        },
        // ── Group / Free Types ──────────────────────────────────────
        "band" => Palette {
            primary: "#a855f7",
            accent: "#ec4899",
            dark: "#3b0764",
            light_bg: "#faf5ff",
            light_text: "#f5f3ff",
            dark_text: "#2e1065",
        },
        "roommates" => Palette {
            primary: "#14b8a6",
            accent: "#2dd4bf",
            dark: "#042f2e",
            light_bg: "#f0fdfa",
            light_text: "#ccfbf1",
            dark_text: "#042f2e",
        },
        "classroom" => Palette {
            primary: "#ea580c",
            accent: "#f97316",
            dark: "#431407",
            light_bg: "#fff7ed",
            light_text: "#ffedd5",
            dark_text: "#431407",
        },
        "homeschool" => Palette {
            primary: "#6366f1",
            accent: "#818cf8",
            dark: "#1e1b4b",
            light_bg: "#eef2ff",
            light_text: "#e0e7ff",
            dark_text: "#1e1b4b",
        },
        "sports-team" => Palette {
            primary: "#dc2626",
            accent: "#ef4444",
            dark: "#450a0a",
            light_bg: "#fef2f2",
            light_text: "#fecaca",
            dark_text: "#450a0a",
        },
        "club" => Palette {
            primary: "#16a34a",
            accent: "#4ade80",
            dark: "#052e16",
            light_bg: "#f0fdf4",
            light_text: "#dcfce7",
            dark_text: "#052e16",
        },
        "book-club" => Palette {
            primary: "#92400e",
            accent: "#d97706",
            dark: "#451a03",
            light_bg: "#fffbeb",
            light_text: "#fef3c7",
            dark_text: "#451a03",
        },
        "nonprofit" => Palette {
            primary: "#10b981",
            accent: "#34d399",
            dark: "#022c22",
            light_bg: "#ecfdf5",
            light_text: "#d1fae5",
            dark_text: "#022c22",
        },
        "neighborhood" => Palette {
            primary: "#84cc16",
            accent: "#a3e635",
            dark: "#1a2e05",
            light_bg: "#f7fee7",
            light_text: "#ecfccb",
            dark_text: "#1a2e05",
        },
        "travel" => Palette {
            primary: "#0ea5e9",
            accent: "#38bdf8",
            dark: "#0c4a6e",
            light_bg: "#f0f9ff",
            light_text: "#e0f2fe",
            dark_text: "#0c4a6e",
        },
        "elder-care" => Palette {
            primary: "#ec4899",
            accent: "#f472b6",
            dark: "#500724",
            light_bg: "#fdf2f8",
            light_text: "#fce7f3",
            dark_text: "#500724",
        },
        "wedding" => Palette {
            primary: "#f43f5e",
            accent: "#fb7185",
            dark: "#4c0519",
            light_bg: "#fff1f2",
            light_text: "#ffe4e6",
            dark_text: "#4c0519",
        },
        "pet-owners" => Palette {
            primary: "#f97316",
            accent: "#fb923c",
            dark: "#431407",
            light_bg: "#fff7ed",
            light_text: "#ffedd5",
            dark_text: "#431407",
        },
        "scouts" => Palette {
            primary: "#059669",
            accent: "#34d399",
            dark: "#022c22",
            light_bg: "#ecfdf5",
            light_text: "#d1fae5",
            dark_text: "#022c22",
        },
        "fitness" => Palette {
            primary: "#dc2626",
            accent: "#f87171",
            dark: "#450a0a",
            light_bg: "#fef2f2",
            light_text: "#fecaca",
            dark_text: "#450a0a",
        },
        "farm" => Palette {
            primary: "#65a30d",
            accent: "#84cc16",
            dark: "#1a2e05",
            light_bg: "#f7fee7",
            light_text: "#ecfccb",
            dark_text: "#1a2e05",
        },
        "support-group" => Palette {
            primary: "#8b5cf6",
            accent: "#a78bfa",
            dark: "#2e1065",
            light_bg: "#faf5ff",
            light_text: "#f5f3ff",
            dark_text: "#2e1065",
        },
        "maker-space" => Palette {
            primary: "#d97706",
            accent: "#fbbf24",
            dark: "#451a03",
            light_bg: "#fffbeb",
            light_text: "#fef3c7",
            dark_text: "#451a03",
        },
        // Intimate Bible study — leather journal warmth, living room safety
        "small-group" => Palette {
            primary: "#2C3E6B",
            accent: "#C9A96E",
            dark: "#1a2440",
            light_bg: "#F5F0E8",
            light_text: "#F5F0E8",
            dark_text: "#2C3E6B",
        },
        // Mission trips — bold purpose, field adventure, sunset energy
        "mission-team" => Palette {
            primary: "#1A6B5C",
            accent: "#E07A3A",
            dark: "#0d3a30",
            light_bg: "#FAF8F5",
            light_text: "#FAF8F5",
            dark_text: "#1A3A30",
        },
        // Multi-family co-op — cheerful classroom, collaborative spirit
        "homeschool-coop" => Palette {
            primary: "#4A90B8",
            accent: "#F2C94C",
            dark: "#1e3d4f",
            light_bg: "#F7FBFE",
            light_text: "#E8F4FD",
            dark_text: "#1e3d4f",
        },
        // Small work team — calm productivity, focused minimal
        "business" | "business-team" => Palette {
            primary: "#3B5998",
            accent: "#00C853",
            dark: "#1A1A2E",
            light_bg: "#F7F8FA",
            light_text: "#E8EAF0",
            dark_text: "#1A1A2E",
        },
        // Reunion — nostalgic warmth, scrapbook feel, coming home
        "reunion" => Palette {
            primary: "#722F37",
            accent: "#D4A843",
            dark: "#3a1820",
            light_bg: "#FFF8F0",
            light_text: "#FFF0E0",
            dark_text: "#3a1820",
        },
        // Memorial — quiet dignity, candlelight, gentle permanence
        "memorial" => Palette {
            primary: "#1B2A4A",
            accent: "#9B8EC4",
            dark: "#0e1627",
            light_bg: "#F8F6F0",
            light_text: "#F0EDE6",
            dark_text: "#1B2A4A",
        },
        _ => Palette {
            primary: "#2563eb",
            accent: "#3b82f6",
            dark: "#1e293b",
            light_bg: "#f8fafc",
            light_text: "#e2e8f0",
            dark_text: "#1e293b",
        },
    }
}
