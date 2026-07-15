// luperiq-cms/src/modules/theme_studio/config.rs
//! Data structures for the Theme Studio module.
//!
//! All types use serde for JSON serialization via ForgeJournal.
//! The Block system uses a wrapper struct with common fields +
//! a BlockType enum (tagged union) for type-specific data.

use serde::{Deserialize, Serialize};

// ── Aggregate key constants ─────────────────────────────────────────

pub const AGG_META: &str = "ThemeStudio:Meta";
pub const AGG_PROFILE: &str = "ThemeStudio:Profile";
pub const AGG_POPUP: &str = "ThemeStudio:Popup";
pub const AGG_SCHEDULE: &str = "ThemeStudio:Schedule";
pub const AGG_NAV_MENU: &str = "ThemeStudio:NavMenu";
pub const AGG_TEMPLATE: &str = "ThemeStudio:Template";
pub const AGG_BLOCK_PRESET: &str = "ThemeStudio:BlockPreset";
pub const AGG_PAGE_LAYOUT: &str = "ThemeStudio:PageLayout";
pub const AGG_REVISION: &str = "ThemeStudio:Revision";
pub const AGG_PRESET: &str = "TS:Preset";
pub const AGG_SAVED_DESIGN: &str = "ThemeStudio:SavedDesign";
pub const AGG_SCOPE_STYLE: &str = "ThemeStudio:ScopeStyle";
pub const TOMBSTONE: &[u8] = b"__TS_DELETED__";

// ── Meta ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeStudioMeta {
    pub active_profile: String,
    #[serde(default)]
    pub active_by_theme: std::collections::HashMap<String, String>,
}

// ── Profile ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub label: String,
    #[serde(default)]
    pub status: ProfileStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_industry: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_header_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_footer_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter_ai_prompt_hint: Option<String>,
    pub tokens: DesignTokens,
    #[serde(default)]
    pub branding: Branding,
    #[serde(default)]
    pub header: HeaderConfig,
    #[serde(default)]
    pub footer: FooterConfig,
    #[serde(default)]
    pub sidebars: SidebarConfig,
    #[serde(default)]
    pub floating_guide: FloatingGuideConfig,
    #[serde(default)]
    pub layouts: LayoutRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_theme_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ProfileStatus {
    #[default]
    #[serde(alias = "active")]
    Active,
    #[serde(alias = "archived")]
    Archived,
}

// ── Design Tokens ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignTokens {
    // Colors
    #[serde(default = "default_primary")]
    pub primary: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default = "default_link")]
    pub link: String,
    #[serde(default = "default_button_text")]
    pub button_text: String,
    #[serde(default = "default_header_bg")]
    pub header_bg: String,
    #[serde(default = "default_header_text")]
    pub header_text: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_surface")]
    pub surface: String,
    #[serde(default = "default_text")]
    pub text: String,
    // Numeric
    #[serde(default = "default_radius")]
    pub radius: u32,
    #[serde(default = "default_container")]
    pub container: u32,
    #[serde(default = "default_brand_size")]
    pub brand_size: u32,
    #[serde(default = "default_nav_size")]
    pub nav_size: u32,
    #[serde(default = "default_nav_gap")]
    pub nav_gap: u32,
    #[serde(default = "default_body_size")]
    pub body_size: u32,
    #[serde(default = "default_body_line_height")]
    pub body_line_height: u32,
    // Typography
    #[serde(default)]
    pub body_font: FontStack,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading_font: Option<FontStack>,
    #[serde(default = "default_heading_size")]
    pub heading_size: u32,
    // Layout
    #[serde(default = "default_true")]
    pub full_bleed: bool,
    // Custom CSS
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
    // Responsive
    #[serde(default)]
    pub tablet: Option<TokenOverrides>,
    #[serde(default)]
    pub mobile: Option<TokenOverrides>,
}

impl Default for DesignTokens {
    fn default() -> Self {
        Self {
            primary: default_primary(),
            accent: default_accent(),
            link: default_link(),
            button_text: default_button_text(),
            header_bg: default_header_bg(),
            header_text: default_header_text(),
            background: default_background(),
            surface: default_surface(),
            text: default_text(),
            radius: default_radius(),
            container: default_container(),
            brand_size: default_brand_size(),
            nav_size: default_nav_size(),
            nav_gap: default_nav_gap(),
            body_size: default_body_size(),
            body_line_height: default_body_line_height(),
            body_font: FontStack::default(),
            heading_font: None,
            heading_size: default_heading_size(),
            full_bleed: true,
            custom_css: None,
            tablet: None,
            mobile: None,
        }
    }
}

fn default_primary() -> String {
    "#0f1115".into()
}
fn default_accent() -> String {
    "#22c55e".into()
}
fn default_link() -> String {
    "#0b57d0".into()
}
fn default_button_text() -> String {
    "#ffffff".into()
}
fn default_header_bg() -> String {
    "#0f1115".into()
}
fn default_header_text() -> String {
    "#ffffff".into()
}
fn default_background() -> String {
    "#f6f7f8".into()
}
fn default_surface() -> String {
    "#ffffff".into()
}
fn default_text() -> String {
    "#111111".into()
}
fn default_radius() -> u32 {
    16
}
fn default_container() -> u32 {
    1100
}
fn default_brand_size() -> u32 {
    56
}
fn default_nav_size() -> u32 {
    16
}
fn default_nav_gap() -> u32 {
    16
}
fn default_heading_size() -> u32 { 32 }

