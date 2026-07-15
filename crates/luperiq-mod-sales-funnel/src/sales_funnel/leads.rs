//! Lead tracking WAL operations — Lead aggregate read/write, funnel stats.

use luperiq_forge::{ApexEvent, ForgeJournal};
use serde::{Deserialize, Serialize};

/// WAL aggregate type for leads.
pub const AGG_LEAD: &str = "SalesPipeline:Lead";

/// A lead record persisted in the WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub lead_id: String,
    pub email: String,
    pub industry_slug: String,
    pub source_page: String,
    pub referrer: String,
    pub utm_source: String,
    pub utm_medium: String,
    pub utm_campaign: String,
    pub stage: String, // discovered | trial_started | trial_paid | converted | churned
    pub stage_timestamps: serde_json::Value,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Funnel stage counts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunnelStats {
    pub discovered: u64,
    pub trial_started: u64,
    pub trial_paid: u64,
    pub converted: u64,
    pub churned: u64,
    pub total: u64,
}

/// Get the latest lead for a given email address.
pub fn get_lead(journal: &ForgeJournal, email: &str) -> Option<Lead> {
    let all = journal.latest_by_aggregate_type(AGG_LEAD);
    let lower = email.to_lowercase();
    all.into_iter()
        .filter_map(|e| serde_json::from_slice::<Lead>(&e.payload).ok())
        .filter(|l| l.email.to_lowercase() == lower)
        .max_by_key(|l| l.created_at)
}

/// List all leads (latest version of each aggregate).
pub fn list_leads(journal: &ForgeJournal) -> Vec<Lead> {
    journal
        .latest_by_aggregate_type(AGG_LEAD)
        .into_iter()
        .filter_map(|e| serde_json::from_slice::<Lead>(&e.payload).ok())
        .collect()
}

/// Compute funnel stage counts from all leads.
pub fn funnel_stats(journal: &ForgeJournal) -> FunnelStats {
    let leads = list_leads(journal);
    let mut stats = FunnelStats {
        discovered: 0,
        trial_started: 0,
        trial_paid: 0,
        converted: 0,
        churned: 0,
        total: leads.len() as u64,
    };
    for lead in &leads {
        match lead.stage.as_str() {
            "discovered" => stats.discovered += 1,
            "trial_started" => stats.trial_started += 1,
            "trial_paid" => stats.trial_paid += 1,
            "converted" => stats.converted += 1,
            "churned" => stats.churned += 1,
            _ => {}
        }
    }
    stats
}

/// Append (or update) a lead event to the WAL.
pub fn write_lead(journal: &mut ForgeJournal, lead: &Lead) -> Result<(), String> {
    let bytes = serde_json::to_vec(lead).map_err(|e| format!("Serialize error: {e}"))?;
    let event = ApexEvent::new(AGG_LEAD, &lead.lead_id, bytes);
    journal
        .append(event)
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}
