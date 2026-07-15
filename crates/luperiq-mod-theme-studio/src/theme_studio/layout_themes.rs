//! Layout theme CSS definitions for the Design Playground.
//!
//! Each theme applies structural + visual overrides via CSS selectors.
//! The active theme is stored in Profile.layout_theme_id and its CSS
//! is included in the generated page CSS via generate_full_css().

/// Returns the CSS for the given layout theme ID, or empty string for "clean-modern"
/// (the default — no extra rules needed).
pub fn css_for_theme(theme_id: &str) -> &'static str {
    match theme_id {
        "parallax-pro" => PARALLAX_PRO_CSS,
        "magazine" => MAGAZINE_CSS,
        "landing-page" => LANDING_PAGE_CSS,
        "earth-nature" => EARTH_NATURE_CSS,
        "bold-agency" => BOLD_AGENCY_CSS,
        _ => "", // "clean-modern" is the baseline, no extra CSS
    }
}

/// All 6 theme definitions for the playground gallery.
pub struct ThemeCard {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub hero_style: &'static str,   // for preview
    pub accent_hint: &'static str,  // preview gradient stop
}

pub const THEMES: &[ThemeCard] = &[
    ThemeCard {
        id: "clean-modern",
        name: "Clean Modern",
        description: "Crisp whitespace, subtle shadows, centered layouts. The balanced default.",
        hero_style: "gradient",
        accent_hint: "#3b82f6",
    },
    ThemeCard {
        id: "parallax-pro",
        name: "Parallax Pro",
        description: "Full-viewport hero with parallax scroll, bold section entrances.",
        hero_style: "fullscreen",
        accent_hint: "#7c3aed",
    },
    ThemeCard {
        id: "magazine",
        name: "Magazine",
        description: "Editorial grid, heavy typography, image-led sections.",
        hero_style: "split",
        accent_hint: "#dc2626",
    },
    ThemeCard {
        id: "landing-page",
        name: "Landing Page",
        description: "Single-column conversion funnel. Every section drives one action.",
        hero_style: "full-cta",
        accent_hint: "#059669",
    },
    ThemeCard {
        id: "earth-nature",
        name: "Earth & Nature",
        description: "Organic shapes, warm earth tones, gentle fade animations.",
        hero_style: "organic",
        accent_hint: "#78716c",
    },
    ThemeCard {
        id: "bold-agency",
        name: "Bold Agency",
        description: "Dark backgrounds, overlapping elements, high-contrast type.",
        hero_style: "dark-overlap",
        accent_hint: "#f59e0b",
    },
];

// ── Parallax Pro ────────────────────────────────────────────────────

const PARALLAX_PRO_CSS: &str = r#"
/* ── Parallax Pro layout theme ── */
body.liq-lt-parallax-pro [data-smart-block="company-hero"] {
    min-height: 100vh;
    display: flex;
    align-items: center;
    position: relative;
    overflow: hidden;
    background-attachment: fixed;
    background-size: cover;
    background-position: center;
}
body.liq-lt-parallax-pro [data-smart-block="company-hero"]::before {
    content: '';
    position: absolute;
    inset: 0;
    background: linear-gradient(135deg, rgba(0,0,0,0.55) 0%, rgba(0,0,0,0.2) 100%);
    z-index: 0;
}
body.liq-lt-parallax-pro [data-smart-block="company-hero"] > * { position: relative; z-index: 1; }

/* Fade-up animation for sections */
body.liq-lt-parallax-pro [data-liq-animate="fade-up"] {
    opacity: 0;
    transform: translateY(40px);
    transition: opacity 0.7s ease, transform 0.7s ease;
}
body.liq-lt-parallax-pro [data-liq-animate="fade-up"].liq-visible {
    opacity: 1;
    transform: translateY(0);
}

/* Spacious section rhythm */
body.liq-lt-parallax-pro [data-smart-block] { padding-block: 80px; }
body.liq-lt-parallax-pro [data-smart-block="company-hero"] { padding-block: 0; }

/* Large display headline */
body.liq-lt-parallax-pro [data-smart-block="company-hero"] h1 {
    font-size: clamp(2.5rem, 6vw, 4.5rem);
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1.1;
}
"#;

// ── Magazine ────────────────────────────────────────────────────────