fn default_body_size() -> u32 {
    16
}
fn default_body_line_height() -> u32 {
    16
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenOverrides {
    pub radius: Option<u32>,
    pub container: Option<u32>,
    pub brand_size: Option<u32>,
    pub nav_size: Option<u32>,
    pub nav_gap: Option<u32>,
    pub body_size: Option<u32>,
    pub body_line_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum FontStack {
    #[default]
    #[serde(alias = "system", alias = "system-ui", alias = "")]
    System,
    #[serde(alias = "humanist")]
    Humanist,
    #[serde(alias = "transitional")]
    Transitional,
    #[serde(alias = "old-style", alias = "oldstyle")]
    OldStyle,
    #[serde(alias = "geometric")]
    Geometric,
    #[serde(alias = "mono", alias = "monospace")]
    Mono,
    // Google Fonts — sans-serif
    #[serde(alias = "inter")]
    Inter,
    #[serde(alias = "roboto")]
    Roboto,
    #[serde(alias = "open-sans", alias = "opensans")]
    OpenSans,
    #[serde(alias = "lato")]
    Lato,
    #[serde(alias = "poppins")]
    Poppins,
    #[serde(alias = "nunito")]
    Nunito,
    // Google Fonts — serif
    #[serde(alias = "merriweather")]
    Merriweather,
    #[serde(alias = "playfair-display", alias = "playfairdisplay")]
    PlayfairDisplay,
    #[serde(alias = "lora")]
    Lora,
    // Google Fonts — display
    #[serde(alias = "montserrat")]
    Montserrat,
    #[serde(alias = "oswald")]
    Oswald,
    #[serde(alias = "raleway")]
    Raleway,
}

impl FontStack {
    pub fn css_value(&self) -> &'static str {
        match self {
            Self::System        => "system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
            Self::Humanist      => "Seravek, 'Gill Sans Nova', Ubuntu, Calibri, sans-serif",
            Self::Transitional  => "Charter, 'Bitstream Charter', 'Sitka Text', Cambria, serif",
            Self::OldStyle      => "'Iowan Old Style', 'Palatino Linotype', Palatino, serif",
            Self::Geometric     => "Avenir, Montserrat, Corbel, 'URW Gothic', sans-serif",
            Self::Mono          => "'Cascadia Code', 'Source Code Pro', Menlo, monospace",
            Self::Inter         => "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
            Self::Roboto        => "'Roboto', -apple-system, BlinkMacSystemFont, Arial, sans-serif",
            Self::OpenSans      => "'Open Sans', Arial, Helvetica, sans-serif",
            Self::Lato          => "'Lato', -apple-system, BlinkMacSystemFont, sans-serif",
            Self::Poppins       => "'Poppins', -apple-system, BlinkMacSystemFont, sans-serif",
            Self::Nunito        => "'Nunito', -apple-system, BlinkMacSystemFont, sans-serif",
            Self::Merriweather  => "'Merriweather', Georgia, 'Times New Roman', serif",
            Self::PlayfairDisplay => "'Playfair Display', Georgia, 'Times New Roman', serif",
            Self::Lora          => "'Lora', Georgia, 'Times New Roman', serif",
            Self::Montserrat    => "'Montserrat', 'Gill Sans', Optima, sans-serif",
            Self::Oswald        => "'Oswald', 'Arial Narrow', Gadget, sans-serif",
            Self::Raleway       => "'Raleway', 'Gill Sans', Optima, sans-serif",
        }
    }

    pub fn google_fonts_url(&self) -> Option<&'static str> {
        match self {
            Self::Inter         => Some("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap"),
            Self::Roboto        => Some("https://fonts.googleapis.com/css2?family=Roboto:wght@400;500;700&display=swap"),
            Self::OpenSans      => Some("https://fonts.googleapis.com/css2?family=Open+Sans:wght@400;600;700&display=swap"),
            Self::Lato          => Some("https://fonts.googleapis.com/css2?family=Lato:wght@400;700&display=swap"),
            Self::Poppins       => Some("https://fonts.googleapis.com/css2?family=Poppins:wght@400;500;600;700&display=swap"),
            Self::Nunito        => Some("https://fonts.googleapis.com/css2?family=Nunito:wght@400;600;700&display=swap"),
            Self::Merriweather  => Some("https://fonts.googleapis.com/css2?family=Merriweather:wght@400;700&display=swap"),
            Self::PlayfairDisplay => Some("https://fonts.googleapis.com/css2?family=Playfair+Display:wght@400;700&display=swap"),
            Self::Lora          => Some("https://fonts.googleapis.com/css2?family=Lora:wght@400;700&display=swap"),
            Self::Montserrat    => Some("https://fonts.googleapis.com/css2?family=Montserrat:wght@400;500;600;700&display=swap"),
            Self::Oswald        => Some("https://fonts.googleapis.com/css2?family=Oswald:wght@400;500;700&display=swap"),
            Self::Raleway       => Some("https://fonts.googleapis.com/css2?family=Raleway:wght@400;500;600;700&display=swap"),
            _ => None,
        }
    }
}

