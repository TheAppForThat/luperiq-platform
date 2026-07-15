//! Batch content generation — generate an entire site's worth of content.
//!
//! Determines which pages are needed based on the site's profiles and service catalog,
//! creates ContentJob records for each, and processes them sequentially with delays
//! to avoid overwhelming the AI provider.

use super::ai_client::AiClient;
use luperiq_module_api::SharedJournal;

use super::context::assemble_context;
use super::generator::generate_page;
use super::jobs::{self, ContentJob};
use super::templates;

// ── Batch generation ────────────────────────────────────────────────

/// Generate content for an entire site.
///
/// Determines which pages are needed, creates a ContentJob for each,
/// and processes them sequentially with 2-second delays between calls.
///
/// # Returns
/// A vector of job IDs for tracking progress.
pub async fn generate_entire_site(
    journal: SharedJournal,
    ai_client: &AiClient,
    quality: &str,
) -> Vec<String> {
    // Determine which pages to generate
    let pages = {
        let j = journal.lock().await;
        determine_pages(&j)
    };

    let mut job_ids = Vec::new();

    for (page_type, target_slug) in &pages {
        // Create a pending job
        let job_id = ulid::Ulid::new().to_string();
        let mut job = ContentJob {
            id: job_id.clone(),
            page_type: page_type.clone(),
            target_slug: target_slug.clone(),
            quality_level: quality.to_string(),
            status: "pending".to_string(),
            ..Default::default()
        };

        // Persist the pending job
        {
            let mut j = journal.lock().await;
            if let Err(e) = jobs::persist_job(&mut j, &job) {
                eprintln!(
                    "Failed to create job for {}/{}: {}",
                    page_type, target_slug, e
                );
                continue;
            }
        }
        job_ids.push(job_id.clone());

        // Process the job
        job.status = "generating".to_string();
        job.model_used = ai_client.model().to_string();
        {
            let mut j = journal.lock().await;
            let _ = jobs::persist_job(&mut j, &job);
        }

        // Assemble context and generate
        let result = {
            let j = journal.lock().await;
            let ctx_result = assemble_context(&j, page_type, target_slug);
            let tpl = templates::find_template(&j, page_type, "");
            (ctx_result, tpl)
        };

        match result {
            (Ok(ctx), Some(tpl)) => {
                // Save context for auditing
                job.prompt_context_json = serde_json::to_string(&ctx).unwrap_or_default();

                match generate_page(ai_client, &tpl, &ctx, quality).await {
                    Ok(content) => {
                        job.status = "review".to_string();
                        job.generated_content = content.html;
                        job.token_count = content.token_count;
                        job.generation_time_ms = content.generation_time_ms;
                        job.model_used = content.model_used;
                    }
                    Err(e) => {
                        job.status = "failed".to_string();
                        job.error_message = e;
                    }
                }
            }
            (Err(e), _) => {
                job.status = "failed".to_string();
                job.error_message = format!("Context assembly failed: {}", e);
            }
            (_, None) => {
                job.status = "failed".to_string();
                job.error_message = format!("No template found for page type: {}", page_type);
            }
        }

        // Persist final state
        {
            let mut j = journal.lock().await;
            let _ = jobs::persist_job(&mut j, &job);
        }

        // Delay between generations to avoid rate limiting
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    job_ids
}

// ── Page determination ──────────────────────────────────────────────

/// Determine which pages need to be generated for a full site.
///
/// Returns a list of (page_type, target_slug) tuples.
fn determine_pages(j: &luperiq_forge::ForgeJournal) -> Vec<(String, String)> {
    let mut pages = Vec::new();

    // Core pages every site needs
    pages.push(("homepage".to_string(), "home".to_string()));
    pages.push(("about".to_string(), "about".to_string()));

    // Load company profile for industry context
    let company = luperiq_mod_company_profile::company_profile::profile::load_company_profile(j);
    let industry_slug = company
        .as_ref()
        .map(|c| c.industry_slug.clone())
        .unwrap_or_default();

    // Service pages from industry profile's common services
    if !industry_slug.is_empty() {
        if let Some(industry) =
            luperiq_mod_industry_profile::industry_profile::profile::load_profile_by_slug(
                j,
                &industry_slug,
            )
        {
            for svc in &industry.common_services {
                pages.push(("service-page".to_string(), svc.slug.clone()));
            }
        }
    }

    // Area pages from location profiles
    let locations = luperiq_mod_location_profile::location_profile::profile::load_all_locations(j);
    for loc in &locations {
        if loc.active {
            pages.push(("area-page".to_string(), loc.slug.clone()));
        }
    }

    // Blog posts from customer pain points
    if !industry_slug.is_empty() {
        if let Some(industry) =
            luperiq_mod_industry_profile::industry_profile::profile::load_profile_by_slug(
                j,
                &industry_slug,
            )
        {
            let pain_points: Vec<&String> = industry.customer_pain_points.iter().take(5).collect();
            for point in pain_points {
                // Convert pain point to a slug
                let slug = point
                    .to_lowercase()
                    .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
                    .replace(' ', "-");
                pages.push(("blog-post".to_string(), slug));
            }
        }
    }

    pages
}
