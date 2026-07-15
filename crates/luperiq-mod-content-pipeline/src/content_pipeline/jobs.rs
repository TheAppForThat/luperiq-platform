//! ContentJob aggregate — tracks AI content generation jobs.
//!
//! Each job represents a single content generation request (one page, one blog post, etc.)
//! with lifecycle tracking from pending through generation, review, and publication.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for content jobs in the ForgeJournal.
pub const AGG_JOB: &str = "CntPipe:Job";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Primary aggregate ────────────────────────────────────────────────

/// A single content generation job, tracking the full lifecycle from
/// request through generation, review, and publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentJob {
    pub id: String,
    /// Page type: "homepage", "about", "service-page", "area-page", "blog-post", etc.
    #[serde(default)]
    pub page_type: String,
    /// Target slug — the specific service, area, or topic for this job.
    #[serde(default)]
    pub target_slug: String,
    /// Quality level: "quick_draft" (local model) or "premium" (cloud model).
    #[serde(default)]
    pub quality_level: String,
    /// Which AI model was used for generation.
    #[serde(default)]
    pub model_used: String,
    /// Job status: "pending", "generating", "review", "published", "failed".
    #[serde(default)]
    pub status: String,
    /// Serialized GenerationContext JSON for debugging and auditing.
    #[serde(default)]
    pub prompt_context_json: String,
    /// The generated HTML content.
    #[serde(default)]
    pub generated_content: String,
    /// Total tokens consumed (input + output).
    #[serde(default)]
    pub token_count: u32,
    /// How long generation took in milliseconds.
    #[serde(default)]
    pub generation_time_ms: u64,
    /// Estimated cost in cents.
    #[serde(default)]
    pub cost_cents: u32,
    /// Unix timestamp when the job was created.
    #[serde(default)]
    pub created_at: u64,
    /// Unix timestamp when the content was reviewed.
    #[serde(default)]
    pub reviewed_at: u64,
    /// Unix timestamp when the content was published.
    #[serde(default)]
    pub published_at: u64,
    /// Error message if the job failed.
    #[serde(default)]
    pub error_message: String,
}

impl Default for ContentJob {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: String::new(),
            page_type: String::new(),
            target_slug: String::new(),
            quality_level: "quick_draft".to_string(),
            model_used: String::new(),
            status: "pending".to_string(),
            prompt_context_json: String::new(),
            generated_content: String::new(),
            token_count: 0,
            generation_time_ms: 0,
            cost_cents: 0,
            created_at: now,
            reviewed_at: 0,
            published_at: 0,
            error_message: String::new(),
        }
    }
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load all non-deleted content jobs from the journal.
pub fn load_all_jobs(j: &luperiq_forge::ForgeJournal) -> Vec<ContentJob> {
    j.latest_by_aggregate_type(AGG_JOB)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<ContentJob>(&e.payload).ok())
        .collect()
}

/// Load a single content job by ID.
pub fn load_job(j: &luperiq_forge::ForgeJournal, id: &str) -> Option<ContentJob> {
    j.get_latest(AGG_JOB, id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<ContentJob>(&e.payload).ok())
}

/// Persist a content job to the journal.
pub fn persist_job(j: &mut luperiq_forge::ForgeJournal, job: &ContentJob) -> Result<(), String> {
    let bytes = serde_json::to_vec(job).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_JOB, &job.id, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete a content job by ID.
pub fn delete_job(j: &mut luperiq_forge::ForgeJournal, id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_JOB, id, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