/// Clamp numeric token values to their valid ranges.
pub fn clamp_tokens(t: &mut DesignTokens) {
    t.radius = t.radius.clamp(0, 40);
    t.container = t.container.clamp(860, 1600);
    t.brand_size = t.brand_size.clamp(10, 80);
    t.nav_size = t.nav_size.clamp(10, 60);
    t.nav_gap = t.nav_gap.clamp(10, 60);
    t.body_size = t.body_size.clamp(12, 28);
    t.heading_size = t.heading_size.clamp(14, 80);
    t.body_line_height = t.body_line_height.clamp(10, 24);
}

fn parse_hex_color(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(hex, 16).ok()?;
    Some((
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ))
}

pub fn normalize_hex_color(color: &str) -> Option<String> {
    let (r, g, b) = parse_hex_color(color)?;
    Some(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn srgb_to_linear(channel: u8) -> f64 {
    let value = channel as f64 / 255.0;
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: &str) -> Option<f64> {
    let (r, g, b) = parse_hex_color(color)?;
    Some(0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b))
}

pub fn contrast_ratio(foreground: &str, background: &str) -> Option<f64> {
    let fg = relative_luminance(foreground)?;
    let bg = relative_luminance(background)?;
    let (lighter, darker) = if fg > bg { (fg, bg) } else { (bg, fg) };
    Some((lighter + 0.05) / (darker + 0.05))
}

pub fn best_button_text_color(background: &str) -> String {
    let dark = "#111111";
    let light = "#ffffff";
    let dark_ratio = contrast_ratio(dark, background).unwrap_or(0.0);
    let light_ratio = contrast_ratio(light, background).unwrap_or(0.0);
    if dark_ratio > light_ratio {
        dark.to_string()
    } else {
        light.to_string()
    }
}

pub fn accessible_button_text_color(background: &str, preferred: &str) -> String {
    let normalized = normalize_hex_color(preferred);
    if let Some(ref chosen) = normalized {
        if contrast_ratio(chosen, background).unwrap_or(0.0) >= 4.5 {
            return chosen.clone();
        }
    }
    best_button_text_color(background)
}

pub fn derive_hover_color(color: &str) -> String {
    let Some((r, g, b)) = parse_hex_color(color) else {
        return color.to_string();
    };
    let shade = |channel: u8| -> u8 { ((channel as f64) * 0.86).round().clamp(0.0, 255.0) as u8 };
    format!("#{:02x}{:02x}{:02x}", shade(r), shade(g), shade(b))
}

/// Produce a very dark version of the primary for footer backgrounds.
/// Keeps the hue but reduces brightness to ~20%, guaranteeing a deep tone
/// that pairs well with dark gradient heroes used on marketing pages.
pub fn derive_footer_bg(color: &str) -> String {
    let Some((r, g, b)) = parse_hex_color(color) else {
        return "#0f172a".to_string();
    };
    let dark = |ch: u8| -> u8 { ((ch as f64) * 0.20).round().clamp(5.0, 255.0) as u8 };
    format!("#{:02x}{:02x}{:02x}", dark(r), dark(g), dark(b))
}

// ── Branding ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Branding {
    #[serde(default)]
    pub logo_path: String,
    #[serde(default)]
    pub hero_path: String,
    #[serde(default)]
    pub favicon_path: String,
}

// ── Top Bar (Announcement Bar) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopBar {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default = "default_topbar_bg")]
    pub bg_color: String,
    #[serde(default = "default_topbar_text")]
    pub text_color: String,
    #[serde(default)]
    pub dismissible: bool,
    #[serde(default = "default_true")]
    pub hide_on_mobile: bool,
}

fn default_topbar_bg() -> String {
    "#1e40af".into()
}
fn default_topbar_text() -> String {
    "#ffffff".into()
}

impl Default for TopBar {
    fn default() -> Self {
        Self {
            enabled: false,
            text: String::new(),
            link: None,
            bg_color: default_topbar_bg(),
            text_color: default_topbar_text(),
            dismissible: true,
            hide_on_mobile: true,
        }
    }
}

// ── Header / Footer ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub sticky: bool,
    #[serde(default)]
    pub top_bar: TopBar,
    #[serde(default)]
    pub layout_builder: Vec<Row>,
    #[serde(default)]
    pub responsive: ResponsiveConfig,
}

impl Default for HeaderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sticky: false,
            top_bar: TopBar::default(),
            layout_builder: vec![],
            responsive: ResponsiveConfig {
                // Headers hide rotating text on mobile by default (matches old behavior)
                hidden_blocks: vec!["rotating_text".to_string()],
                // Headers center content on mobile by default
                center_content: true,
                ..ResponsiveConfig::default()
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FooterConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub layout_builder: Vec<Row>,
    #[serde(default)]
    pub sticky_bar: Option<StickyBar>,
    #[serde(default)]
    pub responsive: ResponsiveConfig,
}

