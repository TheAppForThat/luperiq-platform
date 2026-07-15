//! Context assembly — combines all profile data + SEO data into a single
//! generation context for AI content generation.
//!
//! This is the KEY function of the Content Pipeline. It loads:
//! 1. CompanyProfile (singleton "global")
//! 2. IndustryProfile (by company's industry_slug)
//! 3. LocationProfile (for area-specific pages)
//! 4. SeoGuideline (matching topic/page_type + industry)
//! 5. FactPack (matching subject + industry)
//!
//! And assembles them into a single `GenerationContext` that Handlebars templates
//! can render into rich system prompts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::seo_data::{find_fact_pack, find_guideline, FactPack, SeoGuideline};
use luperiq_mod_company_profile::company_profile::profile::load_company_profile;
use luperiq_mod_industry_profile::industry_profile::profile::load_profile_by_slug;
use luperiq_mod_location_profile::location_profile::profile::load_location_by_slug;

// ── GenerationContext ───────────────────────────────────────────────

/// The assembled context for AI content generation.
///
/// All fields are serialized to JSON-compatible values so that Handlebars
/// templates can access them via dotted paths (e.g. `{{company.name}}`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationContext {
    /// CompanyProfile data (serialized to JSON value for Handlebars access).
    pub company: serde_json::Value,
    /// IndustryProfile data (serialized to JSON value for Handlebars access).
    pub industry: serde_json::Value,
    /// LocationProfile data, if this is an area-specific page.
    pub location: Option<serde_json::Value>,
    /// Matching SEO guideline, if one exists.
    pub seo: Option<SeoGuideline>,
    /// Matching fact pack, if one exists.
    pub fact_pack: Option<FactPack>,
    /// Formatted fact pack excerpt for prompt injection.
    pub facts: Option<String>,
    /// Page type: "homepage", "service-page", "area-page", etc.
    pub page_type: String,
    /// The specific target (service name, area slug, blog topic, etc.)
    pub target: String,
    /// Additional context variables for template rendering.
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

// ── Page types that involve locations ───────────────────────────────

/// Page types that should pull in LocationProfile data.
const LOCATION_PAGE_TYPES: &[&str] = &["area-page"];

/// Check if a page type involves location-specific content.
fn is_location_page(page_type: &str) -> bool {
    LOCATION_PAGE_TYPES.contains(&page_type)
}

// ── Context assembly ────────────────────────────────────────────────

/// Assemble a complete generation context from journal data.
///
/// This is the main entry point for context assembly. It loads all relevant
/// profile data and SEO reference material for the given page type and target.
///
/// # Arguments
///
/// * `journal` - The ForgeJournal to read data from.
/// * `page_type` - The type of page being generated.
/// * `target_slug` - The specific thing being generated (service slug, area slug, etc.)
///
/// # Returns
///
/// `Ok(GenerationContext)` on success, `Err(String)` if required data is missing.
pub fn assemble_context(
    journal: &luperiq_forge::ForgeJournal,
    page_type: &str,
    target_slug: &str,
) -> Result<GenerationContext, String> {
    // 1. Load CompanyProfile (required)
    let company = load_company_profile(journal).ok_or_else(|| {
        "Company profile not found. Create one before generating content.".to_string()
    })?;

    let industry_slug = company.industry_slug.clone();

    let company_json = serde_json::to_value(&company)
        .map_err(|e| format!("Failed to serialize company profile: {}", e))?;

    // 2. Load IndustryProfile by company's industry_slug (optional but recommended)
    let industry_json = if !industry_slug.is_empty() {
        match load_profile_by_slug(journal, &industry_slug) {
            Some(profile) => serde_json::to_value(&profile)
                .unwrap_or_else(|_| serde_json::json!({"name": industry_slug})),
            None => serde_json::json!({"name": industry_slug}),
        }
    } else {
        serde_json::json!({"name": "General"})
    };

    // 3. If page type involves a location, load LocationProfile
    let location_json = if is_location_page(page_type) {
        load_location_by_slug(journal, target_slug).and_then(|loc| serde_json::to_value(&loc).ok())
    } else {
        None
    };

    // 4. Find matching SeoGuideline
    let seo_guideline = find_guideline(journal, page_type, target_slug, &industry_slug);

    // 5. Find matching FactPack
    let fact_pack = find_fact_pack(journal, target_slug, &industry_slug);

    // 6. Format fact pack data for prompt injection
    let facts_text = fact_pack.as_ref().map(|fp| format_fact_pack(fp));

    Ok(GenerationContext {
        company: company_json,
        industry: industry_json,
        location: location_json,
        seo: seo_guideline,
        fact_pack: fact_pack.clone(),
        facts: facts_text,
        page_type: page_type.to_string(),
        target: target_slug.to_string(),
        extra: HashMap::new(),
    })
}

