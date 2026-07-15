//! Site Type Catalog data structures.
//!
//! Each `SiteTypeDefinition` holds everything needed to provision a site of
//! that type: identity, modules, theme defaults, nav menu, default pages,
//! homepage content, company profile, pricing, onboarding config, and SEO.

use serde::{Deserialize, Serialize};

pub const AGG_SITE_TYPE: &str = "SiteCatalog:SiteType";

/// Complete definition of a site type — everything the provisioner needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteTypeDefinition {
    // ── 1. Identity ─────────────────────────────────────────────────
    pub slug: String,
    pub name: String,
    pub emoji: String,
    /// "free" or "business"
    pub category: String,
    pub description: String,
    pub default_tagline: String,
    /// Show in public catalog / get-started page
    pub publicly_listed: bool,
    /// Display order within category
    pub display_order: u32,

    // ── 2. Modules ──────────────────────────────────────────────────
    /// Module slugs to enable on new sites of this type
    pub enabled_modules: Vec<String>,

    // ── 3. Theme Presets ───────────────────────────────────────────
    /// Up to 8 theme presets per site type. First preset is the default.
    /// Can be captured from the Design Playground by clicking "Save as Default"
    pub theme_presets: Vec<ThemePreset>,
    /// Legacy single profile (kept for backward compat, use theme_presets instead)
    #[serde(default)]
    pub theme_profile: Option<serde_json::Value>,

    // ── 4. Nav Menu ─────────────────────────────────────────────────
    /// Default navigation items with sub-menu support
    pub default_nav_items: Vec<NavItem>,

    // ── 5. Default Pages ────────────────────────────────────────────
    /// Pages to create during provisioning (slug, title, smart blocks)
    pub default_pages: Vec<DefaultPage>,

    // ── 6. Homepage Content ─────────────────────────────────────────
    /// Smart blocks for the homepage (JSON array)
    pub homepage_blocks: Option<serde_json::Value>,

    // ── 7. Company Profile Defaults ─────────────────────────────────
    pub default_tone: String,
    pub default_brand_colors: BrandColors,

    // ── 8. Pricing / Tier ───────────────────────────────────────────
    /// Default tier slug for new sites ("free", "pro-monthly", "founding", etc.)
    pub default_tier: String,
    /// Whether this type is always free (family, creator, blog)
    pub always_free: bool,
    /// Price override in cents (0 = use standard tier pricing)
    pub price_override_cents: i64,
    /// Discount code that unlocks this type for free or at a reduced price
    pub discount_codes: Vec<DiscountCode>,
    /// Limited-time offer (e.g., "free during early release")
    pub limited_time_offer: Option<LimitedOffer>,

    // ── 9. Onboarding Wizard ────────────────────────────────────────
    /// Custom wizard steps for this type (if empty, uses generic)
    pub onboarding_steps: Vec<OnboardingStep>,

    // ── 10. SEO Defaults ────────────────────────────────────────────
    pub seo_title_template: String,
    pub seo_description_template: String,

    // ── Metadata ────────────────────────────────────────────────────
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemePreset {
    pub id: String,
    pub name: String,
    pub description: String,
    /// Full ThemeStudio:Profile JSON
    pub profile: serde_json::Value,
    /// Nav style: "card_grid_mega", "horizontal", "vertical", "icon_bar"
    pub nav_style: String,
    /// Display order (1 = default)
    pub display_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItem {
    pub item_id: String,
    pub title: String,
    pub url: String,
    pub emoji: String,
    pub position: u32,
    /// Sub-menu items (Card Grid Mega shows these as cards)
    #[serde(default)]
    #[allow(clippy::vec_init_then_push)]
    pub children: Vec<NavItem>,
}

impl Default for NavItem {
    fn default() -> Self {
        Self {
            item_id: String::new(),
            title: String::new(),
            url: String::new(),
            emoji: String::new(),
            position: 0,
            children: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultPage {
    pub slug: String,
    pub title: String,
    /// Smart blocks JSON for page content
    pub blocks: Option<serde_json::Value>,
    pub seo_title: String,
    pub seo_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrandColors {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountCode {
    pub code: String,
    /// "free", "percent", "fixed"
    pub discount_type: String,
    /// Percentage (0-100) or fixed amount in cents
    pub value: i64,
    /// Optional expiry timestamp (0 = never)
    pub expires_at: u64,
    /// Max uses (0 = unlimited)
    pub max_uses: u32,
    pub uses: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitedOffer {
    pub label: String,
    pub description: String,
    /// When the offer expires (0 = manual end)
    pub expires_at: u64,
    /// Override tier during this offer
    pub tier_override: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingStep {
    pub step_id: String,
    pub label: String,
    pub fields: Vec<OnboardingField>,
    pub skippable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingField {
    pub key: String,
    pub label: String,
    /// "text", "textarea", "select", "checkbox_grid", "tags"
    pub field_type: String,
    pub placeholder: String,
    pub required: bool,
    pub options: Vec<String>,
    #[serde(default)]
    pub help_text: String,
    #[serde(default)]
    pub admin_notes: String,
}

fn normalized_field_key(field_key: &str) -> String {
    field_key.trim().to_ascii_lowercase().replace('-', "_")
}

pub fn default_onboarding_help_for(
    site_slug: &str,
    step_id: &str,
    field_key: &str,
    field_label: &str,
) -> String {
    let key = normalized_field_key(field_key);
    let step = normalized_field_key(step_id);
    let label = if field_label.trim().is_empty() {
        field_key.replace('_', " ")
    } else {
        field_label.trim().to_string()
    };

    if key == "business_name" || key == "group_name" || key == "creator_name" || key == "blog_name"
    {
        return "This becomes the main public name for the site and is used in headers, page titles, and branding.".into();
    }
    if key == "family_name" {
        return "This becomes the main family name shown across the site, profile areas, and shared spaces.".into();
    }
    if key == "tagline" {
        return "A short one-line message that helps shape the homepage hero, SEO snippets, and starter marketing copy.".into();
    }
    if key == "phone" {
        return "Shown in contact areas, headers, booking prompts, and other call-to-action spots where visitors expect a phone number.".into();
    }
    if key == "email" {
        return "Used for contact details, footer/contact sections, and customer-facing communication touchpoints.".into();
    }
    if key == "address" || key == "city" || key == "state" || key == "zip" || key == "postal_code" {
        return "Used for local trust signals like contact details, map/context blocks, schema, and local SEO.".into();
    }
    if key == "cities" || key.contains("service_area") || key == "radius" || key == "destination" {
        return "This helps the site talk about where you serve or gather, and can feed service-area, location, and local landing-page content.".into();
    }
    if key == "services" || key == "menu" || key.contains("service") {
        return "These answers help build service or offering sections, booking choices, and the strongest first-draft pages for this site type.".into();
    }
    if key == "pests" || key.contains("pest") {
        return "Used to shape pest-specific messaging, treatment positioning, and any pest-focused page generation for the site.".into();
    }
    if key.contains("treatment") {
        return "Used in treatment/service messaging, internal setup defaults, and any treatment-focused page generation.".into();
    }
    if key.contains("license") || key.contains("compliance") || key.contains("warranty") {
        return "This supports trust-building details, compliance sections, and admin records that may also surface on public pages.".into();
    }
    if key.contains("booking")
        || key.contains("reservation")
        || key.contains("scheduling")
        || key.contains("portal")
        || key == "walk_ins"
    {
        return "This affects the calls to action, booking flow, and which customer-facing scheduling features are highlighted.".into();
    }
    if key == "members" || key.contains("member") || key.contains("team") || key.contains("teacher")
    {
        return "This helps set up people-related sections like directories, team/about areas, and collaboration features.".into();
    }
    if key == "bio" || key == "story" || key == "mission" || key == "about" {
        return "Used to ground the About page, homepage story sections, and AI-generated starter copy so the site sounds like you.".into();
    }
    if key == "youtube" || key == "instagram" || key == "tiktok" || key.contains("social") {
        return "Used to connect social/profile links and creator-facing sections where visitors can follow you elsewhere.".into();
    }
    if key.contains("fundraising") || key == "donations" {
        return "This influences giving or fundraising features, starter widgets, and campaign-friendly site sections.".into();
    }
    if key.contains("rsvp") || key.contains("guest") || key.contains("vendor") {
        return "This helps configure attendance, guest, or event-planning features for this type of site.".into();
    }
    if key.contains("inventory")
        || key.contains("equipment")
        || key.contains("gear")
        || key.contains("supply")
        || key.contains("packing")
    {
        return "This helps turn on inventory, shared resources, or planning workflows that fit this type of site.".into();
    }
    if key.contains("frequency") || key.contains("calendar") || key.contains("events") {
        return "Used to shape calendar/event defaults, reminders, and recurring activity sections for this site type.".into();
    }
    if site_slug == "blog" {
        return format!(
            "{label} helps shape your blog setup, homepage positioning, and the first content structure we create for you."
        );
    }
    if site_slug == "creator" {
        return format!(
            "{label} helps shape your creator profile, linkouts, and the way your public brand is presented across the site."
        );
    }
    if step == "welcome" || step == "contact" || step == "service_area" {
        return format!(
            "{label} helps us personalize the site setup, public-facing messaging, and starter content for this site type."
        );
    }

    format!(
        "{label} helps tailor the starter pages, wording, and enabled features for this type of site."
    )
}

pub fn onboarding_usage_hints_for(site_slug: &str, step_id: &str, field_key: &str) -> Vec<String> {
    let key = normalized_field_key(field_key);
    let mut hints: Vec<String> = Vec::new();

    let push = |list: &mut Vec<String>, value: &str| {
        if !list.iter().any(|existing| existing == value) {
            list.push(value.to_string());
        }
    };

    if key == "business_name" || key == "group_name" || key == "creator_name" || key == "blog_name"
    {
        push(&mut hints, "Site title and public branding");
        push(&mut hints, "Homepage hero and navigation");
        push(&mut hints, "SEO titles and starter page headings");
    }
    if key == "family_name" {
        push(&mut hints, "Family profile and shared-space branding");
        push(&mut hints, "Homepage hero and site title");
    }
    if key == "tagline" {
        push(&mut hints, "Homepage hero copy");
        push(&mut hints, "SEO description and marketing snippets");
        push(&mut hints, "AI grounding for page drafts");
    }
    if key == "phone" {
        push(&mut hints, "Header, footer, and contact sections");
        push(&mut hints, "Booking and call CTA blocks");
        push(&mut hints, "Company/contact profile data");
    }
    if key == "email" {
        push(&mut hints, "Contact sections and notifications");
        push(&mut hints, "Company/contact profile data");
    }
    if key == "address" || key == "city" || key == "state" || key == "zip" || key == "postal_code" {
        push(&mut hints, "Contact page and footer details");
        push(&mut hints, "Local SEO and schema");
        push(&mut hints, "Map or area-context blocks");
    }
    if key == "cities" || key.contains("service_area") || key == "radius" || key == "destination" {
        push(&mut hints, "Service-area or location pages");
        push(&mut hints, "Homepage geo messaging");
        push(&mut hints, "Local landing pages and SEO");
    }
    if key == "services" || key == "menu" || key.contains("service") {
        push(&mut hints, "Service or offering cards");
        push(&mut hints, "Booking choices and CTA paths");
        push(&mut hints, "AI-generated starter pages");
    }
    if key == "pests" || key.contains("pest") {
        push(&mut hints, "Pest-specific public pages");
        push(&mut hints, "Treatment/service messaging");
        push(&mut hints, "AI silo page generation");
    }
    if key.contains("treatment") {
        push(&mut hints, "Treatment-specific public pages");
        push(&mut hints, "Service positioning and AI grounding");
    }
    if key.contains("license") || key.contains("compliance") || key.contains("warranty") {
        push(&mut hints, "Trust and compliance sections");
        push(&mut hints, "Admin profile and operational records");
    }
    if key.contains("booking")
        || key.contains("reservation")
        || key.contains("scheduling")
        || key.contains("portal")
        || key == "walk_ins"
    {
        push(&mut hints, "Primary CTA labels and links");
        push(&mut hints, "Booking or scheduling module behavior");
        push(&mut hints, "Customer journey setup");
    }
    if key == "members" || key.contains("member") || key.contains("team") || key.contains("teacher")
    {
        push(&mut hints, "Directory and profile sections");
        push(&mut hints, "About/team presentation");
        push(&mut hints, "Portal or collaboration setup");
    }
    if key == "bio" || key == "story" || key == "mission" || key == "about" {
        push(&mut hints, "About page and story blocks");
        push(&mut hints, "Homepage supporting copy");
        push(&mut hints, "AI grounding and tone");
    }
    if key == "youtube" || key == "instagram" || key == "tiktok" || key.contains("social") {
        push(&mut hints, "Social/profile link sections");
        push(&mut hints, "Creator/profile pages");
    }
    if key.contains("fundraising") || key == "donations" {
        push(&mut hints, "Fundraising or giving widgets");
        push(&mut hints, "Campaign-ready pages and CTAs");
    }
    if key.contains("rsvp") || key.contains("guest") || key.contains("vendor") {
        push(&mut hints, "Event and RSVP flows");
        push(&mut hints, "Guest/vendor planning pages");
    }
    if key.contains("inventory")
        || key.contains("equipment")
        || key.contains("gear")
        || key.contains("supply")
        || key.contains("packing")
    {
        push(&mut hints, "Inventory or shared-resource features");
        push(&mut hints, "Planning workflows and starter tools");
    }
    if key.contains("frequency") || key.contains("calendar") || key.contains("events") {
        push(&mut hints, "Calendar defaults and reminders");
        push(&mut hints, "Event pages and recurring flows");
    }
    if site_slug == "creator" {
        push(&mut hints, "Creator profile and public brand setup");
    }
    if site_slug == "blog" {
        push(&mut hints, "Blog positioning and starter content structure");
    }
    if step_id == "welcome" || step_id == "contact" || step_id == "service_area" {
        push(&mut hints, "Core site setup and starter personalization");
    }
    if hints.is_empty() {
        push(&mut hints, "Starter pages and feature defaults");
        push(&mut hints, "AI-grounded setup and personalization");
    }

    hints
}