impl Default for FooterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            layout_builder: vec![],
            sticky_bar: None,
            responsive: ResponsiveConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyBar {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub show_schedule: bool,
    #[serde(default)]
    pub show_portal: bool,
    #[serde(default)]
    pub show_call: bool,
    #[serde(default)]
    pub buttons: Vec<CtaButton>,
}

fn default_true() -> bool {
    true
}

fn default_rotate_token() -> String {
    "[[rotate]]".into()
}

fn default_rotate_interval_ms() -> u32 {
    2400
}

// ── Layout (rows -> columns -> blocks) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    #[serde(flatten)]
    pub block_type: BlockType,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default)]
    pub align: Align,
    // Common styling overrides (imported from WP, optional)
    #[serde(default)]
    pub bg_color: String,
    #[serde(default)]
    pub text_color: String,
    #[serde(default)]
    pub font_size: u32,
    #[serde(default)]
    pub padding: u32,
    #[serde(default)]
    pub border_radius: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Tone {
    #[default]
    #[serde(alias = "surface")]
    Surface,
    #[serde(alias = "accent")]
    Accent,
    #[serde(alias = "primary")]
    Primary,
    #[serde(alias = "muted")]
    Muted,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Align {
    #[default]
    #[serde(rename = "", alias = "none")]
    None,
    #[serde(alias = "left")]
    Left,
    #[serde(alias = "center")]
    Center,
    #[serde(alias = "right")]
    Right,
}

impl Tone {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Surface => "is-tone-surface",
            Self::Accent => "is-tone-accent",
            Self::Primary => "is-tone-primary",
            Self::Muted => "is-tone-muted",
        }
    }
}

impl Align {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::None => "",
            Self::Left => "is-align-left",
            Self::Center => "is-align-center",
            Self::Right => "is-align-right",
        }
    }
}

