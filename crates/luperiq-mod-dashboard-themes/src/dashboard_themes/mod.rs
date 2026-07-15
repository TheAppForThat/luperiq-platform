//! Admin Dashboard Themes Module — panel appearance, color presets, and customization.
//!
//! Owns the admin panel's visual theme system: 14 named presets (Texas towns
//! and family), custom theme builder, AI theme generation, and export/import.
//!
//! This is the admin panel's own appearance — completely separate from
//! Theme Studio which controls the public site's design tokens.

pub mod admin_js;

use serde::Serialize;

use luperiq_module_api::{AdminView, AppContext, CmsModule};

#[derive(Debug, Clone, Serialize)]
pub struct DashboardTheme {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    #[serde(skip)]
    pub css_vars: &'static str,
    /// Whether this is a light theme (affects sidebar/nav styling).
    pub light: bool,
}

pub const THEMES: &[DashboardTheme] = &[
    DashboardTheme {
        id: "azle",
        name: "Azle",
        description: "Clean dark with blue accents. The default look.",
        light: false,
        css_vars: "--bg: #0f172a; --surface: #1e293b; --surface2: #273548; --border: #334155; --text: #e2e8f0; --text-muted: #94a3b8; --accent: #3b82f6; --accent-hover: #2563eb; --success: #22c55e; --danger: #ef4444; --warning: #f59e0b",
    },
    DashboardTheme {
        id: "granbury",
        name: "Granbury",
        description: "Warm earth tones with amber and copper. Sunset at the town square.",
        light: false,
        css_vars: "--bg: #1a1410; --surface: #2a2218; --surface2: #342c20; --border: #3d3428; --text: #f5e6d3; --text-muted: #a89280; --accent: #d97706; --accent-hover: #b45309; --success: #65a30d; --danger: #dc2626; --warning: #eab308",
    },
    DashboardTheme {
        id: "mineral-wells",
        name: "Mineral Wells",
        description: "Deep teal and mineral greens. Spa-like calm.",
        light: false,
        css_vars: "--bg: #0a1a1a; --surface: #132828; --surface2: #1a3535; --border: #1e3d3d; --text: #d1f0ed; --text-muted: #6bb3aa; --accent: #0d9488; --accent-hover: #0f766e; --success: #22c55e; --danger: #f43f5e; --warning: #fbbf24",
    },
    DashboardTheme {
        id: "jacksboro",
        name: "Jacksboro",
        description: "Bold western style. Dark leather with gold accents.",
        light: false,
        css_vars: "--bg: #1c1611; --surface: #2e2419; --surface2: #3a2e20; --border: #4a3b28; --text: #f0e4d4; --text-muted: #9c8b72; --accent: #d4a437; --accent-hover: #b8902c; --success: #4ade80; --danger: #ef4444; --warning: #fb923c",
    },
    DashboardTheme {
        id: "stephenville",
        name: "Stephenville",
        description: "College town energy. Vibrant purple and gold, Tarleton colors.",
        light: false,
        css_vars: "--bg: #1a0f24; --surface: #261a34; --surface2: #30223f; --border: #3d2b50; --text: #ede5f7; --text-muted: #a78dc4; --accent: #a855f7; --accent-hover: #9333ea; --success: #4ade80; --danger: #f43f5e; --warning: #fcd34d",
    },
    DashboardTheme {
        id: "abilene",
        name: "Abilene",
        description: "Wide open sky. Light backgrounds, airy. West Texas brightness.",
        light: true,
        css_vars: "--bg: #f0f4f8; --surface: #ffffff; --surface2: #e8edf3; --border: #d1d8e0; --text: #1e293b; --text-muted: #64748b; --accent: #2563eb; --accent-hover: #1d4ed8; --success: #16a34a; --danger: #dc2626; --warning: #d97706",
    },
    DashboardTheme {
        id: "fort-worth",
        name: "Fort Worth",
        description: "Stockyards grit. Deep burgundy and charcoal, industrial steel.",
        light: false,
        css_vars: "--bg: #1a1118; --surface: #2a1f28; --surface2: #342838; --border: #3e2e3a; --text: #f0e0ea; --text-muted: #a07090; --accent: #be185d; --accent-hover: #9d174d; --success: #22c55e; --danger: #ef4444; --warning: #f59e0b",
    },
    DashboardTheme {
        id: "willie-jim",
        name: "Willie Jim",
        description: "Floral garden. Soft sage greens with rose gold warmth.",
        light: false,
        css_vars: "--bg: #141a14; --surface: #1e281e; --surface2: #263026; --border: #2e3e2e; --text: #e8f0e4; --text-muted: #88a080; --accent: #b5838d; --accent-hover: #9d6b75; --success: #4ade80; --danger: #f87171; --warning: #fbbf24",
    },
    DashboardTheme {
        id: "eugene",
        name: "Eugene",
        description: "Workshop classic. Navy blue with burnt orange tools, sturdy.",
        light: false,
        css_vars: "--bg: #0c1222; --surface: #162033; --surface2: #1c2a40; --border: #243448; --text: #dce6f2; --text-muted: #7898b8; --accent: #ea580c; --accent-hover: #c2410c; --success: #22c55e; --danger: #ef4444; --warning: #f59e0b",
    },
    DashboardTheme {
        id: "grace",
        name: "Grace",
        description: "Elegant and refined. Cream and pearl with soft gold accents.",
        light: true,
        css_vars: "--bg: #faf8f5; --surface: #ffffff; --surface2: #f0ebe3; --border: #e8e0d4; --text: #2c2418; --text-muted: #8c7e6e; --accent: #a67c52; --accent-hover: #8b6542; --success: #15803d; --danger: #b91c1c; --warning: #a16207",
    },
    DashboardTheme {
        id: "erin",
        name: "Erin",
        description: "Bright and energetic. Coral and teal combo, playful and modern.",
        light: false,
        css_vars: "--bg: #0f1720; --surface: #1a2430; --surface2: #22303e; --border: #2a3848; --text: #e8f0f8; --text-muted: #80a0b8; --accent: #f97316; --accent-hover: #ea580c; --success: #06b6d4; --danger: #f43f5e; --warning: #fbbf24",
    },
    DashboardTheme {
        id: "langley",
        name: "Langley",
        description: "Country road. Forest green with warm tan, natural stone.",
        light: false,
        css_vars: "--bg: #121810; --surface: #1c2618; --surface2: #243020; --border: #2e3c28; --text: #e4ece0; --text-muted: #88a880; --accent: #16a34a; --accent-hover: #15803d; --success: #4ade80; --danger: #ef4444; --warning: #fbbf24",
    },
    DashboardTheme {
        id: "south-padre-sunrise",
        name: "South Padre Sunrise",
        description: "Warm seashell and sand. Coral sunrise over the Gulf, breezy and bright.",
        light: true,
        css_vars: "--bg: #FFF5EE; --surface: #ffffff; --surface2: #FAEBD7; --border: #E8D5C4; --text: #3E2723; --text-muted: #8D6E63; --accent: #E07040; --accent-hover: #C85A30; --success: #2E7D32; --danger: #C62828; --warning: #E65100",
    },
    DashboardTheme {
        id: "bella-vista",
        name: "Bella Vista",
        description: "Italian elegance. Deep wine and burgundy with warm gold, candlelit ambiance.",
        light: false,
        css_vars: "--bg: #1a0e0e; --surface: #2a1818; --surface2: #342020; --border: #4a2828; --text: #f5e8e0; --text-muted: #b89080; --accent: #c4944a; --accent-hover: #a87d3a; --success: #4ade80; --danger: #ef4444; --warning: #f59e0b",
    },
];

