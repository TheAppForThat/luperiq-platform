//! SEO A/B testing — time-based switching of meta field variants.
//!
//! Uses time-based switching (not visitor-based) because search engines
//! need one consistent version at a time for clean ranking signals.

use serde::{Deserialize, Serialize};

pub(crate) const AGG_SEO_AB_EXPERIMENT: &str = "SeoAbExperiment";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeoAbField {
    Title,
    Description,
    FocusKeyword,
    Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SeoAbStatus {
    Running,
    AwaitingDecision,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoAbSchedule {
    pub period_days: u32,
}

impl Default for SeoAbSchedule {
    fn default() -> Self {
        Self { period_days: 7 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoAbExperiment {
    pub experiment_id: String,
    pub content_id: String,
    pub field: SeoAbField,
    pub variant_a: String,
    pub variant_b: String,
    pub status: SeoAbStatus,
    pub schedule: SeoAbSchedule,
    pub start_date: String,
    pub end_date: String,
    #[serde(default)]
    pub winner: Option<String>,
    #[serde(default)]
    pub winner_confidence: Option<String>,
    pub created_at: u64,
}

/// Determine which variant to serve for a given experiment on a given date.
///
/// Returns "a" or "b" based on how many days have elapsed since start_date
/// relative to the schedule period.
///
/// - Days 0 to period_days-1: variant A
/// - Days period_days to 2*period_days-1: variant B
/// - Days 2*period_days to 3*period_days-1: variant A (cycle continues)
pub fn active_variant<'a>(experiment: &'a SeoAbExperiment, today: &'a str) -> &'a str {
    if experiment.status != SeoAbStatus::Running {
        return "a"; // Default to control if not running
    }

    let start = parse_date(&experiment.start_date);
    let current = parse_date(today);

    if current < start {
        return "a"; // Before experiment starts
    }

    let days_elapsed = (current - start) / 86400;
    let period = experiment.schedule.period_days as i64;
    if period <= 0 {
        return "a";
    }

    let cycle_position = (days_elapsed / period) % 2;
    if cycle_position == 0 {
        "a"
    } else {
        "b"
    }
}

/// Check if an experiment has reached its end date.
pub fn experiment_ended(experiment: &SeoAbExperiment, today: &str) -> bool {
    let end = parse_date(&experiment.end_date);
    let current = parse_date(today);
    current >= end
}

/// Calculate the end_date given start_date and duration_days.
pub fn calculate_end_date(start_date: &str, duration_days: u32) -> String {
    let start = parse_date(start_date);
    let end = start + (duration_days as i64 * 86400);
    format_date(end)
}

/// Get the value to serve for a meta field, considering active experiments.
///
/// If there's an active experiment for this content_id + field, returns the
/// appropriate variant value. Otherwise returns None (use the stored value).
pub fn get_experiment_override(
    experiments: &[SeoAbExperiment],
    content_id: &str,
    field: &SeoAbField,
    today: &str,
) -> Option<String> {
    experiments
        .iter()
        .find(|e| {
            e.content_id == content_id && &e.field == field && e.status == SeoAbStatus::Running
        })
        .map(|e| {
            let variant = active_variant(e, today);
            if variant == "b" {
                e.variant_b.clone()
            } else {
                e.variant_a.clone()
            }
        })
}

// ── Journal helpers ──────────────────────────────────────────────────

pub fn save_experiment(
    journal: &mut luperiq_forge::ForgeJournal,
    experiment: &SeoAbExperiment,
) -> Result<(), String> {
    let payload =
        serde_json::to_vec(experiment).map_err(|e| format!("Serialize experiment: {e}"))?;
    let event =
        luperiq_forge::ApexEvent::new(AGG_SEO_AB_EXPERIMENT, &experiment.experiment_id, payload);
    journal
        .append(event)
        .map_err(|e| format!("Journal append: {e}"))?;
    Ok(())
}

pub(crate) fn load_experiment(
    journal: &luperiq_forge::ForgeJournal,
    experiment_id: &str,
) -> Option<SeoAbExperiment> {
    journal
        .get_latest(AGG_SEO_AB_EXPERIMENT, experiment_id)
        .and_then(|e| serde_json::from_slice(&e.payload).ok())
}

pub(crate) fn list_experiments(
    journal: &luperiq_forge::ForgeJournal,
    status_filter: Option<&SeoAbStatus>,
) -> Vec<SeoAbExperiment> {
    let events = journal.latest_by_aggregate_type(AGG_SEO_AB_EXPERIMENT);
    events
        .into_iter()
        .filter_map(|e| serde_json::from_slice::<SeoAbExperiment>(&e.payload).ok())
        .filter(|exp| {
            if let Some(status) = status_filter {
                &exp.status == status
            } else {
                true
            }
        })
        .collect()
}

/// Get all running experiments (for variant serving in lookup_seo_meta).
pub fn running_experiments(journal: &luperiq_forge::ForgeJournal) -> Vec<SeoAbExperiment> {
    list_experiments(journal, Some(&SeoAbStatus::Running))
}

/// Get content_ids with active experiments (for GSC tracking).
pub fn actively_tested_content_ids(journal: &luperiq_forge::ForgeJournal) -> Vec<String> {
    running_experiments(journal)
        .into_iter()
        .map(|e| e.content_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect()
}

// ── Date helpers (YYYY-MM-DD) ────────────────────────────────────────

fn parse_date(date_str: &str) -> i64 {
    // Parse YYYY-MM-DD to unix timestamp (midnight UTC)
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp())
        .unwrap_or(0)
}

fn format_date(timestamp: i64) -> String {
    chrono::DateTime::from_timestamp(timestamp, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "1970-01-01".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_experiment(start: &str, period_days: u32, duration_days: u32) -> SeoAbExperiment {
        SeoAbExperiment {
            experiment_id: "exp-1".into(),
            content_id: "page-123".into(),
            field: SeoAbField::Title,
            variant_a: "Control Title".into(),
            variant_b: "Challenger Title".into(),
            status: SeoAbStatus::Running,
            schedule: SeoAbSchedule { period_days },
            start_date: start.into(),
            end_date: calculate_end_date(start, duration_days),
            winner: None,
            winner_confidence: None,
            created_at: 1710000000,
        }
    }

    #[test]
    fn test_variant_a_first_week() {
        let exp = make_experiment("2026-03-01", 7, 14);
        assert_eq!(active_variant(&exp, "2026-03-01"), "a"); // Day 0
        assert_eq!(active_variant(&exp, "2026-03-05"), "a"); // Day 4
        assert_eq!(active_variant(&exp, "2026-03-07"), "a"); // Day 6
    }

    #[test]
    fn test_variant_b_second_week() {
        let exp = make_experiment("2026-03-01", 7, 14);
        assert_eq!(active_variant(&exp, "2026-03-08"), "b"); // Day 7
        assert_eq!(active_variant(&exp, "2026-03-12"), "b"); // Day 11
        assert_eq!(active_variant(&exp, "2026-03-14"), "b"); // Day 13
    }

    #[test]
    fn test_variant_before_start_returns_a() {
        let exp = make_experiment("2026-03-10", 7, 14);
        assert_eq!(active_variant(&exp, "2026-03-05"), "a");
    }

    #[test]
    fn test_non_running_returns_a() {
        let mut exp = make_experiment("2026-03-01", 7, 14);
        exp.status = SeoAbStatus::Completed;
        assert_eq!(active_variant(&exp, "2026-03-10"), "a");
    }

    #[test]
    fn test_experiment_ended() {
        let exp = make_experiment("2026-03-01", 7, 14);
        assert!(!experiment_ended(&exp, "2026-03-10")); // Day 9
        assert!(experiment_ended(&exp, "2026-03-15")); // Day 14 = end
        assert!(experiment_ended(&exp, "2026-03-20")); // Past end
    }

    #[test]
    fn test_calculate_end_date() {
        assert_eq!(calculate_end_date("2026-03-01", 14), "2026-03-15");
        assert_eq!(calculate_end_date("2026-03-01", 28), "2026-03-29");
    }

    #[test]
    fn test_get_experiment_override() {
        let exp = make_experiment("2026-03-01", 7, 14);
        let experiments = vec![exp];

        // Day 3 → variant A
        let result =
            get_experiment_override(&experiments, "page-123", &SeoAbField::Title, "2026-03-04");
        assert_eq!(result, Some("Control Title".into()));

        // Day 10 → variant B
        let result =
            get_experiment_override(&experiments, "page-123", &SeoAbField::Title, "2026-03-11");
        assert_eq!(result, Some("Challenger Title".into()));

        // Wrong content_id → None
        let result =
            get_experiment_override(&experiments, "page-999", &SeoAbField::Title, "2026-03-04");
        assert_eq!(result, None);

        // Wrong field → None
        let result = get_experiment_override(
            &experiments,
            "page-123",
            &SeoAbField::Description,
            "2026-03-04",
        );
        assert_eq!(result, None);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let exp = make_experiment("2026-03-01", 7, 14);
        let bytes = serde_json::to_vec(&exp).unwrap();
        let decoded: SeoAbExperiment = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.experiment_id, "exp-1");
        assert_eq!(decoded.field, SeoAbField::Title);
        assert_eq!(decoded.status, SeoAbStatus::Running);
        assert_eq!(decoded.schedule.period_days, 7);
    }
}
