//! WordPress config import -- convert WP Theme Studio JSON export to native
//! ForgeJournal aggregates.
//!
//! The WordPress Theme Studio exports a single JSON blob containing profiles,
//! popup templates, schedules, header/footer/sidebar templates, and block
//! presets. This module deserializes that blob and writes each sub-object as
//! an individual ForgeJournal aggregate, skipping any sub-object that fails
//! to deserialize (counted as a warning).

use luperiq_forge::{ApexEvent, ForgeJournal};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

use super::config::*;

// ── Public types ──────────────────────────────────────────────────────────

/// Summary of a WordPress config import operation.
#[derive(Debug, Clone, Serialize)]
pub struct ImportSummary {
    pub profiles: usize,
    pub popups: usize,
    pub schedules: usize,
    pub header_templates: usize,
    pub footer_templates: usize,
    pub sidebar_templates: usize,
    pub block_presets: usize,
    pub warnings: Vec<String>,
}

// ── Import entry point ───────────────────────────────────────────────────

/// Import a WordPress Theme Studio JSON export into the ForgeJournal.
///
/// Parses the JSON string and writes each discovered entity as a separate
/// aggregate event. Sub-objects that fail to deserialize are skipped and
/// recorded as warnings in the returned summary.
pub fn import_wordpress_config(
    journal: &mut ForgeJournal,
    json: &str,
) -> Result<ImportSummary, String> {
    let root: Value = serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let obj = root
        .as_object()
        .ok_or_else(|| "Top-level value must be a JSON object".to_string())?;

    let mut summary = ImportSummary {
        profiles: 0,
        popups: 0,
        schedules: 0,
        header_templates: 0,
        footer_templates: 0,
        sidebar_templates: 0,
        block_presets: 0,
        warnings: vec![],
    };

    // ── 1. Meta aggregate (active_profile + active_by_theme) ─────────
    {
        let meta = ThemeStudioMeta {
            active_profile: obj
                .get("active_profile")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            active_by_theme: obj
                .get("active_by_theme")
                .and_then(|v| serde_json::from_value::<HashMap<String, String>>(v.clone()).ok())
                .unwrap_or_default(),
        };
        let bytes = serde_json::to_vec(&meta).map_err(|e| format!("Meta serialization failed: {e}"))?;
        let _ = journal.append(ApexEvent::new(AGG_META, "global", bytes));
    }

    // ── 2. Profiles ──────────────────────────────────────────────────
    if let Some(profiles) = obj.get("profiles").and_then(|v| v.as_object()) {
        for (slug, value) in profiles {
            match serde_json::from_value::<Profile>(value.clone()) {
                Ok(profile) => {
                    let bytes = serde_json::to_vec(&profile).map_err(|e| format!("Profile serialization failed: {e}"))?;
                    let _ = journal.append(ApexEvent::new(AGG_PROFILE, slug.as_str(), bytes));
                    summary.profiles += 1;
                }
                Err(e) => {
                    summary.warnings.push(format!("Profile '{}': {}", slug, e));
                }
            }
        }
    }

    // ── 3. Popup templates ───────────────────────────────────────────
    if let Some(popups) = obj.get("popup_templates").and_then(|v| v.as_object()) {
        for (key, value) in popups {
            match serde_json::from_value::<PopupTemplate>(value.clone()) {
                Ok(popup) => {
                    let bytes = serde_json::to_vec(&popup).map_err(|e| format!("Popup serialization failed: {e}"))?;
                    let _ = journal.append(ApexEvent::new(AGG_POPUP, key.as_str(), bytes));
                    summary.popups += 1;
                }
                Err(e) => {
                    summary.warnings.push(format!("Popup '{}': {}", key, e));
                }
            }
        }
    }

    // ── 4. Schedules ─────────────────────────────────────────────────
    if let Some(schedules) = obj.get("schedules").and_then(|v| v.as_array()) {
        for (i, value) in schedules.iter().enumerate() {
            match serde_json::from_value::<Schedule>(value.clone()) {
                Ok(schedule) => {
                    let id = ulid::Ulid::new().to_string();
                    let bytes = serde_json::to_vec(&schedule).map_err(|e| format!("Schedule serialization failed: {e}"))?;
                    let _ = journal.append(ApexEvent::new(AGG_SCHEDULE, &id, bytes));
                    summary.schedules += 1;
                }
                Err(e) => {
                    summary.warnings.push(format!("Schedule [{}]: {}", i, e));
                }
            }
        }
    }

    // ── 5. Header templates ──────────────────────────────────────────
    if let Some(headers) = obj.get("header_templates").and_then(|v| v.as_object()) {
        for (key, value) in headers {
            match serde_json::from_value::<Vec<Row>>(value.clone()) {
                Ok(rows) => {
                    let bytes = serde_json::to_vec(&rows).map_err(|e| format!("Header template serialization failed: {e}"))?;
                    let agg = format!("{}:header", AGG_TEMPLATE);
                    let _ = journal.append(ApexEvent::new(&agg, key.as_str(), bytes));
                    summary.header_templates += 1;
                }
                Err(e) => {
                    summary
                        .warnings
                        .push(format!("Header template '{}': {}", key, e));
                }
            }
        }
    }

    // ── 6. Footer templates ──────────────────────────────────────────
    if let Some(footers) = obj.get("footer_templates").and_then(|v| v.as_object()) {
        for (key, value) in footers {
            match serde_json::from_value::<Vec<Row>>(value.clone()) {
                Ok(rows) => {
                    let bytes = serde_json::to_vec(&rows).map_err(|e| format!("Footer template serialization failed: {e}"))?;
                    let agg = format!("{}:footer", AGG_TEMPLATE);
                    let _ = journal.append(ApexEvent::new(&agg, key.as_str(), bytes));
                    summary.footer_templates += 1;
                }
                Err(e) => {
                    summary
                        .warnings
                        .push(format!("Footer template '{}': {}", key, e));
                }
            }
        }
    }

    // ── 7. Sidebar templates ─────────────────────────────────────────
    if let Some(sidebars) = obj.get("sidebar_templates").and_then(|v| v.as_object()) {
        for (key, value) in sidebars {
            match serde_json::from_value::<Vec<Row>>(value.clone()) {
                Ok(rows) => {
                    let bytes = serde_json::to_vec(&rows).map_err(|e| format!("Sidebar template serialization failed: {e}"))?;
                    let agg = format!("{}:sidebar", AGG_TEMPLATE);
                    let _ = journal.append(ApexEvent::new(&agg, key.as_str(), bytes));
                    summary.sidebar_templates += 1;
                }
                Err(e) => {
                    summary
                        .warnings
                        .push(format!("Sidebar template '{}': {}", key, e));
                }
            }
        }
    }

    // ── 8. Block presets ─────────────────────────────────────────────
    if let Some(presets) = obj.get("block_presets").and_then(|v| v.as_array()) {
        for (i, value) in presets.iter().enumerate() {
            match serde_json::from_value::<BlockPreset>(value.clone()) {
                Ok(preset) => {
                    let id = ulid::Ulid::new().to_string();
                    let bytes = serde_json::to_vec(&preset).map_err(|e| format!("BlockPreset serialization failed: {e}"))?;
                    let _ = journal.append(ApexEvent::new(AGG_BLOCK_PRESET, &id, bytes));
                    summary.block_presets += 1;
                }
                Err(e) => {
                    summary
                        .warnings
                        .push(format!("Block preset [{}]: {}", i, e));
                }
            }
        }
    }

    Ok(summary)
}

