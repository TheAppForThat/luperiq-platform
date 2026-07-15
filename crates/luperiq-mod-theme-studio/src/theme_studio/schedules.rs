//! Schedule management -- time-based profile/header/footer switching.
//!
//! Schedules let the site owner swap design profiles or toggle header/footer
//! layouts based on day-of-week and time-of-day windows.

use chrono::Local;
use luperiq_forge::{ApexEvent, ForgeJournal};

use super::config::{Schedule, ScheduleMode, ScheduleStatus, AGG_SCHEDULE, TOMBSTONE};

// ── Schedule override result ────────────────────────────────────────────

/// The result of evaluating schedules against the current local time.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleOverride {
    pub mode: ScheduleMode,
    pub target: String,
}

// ── CRUD ────────────────────────────────────────────────────────────────

pub fn get_schedule(journal: &ForgeJournal, id: &str) -> Option<Schedule> {
    let event = journal.get_latest(AGG_SCHEDULE, id)?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

pub fn save_schedule(journal: &mut ForgeJournal, id: &str, schedule: &Schedule) {
    let bytes = serde_json::to_vec(schedule).expect("Schedule serialization");
    let event = ApexEvent::new(AGG_SCHEDULE, id, bytes);
    let _ = journal.append(event);
}

pub fn delete_schedule(journal: &mut ForgeJournal, id: &str) {
    let event = ApexEvent::new(AGG_SCHEDULE, id, TOMBSTONE.to_vec());
    let _ = journal.append(event);
}

pub fn list_schedules(journal: &ForgeJournal) -> Vec<(String, Schedule)> {
    journal
        .latest_by_aggregate_type(AGG_SCHEDULE)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| {
            let sched: Schedule = serde_json::from_slice(&e.payload).ok()?;
            Some((e.aggregate_id.clone(), sched))
        })
        .collect()
}

// ── Evaluation ──────────────────────────────────────────────────────────

/// Check all provided schedules against the current local time and return the
/// first match, if any.
pub fn evaluate_schedules(schedules: &[(String, Schedule)]) -> Option<ScheduleOverride> {
    let now = Local::now();
    evaluate_schedules_at(schedules, &now)
}

/// Testable inner function: evaluate schedules at a specific local datetime.
fn evaluate_schedules_at(
    schedules: &[(String, Schedule)],
    now: &chrono::DateTime<Local>,
) -> Option<ScheduleOverride> {
    let day_name = now.format("%A").to_string().to_lowercase(); // e.g. "monday"
    let current_time = now.format("%H:%M").to_string(); // e.g. "14:30"

    for (_id, schedule) in schedules {
        // Only consider Active schedules
        if schedule.status != ScheduleStatus::Active {
            continue;
        }

        // Check day match
        if !schedule.days.is_empty() {
            let matches_day = schedule.days.iter().any(|d| d.to_lowercase() == day_name);
            if !matches_day {
                continue;
            }
        }

        // Check time range (both must be present for a time constraint)
        if let (Some(ref start), Some(ref end)) = (&schedule.start_time, &schedule.end_time) {
            if !start.is_empty() && !end.is_empty() {
                // Simple lexicographic HH:MM comparison (works for 24h format)
                if current_time < *start || current_time > *end {
                    continue;
                }
            }
        }

        return Some(ScheduleOverride {
            mode: schedule.mode.clone(),
            target: schedule.target.clone(),
        });
    }

    None
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use luperiq_forge::{DurabilityMode, ForgeJournal};

    fn tmp_journal() -> (ForgeJournal, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let j = ForgeJournal::open(wal, snap, DurabilityMode::Async).unwrap();
        (j, dir)
    }

    fn make_schedule(label: &str, days: Vec<&str>, start: &str, end: &str) -> Schedule {
        Schedule {
            label: label.into(),
            status: ScheduleStatus::Active,
            mode: ScheduleMode::Profile,
            target: "dark-mode".into(),
            start_time: if start.is_empty() {
                None
            } else {
                Some(start.into())
            },
            end_time: if end.is_empty() {
                None
            } else {
                Some(end.into())
            },
            days: days.into_iter().map(String::from).collect(),
            enabled: true,
            theme: String::new(),
        }
    }

    #[test]
    fn crud_round_trip() {
        let (mut j, _dir) = tmp_journal();
        let sched = make_schedule("Evening", vec!["monday"], "18:00", "23:59");

        save_schedule(&mut j, "evening", &sched);
        let got = get_schedule(&j, "evening").expect("should exist");
        assert_eq!(got.label, "Evening");
        assert_eq!(got.target, "dark-mode");

        delete_schedule(&mut j, "evening");
        assert!(get_schedule(&j, "evening").is_none());
    }

    #[test]
    fn list_filters_tombstones() {
        let (mut j, _dir) = tmp_journal();
        save_schedule(&mut j, "a", &make_schedule("A", vec![], "", ""));
        save_schedule(&mut j, "b", &make_schedule("B", vec![], "", ""));
        delete_schedule(&mut j, "b");

        let list = list_schedules(&j);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "a");
    }

    #[test]
    fn evaluate_matches_day_and_time() {
        // Simulate a Monday at 14:30
        let dt = Local.with_ymd_and_hms(2026, 2, 23, 14, 30, 0).unwrap(); // 2026-02-23 is Monday

        let schedules = vec![(
            "weekday-afternoon".into(),
            make_schedule("Afternoon", vec!["monday", "tuesday"], "13:00", "17:00"),
        )];

        let result = evaluate_schedules_at(&schedules, &dt);
        assert!(result.is_some());
        let ov = result.unwrap();
        assert_eq!(ov.mode, ScheduleMode::Profile);
        assert_eq!(ov.target, "dark-mode");
    }

    #[test]
    fn evaluate_no_match_wrong_day() {
        // Wednesday at 14:30
        let dt = Local.with_ymd_and_hms(2026, 2, 25, 14, 30, 0).unwrap(); // Wednesday

        let schedules = vec![(
            "monday-only".into(),
            make_schedule("Monday", vec!["monday"], "00:00", "23:59"),
        )];

        let result = evaluate_schedules_at(&schedules, &dt);
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_no_match_outside_time() {
        // Monday at 08:00 but schedule is 13:00-17:00
        let dt = Local.with_ymd_and_hms(2026, 2, 23, 8, 0, 0).unwrap();

        let schedules = vec![(
            "afternoon".into(),
            make_schedule("Afternoon", vec!["monday"], "13:00", "17:00"),
        )];

        let result = evaluate_schedules_at(&schedules, &dt);
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_skips_archived() {
        let dt = Local.with_ymd_and_hms(2026, 2, 23, 14, 30, 0).unwrap();

        let mut sched = make_schedule("Archived", vec!["monday"], "00:00", "23:59");
        sched.status = ScheduleStatus::Archived;

        let schedules = vec![("archived".into(), sched)];

        let result = evaluate_schedules_at(&schedules, &dt);
        assert!(result.is_none());
    }

    #[test]
    fn evaluate_no_time_constraint() {
        // If start_time and end_time are None, match any time on the right day
        let dt = Local.with_ymd_and_hms(2026, 2, 23, 3, 0, 0).unwrap(); // Monday 3am

        let schedules = vec![(
            "all-day".into(),
            make_schedule("AllDay", vec!["monday"], "", ""),
        )];

        let result = evaluate_schedules_at(&schedules, &dt);
        assert!(result.is_some());
    }
}
