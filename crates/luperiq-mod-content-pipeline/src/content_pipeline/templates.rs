//! ContentTemplate aggregate — prompt templates for AI content generation.
//!
//! Each template defines the system prompt and section-level user messages
//! for a specific page type (homepage, service-page, area-page, etc.).
//! Templates use Handlebars syntax for variable substitution with data
//! from CompanyProfile, IndustryProfile, LocationProfile, and SEO data.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for content templates in the ForgeJournal.
pub const AGG_TEMPLATE: &str = "CntPipe:Template";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Primary aggregate ────────────────────────────────────────────────

/// A prompt template that defines how to generate content for a page type.
///
/// The `prompt_template` field is a Handlebars template for the system prompt.
/// Each `section_prompts` entry is a user message template that generates one
/// section of the final page content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentTemplate {
    pub id: String,
    /// Page type this template is for: "homepage", "about", "service-page",
    /// "equipment-page", "area-page", "blog-post".
    #[serde(default)]
    pub page_type: String,
    /// Optional industry slug restriction. Empty = universal template.
    #[serde(default)]
    pub industry_slug: String,
    /// Handlebars template for the system prompt that sets up AI context.
    #[serde(default)]
    pub prompt_template: String,
    /// Per-section user message templates. Each generates one content section.
    #[serde(default)]
    pub section_prompts: Vec<String>,
    /// Whether this template is active and available for use.
    #[serde(default = "default_true")]
    pub active: bool,
    /// Unix timestamp when created.
    #[serde(default)]
    pub created_at: u64,
}

fn default_true() -> bool {
    true
}

impl Default for ContentTemplate {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: String::new(),
            page_type: String::new(),
            industry_slug: String::new(),
            prompt_template: String::new(),
            section_prompts: Vec::new(),
            active: true,
            created_at: now,
        }
    }
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load all non-deleted content templates from the journal.
pub fn load_all_templates(j: &luperiq_forge::ForgeJournal) -> Vec<ContentTemplate> {
    j.latest_by_aggregate_type(AGG_TEMPLATE)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentTemplate>(&e.payload).ok())
        .collect()
}

/// Load a single template by ID.
pub fn load_template(j: &luperiq_forge::ForgeJournal, id: &str) -> Option<ContentTemplate> {
    j.get_latest(AGG_TEMPLATE, id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<ContentTemplate>(&e.payload).ok())
}

/// Find the best matching template for a page type and optional industry.
///
/// Priority: industry-specific template > universal template.
pub fn find_template(
    j: &luperiq_forge::ForgeJournal,
    page_type: &str,
    industry_slug: &str,
) -> Option<ContentTemplate> {
    let all = load_all_templates(j);
    let matching: Vec<&ContentTemplate> = all
        .iter()
        .filter(|t| t.active && t.page_type == page_type)
        .collect();

    // Prefer industry-specific template
    if !industry_slug.is_empty() {
        if let Some(t) = matching.iter().find(|t| t.industry_slug == industry_slug) {
            return Some((*t).clone());
        }
    }

    // Fall back to universal template (empty industry_slug)
    matching
        .iter()
        .find(|t| t.industry_slug.is_empty())
        .map(|t| (*t).clone())
}

