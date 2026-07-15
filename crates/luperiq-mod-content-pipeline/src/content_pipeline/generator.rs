//! AI content generator — renders templates with context and calls the AI model.
//!
//! The generator takes a ContentTemplate + GenerationContext, renders the Handlebars
//! system prompt, then iterates through section prompts calling the AI for each one.
//! The final output is assembled HTML with token counts and timing metrics.

use super::ai_client::AiClient;
use super::context::{context_to_json, GenerationContext};
use super::templates::ContentTemplate;
use serde::{Deserialize, Serialize};

// ── Output types ────────────────────────────────────────────────────

/// The result of generating content for a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedContent {
    /// The assembled HTML output from all sections.
    pub html: String,
    /// Total tokens consumed (input + output across all sections).
    pub token_count: u32,
    /// Total generation time in milliseconds.
    pub generation_time_ms: u64,
    /// Which AI model was used.
    pub model_used: String,
    /// Individual section outputs.
    pub sections: Vec<String>,
}

// ── Generator ───────────────────────────────────────────────────────

/// Generate a complete page of content using the AI model.
///
/// # Process
/// 1. Build system prompt by rendering the Handlebars template with context
/// 2. For each section_prompt in the template:
///    a. Render the section prompt template with context
///    b. Call ai_client.generate(system_prompt, rendered_section_prompt)
///    c. Collect section content
/// 3. Assemble sections into full page HTML
/// 4. Return with token counts, timing, model info
///
/// # Arguments
/// * `ai_client` - The AI client to use for generation
/// * `template` - The content template with system + section prompts
/// * `context` - The assembled generation context
/// * `_quality` - Quality level ("quick_draft" or "premium") - used for logging
pub async fn generate_page(
    ai_client: &AiClient,
    template: &ContentTemplate,
    context: &GenerationContext,
    _quality: &str,
) -> Result<GeneratedContent, String> {
    let start = std::time::Instant::now();

    // Build the context JSON for Handlebars
    let ctx_json = context_to_json(context);

    // Render system prompt from Handlebars template
    let system_prompt = render_handlebars(&template.prompt_template, &ctx_json)?;

    // If no section prompts defined, use a default one
    let section_prompts = if template.section_prompts.is_empty() {
        vec![format!(
            "Write content for a {} page about \"{}\".",
            context.page_type, context.target
        )]
    } else {
        template.section_prompts.clone()
    };

    let mut sections = Vec::new();
    let mut total_tokens: u32 = 0;

    // Generate each section
    for section_template in &section_prompts {
        // Render the section prompt with context
        let user_message = render_handlebars(section_template, &ctx_json)?;

        // Call AI
        let response = ai_client
            .generate(&system_prompt, &user_message)
            .await
            .map_err(|e| format!("AI generation failed: {}", e))?;

        total_tokens = total_tokens.saturating_add(response.input_tokens);
        total_tokens = total_tokens.saturating_add(response.output_tokens);
        sections.push(response.content);
    }

    // Assemble sections into a single HTML document
    let html = sections.join("\n\n");

    let elapsed_ms = start.elapsed().as_millis() as u64;

    Ok(GeneratedContent {
        html,
        token_count: total_tokens,
        generation_time_ms: elapsed_ms,
        model_used: ai_client.model().to_string(),
        sections,
    })
}

// ── Handlebars rendering ────────────────────────────────────────────

/// Render a Handlebars template string with the given context data.
///
/// Returns the rendered string, or an error if the template is invalid.
fn render_handlebars(template_str: &str, data: &serde_json::Value) -> Result<String, String> {
    let mut hbs = handlebars::Handlebars::new();
    // Don't HTML-escape output — we want raw content
    hbs.register_escape_fn(handlebars::no_escape);

    hbs.register_template_string("tpl", template_str)
        .map_err(|e| format!("Invalid Handlebars template: {}", e))?;

    hbs.render("tpl", data)
        .map_err(|e| format!("Handlebars render error: {}", e))
}

/// Render a single template string with context (public helper for previewing).
pub fn preview_prompt(template_str: &str, context: &GenerationContext) -> Result<String, String> {
    let ctx_json = context_to_json(context);
    render_handlebars(template_str, &ctx_json)
}