/// Look up a theme by ID.
pub fn get_theme(id: &str) -> Option<&'static DashboardTheme> {
    THEMES.iter().find(|t| t.id == id)
}

/// Wrap raw CSS variable declarations in a `:root { ... }` block.
fn css_root(vars: &str) -> String {
    format!(":root {{ {} }}", vars)
}

/// Generate CSS for a theme (`:root { ... }`). Returns empty string for unknown themes.
pub fn theme_css(theme_id: &str) -> String {
    if let Some(theme) = get_theme(theme_id) {
        css_root(theme.css_vars)
    } else {
        String::new()
    }
}

/// Return all themes as a JSON-serializable list (includes CSS vars as a separate field).
pub fn themes_json() -> Vec<ThemeInfo> {
    THEMES
        .iter()
        .map(|t| ThemeInfo {
            id: t.id.to_string(),
            name: t.name.to_string(),
            description: t.description.to_string(),
            light: t.light,
            css: css_root(t.css_vars),
            accent: extract_var(t.css_vars, "--accent"),
            bg: extract_var(t.css_vars, "--bg"),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub light: bool,
    pub css: String,
    pub accent: String,
    pub bg: String,
}

/// Extract a CSS variable value from a semicolon-separated vars string.
///
/// Returns an empty string if the variable is not found. Whitespace-only
/// segments (e.g. trailing `;`) are skipped rather than producing empty
/// matches, and segments with no `:` separator are also skipped safely.
fn extract_var(vars: &str, name: &str) -> String {
    for part in vars.split(';').filter(|p| !p.trim().is_empty()) {
        let part = part.trim();
        if part.starts_with(name) {
            let mut cols = part.splitn(2, ':');
            let _ = cols.next(); // discard the key side
            if let Some(val) = cols.next() {
                return val.trim().to_string();
            }
        }
    }
    String::new()
}

pub struct DashboardThemesModule;

impl CmsModule for DashboardThemesModule {
    fn slug(&self) -> &str {
        "dashboard-themes"
    }
    fn name(&self) -> &str {
        "Dashboard Themes"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Admin panel theme presets, custom theme builder, and AI theme generation."
    }
    fn category(&self) -> &str {
        "Platform"
    }
    fn routes(&self, _ctx: &AppContext) -> Option<axum::Router> {
        None
    }
    fn admin_views(&self) -> Vec<AdminView> {
        vec![]
    }
    fn admin_js(&self) -> Option<String> {
        Some(admin_js::theme_system_js())
    }
}
