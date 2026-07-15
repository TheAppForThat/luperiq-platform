//! WAL persistence for site type catalog.

use super::types::*;
use luperiq_forge::{ApexEvent, ForgeJournal};

fn merge_default_module_additions(def: &mut SiteTypeDefinition, default_def: &SiteTypeDefinition) {
    for module in &default_def.enabled_modules {
        if !def
            .enabled_modules
            .iter()
            .any(|existing| existing == module)
        {
            def.enabled_modules.push(module.clone());
        }
    }
}

/// Append any DefaultPage entries (by slug) present in `default_def` but missing
/// from `def`. Mirrors `merge_default_module_additions` so default_pages stays
/// in sync when new pages are added to the chassis catalog (e.g. HIPAA NPP for
/// medical-office). Existing pages in the WAL are preserved unchanged — only
/// missing slugs are appended.
fn merge_default_page_additions(def: &mut SiteTypeDefinition, default_def: &SiteTypeDefinition) {
    for page in &default_def.default_pages {
        if !def
            .default_pages
            .iter()
            .any(|existing| existing.slug == page.slug)
        {
            def.default_pages.push(page.clone());
        }
    }
}

fn hydrate_site_type_defaults(mut def: SiteTypeDefinition) -> SiteTypeDefinition {
    if let Some(default_def) = super::defaults::all_defaults_ref()
        .iter()
        .find(|candidate| candidate.slug == def.slug)
    {
        let stale_onboarding = default_def.onboarding_steps.len() != def.onboarding_steps.len()
            || default_def
                .onboarding_steps
                .iter()
                .map(|step| (&step.step_id, &step.label))
                .ne(def
                    .onboarding_steps
                    .iter()
                    .map(|step| (&step.step_id, &step.label)));
        merge_default_module_additions(&mut def, default_def);
        merge_default_page_additions(&mut def, default_def);

        if stale_onboarding {
            def.onboarding_steps = default_def.onboarding_steps.clone();
        }

        if matches!(
            def.slug.as_str(),
            "app-publisher" | "church" | "farm" | "wedding"
        ) {
            def.enabled_modules = default_def.enabled_modules.clone();
            def.default_pages = default_def.default_pages.clone();
            def.default_nav_items = default_def.default_nav_items.clone();
            def.homepage_blocks = default_def.homepage_blocks.clone();
            def.description = default_def.description.clone();
            def.default_tagline = default_def.default_tagline.clone();
            def.seo_title_template = default_def.seo_title_template.clone();
            def.seo_description_template = default_def.seo_description_template.clone();
        } else if matches!(
            def.slug.as_str(),
            "cell-phone-repair" | "electronics-repair" | "homeschool" | "insurance"
        ) {
            def.default_pages = default_def.default_pages.clone();
            def.default_nav_items = default_def.default_nav_items.clone();
        } else if def.slug.as_str() == "memorial" {
            def.default_nav_items = default_def.default_nav_items.clone();
            def.homepage_blocks = default_def.homepage_blocks.clone();
        }
    }
    def
}

/// Load a specific site type from WAL by slug.
pub fn load_site_type(journal: &ForgeJournal, slug: &str) -> Option<SiteTypeDefinition> {
    journal
        .get_latest(AGG_SITE_TYPE, slug)
        .and_then(|e| serde_json::from_slice(&e.payload).ok())
        .map(hydrate_site_type_defaults)
}

/// Load all site types from WAL.
pub fn load_all_site_types(journal: &ForgeJournal) -> Vec<SiteTypeDefinition> {
    journal
        .latest_by_aggregate_type(AGG_SITE_TYPE)
        .into_iter()
        .filter_map(|e| serde_json::from_slice::<SiteTypeDefinition>(&e.payload).ok())
        .map(hydrate_site_type_defaults)
        .collect()
}

/// Save a site type definition to WAL.
pub fn save_site_type(journal: &mut ForgeJournal, def: &SiteTypeDefinition) -> Result<(), String> {
    let bytes = serde_json::to_vec(def).map_err(|e| format!("Serialize error: {e}"))?;
    let event = ApexEvent::new(AGG_SITE_TYPE, &def.slug, bytes);
    journal
        .append(event)
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}

/// Delete a site type by writing a tombstone event.
pub fn delete_site_type(journal: &mut ForgeJournal, slug: &str) -> Result<(), String> {
    let event = ApexEvent::new(AGG_SITE_TYPE, slug, b"__TOMBSTONE__".to_vec());
    journal
        .append(event)
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}

/// Load all site types, falling back to built-in defaults if WAL is empty.
pub fn load_or_seed_catalog(journal: &mut ForgeJournal) -> Vec<SiteTypeDefinition> {
    let existing = load_all_site_types(journal);
    if !existing.is_empty() {
        return existing;
    }
    // Seed defaults
    let defaults = super::defaults::all_defaults();
    for def in &defaults {
        let _ = save_site_type(journal, def);
    }
    defaults
}

/// Idempotently add any built-in default site types that aren't already in the
/// WAL. Returns the number of types written. Safe to call at every startup —
/// noop when the catalog is already complete; tops up partial seeds (e.g. when
/// only a handful of types got seeded incidentally via auth-gated reads).
pub fn top_up_catalog(journal: &mut ForgeJournal) -> usize {
    let existing_slugs: std::collections::HashSet<String> = load_all_site_types(journal)
        .into_iter()
        .map(|d| d.slug)
        .collect();
    let defaults = super::defaults::all_defaults();
    let mut added = 0usize;
    for def in &defaults {
        if !existing_slugs.contains(&def.slug) {
            if save_site_type(journal, def).is_ok() {
                added += 1;
            }
        }
    }
    added
}