/// Assemble context with explicit location slug override.
///
/// Used when the location is not implied by the target slug (e.g., generating
/// a service page that's location-specific).
pub fn assemble_context_with_location(
    journal: &luperiq_forge::ForgeJournal,
    page_type: &str,
    target_slug: &str,
    location_slug: &str,
) -> Result<GenerationContext, String> {
    let mut ctx = assemble_context(journal, page_type, target_slug)?;

    // Override location data with explicit location
    if !location_slug.is_empty() {
        ctx.location = load_location_by_slug(journal, location_slug)
            .and_then(|loc| serde_json::to_value(&loc).ok());
    }

    Ok(ctx)
}

// ── Fact pack formatting ────────────────────────────────────────────

/// Format a FactPack's data into a human-readable string for prompt injection.
///
/// Extracts the most useful information from the flexible JSON structure
/// and formats it as a reference section that the AI can use.
fn format_fact_pack(fp: &FactPack) -> String {
    let mut sections = Vec::new();

    sections.push(format!("### {} ({})", fp.title, fp.subject_type));

    // Format the main data object
    if let Some(obj) = fp.data.as_object() {
        for (key, value) in obj {
            let label = key.replace('_', " ");
            match value {
                serde_json::Value::String(s) => {
                    sections.push(format!("- {}: {}", label, s));
                }
                serde_json::Value::Number(n) => {
                    sections.push(format!("- {}: {}", label, n));
                }
                serde_json::Value::Bool(b) => {
                    sections.push(format!("- {}: {}", label, b));
                }
                serde_json::Value::Array(arr) => {
                    let items: Vec<String> = arr
                        .iter()
                        .filter_map(|v| match v {
                            serde_json::Value::String(s) => Some(s.clone()),
                            other => Some(other.to_string()),
                        })
                        .collect();
                    if !items.is_empty() {
                        sections.push(format!("- {}: {}", label, items.join(", ")));
                    }
                }
                serde_json::Value::Object(inner) => {
                    sections.push(format!("#### {}", label));
                    for (k, v) in inner {
                        let inner_label = k.replace('_', " ");
                        match v {
                            serde_json::Value::String(s) => {
                                sections.push(format!("  - {}: {}", inner_label, s));
                            }
                            serde_json::Value::Number(n) => {
                                sections.push(format!("  - {}: {}", inner_label, n));
                            }
                            _ => {
                                sections.push(format!("  - {}: {}", inner_label, v));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    } else if !fp.data.is_null() {
        sections.push(format!("{}", fp.data));
    }

    // Add source citations
    if !fp.sources.is_empty() {
        sections.push(String::new());
        sections.push("Sources:".to_string());
        for src in &fp.sources {
            let url_part = src
                .url
                .as_ref()
                .map(|u| format!(" ({})", u))
                .unwrap_or_default();
            sections.push(format!("- [{}] {}{}", src.code, src.citation, url_part));
        }
    }

    sections.join("\n")
}

/// Convert a GenerationContext to a serde_json::Value for Handlebars registration.
///
/// This flattens the context into a single JSON object that Handlebars can
/// traverse with dotted paths.
pub fn context_to_json(ctx: &GenerationContext) -> serde_json::Value {
    serde_json::json!({
        "company": ctx.company,
        "industry": ctx.industry,
        "location": ctx.location.clone().unwrap_or(serde_json::Value::Null),
        "seo": ctx.seo.as_ref().map(|s| serde_json::to_value(s).unwrap_or(serde_json::Value::Null)),
        "facts": ctx.facts.clone().unwrap_or_default(),
        "page_type": ctx.page_type,
        "target": ctx.target,
        "extra": ctx.extra,
    })
}
