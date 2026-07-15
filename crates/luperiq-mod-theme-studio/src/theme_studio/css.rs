// luperiq-cms/src/modules/theme_studio/css.rs
//! CSS variable generation from design tokens.
//!
//! Outputs `:root` custom properties matching the `--luperiq-*` namespace
//! used by the WordPress theme, plus responsive breakpoints for tablet/mobile.

use super::config::{
    accessible_button_text_color, derive_footer_bg, derive_hover_color, normalize_hex_color,
    DesignTokens, FooterConfig, HeaderConfig, ResponsiveConfig, ResponsiveMode, SidebarConfig,
};

/// Generate CSS `:root` variables from design tokens, with responsive breakpoints.
pub fn generate_css(tokens: &DesignTokens) -> String {
    let mut css = String::with_capacity(2048);
    // Inject Google Fonts @import before :root so the font loads for all visitors
    if let Some(url) = tokens.body_font.google_fonts_url() {
        css.push_str(&format!("@import url('{}');

", url));
    }
    // Heading font: import separately only if it is a different Google Font
    if let Some(hf) = &tokens.heading_font {
        let same_variant = std::mem::discriminant(hf) == std::mem::discriminant(&tokens.body_font);
        if !same_variant {
            if let Some(hurl) = hf.google_fonts_url() {
                css.push_str(&format!("@import url('{}');

", hurl));
            }
        }
    }
    let accent = normalize_hex_color(&tokens.accent).unwrap_or_else(|| tokens.accent.clone());
    let accent_hover = derive_hover_color(&accent);
    let button_text = accessible_button_text_color(&accent, &tokens.button_text);
    let footer_bg = derive_footer_bg(&tokens.primary);

    // ── Base :root variables ───────────────────────────────────────
    css.push_str(":root {\n");
    css.push_str(&format!("    --luperiq-primary: {};\n", tokens.primary));
    css.push_str(&format!("    --luperiq-accent: {};\n", accent));
    css.push_str(&format!("    --luperiq-accent-hover: {};\n", accent_hover));
    css.push_str(&format!("    --luperiq-link: {};\n", tokens.link));
    css.push_str(&format!("    --luperiq-button-text: {};\n", button_text));
    css.push_str(&format!("    --luperiq-header-bg: {};\n", tokens.header_bg));
    css.push_str(&format!(
        "    --luperiq-header-text: {};\n",
        tokens.header_text
    ));
    css.push_str(&format!("    --luperiq-footer-bg: {};\n", footer_bg));
    css.push_str(&format!(
        "    --luperiq-background: {};\n",
        tokens.background
    ));
    css.push_str(&format!("    --luperiq-surface: {};\n", tokens.surface));
    css.push_str(&format!("    --luperiq-text: {};\n", tokens.text));
    css.push_str(&format!("    --luperiq-radius: {}px;\n", tokens.radius));
    css.push_str(&format!(
        "    --luperiq-container: {}px;\n",
        tokens.container
    ));
    css.push_str(&format!(
        "    --luperiq-brand-size: {}px;\n",
        tokens.brand_size
    ));
    css.push_str(&format!("    --luperiq-nav-size: {}px;\n", tokens.nav_size));
    css.push_str(&format!("    --luperiq-nav-gap: {}px;\n", tokens.nav_gap));
    css.push_str(&format!(
        "    --luperiq-body-font: {};\n",
        tokens.body_font.css_value()
    ));
    if let Some(hf) = &tokens.heading_font {
        css.push_str(&format!(
            "    --luperiq-heading-font: {};\n",
            hf.css_value()
        ));
    }
    css.push_str(&format!(
        "    --luperiq-body-size: {}px;\n",
        tokens.body_size
    ));
    css.push_str(&format!(
        "    --luperiq-heading-size: {}px;\n",
        tokens.heading_size
    ));
    // body_line_height is stored as integer (e.g. 16) but displayed as decimal (1.6)
    let lh = tokens.body_line_height as f64 / 10.0;
    css.push_str(&format!("    --luperiq-body-line-height: {:.1};\n", lh));
    css.push_str(&format!("    --accent: {};\n", accent));
    css.push_str(&format!("    --accent-hover: {};\n", accent_hover));
    css.push_str(&format!("    --bg: {};\n", tokens.background));
    css.push_str(&format!("    --surface: {};\n", tokens.surface));
    css.push_str(&format!("    --text: {};\n", tokens.text));
    css.push_str(&format!("    --header-bg: {};\n", tokens.header_bg));
    css.push_str(&format!("    --header-text: {};\n", tokens.header_text));
    if tokens.full_bleed {
        css.push_str("    --luperiq-full-bleed: 1;\n");
    }
    css.push_str("}\n");

    // Custom CSS (admin-defined overrides)
    if let Some(custom) = &tokens.custom_css {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            css.push_str("\n/* Custom CSS */\n");
            css.push_str(trimmed);
            css.push('\n');
        }
    }

    // ── Full-bleed layout ─────────────────────────────────────────────
    if tokens.full_bleed {
        css.push_str("\n.site-main {\n");
        css.push_str("    max-width: none !important;\n");
        css.push_str("    padding: 0 !important;\n");
        css.push_str("}\n");
        css.push_str(".site-main-inner {\n");
        css.push_str(&format!("    max-width: {}px;\n", tokens.container));
        css.push_str("    margin: 0 auto;\n");
        css.push_str("    padding: 32px 18px;\n");
        css.push_str("    width: 100%;\n");
        css.push_str("    box-sizing: border-box;\n");
        css.push_str("}\n");
    }

    // ── Tablet overrides (max-width: 980px) ────────────────────────
    if let Some(ref tablet) = tokens.tablet {
        css.push_str("\n@media (max-width: 980px) {\n    :root {\n");
        write_overrides(&mut css, tablet, tokens);
        css.push_str("    }\n}\n");
    }

    // ── Mobile overrides (max-width: 860px) ────────────────────────
    if let Some(ref mobile) = tokens.mobile {
        css.push_str("\n@media (max-width: 860px) {\n    :root {\n");
        write_overrides(&mut css, mobile, tokens);
        css.push_str("    }\n}\n");
    }

    css
}

/// Generate full CSS: token variables + responsive layout rules.
///
/// Wraps `generate_css()` and appends responsive rules for header, footer,
/// and sidebar based on their `ResponsiveConfig`.
pub fn generate_full_css(
    tokens: &DesignTokens,
    header: Option<&HeaderConfig>,
    footer: Option<&FooterConfig>,
    sidebar: Option<&SidebarConfig>,
) -> String {
    generate_full_css_with_theme(tokens, header, footer, sidebar, None, &[])
}

pub fn generate_full_css_with_theme(
    tokens: &DesignTokens,
    header: Option<&HeaderConfig>,
    footer: Option<&FooterConfig>,
    sidebar: Option<&SidebarConfig>,
    layout_theme_id: Option<&str>,
    scope_styles: &[super::config::ScopeStyle],
) -> String {
    let mut css = generate_css(tokens);

    if let Some(h) = header {
        let bp = h.responsive.breakpoint;
        if h.top_bar.hide_on_mobile {
            write_topbar_hide(&mut css, bp);
        }
        css.push_str(&format!(
            "\n@media (max-width: {}px) {{\n    .luperiq-ts-layout--header .luperiq-ts-block--rotating-text {{ display: none !important; }}\n}}\n",
            bp
        ));
        write_responsive_rules(&mut css, "header", &h.responsive);
    }
    if let Some(f) = footer {
        write_responsive_rules(&mut css, "footer", &f.responsive);
    }
    if let Some(s) = sidebar {
        write_sidebar_responsive(&mut css, &s.responsive);
    }

    if let Some(theme_id) = layout_theme_id {
        let theme_css = super::layout_themes::css_for_theme(theme_id);
        if !theme_css.is_empty() {
            css.push_str("\n/* layout theme */\n");
            css.push_str(theme_css);
        }
    }

    if !scope_styles.is_empty() {
        let scope_css = super::scope_style::generate_scope_css(scope_styles);
        if !scope_css.is_empty() {
            css.push_str("\n/* scope overrides */\n");
            css.push_str(&scope_css);
        }
    }

    css
}

fn write_topbar_hide(css: &mut String, breakpoint: u16) {
    css.push_str(&format!(
        "\n@media (max-width: {}px) {{\n    .luperiq-ts-layout--header .luperiq-ts-top-bar {{ display: none; }}\n}}\n",
        breakpoint
    ));
}

fn write_responsive_rules(css: &mut String, area: &str, config: &ResponsiveConfig) {
    match config.mode {
        ResponsiveMode::Simple => write_simple_responsive(css, area, config),
        ResponsiveMode::Advanced => write_advanced_responsive(css, area, config),
    }
}

fn write_simple_responsive(css: &mut String, area: &str, config: &ResponsiveConfig) {
    let bp = config.breakpoint;
    let mut rules = String::new();

    // Always shrink CTAs at the breakpoint (matches previous static behavior)
    rules.push_str(&format!(
        "    .luperiq-ts-layout--{} .luperiq-ts-cta {{ padding: 6px 12px; font-size: 12px; }}\n",
        area
    ));

    if config.stack_columns {
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-layout-row {{ grid-template-columns: 1fr !important; gap: 6px; }}\n",
            area
        ));
        // Reduce brand name/subtitle font size on mobile (matches old static behavior)
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-brand-name {{ font-size: 18px; white-space: normal; }}\n",
            area
        ));
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-brand-subtitle {{ font-size: 11px; }}\n",
            area
        ));
        // Center CTA group when stacked (always looks better centered in single-column)
        // !important needed to override .is-align-right specificity from static CSS
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-cta-group {{ justify-content: center !important; }}\n",
            area
        ));
    }

    if config.center_content {
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-layout-row {{ justify-items: center; text-align: center; }}\n",
            area
        ));
    }

    for block_id in &config.hidden_blocks {
        let css_class = block_id.replace('_', "-");
        rules.push_str(&format!(
            "    .luperiq-ts-layout--{} .luperiq-ts-block--{} {{ display: none; }}\n",
            area, css_class
        ));
    }

    if !config.column_order.is_empty() {
        for (css_order, &col_idx) in config.column_order.iter().enumerate() {
            rules.push_str(&format!(
                "    .luperiq-ts-layout--{} .luperiq-ts-layout-col:nth-child({}) {{ order: {}; }}\n",
                area,
                col_idx + 1,
                css_order
            ));
        }
    }

    if !rules.is_empty() {
        css.push_str(&format!("\n@media (max-width: {}px) {{\n{}}}\n", bp, rules));
    }
}