const MAGAZINE_CSS: &str = r#"
/* ── Magazine layout theme ── */
body.liq-lt-magazine [data-smart-block="company-hero"] {
    display: grid;
    grid-template-columns: 1fr 1fr;
    min-height: 70vh;
    gap: 0;
}
@media (max-width: 768px) {
    body.liq-lt-magazine [data-smart-block="company-hero"] { grid-template-columns: 1fr; }
}
body.liq-lt-magazine [data-smart-block="company-hero"] .hero-content { padding: 60px; }
body.liq-lt-magazine [data-smart-block="company-hero"] .hero-image {
    background-color: var(--luperiq-primary, #1e293b);
    background-size: cover;
    background-position: center;
}

/* Slide-in animation */
body.liq-lt-magazine [data-liq-animate="slide-in"] {
    opacity: 0;
    transform: translateX(-30px);
    transition: opacity 0.6s ease, transform 0.6s ease;
}
body.liq-lt-magazine [data-liq-animate="slide-in"].liq-visible {
    opacity: 1;
    transform: translateX(0);
}

/* Editorial typography */
body.liq-lt-magazine h1, body.liq-lt-magazine h2 {
    font-weight: 900;
    letter-spacing: -0.03em;
}
body.liq-lt-magazine [data-smart-block="service-grid"] {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
}

/* Dense section rhythm */
body.liq-lt-magazine [data-smart-block] { padding-block: 40px; }
"#;

// ── Landing Page ────────────────────────────────────────────────────

const LANDING_PAGE_CSS: &str = r#"
/* ── Landing Page layout theme ── */
body.liq-lt-landing-page [data-smart-block] {
    max-width: 720px;
    margin-inline: auto;
    padding-block: 60px;
}
body.liq-lt-landing-page [data-smart-block="company-hero"] {
    max-width: 100%;
    text-align: center;
    padding-block: 80px 60px;
    background: linear-gradient(180deg, var(--luperiq-primary, #1e293b) 0%, var(--luperiq-background, #f8fafc) 100%);
    color: #fff;
}

/* Conversion-focused CTA blocks */
body.liq-lt-landing-page [data-smart-block="cta-bar"],
body.liq-lt-landing-page [data-smart-block="cta-section"] {
    max-width: 100%;
    background: var(--luperiq-accent, #3b82f6);
    color: #fff;
    text-align: center;
    padding-block: 60px;
    border-radius: 0;
}

/* Fade animation */
body.liq-lt-landing-page [data-liq-animate="fade-up"] {
    opacity: 0;
    transform: translateY(20px);
    transition: opacity 0.5s ease, transform 0.5s ease;
}
body.liq-lt-landing-page [data-liq-animate="fade-up"].liq-visible {
    opacity: 1;
    transform: translateY(0);
}
"#;

// ── Earth & Nature ──────────────────────────────────────────────────

const EARTH_NATURE_CSS: &str = r#"
/* ── Earth & Nature layout theme ── */
body.liq-lt-earth-nature {
    background-color: #faf9f7;
}
body.liq-lt-earth-nature [data-smart-block="company-hero"] {
    position: relative;
    overflow: hidden;
    padding-block: 80px;
}
body.liq-lt-earth-nature [data-smart-block="company-hero"]::after {
    content: '';
    position: absolute;
    bottom: -40px;
    left: 0;
    right: 0;
    height: 80px;
    background: #faf9f7;
    border-radius: 50% 50% 0 0 / 60px 60px 0 0;
}

/* Organic card radius */
body.liq-lt-earth-nature .service-card,
body.liq-lt-earth-nature .trust-item,
body.liq-lt-earth-nature .block-card {
    border-radius: 16px;
    border: none;
    box-shadow: 0 2px 20px rgba(0,0,0,0.06);
}

/* Gentle fade */
body.liq-lt-earth-nature [data-liq-animate="fade-up"] {
    opacity: 0;
    transform: translateY(24px);
    transition: opacity 0.9s ease, transform 0.9s ease;
}
body.liq-lt-earth-nature [data-liq-animate="fade-up"].liq-visible {
    opacity: 1;
    transform: translateY(0);
}

/* Warm, spacious rhythm */
body.liq-lt-earth-nature [data-smart-block] { padding-block: 72px; }
body.liq-lt-earth-nature h2 { font-weight: 700; letter-spacing: -0.01em; }
"#;

// ── Bold Agency ─────────────────────────────────────────────────────

const BOLD_AGENCY_CSS: &str = r#"
/* ── Bold Agency layout theme ── */
body.liq-lt-bold-agency {
    background-color: #0a0a0a;
    color: #f5f5f5;
}
body.liq-lt-bold-agency [data-smart-block="company-hero"] {
    background: #0a0a0a;
    color: #fff;
    min-height: 90vh;
    display: flex;
    align-items: center;
    position: relative;
}
body.liq-lt-bold-agency [data-smart-block="company-hero"]::before {
    content: '';
    position: absolute;
    top: -50%;
    right: -20%;
    width: 70%;
    height: 200%;
    background: var(--luperiq-accent, #f59e0b);
    opacity: 0.07;
    border-radius: 50%;
    z-index: 0;
}
body.liq-lt-bold-agency [data-smart-block="company-hero"] > * { position: relative; z-index: 1; }

/* High-contrast typography */
body.liq-lt-bold-agency h1 {
    font-size: clamp(3rem, 7vw, 6rem);
    font-weight: 900;
    letter-spacing: -0.04em;
    line-height: 0.95;
    text-transform: uppercase;
}
body.liq-lt-bold-agency h2 {
    font-weight: 900;
    text-transform: uppercase;
    letter-spacing: -0.02em;
}

/* Dark section alternation */
body.liq-lt-bold-agency [data-smart-block]:nth-child(even) {
    background: #111;
}
body.liq-lt-bold-agency [data-smart-block]:nth-child(odd) {
    background: #0a0a0a;
}

/* Pop animation */
body.liq-lt-bold-agency [data-liq-animate="fade-up"] {
    opacity: 0;
    transform: translateY(30px) scale(0.97);
    transition: opacity 0.5s ease, transform 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
body.liq-lt-bold-agency [data-liq-animate="fade-up"].liq-visible {
    opacity: 1;
    transform: translateY(0) scale(1);
}

body.liq-lt-bold-agency [data-smart-block] { padding-block: 80px; }
"#;