// ── Block Types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BlockType {
    // Navigation / Layout
    SiteBrand {
        #[serde(default)]
        name: String,
        #[serde(default)]
        subtitle: String,
        #[serde(default = "default_slash")]
        url: String,
        #[serde(default = "default_true")]
        show_logo: bool,
        #[serde(default = "default_true")]
        show_name: bool,
        #[serde(default)]
        show_subtitle: bool,
    },
    Nav {
        #[serde(default)]
        mode: NavMode,
        #[serde(default)]
        menu_location: String,
    },
    MegaNav {
        #[serde(default = "default_classic")]
        nav_style: String,
        #[serde(default)]
        mode: NavMode,
        #[serde(default)]
        menu_location: String,
        #[serde(default = "default_auto")]
        panel_columns: String,
        #[serde(default)]
        trigger: Trigger,
        #[serde(default = "default_full")]
        panel_width: String,
        #[serde(default = "default_3u8")]
        max_depth: u8,
        #[serde(default = "default_true")]
        show_descriptions: bool,
        #[serde(default)]
        panel_mode: PanelMode,
        #[serde(default)]
        desc_color: String,
        #[serde(default)]
        desc_font_size: u32,
        #[serde(default)]
        module_name_color: String,
    },
    NavToggle,
    CtaGroup {
        #[serde(default)]
        buttons: Vec<CtaButton>,
    },
    UserMenu,

    // Content
    Heading {
        #[serde(default)]
        text: String,
        #[serde(default = "default_2u8")]
        level: u8,
    },
    Paragraph {
        #[serde(default)]
        text: String,
    },
    Image {
        #[serde(default)]
        path: String,
        #[serde(default)]
        alt: String,
        #[serde(default)]
        max_width: Option<u32>,
    },
    Button {
        #[serde(default)]
        label: String,
        #[serde(default)]
        url: String,
        #[serde(default)]
        style: ButtonStyle,
    },
    Link {
        #[serde(default)]
        label: String,
        #[serde(default)]
        url: String,
    },
    Spacer {
        #[serde(default = "default_20u32")]
        height: u32,
    },
    Divider {
        #[serde(default)]
        variant: DividerVariant,
    },
    Icon {
        #[serde(default)]
        name: String,
    },
    AlertBox {
        #[serde(default)]
        text: String,
        #[serde(default)]
        variant: AlertVariant,
    },
    Quote {
        #[serde(default)]
        text: String,
        #[serde(default)]
        attribution: Option<String>,
    },
    Code {
        #[serde(default)]
        text: String,
    },
    Video {
        #[serde(default)]
        url: String,
    },
    Countdown {
        #[serde(default)]
        target: String,
    },
    ProgressBar {
        #[serde(default)]
        value: u32,
        #[serde(default = "default_100u32")]
        max: u32,
        #[serde(default)]
        label: Option<String>,
    },
    Announcement {
        #[serde(default)]
        text: String,
        #[serde(default)]
        url: Option<String>,
    },
    RotatingText {
        #[serde(default)]
        line_one: String,
        #[serde(default)]
        line_two: String,
        #[serde(default = "default_rotate_token")]
        swap_token: String,
        #[serde(default)]
        words: Vec<String>,
        #[serde(default = "default_rotate_interval_ms")]
        interval_ms: u32,
        #[serde(default)]
        font_family: String,
        #[serde(default)]
        font_weight: String,
        #[serde(default)]
        line_height: String,
        #[serde(default)]
        letter_spacing: String,
        #[serde(default)]
        rotate_color: String,
        #[serde(default)]
        min_word_width_ch: u32,
    },
    CtaBar {
        #[serde(default)]
        heading: String,
        #[serde(default)]
        subheading: String,
        #[serde(default)]
        buttons: Vec<CtaButton>,
    },
    CustomHtml {
        #[serde(default)]
        html: String,
    },
    NewsletterSignup {
        #[serde(default = "default_email_placeholder")]
        placeholder: String,
        #[serde(default = "default_subscribe")]
        button_text: String,
    },
    CouponCode {
        #[serde(default)]
        code: String,
        #[serde(default)]
        label: Option<String>,
    },
    PopupClose,
    WpContent,

    // Form inputs
    TextInput {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default)]
        required: bool,
    },
    EmailInput {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default)]
        required: bool,
    },
    PhoneInput {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default)]
        required: bool,
    },
    NumberInput {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        min: Option<f64>,
        #[serde(default)]
        max: Option<f64>,
        #[serde(default)]
        step: Option<f64>,
        #[serde(default)]
        required: bool,
    },
    DateInput {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        required: bool,
    },
    Textarea {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        placeholder: String,
        #[serde(default = "default_4u32")]
        rows: u32,
        #[serde(default)]
        required: bool,
    },
    Select {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        required: bool,
    },
    Radio {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        options: Vec<String>,
        #[serde(default)]
        required: bool,
    },
    Checkbox {
        #[serde(default)]
        label: String,
        #[serde(default)]
        name: String,
    },

    // ── Restaurant blocks ──────────────────────────────────────────
    MenuGrid {
        #[serde(default = "default_true")]
        show_images: bool,
        #[serde(default = "default_true")]
        show_prices: bool,
        #[serde(default = "default_true")]
        show_dietary: bool,
        #[serde(default = "default_true")]
        show_sections: bool,
        #[serde(default = "default_3u8")]
        columns: u8,
    },
    ReservationWidget {
        #[serde(default)]
        title: String,
        #[serde(default)]
        description: String,
    },
    ReviewCarousel {
        #[serde(default = "default_6u8")]
        count: u8,
        #[serde(default)]
        show_photos: bool,
    },
    HoursDisplay {
        #[serde(default)]
        title: String,
    },
    OrderButton {
        #[serde(default)]
        label: String,
        #[serde(default)]
        style: String,
        #[serde(default)]
        menu_item_id: Option<String>,
    },

    // ── Brooke Grace Market blocks ──────────────────────────────────
    BgProductGrid {
        #[serde(default = "default_3u8")]
        columns: u8,
        #[serde(default = "default_true")]
        show_prices: bool,
        #[serde(default)]
        category_id: String,
        #[serde(default = "default_8u8")]
        limit: u8,
    },
    BgFeaturedProducts {
        #[serde(default = "default_3u8")]
        columns: u8,
        #[serde(default = "default_true")]
        show_prices: bool,
    },
    BgCategoryNav {
        #[serde(default = "default_true")]
        show_counts: bool,
        #[serde(default)]
        layout: String,
    },
    BgCartWidget {
        #[serde(default = "default_true")]
        show_total: bool,
    },
    BgRewardsBanner {
        #[serde(default)]
        title: String,
        #[serde(default)]
        description: String,
    },
    BgSocialFeed {
        #[serde(default = "default_6u8")]
        count: u8,
        #[serde(default = "default_true")]
        show_images: bool,
    },
    BgBlogPreview {
        #[serde(default = "default_3u8")]
        count: u8,
        #[serde(default = "default_true")]
        show_excerpts: bool,
    },
    BgCreatorSpotlight {
        #[serde(default = "default_3u8")]
        count: u8,
        #[serde(default = "default_true")]
        show_bio: bool,
    },

    // ── Commerce / Checkout blocks ────────────────────────────────
    OrderSummary {
        #[serde(default = "default_true")]
        show_line_items: bool,
        #[serde(default = "default_true")]
        show_totals: bool,
        #[serde(default = "default_true")]
        show_payment_confirmation: bool,
    },
    CreatorShoutout {
        #[serde(default = "default_true")]
        show_badge: bool,
        #[serde(default = "default_true")]
        show_handle: bool,
    },
    RewardsEarned {
        #[serde(default = "default_true")]
        show_points: bool,
        #[serde(default = "default_true")]
        show_tier_progress: bool,
    },
    AlsoViewed {
        #[serde(default = "default_4u8")]
        columns: u8,
        #[serde(default = "default_true")]
        show_prices: bool,
    },
    EstimatedReady {
        #[serde(default = "default_true")]
        show_timeline: bool,
        #[serde(default)]
        label: String,
    },
    NextSteps {
        #[serde(default = "default_true")]
        show_track_order: bool,
        #[serde(default = "default_true")]
        show_account_link: bool,
        #[serde(default = "default_true")]
        show_continue_shopping: bool,
    },
}