fn write_advanced_responsive(css: &mut String, area: &str, config: &ResponsiveConfig) {
    if config.mobile_layout.is_empty() {
        return;
    }
    let bp = config.breakpoint;
    css.push_str(&format!(
        "\n@media (min-width: {}px) {{\n    .luperiq-ts-layout--{} .luperiq-ts-layout-row--mobile {{ display: none !important; }}\n}}\n",
        bp + 1, area
    ));
    css.push_str(&format!(
        "@media (max-width: {}px) {{\n    .luperiq-ts-layout--{} .luperiq-ts-layout-row--desktop {{ display: none !important; }}\n    .luperiq-ts-layout--{} .luperiq-ts-layout-row--mobile {{ display: grid; }}\n}}\n",
        bp, area, area
    ));
}

fn write_sidebar_responsive(css: &mut String, config: &super::config::SidebarResponsive) {
    if config.hide_on_mobile {
        css.push_str(&format!(
            "\n@media (max-width: {}px) {{\n    .luperiq-ts-sidebar {{ display: none !important; }}\n}}\n",
            config.collapse_breakpoint
        ));
    }
}

/// Write only changed override values into a media query block.
fn write_overrides(
    css: &mut String,
    overrides: &super::config::TokenOverrides,
    base: &DesignTokens,
) {
    if let Some(v) = overrides.radius {
        if v != base.radius {
            css.push_str(&format!("        --luperiq-radius: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.container {
        if v != base.container {
            css.push_str(&format!("        --luperiq-container: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.brand_size {
        if v != base.brand_size {
            css.push_str(&format!("        --luperiq-brand-size: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.nav_size {
        if v != base.nav_size {
            css.push_str(&format!("        --luperiq-nav-size: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.nav_gap {
        if v != base.nav_gap {
            css.push_str(&format!("        --luperiq-nav-gap: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.body_size {
        if v != base.body_size {
            css.push_str(&format!("        --luperiq-body-size: {}px;\n", v));
        }
    }
    if let Some(v) = overrides.body_line_height {
        if v != base.body_line_height {
            let lh = v as f64 / 10.0;
            css.push_str(&format!("        --luperiq-body-line-height: {:.1};\n", lh));
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme_studio::config::{
        DesignTokens, FooterConfig, HeaderConfig, ResponsiveMode, Row, SidebarConfig,
        TokenOverrides,
    };

    #[test]
    fn base_css_contains_all_vars() {
        let tokens = DesignTokens::default();
        let css = generate_css(&tokens);
        assert!(css.contains("--luperiq-primary:"));
        assert!(css.contains("--luperiq-accent:"));
        assert!(css.contains("--luperiq-accent-hover:"));
        assert!(css.contains("--luperiq-link:"));
        assert!(css.contains("--luperiq-button-text:"));
        assert!(css.contains("--luperiq-header-bg:"));
        assert!(css.contains("--luperiq-header-text:"));
        assert!(css.contains("--luperiq-footer-bg:"));
        assert!(css.contains("--luperiq-background:"));
        assert!(css.contains("--luperiq-surface:"));
        assert!(css.contains("--luperiq-text:"));
        assert!(css.contains("--luperiq-radius: 16px;"));
        assert!(css.contains("--luperiq-container: 1100px;"));
        assert!(css.contains("--luperiq-brand-size: 56px;"));
        assert!(css.contains("--luperiq-nav-size: 16px;"));
        assert!(css.contains("--luperiq-nav-gap: 16px;"));
        assert!(css.contains("--luperiq-body-size: 16px;"));
        assert!(css.contains("--luperiq-body-line-height: 1.6;"));
        assert!(css.contains("--luperiq-body-font:"));
        assert!(css.contains("--accent:"));
        assert!(css.contains("--accent-hover:"));
    }

    #[test]
    fn no_media_queries_without_overrides() {
        let tokens = DesignTokens::default();
        let css = generate_css(&tokens);
        assert!(!css.contains("@media"));
    }

    #[test]
    fn tablet_override_emits_media_query() {
        let mut tokens = DesignTokens::default();
        tokens.tablet = Some(TokenOverrides {
            container: Some(860),
            ..Default::default()
        });
        let css = generate_css(&tokens);
        assert!(css.contains("@media (max-width: 980px)"));
        assert!(css.contains("--luperiq-container: 860px;"));
    }

    #[test]
    fn same_value_override_skipped() {
        let mut tokens = DesignTokens::default();
        tokens.tablet = Some(TokenOverrides {
            radius: Some(16), // same as default
            ..Default::default()
        });
        let css = generate_css(&tokens);
        // Media query is emitted but no overrides written inside
        assert!(css.contains("@media (max-width: 980px)"));
        // The override block should be empty (just the wrapper)
        let tablet_section = css.split("@media (max-width: 980px)").nth(1).unwrap();
        assert!(!tablet_section.contains("--luperiq-radius:"));
    }

    #[test]
    fn mobile_override() {
        let mut tokens = DesignTokens::default();
        tokens.mobile = Some(TokenOverrides {
            body_size: Some(14),
            body_line_height: Some(14),
            ..Default::default()
        });
        let css = generate_css(&tokens);
        assert!(css.contains("@media (max-width: 860px)"));
        assert!(css.contains("--luperiq-body-size: 14px;"));
        assert!(css.contains("--luperiq-body-line-height: 1.4;"));
    }

    #[test]
    fn simple_mode_stack_columns_generates_media_query() {
        let tokens = DesignTokens::default();
        let header = HeaderConfig::default();
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains("@media (max-width: 480px)"));
        assert!(css.contains(".luperiq-ts-layout--header"));
        assert!(css.contains("grid-template-columns: 1fr"));
    }

    #[test]
    fn hidden_blocks_generates_display_none() {
        let tokens = DesignTokens::default();
        let mut header = HeaderConfig::default();
        header.responsive.hidden_blocks = vec!["rotating_text".to_string()];
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains(".luperiq-ts-block--rotating-text"));
        assert!(css.contains("display: none"));
    }

    #[test]
    fn topbar_hide_on_mobile_generates_rule() {
        let tokens = DesignTokens::default();
        let header = HeaderConfig::default();
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains(".luperiq-ts-top-bar"));
        assert!(css.contains("display: none"));
    }

    #[test]
    fn center_content_generates_text_align() {
        let tokens = DesignTokens::default();
        let mut header = HeaderConfig::default();
        header.responsive.center_content = true;
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains("text-align: center"));
        assert!(css.contains("justify-items: center"));
    }

    #[test]
    fn no_stack_columns_still_shrinks_ctas() {
        let tokens = DesignTokens::default();
        let mut header = HeaderConfig::default();
        header.responsive.stack_columns = false;
        let css = generate_full_css(&tokens, Some(&header), None, None);
        // CTA shrink always happens
        assert!(css.contains("font-size: 12px"));
        // But grid override should NOT be present
        assert!(!css.contains("grid-template-columns: 1fr"));
    }

    #[test]
    fn sidebar_responsive_generates_hide_rule() {
        let tokens = DesignTokens::default();
        let sidebar = SidebarConfig::default();
        let css = generate_full_css(&tokens, None, None, Some(&sidebar));
        assert!(css.contains("@media (max-width: 860px)"));
        assert!(css.contains(".luperiq-ts-sidebar"));
        assert!(css.contains("display: none"));
    }

    #[test]
    fn footer_responsive_generates_rules() {
        let tokens = DesignTokens::default();
        let footer = FooterConfig::default();
        let css = generate_full_css(&tokens, None, Some(&footer), None);
        assert!(css.contains(".luperiq-ts-layout--footer"));
    }

    #[test]
    fn column_order_generates_css_order() {
        let tokens = DesignTokens::default();
        let mut header = HeaderConfig::default();
        header.responsive.column_order = vec![1, 0];
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains("order:"));
    }

    #[test]
    fn advanced_mode_generates_visibility_swap() {
        let tokens = DesignTokens::default();
        let mut header = HeaderConfig::default();
        header.responsive.mode = ResponsiveMode::Advanced;
        header.responsive.mobile_layout = vec![Row { columns: vec![] }];
        let css = generate_full_css(&tokens, Some(&header), None, None);
        assert!(css.contains("layout-row--mobile"));
        assert!(css.contains("layout-row--desktop"));
    }
}