// ── Tests ────────────────────────────────────────────────────────────────

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
    fn import_minimal_config() {
        let (mut j, _dir) = tmp_journal();
        let json = r#"{
            "active_profile": "cool_slate",
            "active_by_theme": {"luperiq-theme": "default"},
            "profiles": {},
            "popup_templates": {},
            "schedules": [],
            "header_templates": {},
            "footer_templates": {},
            "sidebar_templates": {},
            "block_presets": []
        }"#;

        let result = import_wordpress_config(&mut j, json);
        assert!(result.is_ok());
        let summary = result.unwrap();
        assert_eq!(summary.profiles, 0);
        assert_eq!(summary.popups, 0);
        assert_eq!(summary.schedules, 0);
        assert!(summary.warnings.is_empty());

        // Check meta was written
        let meta_event = j.get_latest(AGG_META, "global").expect("meta should exist");
        let meta: ThemeStudioMeta = serde_json::from_slice(&meta_event.payload).unwrap();
        assert_eq!(meta.active_profile, "cool_slate");
    }

    #[test]
    fn import_with_profiles() {
        let (mut j, _dir) = tmp_journal();
        let json = r#"{
            "active_profile": "default",
            "active_by_theme": {},
            "profiles": {
                "default": {
                    "label": "Default",
                    "tokens": {}
                }
            },
            "popup_templates": {},
            "schedules": [],
            "header_templates": {},
            "footer_templates": {},
            "sidebar_templates": {},
            "block_presets": []
        }"#;

        let summary = import_wordpress_config(&mut j, json).unwrap();
        assert_eq!(summary.profiles, 1);

        let event = j
            .get_latest(AGG_PROFILE, "default")
            .expect("profile should exist");
        let profile: Profile = serde_json::from_slice(&event.payload).unwrap();
        assert_eq!(profile.label, "Default");
    }

    #[test]
    fn import_skips_bad_profiles_with_warning() {
        let (mut j, _dir) = tmp_journal();
        let json = r#"{
            "active_profile": "default",
            "active_by_theme": {},
            "profiles": {
                "bad_profile": "not an object"
            },
            "popup_templates": {},
            "schedules": [],
            "header_templates": {},
            "footer_templates": {},
            "sidebar_templates": {},
            "block_presets": []
        }"#;

        let summary = import_wordpress_config(&mut j, json).unwrap();
        assert_eq!(summary.profiles, 0);
        assert_eq!(summary.warnings.len(), 1);
        assert!(summary.warnings[0].contains("bad_profile"));
    }

    #[test]
    fn import_invalid_json_returns_error() {
        let (mut j, _dir) = tmp_journal();
        let result = import_wordpress_config(&mut j, "not json");
        assert!(result.is_err());
    }
}
