//! Revision tracking -- snapshot storage with auto-pruning.
//!
//! Each revision is stored as a ForgeJournal event under the aggregate key
//! `ThemeStudio:Revision:{entity_type}:{entity_id}` with `aggregate_id` set
//! to the version number (as a string). Max 25 revisions per entity; older
//! ones are tombstoned when the limit is exceeded.

use luperiq_forge::{ApexEvent, ForgeJournal};

use super::config::{Revision, TOMBSTONE};

/// Maximum number of revisions to keep per entity.
const MAX_REVISIONS: usize = 25;

/// Build the aggregate type key for a specific entity's revisions.
fn revision_agg(entity_type: &str, entity_id: &str) -> String {
    format!("ThemeStudio:Revision:{}:{}", entity_type, entity_id)
}

// ── Public API ──────────────────────────────────────────────────────────

/// Save a new revision snapshot. Returns the assigned version number.
///
/// The version is auto-incremented by counting existing (non-tombstoned)
/// revisions. If the count exceeds `MAX_REVISIONS`, the oldest revision
/// is tombstoned.
pub fn save_revision(
    journal: &mut ForgeJournal,
    entity_type: &str,
    entity_id: &str,
    snapshot_json: &str,
) -> u32 {
    let agg = revision_agg(entity_type, entity_id);

    // Count existing live revisions to determine next version
    let existing = load_raw_revisions(journal, &agg);
    let next_version = existing.iter().map(|r| r.version).max().unwrap_or(0) + 1;

    let revision = Revision {
        entity_type: entity_type.into(),
        entity_id: entity_id.into(),
        version: next_version,
        snapshot_json: snapshot_json.into(),
        created_at: now_secs(),
    };

    let bytes = serde_json::to_vec(&revision).expect("Revision serialization");
    let event = ApexEvent::new(&agg, next_version.to_string(), bytes);
    let _ = journal.append(event);

    // Prune if over limit -- tombstone the oldest
    let mut live: Vec<Revision> = load_raw_revisions(journal, &agg);
    if live.len() > MAX_REVISIONS {
        // Sort ascending by version so oldest is first
        live.sort_by_key(|r| r.version);
        let to_prune = live.len() - MAX_REVISIONS;
        for r in live.iter().take(to_prune) {
            let tomb = ApexEvent::new(&agg, r.version.to_string(), TOMBSTONE.to_vec());
            let _ = journal.append(tomb);
        }
    }

    next_version
}

/// List all live revisions for an entity, sorted newest first.
pub fn list_revisions(journal: &ForgeJournal, entity_type: &str, entity_id: &str) -> Vec<Revision> {
    let agg = revision_agg(entity_type, entity_id);
    let mut revisions = load_raw_revisions(journal, &agg);
    revisions.sort_by(|a, b| b.version.cmp(&a.version));
    revisions
}

/// Get a specific revision by version.
pub fn get_revision(
    journal: &ForgeJournal,
    entity_type: &str,
    entity_id: &str,
    version: u32,
) -> Option<Revision> {
    let agg = revision_agg(entity_type, entity_id);
    let event = journal.get_latest(&agg, &version.to_string())?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

// ── Internal helpers ────────────────────────────────────────────────────

/// Load all non-tombstoned revisions for the given aggregate type.
fn load_raw_revisions(journal: &ForgeJournal, agg: &str) -> Vec<Revision> {
    journal
        .latest_by_aggregate_type(agg)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<Revision>(&e.payload).ok())
        .collect()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use luperiq_forge::{DurabilityMode, ForgeJournal};

    fn tmp_journal() -> (ForgeJournal, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let j = ForgeJournal::open(wal, snap, DurabilityMode::Async).unwrap();
        (j, dir)
    }

    #[test]
    fn save_and_list_revisions() {
        let (mut j, _dir) = tmp_journal();

        let v1 = save_revision(&mut j, "profile", "default", r#"{"v":1}"#);
        let v2 = save_revision(&mut j, "profile", "default", r#"{"v":2}"#);
        let v3 = save_revision(&mut j, "profile", "default", r#"{"v":3}"#);

        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
        assert_eq!(v3, 3);

        let list = list_revisions(&j, "profile", "default");
        assert_eq!(list.len(), 3);
        // Newest first
        assert_eq!(list[0].version, 3);
        assert_eq!(list[1].version, 2);
        assert_eq!(list[2].version, 1);
    }

    #[test]
    fn get_specific_revision() {
        let (mut j, _dir) = tmp_journal();
        save_revision(&mut j, "popup", "promo", r#"{"state":"draft"}"#);
        save_revision(&mut j, "popup", "promo", r#"{"state":"published"}"#);

        let r = get_revision(&j, "popup", "promo", 1).expect("version 1 should exist");
        assert!(r.snapshot_json.contains("draft"));

        let r2 = get_revision(&j, "popup", "promo", 2).expect("version 2 should exist");
        assert!(r2.snapshot_json.contains("published"));

        assert!(get_revision(&j, "popup", "promo", 999).is_none());
    }

    #[test]
    fn auto_pruning_at_max() {
        let (mut j, _dir) = tmp_journal();

        // Save MAX_REVISIONS + 3 revisions
        for i in 1..=(MAX_REVISIONS + 3) {
            save_revision(&mut j, "profile", "test", &format!(r#"{{"v":{}}}"#, i));
        }

        let list = list_revisions(&j, "profile", "test");
        assert_eq!(
            list.len(),
            MAX_REVISIONS,
            "should be pruned to MAX_REVISIONS"
        );

        // Oldest versions (1, 2, 3) should have been tombstoned
        assert!(get_revision(&j, "profile", "test", 1).is_none());
        assert!(get_revision(&j, "profile", "test", 2).is_none());
        assert!(get_revision(&j, "profile", "test", 3).is_none());

        // Newest should still exist
        let newest = (MAX_REVISIONS + 3) as u32;
        assert!(get_revision(&j, "profile", "test", newest).is_some());
    }

    #[test]
    fn separate_entities_dont_interfere() {
        let (mut j, _dir) = tmp_journal();
        save_revision(&mut j, "profile", "a", r#"{"a":1}"#);
        save_revision(&mut j, "profile", "b", r#"{"b":1}"#);
        save_revision(&mut j, "popup", "a", r#"{"popup":1}"#);

        assert_eq!(list_revisions(&j, "profile", "a").len(), 1);
        assert_eq!(list_revisions(&j, "profile", "b").len(), 1);
        assert_eq!(list_revisions(&j, "popup", "a").len(), 1);
    }
}