fn default_classic() -> String {
    "classic".into()
}
fn default_slash() -> String {
    "/".into()
}
fn default_auto() -> String {
    "auto".into()
}
fn default_full() -> String {
    "full".into()
}
fn default_3u8() -> u8 {
    3
}
fn default_2u8() -> u8 {
    2
}
fn default_6u8() -> u8 {
    6
}
fn default_4u8() -> u8 {
    4
}
fn default_8u8() -> u8 {
    8
}
fn default_20u32() -> u32 {
    20
}
fn default_100u32() -> u32 {
    100
}
fn default_4u32() -> u32 {
    4
}
fn default_email_placeholder() -> String {
    "Your email address".into()
}
fn default_subscribe() -> String {
    "Subscribe".into()
}

// ── Supporting enums ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtaButton {
    pub label: String,
    pub url: String,
    #[serde(default)]
    pub style: ButtonStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum NavMode {
    #[default]
    #[serde(alias = "auto")]
    Auto,
    #[serde(alias = "menu")]
    Menu,
    #[serde(alias = "inherit")]
    Inherit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Trigger {
    #[default]
    #[serde(alias = "hover")]
    Hover,
    #[serde(alias = "click")]
    Click,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum PanelMode {
    #[default]
    #[serde(alias = "expanded")]
    Expanded,
    #[serde(alias = "tabbed")]
    Tabbed,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ButtonStyle {
    #[default]
    #[serde(alias = "primary")]
    Primary,
    #[serde(alias = "outline", alias = "secondary")]
    Outline,
    #[serde(alias = "ghost", alias = "text")]
    Ghost,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DividerVariant {
    #[default]
    #[serde(alias = "line")]
    Line,
    #[serde(alias = "dashed")]
    Dashed,
    #[serde(alias = "dotted", alias = "dots")]
    Dotted,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum AlertVariant {
    #[default]
    #[serde(alias = "info")]
    Info,
    #[serde(alias = "success")]
    Success,
    #[serde(alias = "warning")]
    Warning,
    #[serde(alias = "error")]
    Error,
}

// ── Sidebar ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SidebarConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub position: SidebarPosition,
    #[serde(default = "default_340u32")]
    pub width: u32,
    #[serde(default)]
    pub sticky: bool,
    #[serde(default)]
    pub show_on: SidebarVisibility,
    #[serde(default)]
    pub blocks: Vec<Block>,
    #[serde(default)]
    pub responsive: SidebarResponsive,
}

fn default_340u32() -> u32 {
    340
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum SidebarPosition {
    #[default]
    #[serde(alias = "right")]
    Right,
    #[serde(alias = "left")]
    Left,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarVisibility {
    #[serde(default = "default_true")]
    pub pages: bool,
    #[serde(default)]
    pub posts: bool,
    #[serde(default)]
    pub products: bool,
}

impl Default for SidebarVisibility {
    fn default() -> Self {
        Self {
            pages: true,
            posts: false,
            products: false,
        }
    }
}

// ── Responsive Config (shared by header, footer, page sections) ────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveConfig {
    #[serde(default)]
    pub mode: ResponsiveMode,
    #[serde(default = "default_breakpoint")]
    pub breakpoint: u16,

    // Simple mode
    #[serde(default = "default_true")]
    pub stack_columns: bool,
    #[serde(default)]
    pub center_content: bool,
    #[serde(default)]
    pub hidden_blocks: Vec<String>,
    #[serde(default)]
    pub column_order: Vec<usize>,

    // Advanced mode
    #[serde(default)]
    pub mobile_layout: Vec<Row>,
}

impl Default for ResponsiveConfig {
    fn default() -> Self {
        Self {
            mode: ResponsiveMode::Simple,
            breakpoint: 480,
            stack_columns: true,
            center_content: false,
            hidden_blocks: vec![],
            column_order: vec![],
            mobile_layout: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ResponsiveMode {
    #[default]
    Simple,
    Advanced,
}

fn default_breakpoint() -> u16 {
    480
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarResponsive {
    #[serde(default = "default_true")]
    pub hide_on_mobile: bool,
    #[serde(default = "default_sidebar_bp")]
    pub collapse_breakpoint: u16,
}

impl Default for SidebarResponsive {
    fn default() -> Self {
        Self {
            hide_on_mobile: true,
            collapse_breakpoint: 860,
        }
    }
}

fn default_sidebar_bp() -> u16 {
    860
}

// ── Floating Guide ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingGuideConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub style: FloatingGuideStyle,
    #[serde(default)]
    pub position: FloatingGuidePosition,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub action_label: String,
    #[serde(default)]
    pub action_url: String,
    #[serde(default = "default_open_guide")]
    pub minimized_label: String,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default = "default_true")]
    pub allow_minimize: bool,
    #[serde(default = "default_true")]
    pub allow_dismiss: bool,
    #[serde(default = "default_two_u32")]
    pub min_viewports_tall: u32,
    #[serde(default)]
    pub show_on: FloatingGuideVisibility,
}

impl Default for FloatingGuideConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            style: FloatingGuideStyle::default(),
            position: FloatingGuidePosition::default(),
            label: String::new(),
            title: String::new(),
            body: String::new(),
            action_label: String::new(),
            action_url: String::new(),
            minimized_label: default_open_guide(),
            start_minimized: false,
            allow_minimize: true,
            allow_dismiss: true,
            min_viewports_tall: default_two_u32(),
            show_on: FloatingGuideVisibility::default(),
        }
    }
}

fn default_open_guide() -> String {
    "Open guide".into()
}

fn default_two_u32() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum FloatingGuideStyle {
    #[default]
    #[serde(alias = "panel")]
    Panel,
    #[serde(alias = "pill")]
    Pill,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum FloatingGuidePosition {
    #[default]
    #[serde(alias = "top-right")]
    TopRight,
    #[serde(alias = "top-left")]
    TopLeft,
    #[serde(alias = "bottom-right")]
    BottomRight,
    #[serde(alias = "bottom-left")]
    BottomLeft,
}

impl FloatingGuidePosition {
    pub fn data_attr(&self) -> &'static str {
        match self {
            Self::TopRight => "top-right",
            Self::TopLeft => "top-left",
            Self::BottomRight => "bottom-right",
            Self::BottomLeft => "bottom-left",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingGuideVisibility {
    #[serde(default = "default_true")]
    pub pages: bool,
    #[serde(default)]
    pub posts: bool,
    #[serde(default)]
    pub products: bool,
}

impl Default for FloatingGuideVisibility {
    fn default() -> Self {
        Self {
            pages: true,
            posts: false,
            products: false,
        }
    }
}

// ── Layout Rules ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutRules {
    #[serde(default)]
    pub contexts: Vec<ContextRule>,
    #[serde(default)]
    pub paths: Vec<PathRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRule {
    pub context: String,
    pub profile_override: Option<String>,
    pub header_enabled: Option<bool>,
    pub footer_enabled: Option<bool>,
    pub sidebar_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathRule {
    pub path: String,
    pub profile_override: Option<String>,
    pub header_enabled: Option<bool>,
    pub footer_enabled: Option<bool>,
    pub sidebar_enabled: Option<bool>,
}

// ── Navigation Menu ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavMenu {
    pub location: String,
    pub items: Vec<NavMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavMenuItem {
    pub item_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub css_classes: Vec<String>,
    #[serde(default)]
    pub position: u32,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
    /// Visibility control: None/"public" = everyone, "authenticated" = logged-in only, "hidden" = not rendered.
    /// Forward-compatible with future RBAC values like "cap:admin", "role:contributor".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

// ── Popup ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupTemplate {
    pub label: String,
    #[serde(default)]
    pub status: TemplateStatus,
    pub trigger: PopupTrigger,
    pub display: PopupDisplay,
    pub appearance: PopupAppearance,
    pub layout: Vec<Row>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum TemplateStatus {
    #[default]
    #[serde(alias = "active")]
    Active,
    #[serde(alias = "inactive")]
    Inactive,
    #[serde(alias = "archived")]
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PopupTrigger {
    TimeDelay {
        #[serde(default, alias = "delay")]
        delay_secs: u32,
    },
    ScrollPercent {
        #[serde(default, alias = "scroll")]
        percent: u32,
    },
    ExitIntent,
    Click {
        #[serde(default)]
        selector: String,
    },
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupDisplay {
    #[serde(default = "default_always")]
    pub frequency: String,
    #[serde(default = "default_all")]
    pub show_on: String,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub hide_on: Vec<String>,
    #[serde(default = "default_any")]
    pub logged_in: String,
}

fn default_always() -> String {
    "always".into()
}
fn default_all() -> String {
    "all".into()
}
fn default_any() -> String {
    "any".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupAppearance {
    #[serde(default = "default_center")]
    pub position: String,
    #[serde(default = "default_520")]
    pub width: String,
    #[serde(default = "default_true")]
    pub backdrop: bool,
    #[serde(default = "default_fade")]
    pub animation: String,
    #[serde(default = "default_true")]
    pub close_button: bool,
    #[serde(default = "default_true")]
    pub close_on_backdrop: bool,
    #[serde(default)]
    pub bg_color: String,
    #[serde(default)]
    pub bg_image_path: String,
}

fn default_center() -> String {
    "center".into()
}
fn default_520() -> String {
    "520px".into()
}
fn default_fade() -> String {
    "fade".into()
}

// ── Schedule ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    pub label: String,
    #[serde(default)]
    pub status: ScheduleStatus,
    pub mode: ScheduleMode,
    #[serde(alias = "profile")]
    pub target: String,
    #[serde(default, alias = "start")]
    pub start_time: Option<String>,
    #[serde(default, alias = "end")]
    pub end_time: Option<String>,
    #[serde(default)]
    pub days: Vec<String>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ScheduleStatus {
    #[default]
    #[serde(alias = "active")]
    Active,
    #[serde(alias = "archived")]
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum ScheduleMode {
    #[default]
    #[serde(alias = "profile")]
    Profile,
    #[serde(alias = "header_footer")]
    HeaderFooter,
    #[serde(alias = "header")]
    Header,
    #[serde(alias = "footer")]
    Footer,
}

// ── Revision ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub entity_type: String,
    pub entity_id: String,
    pub version: u32,
    pub snapshot_json: String,
    pub created_at: u64,
}

// ── Page Layout (Page Studio) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageLayout {
    pub page_slug: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub layout: Vec<Row>,
}

// ── Block Preset ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPreset {
    pub label: String,
    pub block: Block,
}

// ── Preset (rich, multi-block reusable section) ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub preset_id: String,
    pub name: String,
    pub category: String, // "marketing", "layout", "commerce", "interactive", "content"
    pub description: String,
    pub blocks_json: String, // JSON array of blocks
    pub bindings: Vec<PresetBinding>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetBinding {
    pub placeholder: String, // e.g. "{{business.phone}}"
    pub source: String,      // e.g. "site_config.phone"
    pub fallback: String,    // default value if source unavailable
}


// ── Scope Style System ─────────────────────────────────────────────────────────
// Hierarchical per-scope design overrides that cascade on top of the sitewide
// profile. CSS attribute selectors (data-liq-page-slug / data-liq-page-prefixes)
// handle targeting without URL threading through every page handler.

/// Partial design token overrides — only non-None fields are applied.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeStyleOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub button_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_bg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_font: Option<FontStack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_theme_id: Option<String>,
}

impl ScopeStyleOverride {
    pub fn is_empty(&self) -> bool {
        self.primary.is_none()
            && self.accent.is_none()
            && self.link.is_none()
            && self.button_text.is_none()
            && self.header_bg.is_none()
            && self.header_text.is_none()
            && self.background.is_none()
            && self.surface.is_none()
            && self.text.is_none()
            && self.radius.is_none()
            && self.container.is_none()
            && self.body_size.is_none()
            && self.body_font.is_none()
            && self.layout_theme_id.is_none()
    }
}

/// Where a ScopeStyle applies.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScopeTarget {
    /// Applies sitewide — same precedence as the base profile (lowest).
    #[default]
    Sitewide,
    /// Applies to all pages whose URL path starts with this prefix.
    /// Example: "/german-roaches" matches /german-roaches and /german-roaches/austin.
    UrlPrefix(String),
    /// Applies to exactly one page slug.
    /// Example: "/german-roaches/austin"
    PageSlug(String),
}

impl ScopeTarget {
    pub fn specificity(&self) -> u32 {
        match self {
            ScopeTarget::Sitewide => 0,
            ScopeTarget::UrlPrefix(p) => 1 + p.len() as u32,
            ScopeTarget::PageSlug(_) => 1_000_000,
        }
    }
}

/// A named scope override stored in the WAL.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScopeStyle {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub scope: ScopeTarget,
    #[serde(default)]
    pub overrides: ScopeStyleOverride,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[cfg(test)]
mod responsive_tests {
    use super::*;

    #[test]
    fn responsive_config_defaults() {
        let rc = ResponsiveConfig::default();
        assert!(matches!(rc.mode, ResponsiveMode::Simple));
        assert_eq!(rc.breakpoint, 480);
        assert!(rc.stack_columns);
        assert!(!rc.center_content);
        assert!(rc.hidden_blocks.is_empty());
        assert!(rc.column_order.is_empty());
        assert!(rc.mobile_layout.is_empty());
    }

    #[test]
    fn responsive_config_deserializes_from_empty_json() {
        let rc: ResponsiveConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(rc.breakpoint, 480);
        assert!(rc.stack_columns);
    }

    #[test]
    fn sidebar_responsive_defaults() {
        let sr = SidebarResponsive::default();
        assert!(sr.hide_on_mobile);
        assert_eq!(sr.collapse_breakpoint, 860);
    }

    #[test]
    fn header_config_deserializes_without_responsive() {
        let json = r#"{"enabled":true,"sticky":false,"top_bar":{},"layout_builder":[]}"#;
        let hc: HeaderConfig = serde_json::from_str(json).unwrap();
        assert_eq!(hc.responsive.breakpoint, 480);
        assert!(hc.responsive.stack_columns);
    }

    #[test]
    fn topbar_hide_on_mobile_defaults_true() {
        let json = r#"{"enabled":true,"text":"Hello"}"#;
        let tb: TopBar = serde_json::from_str(json).unwrap();
        assert!(tb.hide_on_mobile);
    }

    #[test]
    fn footer_config_deserializes_without_responsive() {
        let json = r#"{"enabled":true,"layout_builder":[]}"#;
        let fc: FooterConfig = serde_json::from_str(json).unwrap();
        assert_eq!(fc.responsive.breakpoint, 480);
    }

    #[test]
    fn sidebar_config_deserializes_without_responsive() {
        let json = r#"{"enabled":true}"#;
        let sc: SidebarConfig = serde_json::from_str(json).unwrap();
        assert!(sc.responsive.hide_on_mobile);
        assert_eq!(sc.responsive.collapse_breakpoint, 860);
    }
}
