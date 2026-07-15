//! CompanyImportJob aggregate — tracks data import attempts from external sources.
//!
//! Each import job records the source, extracted data, and status through the
//! review-before-apply workflow:
//!   pending -> extracting -> review -> applied | failed
//!
//! Admin reviews extracted data before merging it into the CompanyProfile.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for import jobs in the ForgeJournal.
pub const AGG_IMPORT: &str = "CompProf:Import";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

// ── Import job aggregate ─────────────────────────────────────────────

/// A record of an import attempt from an external source.
///
/// Status flow:
/// - `pending`    — job created, extraction not yet started
/// - `extracting` — HTTP fetch / AI processing in progress
/// - `review`     — data extracted, waiting for admin review
/// - `applied`    — admin approved and merged into CompanyProfile
/// - `failed`     — extraction or processing failed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyImportJob {
    /// ULID-based unique identifier.
    pub id: String,
    /// Source type: "google_business", "facebook", "website", "conversation", "questionnaire".
    pub source: String,
    /// URL that was scraped (for URL-based imports).
    pub source_url: Option<String>,
    /// Current status.
    pub status: String,
    /// Extracted partial CompanyProfile fields as a JSON object.
    /// Keys match CompanyProfile field names; admin reviews before applying.
    pub extracted_data: Option<serde_json::Value>,
    /// Error message if status is "failed".
    pub error: Option<String>,
    /// Unix timestamp (seconds) when the job was created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when extraction completed (or failed).
    pub completed_at: u64,
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load all non-deleted import jobs, most recent first.
pub fn load_all_imports(j: &luperiq_forge::ForgeJournal) -> Vec<CompanyImportJob> {
    let mut jobs: Vec<CompanyImportJob> = j
        .latest_by_aggregate_type(AGG_IMPORT)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<CompanyImportJob>(&e.payload).ok())
        .collect();
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    jobs
}

/// Load a single import job by ID.
pub fn load_import(j: &luperiq_forge::ForgeJournal, id: &str) -> Option<CompanyImportJob> {
    j.get_latest(AGG_IMPORT, id)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<CompanyImportJob>(&e.payload).ok())
}

/// Persist an import job to the journal.
pub fn persist_import(
    j: &mut luperiq_forge::ForgeJournal,
    job: &CompanyImportJob,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(job).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_IMPORT, &job.id, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

/// Tombstone-delete an import job.
pub fn delete_import(j: &mut luperiq_forge::ForgeJournal, id: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_IMPORT, id, TOMBSTONE.to_vec());
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