/// Persist a content template to the journal.
pub fn persist_template(
    j: &mut luperiq_forge::ForgeJournal,
    template: &ContentTemplate,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(template).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_TEMPLATE, &template.id, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete a content template by ID.
pub fn delete_template(j: &mut luperiq_forge::ForgeJournal, id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_TEMPLATE, id, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Seed templates ──────────────────────────────────────────────────

/// Create default universal content templates for all standard page types.
pub fn seed_default_templates(j: &mut luperiq_forge::ForgeJournal) -> Result<usize, String> {
    let defaults = vec![
        ContentTemplate {
            id: "tpl-homepage".into(),
            page_type: "homepage".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_HOMEPAGE.into(),
            section_prompts: vec![
                "Write the hero section for {{company.name}}'s homepage. Include a compelling headline that incorporates their tagline \"{{company.tagline}}\", a brief value proposition mentioning {{company.years_in_business}} years in business, and a strong call-to-action. Output as HTML with semantic tags.".into(),
                "Write 3-4 service highlight cards for {{company.name}}. Each card should have an h3, a short description, and mention the service area: {{company.service_area_description}}. Output as HTML.".into(),
                "Write a trust/credibility section for {{company.name}} including certifications, license numbers, and 1-2 review highlights. Output as HTML.".into(),
            ],
            active: true,
            created_at: 0,
        },
        ContentTemplate {
            id: "tpl-about".into(),
            page_type: "about".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_ABOUT.into(),
            section_prompts: vec![
                "Write the company story section for {{company.name}}. Expand on their story: \"{{company.story}}\". Mention the owner {{company.owner_name}} and their service philosophy. Output as HTML.".into(),
                "Write a \"Why Choose Us\" section listing {{company.name}}'s unique selling points and certifications. Output as HTML with a bulleted list.".into(),
            ],
            active: true,
            created_at: 0,
        },
        ContentTemplate {
            id: "tpl-service-page".into(),
            page_type: "service-page".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_SERVICE.into(),
            section_prompts: vec![
                "Write a detailed service page for \"{{target}}\" by {{company.name}} in {{company.city}}, {{company.state}}. Include what the service entails, benefits to the customer, and why {{company.name}} is the best choice. Output as HTML with h2 headings.".into(),
                "Write a FAQ section with 4-5 common questions about \"{{target}}\" that customers in {{company.service_area_description}} might ask. Output as HTML with details/summary elements.".into(),
            ],
            active: true,
            created_at: 0,
        },
        ContentTemplate {
            id: "tpl-equipment-page".into(),
            page_type: "equipment-page".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_SERVICE.into(),
            section_prompts: vec![
                "Write a detailed equipment/product page about \"{{target}}\" for {{company.name}}. Include what it is, how it works, benefits, and maintenance tips. Output as HTML with h2 headings.".into(),
                "Write a section about when to call {{company.name}} for help with \"{{target}}\". Include warning signs and the benefits of professional service. Output as HTML.".into(),
            ],
            active: true,
            created_at: 0,
        },
        ContentTemplate {
            id: "tpl-area-page".into(),
            page_type: "area-page".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_AREA.into(),
            section_prompts: vec![
                "Write a geo-targeted landing page for {{company.name}}'s services in {{location.city}}, {{location.state}}. Mention local neighborhoods, the area's characteristics, and why {{company.name}} is the best local provider. Output as HTML with h2 headings.".into(),
                "Write a section about common {{industry.name}} needs specific to {{location.city}} based on the local climate and housing stock. Output as HTML.".into(),
            ],
            active: true,
            created_at: 0,
        },
        ContentTemplate {
            id: "tpl-blog-post".into(),
            page_type: "blog-post".into(),
            industry_slug: String::new(),
            prompt_template: SYSTEM_BLOG.into(),
            section_prompts: vec![
                "Write a comprehensive blog post about \"{{target}}\" for {{company.name}}'s website. The post should educate homeowners, establish expertise, and naturally include relevant keywords. Target 800-1200 words. Output as HTML with h2/h3 headings, paragraphs, and a conclusion with a call to action.".into(),
            ],
            active: true,
            created_at: 0,
        },
    ];

    let mut count = 0;
    for mut tpl in defaults {
        if load_template(j, &tpl.id).is_none() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            tpl.created_at = now;
            persist_template(j, &tpl)?;
            count += 1;
        }
    }
    Ok(count)
}

// ── System prompt templates ─────────────────────────────────────────

const SYSTEM_HOMEPAGE: &str = r#"You are writing website content for {{company.name}}, a {{industry.name}} company based in {{company.city}}, {{company.state}}.

## Brand Voice
- Tone: {{company.tone}}
{{#each company.voice_notes}}- {{this}}
{{/each}}

## Business Identity
- Tagline: "{{company.tagline}}"
- Years in business: {{company.years_in_business}}
- Service area: {{company.service_area_description}}
- Phone: {{company.phone}}

{{#if seo}}## Content Requirements
- Target word count: {{seo.content_structure.word_count_min}}-{{seo.content_structure.word_count_max}}
- Include {{seo.content_structure.heading_count_min}}-{{seo.content_structure.heading_count_max}} headings

{{#if seo.term_frequencies}}## Key Terms to Use
{{#each seo.term_frequencies}}- "{{this.term}}": use {{this.min_count}}-{{this.max_count}} times
{{/each}}{{/if}}

{{#if seo.fact_groups}}## Facts to Include
{{#each seo.fact_groups}}### {{this.topic}}
{{#each this.facts}}- {{this}}
{{/each}}
{{/each}}{{/if}}{{/if}}

{{#if facts}}## Reference Data
{{{facts}}}
{{/if}}

Output clean, semantic HTML. Do not include <html>, <head>, or <body> tags — only the content sections."#;

const SYSTEM_ABOUT: &str = r#"You are writing the About page for {{company.name}}, a {{industry.name}} company.

## Brand Voice
- Tone: {{company.tone}}
{{#each company.voice_notes}}- {{this}}
{{/each}}

## Company Info
- Owner: {{company.owner_name}}
- Philosophy: "{{company.service_philosophy}}"
- Story: "{{company.story}}"
- Certifications: {{#each company.certifications}}{{this}}, {{/each}}

{{#if seo}}## Content Requirements
- Target word count: {{seo.content_structure.word_count_min}}-{{seo.content_structure.word_count_max}}
{{#if seo.term_frequencies}}## Key Terms to Use
{{#each seo.term_frequencies}}- "{{this.term}}": use {{this.min_count}}-{{this.max_count}} times
{{/each}}{{/if}}{{/if}}

Output clean, semantic HTML. Do not include <html>, <head>, or <body> tags."#;

const SYSTEM_SERVICE: &str = r#"You are writing a service page for {{company.name}}, a {{industry.name}} company in {{company.city}}, {{company.state}}.

## Brand Voice
- Tone: {{company.tone}}
{{#each company.voice_notes}}- {{this}}
{{/each}}

## Industry Context
- Industry: {{industry.name}}
- Common terminology: {{#each industry.terminology}}{{this.term}} ({{this.definition}}), {{/each}}

## Unique Selling Points
{{#each company.unique_selling_points}}- {{this}}
{{/each}}

{{#if seo}}## Content Requirements
- Target word count: {{seo.content_structure.word_count_min}}-{{seo.content_structure.word_count_max}}
- Include {{seo.content_structure.heading_count_min}}-{{seo.content_structure.heading_count_max}} headings

{{#if seo.term_frequencies}}## Key Terms to Use
{{#each seo.term_frequencies}}- "{{this.term}}": use {{this.min_count}}-{{this.max_count}} times
{{/each}}{{/if}}

{{#if seo.fact_groups}}## Facts to Include
{{#each seo.fact_groups}}### {{this.topic}}
{{#each this.facts}}- {{this}}
{{/each}}
{{/each}}{{/if}}{{/if}}

{{#if facts}}## Reference Data
{{{facts}}}
{{/if}}

Output clean, semantic HTML. Do not include <html>, <head>, or <body> tags."#;

const SYSTEM_AREA: &str = r#"You are writing a location-specific landing page for {{company.name}}, a {{industry.name}} company.

## Brand Voice
- Tone: {{company.tone}}

## Location Context
- City: {{location.city}}, {{location.state}}
- Area: {{location.area_description}}
- Neighborhoods: {{#each location.neighborhoods}}{{this}}, {{/each}}
{{#if location.population}}- Population: {{location.population}}{{/if}}
{{#if location.median_home_age}}- Median home age: {{location.median_home_age}} years{{/if}}
{{#if location.climate_zone}}- Climate zone: {{location.climate_zone}}{{/if}}

## Company
- Phone: {{company.phone}}
- Service area: {{company.service_area_description}}

{{#if seo}}## Content Requirements
- Target word count: {{seo.content_structure.word_count_min}}-{{seo.content_structure.word_count_max}}

{{#if seo.term_frequencies}}## Key Terms to Use
{{#each seo.term_frequencies}}- "{{this.term}}": use {{this.min_count}}-{{this.max_count}} times
{{/each}}{{/if}}{{/if}}

{{#if facts}}## Reference Data
{{{facts}}}
{{/if}}

Output clean, semantic HTML. Do not include <html>, <head>, or <body> tags."#;

const SYSTEM_BLOG: &str = r#"You are writing a blog post for {{company.name}}, a {{industry.name}} company in {{company.city}}, {{company.state}}.

## Brand Voice
- Tone: {{company.tone}}
{{#each company.voice_notes}}- {{this}}
{{/each}}

## Audience
- Homeowners and property managers in {{company.service_area_description}}
- Pain points: {{#each industry.customer_pain_points}}{{this}}, {{/each}}

## SEO Goals
- Natural keyword integration
- Authoritative, educational content
- Internal linking opportunities to service pages

{{#if seo}}## Content Requirements
- Target word count: {{seo.content_structure.word_count_min}}-{{seo.content_structure.word_count_max}}

{{#if seo.term_frequencies}}## Key Terms to Use
{{#each seo.term_frequencies}}- "{{this.term}}": use {{this.min_count}}-{{this.max_count}} times
{{/each}}{{/if}}

{{#if seo.fact_groups}}## Facts to Include
{{#each seo.fact_groups}}### {{this.topic}}
{{#each this.facts}}- {{this}}
{{/each}}
{{/each}}{{/if}}{{/if}}

{{#if facts}}## Reference Data
{{{facts}}}
{{/if}}

Output clean, semantic HTML with h2/h3 headings and paragraphs. Do not include <html>, <head>, or <body> tags."#;
