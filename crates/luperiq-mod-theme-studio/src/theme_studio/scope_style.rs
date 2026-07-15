//! ScopeStyle — per-scope design token overrides with CSS attribute selector targeting.
//!
//! Scope overrides stack on top of the sitewide profile using CSS specificity:
//!   Sitewide < UrlPrefix (longer = higher priority) < PageSlug
//!
//! No URL threading required: a small inline script on every page sets
//! `data-liq-page-slug` and `data-liq-page-prefixes` on `<html>`, and
//! the generated CSS uses `html[data-liq-page-slug="..."]` selectors.

use super::config::{AGG_SCOPE_STYLE, TOMBSTONE, ScopeStyle, ScopeStyleOverride, ScopeTarget};
use luperiq_forge::ForgeJournal;

pub fn list_scope_styles(journal: &ForgeJournal) -> Vec<ScopeStyle> {
    let mut styles: Vec<ScopeStyle> = journal
        .latest_by_aggregate_type(AGG_SCOPE_STYLE)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice(&e.payload).ok())
        .filter(|s: &ScopeStyle| s.enabled)
        .collect();

    // Sort by specificity so CSS is generated in cascade order (lowest first)
    styles.sort_by_key(|s| s.scope.specificity());
    styles
}

/// Generate CSS that overrides CSS custom properties for each active scope.
/// Uses `html[data-liq-page-slug]` and `html[data-liq-page-prefixes~=...]` selectors.
pub fn generate_scope_css(styles: &[ScopeStyle]) -> String {
    let mut css = String::new();

    for style in styles {
        if style.overrides.is_empty() {
            continue;
        }
        let selector = scope_selector(&style.scope);
        let vars = token_vars(&style.overrides);
        if vars.is_empty() {
            continue;
        }
        css.push_str(&format!(
            "\n/* scope: {} */\n{} {{\n{}}}\n",
            style.label, selector, vars
        ));

        // Layout theme body class override for this scope — inject a modifier class
        // via a tiny CSS rule that sets a custom property read by the layout theme CSS.
        // The JS playground also applies the body class for admin preview.
        if let Some(ref theme_id) = style.overrides.layout_theme_id {
            if !theme_id.is_empty() && theme_id != "clean-modern" {
                let theme_css = super::layout_themes::css_for_theme(theme_id);
                if !theme_css.is_empty() {
                    // Wrap all layout theme selectors in the scope selector using @layer trick:
                    // scope selector is on <html>, but layout theme targets <body.liq-lt-*>.
                    // We use a data attribute on body too (set by JS for admin preview;
                    // server-side we append a data-liq-scope-theme body attribute).
                    css.push_str(&format!(
                        "\n/* scope layout theme: {} */\nbody[data-liq-scope-theme=\"{}\"] {{\n{}}}\n",
                        theme_id, theme_id, ""
                    ));
                }
            }
        }
    }

    css
}

fn scope_selector(scope: &ScopeTarget) -> String {
    match scope {
        ScopeTarget::Sitewide => ":root".to_string(),
        ScopeTarget::UrlPrefix(prefix) => {
            // html[data-liq-page-prefixes~="/some/path"]
            // The ~= selector matches a space-separated word in the attribute
            format!("html[data-liq-page-prefixes~=\"{}\"]", css_escape_attr(prefix))
        }
        ScopeTarget::PageSlug(slug) => {
            format!("html[data-liq-page-slug=\"{}\"]", css_escape_attr(slug))
        }
    }
}

fn css_escape_attr(s: &str) -> String {
    s.replace('"', "\\\"")
}

fn token_vars(ov: &ScopeStyleOverride) -> String {
    let mut vars = String::new();

    if let Some(ref v) = ov.primary     { vars.push_str(&format!("    --luperiq-primary: {};\n", v)); }
    if let Some(ref v) = ov.accent      { vars.push_str(&format!("    --luperiq-accent: {};\n", v)); }
    if let Some(ref v) = ov.link        { vars.push_str(&format!("    --luperiq-link: {};\n", v)); }
    if let Some(ref v) = ov.button_text { vars.push_str(&format!("    --luperiq-button-text: {};\n", v)); }
    if let Some(ref v) = ov.header_bg   { vars.push_str(&format!("    --luperiq-header-bg: {};\n", v)); }
    if let Some(ref v) = ov.header_text { vars.push_str(&format!("    --luperiq-header-text: {};\n", v)); }
    if let Some(ref v) = ov.background  { vars.push_str(&format!("    --luperiq-background: {};\n", v)); }
    if let Some(ref v) = ov.surface     { vars.push_str(&format!("    --luperiq-surface: {};\n", v)); }
    if let Some(ref v) = ov.text        { vars.push_str(&format!("    --luperiq-text: {};\n", v)); }
    if let Some(v) = ov.radius          { vars.push_str(&format!("    --luperiq-radius: {}px;\n", v)); }
    if let Some(v) = ov.container       { vars.push_str(&format!("    --luperiq-container: {}px;\n", v)); }
    if let Some(v) = ov.body_size       { vars.push_str(&format!("    --luperiq-body-size: {}px;\n", v)); }
    if let Some(ref font) = ov.body_font {
        vars.push_str(&format!("    --luperiq-body-font: {};\n", font.css_value()));
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::config::{ScopeStyle, ScopeStyleOverride, ScopeTarget};

    fn make_style(scope: ScopeTarget, primary: &str) -> ScopeStyle {
        ScopeStyle {
            id: "test".to_string(),
            label: "Test".to_string(),
            scope,
            overrides: ScopeStyleOverride {
                primary: Some(primary.to_string()),
                ..Default::default()
            },
            enabled: true,
        }
    }

    #[test]
    fn test_sitewide_selector() {
        let s = make_style(ScopeTarget::Sitewide, "#ff0000");
        let css = generate_scope_css(&[s]);
        assert!(css.contains(":root {"), "got: {css}");
        assert!(css.contains("--luperiq-primary: #ff0000;"), "got: {css}");
    }

    #[test]
    fn test_prefix_selector() {
        let s = make_style(ScopeTarget::UrlPrefix("/german-roaches".to_string()), "#8b4513");
        let css = generate_scope_css(&[s]);
        assert!(css.contains("html[data-liq-page-prefixes~=\"/german-roaches\"]"), "got: {css}");
    }

    #[test]
    fn test_page_selector() {
        let s = make_style(ScopeTarget::PageSlug("/german-roaches/austin".to_string()), "#daa520");
        let css = generate_scope_css(&[s]);
        assert!(css.contains("html[data-liq-page-slug=\"/german-roaches/austin\"]"), "got: {css}");
    }

    #[test]
    fn test_empty_override_skipped() {
        let s = ScopeStyle {
            id: "t".to_string(),
            label: "Empty".to_string(),
            scope: ScopeTarget::Sitewide,
            overrides: ScopeStyleOverride::default(),
            enabled: true,
        };
        let css = generate_scope_css(&[s]);
        assert!(css.is_empty(), "expected empty css, got: {css}");
    }
}
