//! Site Type Catalog module — central registry for all industry/site types.
//!
//! Admin UI at `/admin#site-catalog` shows every site type as an editable card.
//! Provision script reads from this catalog instead of hardcoded values.

pub mod blueprint;
pub mod defaults;
pub mod presets;
pub mod provision;
pub mod store;
pub mod taxonomy;
pub mod types;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post, put};
use axum::Router;
use axum_extra::extract::cookie::CookieJar;
use chrono::{DateTime, Utc};
use luperiq_forge::nexus::{NexClientPayload, AGG_NEX_CLIENT};
use luperiq_forge::{ApexEvent, ForgeJournal, NexusProjection};
use luperiq_mod_sales_funnel::sales_funnel::trials;
use luperiq_module_api::{extract_session, AdminView, AppContext, CmsModule, SharedJournal};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use types::*;

const AGG_COMMERCE_ENTITLEMENT: &str = "Commerce:Entitlement";

pub struct SiteCatalogModule;

#[derive(Clone)]
struct CatalogState {
    journal: SharedJournal,
    jwt_secret: String,
    is_central: bool,
}

impl CmsModule for SiteCatalogModule {
    fn slug(&self) -> &str {
        "site-catalog"
    }
    fn name(&self) -> &str {
        "Site Type Catalog"
    }
    fn version(&self) -> &str {
        "0.1.0"
    }
    fn description(&self) -> &str {
        "Central registry of every site type with theme defaults, modules, pages, pricing, and onboarding config."
    }
    fn category(&self) -> &str {
        "System"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        let is_central = ctx
            .nexus_config
            .as_ref()
            .and_then(|n| n.role.as_deref())
            .map(|role| role == "central")
            .unwrap_or(true);
        Some(catalog_router(CatalogState {
            journal: ctx.journal.clone(),
            jwt_secret: ctx.jwt_secret.clone(),
            is_central,
        }))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![
            AdminView {
                id: "site-catalog".into(),
                label: "Site Catalog".into(),
                section: "System".into(),
            },
            AdminView {
                id: "site-fleet".into(),
                label: "Site Fleet".into(),
                section: "System".into(),
            },
        ]
    }

    fn admin_js(&self) -> Option<String> {
        Some(ADMIN_JS.to_string())
    }
}

/// Central is marked by `LUPERIQ_IS_CENTRAL=1` in its systemd unit's
/// EnvironmentFile. Customer sites don't get that env and so never
/// register the cross-tenant Site Fleet admin endpoints.
fn is_central_instance() -> bool {
    std::env::var("LUPERIQ_IS_CENTRAL")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        })
        .unwrap_or(false)
}

fn catalog_router(state: CatalogState) -> Router {
    let mut router = Router::new()
        .route("/api/modules/site-catalog/types", get(list_types))
        .route(
            "/api/modules/site-catalog/types/{slug}",
            get(get_type).put(update_type).delete(delete_type),
        )
        .route(
            "/api/modules/site-catalog/types/{slug}/onboarding-usage",
            get(get_onboarding_usage),
        )
        .route(
            "/api/modules/site-catalog/types/{slug}/theme",
            put(update_theme),
        )
        .route(
            "/api/modules/site-catalog/seed",
            axum::routing::post(seed_defaults),
        )
        .route("/api/modules/site-catalog/taxonomy", get(get_taxonomy))
        .route("/api/modules/site-catalog/site-stats", get(site_stats))
        .route(
            "/api/modules/site-catalog/sites/{industry}",
            get(sites_by_industry),
        )
        .route(
            "/api/modules/site-catalog/provision/{slug}",
            get(provision_data),
        );

    // Site Fleet admin endpoints only register on Central. They shell
    // out to sudo scripts that read/write other sites' WAL directories,
    // so leaving them unreachable on customer CMS processes preserves
    // the single-writer rule and closes a cross-tenant escalation path:
    // an admin on any customer site would otherwise be able to
    // /backup /clone /delete / /restore other sites hosted on the same
    // machine.
    if is_central_instance() {
        router = router
            .route("/api/modules/site-catalog/fleet", get(list_fleet_sites))
            .route(
                "/api/modules/site-catalog/fleet/backup",
                post(backup_fleet_site),
            )
            .route(
                "/api/modules/site-catalog/fleet/scheduled-backups",
                post(run_scheduled_fleet_backups),
            )
            .route(
                "/api/modules/site-catalog/fleet/export",
                post(export_fleet_site),
            )
            .route(
                "/api/modules/site-catalog/fleet/export/download/{file}",
                get(download_fleet_export),
            )
            .route(
                "/api/modules/site-catalog/fleet/reconcile",
                get(reconcile_fleet_billing),
            )
            .route(
                "/api/modules/site-catalog/fleet/import-handoff",
                post(import_fleet_handoff),
            )
            .route(
                "/api/modules/site-catalog/fleet/clone",
                post(clone_fleet_site),
            )
            .route(
                "/api/modules/site-catalog/fleet/delete",
                post(delete_fleet_site),
            )
            .route(
                "/api/modules/site-catalog/fleet/hosting-mode",
                post(mark_fleet_hosting_mode),
            )
            .route(
                "/api/modules/site-catalog/fleet/restore",
                post(restore_fleet_site),
            )
            .route(
                "/api/modules/site-catalog/fleet/promote",
                post(promote_fleet_site),
            );
    }

    router.with_state(state)
}

fn registry_created_at(site: &serde_json::Value) -> u64 {
    if let Some(ts) = site.get("created_at").and_then(|v| v.as_u64()) {
        return ts;
    }
    site.get("created")
        .and_then(|v| v.as_str())
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| dt.timestamp().max(0) as u64)
        .unwrap_or(0)
}

fn canonical_catalog_site_type_slug(slug: &str) -> &str {
    match slug.trim() {
        "business" => "business-team",
        "coffee" => "coffee-shop",
        "artisan" => "artisan-market",
        "device-repair" => "cell-phone-repair",
        "salon-barbershop" => "salon",
        "medical" | "healthcare" | "clinic" | "doctor" | "dentist" | "dental" | "med-spa"
        | "medspa" | "medical-spa" | "plastic-surgeon" | "plastic-surgery" | "chiropractic"
        | "mental-health" => "medical-office",
        other => other,
    }
}

fn site_type_slug_candidates(slug: &str) -> Vec<&str> {
    match canonical_catalog_site_type_slug(slug) {
        "business-team" => vec!["business-team", "business"],
        "coffee-shop" => vec!["coffee-shop", "coffee"],
        "artisan-market" => vec!["artisan-market", "artisan"],
        "cell-phone-repair" => vec!["cell-phone-repair", "device-repair"],
        "medical-office" => vec![
            "medical-office",
            "medical",
            "healthcare",
            "clinic",
            "doctor",
            "dentist",
            "dental",
            "med-spa",
            "medspa",
            "medical-spa",
            "plastic-surgeon",
            "plastic-surgery",
            "chiropractic",
            "mental-health",
        ],
        other => vec![other],
    }
}

fn load_site_type_compat(journal: &ForgeJournal, slug: &str) -> Option<SiteTypeDefinition> {
    for candidate in site_type_slug_candidates(slug) {
        if let Some(def) = store::load_site_type(journal, candidate) {
            return Some(def);
        }
    }
    None
}

fn present_site_type(mut def: SiteTypeDefinition) -> SiteTypeDefinition {
    def.slug = canonical_catalog_site_type_slug(&def.slug).to_string();
    def
}

fn registry_industry_matches(site: &serde_json::Value, industry: &str) -> bool {
    let requested = canonical_catalog_site_type_slug(industry);
    let current = site.get("industry").and_then(|v| v.as_str()).unwrap_or("");
    canonical_catalog_site_type_slug(current) == requested
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RegistryEntry {
    #[serde(default)]
    domain: String,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    port: u16,
    #[serde(default)]
    license: String,
    #[serde(default)]
    industry: String,
    #[serde(default)]
    site_name: String,
    #[serde(default)]
    admin_email: String,
    #[serde(default)]
    service: String,
    #[serde(default)]
    service_name: String,
    #[serde(default)]
    dir: String,
    #[serde(default)]
    created: String,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    status: String,
    #[serde(default)]
    mode: String,
    #[serde(default)]
    clone_of: String,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    deleted_at: String,
    #[serde(default)]
    deleted_backup_dir: String,
    #[serde(default)]
    archived_dir: String,
}

#[derive(Debug, Clone, Default)]
struct SiteConfigSummary {
    bind_port: Option<u16>,
    site_name: String,
    site_type: String,
    base_url: String,
    admin_email: String,
    industry: String,
    license_key: String,
    central_url: String,
    modules: Vec<String>,
    has_ai_quick: bool,
    has_ai_content: bool,
    has_ai_escalation: bool,
}

/// Local read-only projection of the `Commerce:Entitlement` aggregate owned by
/// `luperiq-mod-universal-cart` (entitlements.rs). Fields are a subset of the
/// canonical struct; if universal-cart adds fields this projection silently lags.
/// The aggregate key is mirrored in `AGG_COMMERCE_ENTITLEMENT` above — a rename
/// in universal-cart would silently desync. Track with luperiq-engine-keys (map P4).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CommerceEntitlementSummary {
    #[serde(default)]
    entitlement_id: String,
    #[serde(default)]
    owner_email: String,
    #[serde(default)]
    order_id: String,
    #[serde(default)]
    product_id: String,
    #[serde(default)]
    license_key: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    assigned_to_site: Option<String>,
    #[serde(default)]
    assigned_to_user: Option<String>,
    #[serde(default)]
    tier: String,
    #[serde(default)]
    billing_period: String,
    #[serde(default)]
    paid_through: Option<u64>,
    #[serde(default)]
    stripe_subscription_id: Option<String>,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    created_at: u64,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Debug, Deserialize)]
struct FleetBackupRequest {
    domain: String,
}

#[derive(Debug, Deserialize)]
struct FleetScheduledBackupsRequest {
    #[serde(default)]
    domain: Option<String>,
    #[serde(default = "default_true")]
    dry_run: bool,
}

#[derive(Debug, Deserialize)]
struct FleetExportRequest {
    domain: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    include_secrets: bool,
}

#[derive(Debug, Deserialize)]
struct FleetCloneRequest {
    domain: String,
    target_domain: String,
    #[serde(default)]
    site_name: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    license_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FleetImportHandoffRequest {
    archive_path: String,
    domain: String,
    license_key: String,
    #[serde(default)]
    site_name: Option<String>,
    #[serde(default)]
    admin_email: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    allow_internal: bool,
}

#[derive(Debug, Deserialize)]
struct FleetDeleteRequest {
    domain: String,
    #[serde(default)]
    revoke_license: bool,
    #[serde(default)]
    archive_note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FleetHostingModeRequest {
    domain: String,
    hosting_mode: String,
    #[serde(default)]
    deployment_status: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FleetRestoreRequest {
    domain: String,
}

#[derive(Debug, Deserialize)]
struct FleetPromoteRequest {
    domain: String,
    license_key: String,
}

fn registry_path() -> String {
    std::env::var("LUPERIQ_SITE_REGISTRY")
        .unwrap_or_else(|_| "/home/dave/sites/site-registry.json".into())
}

fn site_export_root() -> String {
    std::env::var("LUPERIQ_SITE_EXPORTS_DIR")
        .unwrap_or_else(|_| "/ai/backups/site-fleet/exports".into())
}

/// Returns the directory containing the site-fleet shell scripts. Defaults to the
/// canonical operator path. Override with `LUPERIQ_FLEET_SCRIPTS_DIR` to run
/// fleet operations from an alternate location (e.g. on a new machine or in CI).
/// Follows the same env-var fallback pattern as `LUPERIQ_SITE_REGISTRY` and
/// `LUPERIQ_SITE_EXPORTS_DIR`.
fn fleet_scripts_dir() -> String {
    std::env::var("LUPERIQ_FLEET_SCRIPTS_DIR")
        .unwrap_or_else(|_| "/home/dave/luperiq-apex-db/scripts".into())
}

fn default_true() -> bool {
    true
}

fn safe_export_file_name(file: &str) -> Option<String> {
    let trimmed = file.trim();
    if trimmed.is_empty()
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || !trimmed.ends_with(".tar.gz")
    {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn allowed_import_archive_path(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty()
        || trimmed.contains('\0')
        || !trimmed.ends_with(".tar.gz")
        || trimmed.contains("..")
    {
        return false;
    }
    let requested = std::path::PathBuf::from(trimmed);
    if !requested.is_absolute() {
        return false;
    }
    let Ok(canonical) = requested.canonicalize() else {
        return false;
    };
    let roots = [
        site_export_root(),
        "/ai/backups".to_string(),
        "/mnt/server-bu".to_string(),
        "/home/dave/backups".to_string(),
    ];
    roots.iter().any(|root| {
        std::path::PathBuf::from(root)
            .canonicalize()
            .map(|allowed| canonical.starts_with(allowed))
            .unwrap_or(false)
    })
}

fn run_sudo_capture(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("sudo")
        .arg("-n")
        .arg(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run sudo {program}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("{program} failed: {detail}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_sudo_capture_owned(program: &str, args: &[String]) -> Result<String, String> {
    let mut cmd = Command::new("sudo");
    cmd.arg("-n").arg(program);
    for arg in args {
        cmd.arg(arg);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run sudo {program}: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("{program} failed: {detail}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn read_registry_values() -> Vec<serde_json::Value> {
    let path = registry_path();
    run_sudo_capture("cat", &[&path])
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
        .unwrap_or_default()
}

fn read_registry_entries() -> Vec<RegistryEntry> {
    let path = registry_path();
    run_sudo_capture("cat", &[&path])
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<RegistryEntry>>(&raw).ok())
        .unwrap_or_default()
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn find_nexus_client_for_site(
    journal: &ForgeJournal,
    license_key: &str,
    domain: &str,
) -> Option<(String, NexClientPayload)> {
    let proj = NexusProjection::new(journal);
    if !license_key.trim().is_empty() {
        if let Some(found) = proj.client_by_license_key(license_key).ok().flatten() {
            return Some(found);
        }
    }
    proj.client_by_domain(domain).ok().flatten()
}

fn append_nexus_client(journal: &mut ForgeJournal, id: &str, client: &NexClientPayload) {
    let event = ApexEvent::new(
        AGG_NEX_CLIENT,
        id,
        serde_json::to_vec(client).unwrap_or_default(),
    );
    let _ = journal.append(event);
}

fn normalize_service_name(entry: &RegistryEntry) -> String {
    let slug = if entry.slug.trim().is_empty() {
        entry
            .domain
            .replace(|c: char| !c.is_ascii_alphanumeric(), "-")
            .trim_matches('-')
            .to_string()
    } else {
        entry.slug.clone()
    };
    let candidate = if !entry.service_name.trim().is_empty() {
        entry.service_name.trim()
    } else if !entry.service.trim().is_empty() {
        entry.service.trim()
    } else {
        ""
    };
    if candidate.is_empty() {
        return format!("luperiq-{slug}.service");
    }
    if candidate.ends_with(".service") {
        candidate.to_string()
    } else {
        format!("{candidate}.service")
    }
}

fn systemd_state(service_name: &str) -> String {
    let output = Command::new("sudo")
        .arg("-n")
        .arg("systemctl")
        .arg("is-active")
        .arg(service_name)
        .output();
    match output {
        Ok(out) => {
            let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if state.is_empty() {
                if out.status.success() {
                    "active".into()
                } else {
                    "unknown".into()
                }
            } else {
                state
            }
        }
        Err(_) => "unknown".into(),
    }
}

fn parse_config_summary(raw: &str) -> SiteConfigSummary {
    let Ok(value) = raw.parse::<toml::Value>() else {
        return SiteConfigSummary::default();
    };
    let server = value
        .get("server")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let bootstrap = value
        .get("bootstrap")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let nexus = value
        .get("nexus_network")
        .and_then(|v| v.as_table())
        .cloned()
        .unwrap_or_default();
    let modules = value
        .get("modules")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("enabled"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let bind_port = server
        .get("bind")
        .and_then(|v| v.as_str())
        .and_then(|bind| bind.rsplit(':').next())
        .and_then(|p| p.parse::<u16>().ok());
    SiteConfigSummary {
        bind_port,
        site_name: server
            .get("site_name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        site_type: server
            .get("site_type")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        base_url: server
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        admin_email: bootstrap
            .get("admin_email")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        industry: bootstrap
            .get("industry")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        license_key: nexus
            .get("license_key")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        central_url: nexus
            .get("central_url")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        modules: modules
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        has_ai_quick: value.get("ai_quick").is_some(),
        has_ai_content: value.get("ai_content").is_some(),
        has_ai_escalation: value.get("ai_escalation").is_some(),
    }
}

fn load_site_config(dir: &str) -> SiteConfigSummary {
    if dir.trim().is_empty() {
        return SiteConfigSummary::default();
    }
    let cfg_path = format!("{}/config/cms.toml", dir.trim_end_matches('/'));
    std::fs::read_to_string(cfg_path)
        .map(|raw| parse_config_summary(&raw))
        .unwrap_or_default()
}

fn file_size(path: &str) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn commerce_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn load_commerce_entitlements(journal: &ForgeJournal) -> Vec<CommerceEntitlementSummary> {
    journal
        .latest_by_aggregate_type(AGG_COMMERCE_ENTITLEMENT)
        .into_iter()
        .filter_map(|event| {
            serde_json::from_slice::<CommerceEntitlementSummary>(&event.payload).ok()
        })
        .collect()
}

fn index_commerce_entitlements(
    entitlements: Vec<CommerceEntitlementSummary>,
) -> (
    HashMap<String, Vec<CommerceEntitlementSummary>>,
    HashMap<String, Vec<CommerceEntitlementSummary>>,
) {
    let mut by_license: HashMap<String, Vec<CommerceEntitlementSummary>> = HashMap::new();
    let mut by_site: HashMap<String, Vec<CommerceEntitlementSummary>> = HashMap::new();

    for entitlement in entitlements {
        let license_key = commerce_key(&entitlement.license_key);
        if !license_key.is_empty() {
            by_license
                .entry(license_key)
                .or_default()
                .push(entitlement.clone());
        }
        if let Some(site) = entitlement.assigned_to_site.as_deref() {
            let site_key = commerce_key(site);
            if !site_key.is_empty() {
                by_site.entry(site_key).or_default().push(entitlement);
            }
        }
    }

    (by_license, by_site)
}

fn matching_commerce_entitlements(
    by_license: &HashMap<String, Vec<CommerceEntitlementSummary>>,
    by_site: &HashMap<String, Vec<CommerceEntitlementSummary>>,
    license_key: &str,
    domain: &str,
) -> Vec<CommerceEntitlementSummary> {
    let mut seen = HashSet::new();
    let mut matches = Vec::new();
    for entitlement in by_license
        .get(&commerce_key(license_key))
        .into_iter()
        .flatten()
        .chain(by_site.get(&commerce_key(domain)).into_iter().flatten())
    {
        let id = if entitlement.entitlement_id.trim().is_empty() {
            format!(
                "{}:{}:{}",
                entitlement.license_key, entitlement.owner_email, entitlement.product_id
            )
        } else {
            entitlement.entitlement_id.clone()
        };
        if seen.insert(id) {
            matches.push(entitlement.clone());
        }
    }
    matches.sort_by(|a, b| {
        b.updated_at
            .max(b.created_at)
            .cmp(&a.updated_at.max(a.created_at))
    });
    matches
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    if !values
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(trimmed))
    {
        values.push(trimmed.to_string());
    }
}

fn summarize_commerce_entitlements(matches: &[CommerceEntitlementSummary]) -> serde_json::Value {
    if matches.is_empty() {
        return serde_json::Value::Null;
    }

    let mut owner_emails = Vec::new();
    let mut tiers = Vec::new();
    let mut billing_periods = Vec::new();
    let mut statuses = Vec::new();
    let mut features = Vec::new();
    let mut stripe_subscription_count = 0usize;
    let mut active_count = 0usize;
    let mut paid_through_max = None;

    for entitlement in matches {
        push_unique(&mut owner_emails, &entitlement.owner_email);
        push_unique(&mut tiers, &entitlement.tier);
        push_unique(&mut billing_periods, &entitlement.billing_period);
        push_unique(&mut statuses, &entitlement.status);
        for feature in &entitlement.features {
            push_unique(&mut features, feature);
        }
        if entitlement.status.eq_ignore_ascii_case("active") {
            active_count += 1;
        }
        if entitlement
            .stripe_subscription_id
            .as_deref()
            .map(|id| !id.trim().is_empty())
            .unwrap_or(false)
        {
            stripe_subscription_count += 1;
        }
        if let Some(paid_through) = entitlement.paid_through {
            paid_through_max = Some(paid_through_max.unwrap_or(0).max(paid_through));
        }
    }

    json!({
        "count": matches.len(),
        "active_count": active_count,
        "owner_emails": owner_emails,
        "tiers": tiers,
        "billing_periods": billing_periods,
        "statuses": statuses,
        "paid_through_max": paid_through_max,
        "stripe_subscription_count": stripe_subscription_count,
        "features": features.into_iter().take(20).collect::<Vec<_>>(),
        "items": matches.iter().take(10).map(|entitlement| {
            json!({
                "entitlement_id": entitlement.entitlement_id,
                "owner_email": entitlement.owner_email,
                "order_id": entitlement.order_id,
                "product_id": entitlement.product_id,
                "license_key": entitlement.license_key,
                "status": entitlement.status,
                "assigned_to_site": entitlement.assigned_to_site,
                "assigned_to_user": entitlement.assigned_to_user,
                "tier": entitlement.tier,
                "billing_period": entitlement.billing_period,
                "paid_through": entitlement.paid_through,
                "stripe_subscription_id": entitlement.stripe_subscription_id.as_ref().map(|_| "present"),
                "feature_count": entitlement.features.len(),
                "created_at": entitlement.created_at,
                "updated_at": entitlement.updated_at,
            })
        }).collect::<Vec<_>>(),
    })
}

fn json_error(message: &str) -> Json<serde_json::Value> {
    Json(json!({ "ok": false, "message": message }))
}

async fn require_admin(
    state: &CatalogState,
    jar: &CookieJar,
) -> Result<luperiq_forge::SessionClaims, Json<serde_json::Value>> {
    if !state.is_central {
        return Err(json_error("Site Fleet is only available on Central."));
    }
    extract_session(jar, &state.journal, &state.jwt_secret)
        .await
        .ok_or_else(|| json_error("Unauthorized"))
}

/// GET /api/modules/site-catalog/types — list all site types
async fn list_types(State(state): State<CatalogState>) -> Json<serde_json::Value> {
    let mut j = state.journal.lock().await;
    let types: Vec<SiteTypeDefinition> = store::load_or_seed_catalog(&mut j)
        .into_iter()
        .map(present_site_type)
        .collect();
    Json(json!({ "ok": true, "data": types }))
}

/// GET /api/modules/site-catalog/types/{slug}
async fn get_type(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    match load_site_type_compat(&j, &slug).map(present_site_type) {
        Some(t) => Json(json!({ "ok": true, "data": t })),
        None => Json(json!({ "ok": false, "message": "Site type not found" })),
    }
}

/// GET /api/modules/site-catalog/types/{slug}/onboarding-usage
/// Returns a Central-only explainer map for each onboarding field, including
/// suggested customer help text and where the answer is typically used.
async fn get_onboarding_usage(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    match load_site_type_compat(&j, &slug).map(present_site_type) {
        Some(t) => {
            let steps: Vec<serde_json::Value> = t
                .onboarding_steps
                .iter()
                .map(|step| {
                    let fields: Vec<serde_json::Value> = step
                        .fields
                        .iter()
                        .map(|field| {
                            let help_text = if field.help_text.trim().is_empty() {
                                default_onboarding_help_for(
                                    &t.slug,
                                    &step.step_id,
                                    &field.key,
                                    &field.label,
                                )
                            } else {
                                field.help_text.clone()
                            };
                            let used_in =
                                onboarding_usage_hints_for(&t.slug, &step.step_id, &field.key);
                            json!({
                                "key": field.key,
                                "label": field.label,
                                "help_text": help_text,
                                "admin_notes": field.admin_notes,
                                "used_in": used_in,
                            })
                        })
                        .collect();
                    json!({
                        "step_id": step.step_id,
                        "label": step.label,
                        "field_count": step.fields.len(),
                        "fields": fields,
                    })
                })
                .collect();
            Json(json!({ "ok": true, "data": steps }))
        }
        None => Json(json!({ "ok": false, "message": "Site type not found" })),
    }
}

/// GET /api/modules/site-catalog/provision/{slug}
/// Public — no auth required. Called by the shell provisioning script.
/// Returns a flat `ProvisionPayload` with everything needed to set up a new site.
async fn provision_data(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    match load_site_type_compat(&j, &slug) {
        Some(def) => {
            let def = present_site_type(def);
            let payload = provision::extract(&def);
            Json(json!({ "ok": true, "data": payload }))
        }
        None => Json(json!({ "ok": false, "message": "Site type not found" })),
    }
}

/// GET /api/modules/site-catalog/taxonomy — public, no auth
/// Returns all industry categories and sub-specialties for the funnel/catalog.
async fn get_taxonomy() -> Json<serde_json::Value> {
    let categories = taxonomy::all_categories();
    let subspecialties = taxonomy::all_subspecialties();
    Json(json!({
        "ok": true,
        "categories": categories,
        "subspecialties": subspecialties,
        "category_count": categories.len(),
        "subspecialty_count": subspecialties.len(),
    }))
}

/// PUT /api/modules/site-catalog/types/{slug} — update a site type
async fn update_type(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut j = state.journal.lock().await;
    let existing = load_site_type_compat(&j, &slug);
    let Some(mut def) = existing else {
        return Json(json!({ "ok": false, "message": "Site type not found" }));
    };

    // Merge fields from body
    if let Some(v) = body.get("name").and_then(|v| v.as_str()) {
        def.name = v.into();
    }
    if let Some(v) = body.get("emoji").and_then(|v| v.as_str()) {
        def.emoji = v.into();
    }
    if let Some(v) = body.get("category").and_then(|v| v.as_str()) {
        def.category = v.into();
    }
    if let Some(v) = body.get("description").and_then(|v| v.as_str()) {
        def.description = v.into();
    }
    if let Some(v) = body.get("default_tagline").and_then(|v| v.as_str()) {
        def.default_tagline = v.into();
    }
    if let Some(v) = body.get("publicly_listed") {
        def.publicly_listed = v.as_bool().unwrap_or(def.publicly_listed);
    }
    if let Some(v) = body.get("always_free") {
        def.always_free = v.as_bool().unwrap_or(def.always_free);
    }
    if let Some(v) = body.get("default_tier").and_then(|v| v.as_str()) {
        def.default_tier = v.into();
    }
    if let Some(v) = body.get("enabled_modules") {
        if let Ok(mods) = serde_json::from_value::<Vec<String>>(v.clone()) {
            def.enabled_modules = mods;
        }
    }
    if let Some(v) = body.get("default_nav_items") {
        if let Ok(items) = serde_json::from_value::<Vec<NavItem>>(v.clone()) {
            def.default_nav_items = items;
        }
    }
    if let Some(v) = body.get("default_pages") {
        if let Ok(pages) = serde_json::from_value::<Vec<DefaultPage>>(v.clone()) {
            def.default_pages = pages;
        }
    }
    if let Some(v) = body.get("homepage_blocks") {
        def.homepage_blocks = Some(v.clone());
    }
    if let Some(v) = body.get("theme_profile") {
        def.theme_profile = Some(v.clone());
    }
    if let Some(v) = body.get("discount_codes") {
        if let Ok(codes) = serde_json::from_value::<Vec<DiscountCode>>(v.clone()) {
            def.discount_codes = codes;
        }
    }
    if let Some(v) = body.get("limited_time_offer") {
        def.limited_time_offer = serde_json::from_value::<LimitedOffer>(v.clone()).ok();
    }
    if let Some(v) = body.get("onboarding_steps") {
        if let Ok(steps) = serde_json::from_value::<Vec<OnboardingStep>>(v.clone()) {
            def.onboarding_steps = steps;
        }
    }
    if let Some(v) = body.get("price_override_cents") {
        def.price_override_cents = v.as_i64().unwrap_or(def.price_override_cents);
    }
    if let Some(v) = body.get("default_tone").and_then(|v| v.as_str()) {
        def.default_tone = v.into();
    }

    def.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match store::save_site_type(&mut j, &def) {
        Ok(_) => Json(json!({ "ok": true, "message": "Updated" })),
        Err(e) => Json(json!({ "ok": false, "message": e })),
    }
}

/// PUT /api/modules/site-catalog/types/{slug}/theme — update just the theme profile
/// This is what the Design Playground calls when "Save as Default Theme" is clicked.
async fn update_theme(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
    Json(theme): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut j = state.journal.lock().await;
    let Some(mut def) = load_site_type_compat(&j, &slug) else {
        return Json(json!({ "ok": false, "message": "Site type not found" }));
    };

    def.theme_profile = Some(theme);
    def.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    match store::save_site_type(&mut j, &def) {
        Ok(_) => Json(json!({ "ok": true, "message": "Default theme updated" })),
        Err(e) => Json(json!({ "ok": false, "message": e })),
    }
}

/// DELETE /api/modules/site-catalog/types/{slug} — remove a site type
async fn delete_type(
    State(state): State<CatalogState>,
    Path(slug): Path<String>,
) -> Json<serde_json::Value> {
    let mut j = state.journal.lock().await;
    let Some(def) = load_site_type_compat(&j, &slug) else {
        return Json(json!({ "ok": false, "message": "Site type not found" }));
    };
    match store::delete_site_type(&mut j, &def.slug) {
        Ok(_) => Json(json!({ "ok": true, "message": format!("Deleted '{slug}'") })),
        Err(e) => Json(json!({ "ok": false, "message": e })),
    }
}

/// GET /api/modules/site-catalog/site-stats — count of live sites per industry
async fn site_stats(State(_state): State<CatalogState>) -> Json<serde_json::Value> {
    let registry = read_registry_values();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let day_ago = now - 86400;

    let mut by_industry: std::collections::HashMap<String, Vec<serde_json::Value>> =
        std::collections::HashMap::new();
    let mut recent_count: std::collections::HashMap<String, u32> = std::collections::HashMap::new();

    for site in &registry {
        let ind = site
            .get("industry")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let ind = canonical_catalog_site_type_slug(&ind).to_string();
        by_industry
            .entry(ind.clone())
            .or_default()
            .push(site.clone());

        let created = registry_created_at(site);
        if created >= day_ago {
            *recent_count.entry(ind).or_insert(0) += 1;
        }
    }

    let stats: Vec<serde_json::Value> = by_industry
        .iter()
        .map(|(ind, sites)| {
            json!({
                "industry": ind,
                "total": sites.len(),
                "last_24h": recent_count.get(ind).unwrap_or(&0),
            })
        })
        .collect();

    Json(json!({ "ok": true, "data": stats, "total_sites": registry.len() }))
}

/// GET /api/modules/site-catalog/sites/{industry} — list sites for an industry
async fn sites_by_industry(Path(industry): Path<String>) -> Json<serde_json::Value> {
    let registry = read_registry_values();

    let sites: Vec<&serde_json::Value> = registry
        .iter()
        .filter(|s| registry_industry_matches(s, &industry))
        .collect();

    Json(json!({
        "ok": true,
        "data": sites,
        "industry": canonical_catalog_site_type_slug(&industry),
        "count": sites.len()
    }))
}

/// POST /api/modules/site-catalog/seed — re-seed defaults (admin only)
/// Pass ?force=true to overwrite ALL catalog defaults (not customer sites, just Central's catalog)
async fn seed_defaults(
    State(state): State<CatalogState>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let force = q.get("force").map(|v| v == "true").unwrap_or(false);
    let mut j = state.journal.lock().await;
    let defaults = defaults::all_defaults();
    let mut count = 0;
    for def in &defaults {
        let existing = load_site_type_compat(&j, &def.slug);
        if force || existing.is_none() {
            let mut to_save = def.clone();
            if let Some(existing_def) = existing.as_ref() {
                to_save.slug = existing_def.slug.clone();
            }
            if store::save_site_type(&mut j, &to_save).is_ok() {
                count += 1;
            }
        }
    }
    let msg = if force {
        format!("{count} types force-reseeded (catalog defaults only, no customer sites touched)")
    } else {
        format!("{count} new types seeded")
    };
    Json(json!({ "ok": true, "message": msg, "total": defaults.len() }))
}

async fn list_fleet_sites(
    State(state): State<CatalogState>,
    jar: CookieJar,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };

    let entries = read_registry_entries();
    let (clients_by_license, clients_by_domain, entitlements_by_license, entitlements_by_site) = {
        let j = state.journal.lock().await;
        let proj = NexusProjection::new(&j);
        let clients = proj.all_clients().unwrap_or_default();
        let mut by_license = HashMap::new();
        let mut by_domain = HashMap::new();
        for (id, client) in clients {
            if !client.license_key.trim().is_empty() {
                by_license.insert(client.license_key.clone(), (id.clone(), client.clone()));
            }
            if !client.site_domain.trim().is_empty() {
                by_domain.insert(client.site_domain.clone(), (id, client));
            }
        }
        let (entitlements_by_license, entitlements_by_site) =
            index_commerce_entitlements(load_commerce_entitlements(&j));
        (
            by_license,
            by_domain,
            entitlements_by_license,
            entitlements_by_site,
        )
    };

    let mut rows = Vec::new();
    for entry in entries {
        if entry.domain.trim().is_empty() {
            continue;
        }
        let service_name = normalize_service_name(&entry);
        let cfg = load_site_config(&entry.dir);
        let effective_license = if !cfg.license_key.trim().is_empty() {
            cfg.license_key.clone()
        } else {
            entry.license.clone()
        };
        let nexus = clients_by_license
            .get(&effective_license)
            .or_else(|| clients_by_domain.get(&entry.domain));
        let effective_industry = if !cfg.industry.trim().is_empty() {
            cfg.industry.clone()
        } else if !entry.industry.trim().is_empty() {
            entry.industry.clone()
        } else {
            nexus
                .and_then(|(_, client)| client.site_type_slug.clone())
                .unwrap_or_else(|| "unknown".into())
        };
        let effective_site_name = if !cfg.site_name.trim().is_empty() {
            cfg.site_name.clone()
        } else if !entry.site_name.trim().is_empty() {
            entry.site_name.clone()
        } else {
            nexus
                .map(|(_, client)| client.site_name.clone())
                .unwrap_or_else(|| entry.domain.clone())
        };
        let effective_admin_email = if !cfg.admin_email.trim().is_empty() {
            cfg.admin_email.clone()
        } else if !entry.admin_email.trim().is_empty() {
            entry.admin_email.clone()
        } else {
            nexus
                .and_then(|(_, client)| client.admin_email.clone())
                .unwrap_or_default()
        };
        let service_state = if entry.deleted || entry.status == "deleted" {
            "deleted".to_string()
        } else {
            systemd_state(&service_name)
        };
        let effective_mode = if !entry.mode.trim().is_empty() {
            entry.mode.clone()
        } else if effective_license.trim().is_empty() {
            "dev".into()
        } else {
            "live".into()
        };
        let effective_hosting_mode = nexus
            .and_then(|(_, client)| client.hosting_mode.as_deref())
            .filter(|mode| !mode.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if entry.deleted || entry.status == "deleted" {
                    "archived".into()
                } else if effective_mode == "dev" {
                    "dev".into()
                } else {
                    "hosted".into()
                }
            });
        let effective_deployment_status = nexus
            .and_then(|(_, client)| client.deployment_status.as_deref())
            .filter(|status| !status.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                if entry.deleted || entry.status == "deleted" {
                    "archived".into()
                } else {
                    "active".into()
                }
            });
        let nexus_value = if let Some((id, client)) = nexus {
            json!({
                "client_id": id,
                "license_tier": client.license_tier.clone(),
                "license_status": client.license_status.clone(),
                "credits_remaining": client.credits_remaining,
                "bundle_credits_remaining": client.bundle_credits_remaining,
                "hosting_mode": client.hosting_mode.clone(),
                "deployment_status": client.deployment_status.clone(),
                "update_channel": client.update_channel.clone(),
                "runtime_version": client.runtime_version.clone().or_else(|| client.plugin_version.clone()),
                "last_heartbeat_at": client.last_heartbeat_at.clone(),
                "last_export_at": client.last_export_at.clone(),
                "source_code_included": client.source_code_included,
                "allow_self_hosted": client.allow_self_hosted,
                "allow_hosted_return": client.allow_hosted_return,
                "max_active_installs": client.max_active_installs,
                "tier_grant_active": client.tier_grant_active.clone(),
                "trial_status": client.trial_status.clone(),
            })
        } else {
            serde_json::Value::Null
        };
        let commerce_value = summarize_commerce_entitlements(&matching_commerce_entitlements(
            &entitlements_by_license,
            &entitlements_by_site,
            &effective_license,
            &entry.domain,
        ));
        let wal_path = format!("{}/data/events.wal", entry.dir);
        let snapshot_path = format!("{}/data/snapshot.bin", entry.dir);
        rows.push(json!({
            "domain": entry.domain,
            "url": if cfg.base_url.is_empty() { format!("https://{}", entry.domain) } else { cfg.base_url.clone() },
            "admin_url": format!("https://{}/admin", entry.domain),
            "site_name": effective_site_name,
            "industry": canonical_catalog_site_type_slug(&effective_industry),
            "site_type": if !cfg.site_type.is_empty() { cfg.site_type } else { "customer".into() },
            "mode": effective_mode,
            "hosting_mode": effective_hosting_mode,
            "deployment_status": effective_deployment_status,
            "clone_of": if entry.clone_of.is_empty() { serde_json::Value::Null } else { json!(entry.clone_of) },
            "status": if entry.status.is_empty() { "active" } else { &entry.status },
            "deleted": entry.deleted || entry.status == "deleted",
            "deleted_at": if entry.deleted_at.is_empty() { serde_json::Value::Null } else { json!(entry.deleted_at) },
            "deleted_backup_dir": if entry.deleted_backup_dir.is_empty() { serde_json::Value::Null } else { json!(entry.deleted_backup_dir) },
            "archived_dir": if entry.archived_dir.is_empty() { serde_json::Value::Null } else { json!(entry.archived_dir) },
            "service_name": service_name,
            "service_state": service_state,
            "port": cfg.bind_port.unwrap_or(entry.port),
            "dir": entry.dir,
            "created": entry.created,
            "created_at": entry.created_at,
            "admin_email": effective_admin_email,
            "license_key": effective_license,
            "central_url": if cfg.central_url.is_empty() { serde_json::Value::Null } else { json!(cfg.central_url) },
            "module_count": cfg.modules.len(),
            "has_ai_quick": cfg.has_ai_quick,
            "has_ai_content": cfg.has_ai_content,
            "has_ai_escalation": cfg.has_ai_escalation,
            "wal_bytes": file_size(&wal_path),
            "snapshot_bytes": file_size(&snapshot_path),
            "nexus": nexus_value,
            "commerce": commerce_value,
        }));
    }

    rows.sort_by(|a, b| {
        b.get("created_at")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .cmp(&a.get("created_at").and_then(|v| v.as_u64()).unwrap_or(0))
    });

    Json(json!({
        "ok": true,
        "message": format!("{} sites", rows.len()),
        "data": rows
    }))
}

async fn reconcile_fleet_billing(
    State(state): State<CatalogState>,
    jar: CookieJar,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };

    let entries = read_registry_entries();
    let (clients, entitlements) = {
        let j = state.journal.lock().await;
        let proj = NexusProjection::new(&j);
        (
            proj.all_clients().unwrap_or_default(),
            load_commerce_entitlements(&j),
        )
    };
    let (entitlements_by_license, entitlements_by_site) =
        index_commerce_entitlements(entitlements.clone());

    let mut clients_by_license: HashMap<String, Vec<(String, NexClientPayload)>> = HashMap::new();
    let mut clients_by_domain: HashMap<String, Vec<(String, NexClientPayload)>> = HashMap::new();
    for (id, client) in &clients {
        let license_key = commerce_key(&client.license_key);
        if !license_key.is_empty() {
            clients_by_license
                .entry(license_key)
                .or_default()
                .push((id.clone(), client.clone()));
        }
        let domain_key = commerce_key(&client.site_domain);
        if !domain_key.is_empty() {
            clients_by_domain
                .entry(domain_key)
                .or_default()
                .push((id.clone(), client.clone()));
        }
    }

    let mut all_domains = HashSet::new();
    let mut active_domains = HashSet::new();
    let mut active_domains_by_license: HashMap<String, Vec<String>> = HashMap::new();
    let mut site_license_by_domain: HashMap<String, String> = HashMap::new();
    let mut hosted_without_license = Vec::new();
    let mut hosted_without_nexus = Vec::new();
    let mut hosted_without_commerce = Vec::new();
    let mut commerce_site_license_mismatch = Vec::new();
    let mut nexus_status_commerce_mismatch = Vec::new();

    for entry in &entries {
        let domain = entry.domain.trim();
        if domain.is_empty() {
            continue;
        }
        let domain_key = commerce_key(domain);
        let deleted = entry.deleted || entry.status == "deleted";
        all_domains.insert(domain_key.clone());
        if !deleted {
            active_domains.insert(domain_key.clone());
        }

        let cfg = load_site_config(&entry.dir);
        let effective_license = if !cfg.license_key.trim().is_empty() {
            cfg.license_key.clone()
        } else {
            entry.license.clone()
        };
        let license_key = commerce_key(&effective_license);
        if !license_key.is_empty() {
            site_license_by_domain.insert(domain_key.clone(), license_key.clone());
            if !deleted {
                active_domains_by_license
                    .entry(license_key.clone())
                    .or_default()
                    .push(domain.to_string());
            }
        }

        if deleted {
            continue;
        }

        let site_name = if !cfg.site_name.trim().is_empty() {
            cfg.site_name.clone()
        } else if !entry.site_name.trim().is_empty() {
            entry.site_name.clone()
        } else {
            entry.domain.clone()
        };
        let admin_email = if !cfg.admin_email.trim().is_empty() {
            cfg.admin_email.clone()
        } else {
            entry.admin_email.clone()
        };
        let site_value = json!({
            "domain": domain,
            "site_name": site_name,
            "admin_email": admin_email,
            "industry": canonical_catalog_site_type_slug(if !cfg.industry.trim().is_empty() { &cfg.industry } else { &entry.industry }),
            "license_key": effective_license,
            "status": if entry.status.is_empty() { "active" } else { &entry.status },
        });

        if license_key.is_empty() {
            hosted_without_license.push(site_value.clone());
        }

        let nexus_matches = clients_by_license
            .get(&license_key)
            .into_iter()
            .flatten()
            .chain(clients_by_domain.get(&domain_key).into_iter().flatten())
            .collect::<Vec<_>>();
        if nexus_matches.is_empty() {
            hosted_without_nexus.push(site_value.clone());
        }

        let commerce_matches = matching_commerce_entitlements(
            &entitlements_by_license,
            &entitlements_by_site,
            &effective_license,
            domain,
        );
        if !license_key.is_empty() && commerce_matches.is_empty() {
            hosted_without_commerce.push(site_value.clone());
        }

        if let Some(site_entitlements) = entitlements_by_site.get(&domain_key) {
            for entitlement in site_entitlements {
                let entitlement_license = commerce_key(&entitlement.license_key);
                if !entitlement_license.is_empty()
                    && !license_key.is_empty()
                    && entitlement_license != license_key
                {
                    commerce_site_license_mismatch.push(json!({
                        "domain": domain,
                        "site_license_key": effective_license,
                        "entitlement_id": entitlement.entitlement_id,
                        "entitlement_license_key": entitlement.license_key,
                        "owner_email": entitlement.owner_email,
                        "status": entitlement.status,
                    }));
                }
            }
        }

        let commerce_active = commerce_matches
            .iter()
            .any(|entitlement| entitlement.status.eq_ignore_ascii_case("active"));
        let nexus_inactive = nexus_matches.iter().any(|(_, client)| {
            let status = client.license_status.trim();
            !status.is_empty() && !status.eq_ignore_ascii_case("active")
        });
        if commerce_active && nexus_inactive {
            nexus_status_commerce_mismatch.push(json!({
                "domain": domain,
                "license_key": effective_license,
                "commerce_status": "active",
                "nexus_statuses": nexus_matches.iter().map(|(_, client)| client.license_status.clone()).collect::<Vec<_>>(),
            }));
        }
    }

    let duplicate_hosted_licenses = active_domains_by_license
        .iter()
        .filter(|(_, domains)| domains.len() > 1)
        .map(|(license_key, domains)| {
            json!({
                "license_key": license_key,
                "domains": domains,
                "count": domains.len(),
            })
        })
        .collect::<Vec<_>>();

    let duplicate_nexus_licenses = clients_by_license
        .iter()
        .filter(|(_, clients)| clients.len() > 1)
        .map(|(license_key, clients)| {
            json!({
                "license_key": license_key,
                "count": clients.len(),
                "clients": clients.iter().map(|(id, client)| {
                    json!({
                        "client_id": id,
                        "domain": client.site_domain,
                        "site_name": client.site_name,
                        "status": client.license_status,
                        "hosting_mode": client.hosting_mode,
                        "last_heartbeat_at": client.last_heartbeat_at,
                    })
                }).collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();

    let mut commerce_active_unassigned = Vec::new();
    let mut commerce_assigned_missing_hosted_site = Vec::new();
    let mut commerce_without_nexus = Vec::new();

    for entitlement in &entitlements {
        let license_key = commerce_key(&entitlement.license_key);
        let assigned_site = entitlement
            .assigned_to_site
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let assigned_key = commerce_key(assigned_site);
        if entitlement.status.eq_ignore_ascii_case("active") && assigned_key.is_empty() {
            commerce_active_unassigned.push(json!({
                "entitlement_id": entitlement.entitlement_id,
                "owner_email": entitlement.owner_email,
                "product_id": entitlement.product_id,
                "license_key": entitlement.license_key,
                "tier": entitlement.tier,
                "billing_period": entitlement.billing_period,
                "paid_through": entitlement.paid_through,
            }));
        }
        if !assigned_key.is_empty() && !active_domains.contains(&assigned_key) {
            commerce_assigned_missing_hosted_site.push(json!({
                "entitlement_id": entitlement.entitlement_id,
                "owner_email": entitlement.owner_email,
                "assigned_to_site": assigned_site,
                "site_in_registry": all_domains.contains(&assigned_key),
                "site_active": active_domains.contains(&assigned_key),
                "license_key": entitlement.license_key,
                "status": entitlement.status,
            }));
        }
        if !license_key.is_empty() && !clients_by_license.contains_key(&license_key) {
            commerce_without_nexus.push(json!({
                "entitlement_id": entitlement.entitlement_id,
                "owner_email": entitlement.owner_email,
                "assigned_to_site": entitlement.assigned_to_site,
                "license_key": entitlement.license_key,
                "status": entitlement.status,
                "tier": entitlement.tier,
                "billing_period": entitlement.billing_period,
            }));
        }
    }

    let nexus_without_hosted_site = clients
        .iter()
        .filter_map(|(id, client)| {
            let domain_key = commerce_key(&client.site_domain);
            if domain_key.is_empty() || !active_domains.contains(&domain_key) {
                Some(json!({
                    "client_id": id,
                    "domain": client.site_domain,
                    "site_name": client.site_name,
                    "license_key": client.license_key,
                    "license_tier": client.license_tier,
                    "license_status": client.license_status,
                    "hosting_mode": client.hosting_mode,
                    "deployment_status": client.deployment_status,
                    "last_heartbeat_at": client.last_heartbeat_at,
                    "note": if client.hosting_mode.as_deref() == Some("self_hosted") { "May be a legitimate self-hosted install." } else { "No active hosted registry entry matched this domain." },
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let issue_count = hosted_without_license.len()
        + hosted_without_nexus.len()
        + hosted_without_commerce.len()
        + duplicate_hosted_licenses.len()
        + duplicate_nexus_licenses.len()
        + commerce_active_unassigned.len()
        + commerce_assigned_missing_hosted_site.len()
        + commerce_without_nexus.len()
        + nexus_without_hosted_site.len()
        + commerce_site_license_mismatch.len()
        + nexus_status_commerce_mismatch.len();

    Json(json!({
        "ok": true,
        "generated_at": Utc::now().to_rfc3339(),
        "summary": {
            "hosted_registry_entries": entries.len(),
            "active_hosted_domains": active_domains.len(),
            "nexus_clients": clients.len(),
            "commerce_entitlements": entitlements.len(),
            "issue_groups": 11,
            "issue_count": issue_count,
        },
        "issues": {
            "hosted_without_license": hosted_without_license,
            "hosted_without_nexus": hosted_without_nexus,
            "hosted_without_commerce": hosted_without_commerce,
            "duplicate_hosted_licenses": duplicate_hosted_licenses,
            "duplicate_nexus_licenses": duplicate_nexus_licenses,
            "commerce_active_unassigned": commerce_active_unassigned,
            "commerce_assigned_missing_hosted_site": commerce_assigned_missing_hosted_site,
            "commerce_without_nexus": commerce_without_nexus,
            "nexus_without_hosted_site": nexus_without_hosted_site,
            "commerce_site_license_mismatch": commerce_site_license_mismatch,
            "nexus_status_commerce_mismatch": nexus_status_commerce_mismatch,
        }
    }))
}

async fn backup_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetBackupRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let domain = req.domain.trim();
    if domain.is_empty() {
        return json_error("domain is required");
    }
    let script = format!("{}/site-fleet-backup.sh", fleet_scripts_dir());
    match run_sudo_capture(&script, &["--domain", domain]) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => json_error("Backup completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn run_scheduled_fleet_backups(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetScheduledBackupsRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let script = format!("{}/site-fleet-scheduled-backups.sh", fleet_scripts_dir());
    let mut args = Vec::new();
    if let Some(domain) = req
        .domain
        .as_deref()
        .map(str::trim)
        .filter(|domain| !domain.is_empty())
    {
        args.push("--domain".to_string());
        args.push(domain.to_string());
    }
    if req.dry_run {
        args.push("--dry-run".into());
    }

    match run_sudo_capture_owned(&script, &args) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => json_error("Scheduled backup run completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn export_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetExportRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let domain = req.domain.trim();
    if domain.is_empty() {
        return json_error("domain is required");
    }
    let mode = req
        .mode
        .as_deref()
        .map(|m| m.trim().to_lowercase())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "handoff".into());
    if mode != "handoff" && mode != "internal" {
        return json_error("mode must be 'handoff' or 'internal'");
    }

    let script = format!("{}/site-fleet-export.sh", fleet_scripts_dir());
    let export_mode = mode.clone();
    let mut args = vec![
        "--domain".to_string(),
        domain.to_string(),
        "--mode".to_string(),
        mode,
    ];
    if req.include_secrets {
        args.push("--include-secrets".into());
    }

    match run_sudo_capture_owned(&script, &args) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(mut v) => {
                if let Some(file) = v
                    .get("archive_file")
                    .and_then(|value| value.as_str())
                    .and_then(safe_export_file_name)
                {
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert(
                            "download_url".into(),
                            json!(format!(
                                "/api/modules/site-catalog/fleet/export/download/{file}"
                            )),
                        );
                    }
                }
                let entries = read_registry_entries();
                let entry = entries
                    .iter()
                    .find(|site| site.domain == domain)
                    .cloned()
                    .unwrap_or_default();
                let cfg = load_site_config(&entry.dir);
                let effective_license = if !cfg.license_key.trim().is_empty() {
                    cfg.license_key.clone()
                } else {
                    entry.license.clone()
                };
                {
                    let mut j = state.journal.lock().await;
                    if let Some((client_id, mut client)) =
                        find_nexus_client_for_site(&j, &effective_license, domain)
                    {
                        client.last_export_at = Some(now_rfc3339());
                        client.source_code_included = Some(false);
                        if client.update_channel.is_none() && export_mode == "handoff" {
                            client.update_channel = Some("manual".into());
                        }
                        append_nexus_client(&mut j, &client_id, &client);
                    }
                }
                Json(v)
            }
            Err(_) => json_error("Export completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn download_fleet_export(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Path(file): Path<String>,
) -> Response {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp.into_response(),
    };

    let Some(file) = safe_export_file_name(&file) else {
        return (StatusCode::BAD_REQUEST, "Invalid export file name").into_response();
    };

    let root = std::path::PathBuf::from(site_export_root());
    let requested = root.join(&file);
    let root_canonical = match root.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "ok": false, "message": "Export root not found" })),
            )
                .into_response();
        }
    };
    let requested_canonical = match requested.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "ok": false, "message": "Export not found" })),
            )
                .into_response();
        }
    };
    if !requested_canonical.starts_with(&root_canonical) {
        return (StatusCode::FORBIDDEN, "Invalid export path").into_response();
    }

    match std::fs::read(&requested_canonical) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/gzip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{file}\""),
            )
            .body(axum::body::Body::from(bytes))
            .unwrap_or_else(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to build download response",
                )
                    .into_response()
            }),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "message": "Export not found" })),
        )
            .into_response(),
    }
}

async fn import_fleet_handoff(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetImportHandoffRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let archive_path = req.archive_path.trim();
    let domain = req.domain.trim();
    let license_key = req.license_key.trim();
    if archive_path.is_empty() || domain.is_empty() || license_key.is_empty() {
        return json_error("archive_path, domain, and license_key are required");
    }
    if !allowed_import_archive_path(archive_path) {
        return json_error(
            "Archive path must be an existing .tar.gz under an approved backup/export folder.",
        );
    }

    let script = format!("{}/site-fleet-import-handoff.sh", fleet_scripts_dir());
    let mut args = vec![
        "--archive".to_string(),
        archive_path.to_string(),
        "--domain".to_string(),
        domain.to_string(),
        "--license".to_string(),
        license_key.to_string(),
    ];
    if let Some(site_name) = req
        .site_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        args.push("--site-name".into());
        args.push(site_name.to_string());
    }
    if let Some(admin_email) = req
        .admin_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        args.push("--admin-email".into());
        args.push(admin_email.to_string());
    }
    if req.dry_run {
        args.push("--dry-run".into());
    }
    if req.allow_internal {
        args.push("--allow-internal".into());
    }

    match run_sudo_capture_owned(&script, &args) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(mut v) => {
                if !req.dry_run {
                    let install_id = v
                        .get("install_id")
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                    let canonical_site_id = v
                        .get("canonical_site_id")
                        .and_then(|value| value.as_str())
                        .map(str::to_string);
                    let update_channel = v
                        .get("update_channel")
                        .and_then(|value| value.as_str())
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("hosted-runtime")
                        .to_string();
                    let mut j = state.journal.lock().await;
                    if let Some((client_id, mut client)) =
                        find_nexus_client_for_site(&j, license_key, domain)
                    {
                        let now = now_rfc3339();
                        client.hosting_mode = Some("hosted".into());
                        client.deployment_status = Some("active".into());
                        client.last_import_at = Some(now.clone());
                        client.last_seen_domain = Some(domain.to_string());
                        client.source_code_included = Some(false);
                        client.update_channel = Some(update_channel);
                        if let Some(install_id) = install_id {
                            client.install_id = Some(install_id);
                        }
                        if let Some(canonical_site_id) = canonical_site_id {
                            client.canonical_site_id = Some(canonical_site_id);
                        }
                        let mut notes = client.notes.unwrap_or_default();
                        if !notes.is_empty() {
                            notes.push('\n');
                        }
                        notes.push_str(&format!(
                            "Site Fleet imported handoff archive for {domain} back to hosted at {now}. Source code included=false."
                        ));
                        client.notes = Some(notes);
                        append_nexus_client(&mut j, &client_id, &client);
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("central_client_id".into(), json!(client_id));
                        }
                    }
                }
                Json(v)
            }
            Err(_) => json_error("Import completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn clone_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetCloneRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let source_domain = req.domain.trim();
    let target_domain = req.target_domain.trim();
    if source_domain.is_empty() || target_domain.is_empty() {
        return json_error("domain and target_domain are required");
    }
    let mode = req
        .mode
        .as_deref()
        .map(|m| m.trim().to_lowercase())
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| "dev".into());
    if mode != "dev" && mode != "clone" {
        return json_error("mode must be 'dev' or 'clone'");
    }
    if mode == "clone"
        && req
            .license_key
            .as_deref()
            .map(|s| s.trim())
            .unwrap_or("")
            .is_empty()
    {
        return json_error("license_key is required for live duplicates");
    }

    let script = format!("{}/site-fleet-clone.sh", fleet_scripts_dir());
    let mut args = vec![
        "--source-domain".to_string(),
        source_domain.to_string(),
        "--target-domain".to_string(),
        target_domain.to_string(),
        "--mode".to_string(),
        mode,
    ];
    if let Some(site_name) = req
        .site_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        args.push("--site-name".into());
        args.push(site_name.to_string());
    }
    if let Some(license_key) = req
        .license_key
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        args.push("--license".into());
        args.push(license_key.to_string());
    }
    match run_sudo_capture_owned(&script, &args) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => json_error("Clone completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn delete_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetDeleteRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let domain = req.domain.trim();
    if domain.is_empty() {
        return json_error("domain is required");
    }

    let entries = read_registry_entries();
    let entry = entries
        .iter()
        .find(|site| site.domain == domain)
        .cloned()
        .unwrap_or_default();
    let cfg = load_site_config(&entry.dir);
    let effective_license = if !cfg.license_key.trim().is_empty() {
        cfg.license_key.clone()
    } else {
        entry.license.clone()
    };

    let already_deleted = entry.deleted
        || entry.status == "deleted"
        || (!entry.dir.trim().is_empty() && !std::path::Path::new(&entry.dir).exists());

    let script_result = if already_deleted {
        json!({
            "ok": true,
            "domain": domain,
            "slug": entry.slug,
            "backup_dir": if entry.deleted_backup_dir.is_empty() { serde_json::Value::Null } else { json!(entry.deleted_backup_dir) },
            "archived_dir": if entry.archived_dir.is_empty() { serde_json::Value::Null } else { json!(entry.archived_dir) },
            "service_name": normalize_service_name(&entry),
            "already_deleted": true,
        })
    } else {
        let script = format!("{}/site-fleet-delete.sh", fleet_scripts_dir());
        match run_sudo_capture(&script, &["--domain", domain]) {
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(v) => v,
                Err(_) => return json_error("Delete/archive completed but returned invalid JSON."),
            },
            Err(e) => return json_error(&e),
        }
    };

    let mut central_client_id = None;
    let mut central_client_revoked = false;
    let mut deactivated_trial_ids: Vec<String> = Vec::new();
    {
        let mut j = state.journal.lock().await;
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let now_iso = now_rfc3339();
        for mut trial in trials::list_trials(&j).into_iter().filter(|trial| {
            trial
                .domain
                .as_deref()
                .map(|d| d.eq_ignore_ascii_case(domain))
                .unwrap_or(false)
                && trial.deactivated_at.is_none()
        }) {
            trial.stage = "deactivated".into();
            trial.deactivated_at = Some(now_unix);
            if trials::write_trial(&mut j, &trial).is_ok() {
                deactivated_trial_ids.push(trial.trial_id.clone());
            }
        }

        if let Some((client_id, mut client)) =
            find_nexus_client_for_site(&j, &effective_license, domain)
        {
            if req.revoke_license {
                client.license_status = "revoked".into();
                central_client_revoked = true;
            }
            client.deployment_status = Some("archived".into());
            client.hosting_mode = Some("archived".into());
            client.hosted_archived_at = Some(now_iso);
            client.source_code_included.get_or_insert(false);
            central_client_id = Some(client_id.clone());
            if req.revoke_license || req.archive_note.is_some() {
                let mut notes = client.notes.unwrap_or_default();
                if !notes.is_empty() {
                    notes.push_str("\n");
                }
                if req.revoke_license {
                    notes.push_str(&format!(
                        "License revoked by Site Fleet after hosted archive for {domain}."
                    ));
                } else {
                    notes.push_str(&format!("Hosted copy archived by Site Fleet for {domain}."));
                }
                if let Some(note) = req
                    .archive_note
                    .as_deref()
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                {
                    notes.push_str(&format!(" Note: {note}"));
                }
                client.notes = Some(notes);
            }
            append_nexus_client(&mut j, &client_id, &client);
        }
    }

    let mut v = script_result;
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "central_client_revoked".into(),
            serde_json::Value::Bool(central_client_revoked),
        );
        obj.insert(
            "license_preserved".into(),
            serde_json::Value::Bool(!central_client_revoked),
        );
        if let Some(client_id) = central_client_id {
            obj.insert("central_client_id".into(), json!(client_id));
        }
        obj.insert(
            "deactivated_trial_count".into(),
            json!(deactivated_trial_ids.len()),
        );
        if !deactivated_trial_ids.is_empty() {
            obj.insert("deactivated_trial_ids".into(), json!(deactivated_trial_ids));
        }
        if already_deleted {
            obj.insert("already_deleted".into(), serde_json::Value::Bool(true));
        }
    }
    Json(v)
}

async fn mark_fleet_hosting_mode(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetHostingModeRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };

    let domain = req.domain.trim();
    if domain.is_empty() {
        return json_error("domain is required");
    }
    let hosting_mode = req.hosting_mode.trim();
    if hosting_mode.is_empty() {
        return json_error("hosting_mode is required");
    }
    let allowed = [
        "hosted",
        "self_hosted",
        "exported",
        "self_hosted_pending",
        "dev",
        "archived",
        "retired",
        "unreported",
    ];
    if !allowed.contains(&hosting_mode) {
        return json_error("unsupported hosting_mode");
    }

    let entries = read_registry_entries();
    let entry = entries
        .iter()
        .find(|site| site.domain == domain)
        .cloned()
        .unwrap_or_default();
    let cfg = load_site_config(&entry.dir);
    let effective_license = if !cfg.license_key.trim().is_empty() {
        cfg.license_key.clone()
    } else {
        entry.license.clone()
    };

    let mut j = state.journal.lock().await;
    let Some((client_id, mut client)) = find_nexus_client_for_site(&j, &effective_license, domain)
    else {
        return json_error("No Central Nexus client found for that site/domain.");
    };

    client.hosting_mode = Some(hosting_mode.to_string());
    client.deployment_status = req
        .deployment_status
        .as_deref()
        .map(str::trim)
        .filter(|status| !status.is_empty())
        .map(str::to_string)
        .or_else(|| {
            Some(
                if hosting_mode == "self_hosted" {
                    "active"
                } else {
                    hosting_mode
                }
                .into(),
            )
        });
    client.last_seen_domain = Some(domain.to_string());
    client.source_code_included.get_or_insert(false);
    if client.update_channel.is_none() {
        client.update_channel = Some(if hosting_mode == "self_hosted" {
            "manual".into()
        } else {
            "hosted-runtime".into()
        });
    }

    let mut notes = client.notes.unwrap_or_default();
    if !notes.is_empty() {
        notes.push_str("\n");
    }
    notes.push_str(&format!(
        "Site Fleet marked {domain} hosting_mode={hosting_mode} at {}.",
        now_rfc3339()
    ));
    if let Some(note) = req
        .note
        .as_deref()
        .map(str::trim)
        .filter(|note| !note.is_empty())
    {
        notes.push_str(&format!(" Note: {note}"));
    }
    client.notes = Some(notes);

    append_nexus_client(&mut j, &client_id, &client);
    Json(json!({
        "ok": true,
        "message": "Hosting mode updated",
        "domain": domain,
        "central_client_id": client_id,
        "hosting_mode": client.hosting_mode,
        "deployment_status": client.deployment_status,
        "license_status": client.license_status,
        "source_code_included": client.source_code_included,
    }))
}

async fn restore_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetRestoreRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let domain = req.domain.trim();
    if domain.is_empty() {
        return json_error("domain is required");
    }
    let script = format!("{}/site-fleet-restore.sh", fleet_scripts_dir());
    match run_sudo_capture(&script, &["--domain", domain]) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => json_error("Restore completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

async fn promote_fleet_site(
    State(state): State<CatalogState>,
    jar: CookieJar,
    Json(req): Json<FleetPromoteRequest>,
) -> Json<serde_json::Value> {
    let _claims = match require_admin(&state, &jar).await {
        Ok(claims) => claims,
        Err(resp) => return resp,
    };
    let domain = req.domain.trim();
    let license_key = req.license_key.trim();
    if domain.is_empty() || license_key.is_empty() {
        return json_error("domain and license_key are required");
    }
    let script = format!("{}/site-fleet-promote.sh", fleet_scripts_dir());
    match run_sudo_capture_owned(
        &script,
        &[
            "--domain".to_string(),
            domain.to_string(),
            "--license".to_string(),
            license_key.to_string(),
        ],
    ) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => Json(v),
            Err(_) => json_error("Promote completed but returned invalid JSON."),
        },
        Err(e) => json_error(&e),
    }
}

const ADMIN_JS: &str = r##"
// All catalog functions in a namespace object — avoids scoping issues
window._cat = window._cat || {};

window.moduleViews = window.moduleViews || {};
window.moduleViews['site-catalog'] = async function() {
    var main = document.getElementById('adminMain');
    if (!main) return;

    while (main.firstChild) main.removeChild(main.firstChild);

    var h = document.createElement('h2');
    h.textContent = 'Site Type Catalog';
    main.appendChild(h);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:24px;';
    desc.textContent = 'Every site type offered on the platform. Edit defaults for theme, modules, pages, pricing, and onboarding.';
    main.appendChild(desc);

    var resp = await fetch('/api/modules/site-catalog/types', { credentials: 'include' });
    var data = await resp.json();

    // Fetch live site stats
    var statsResp = await fetch('/api/modules/site-catalog/site-stats', { credentials: 'include' });
    var statsData = await statsResp.json();
    var siteStats = {};
    var totalSites = 0;
    if (statsData.ok) {
        totalSites = statsData.total_sites || 0;
        (statsData.data || []).forEach(function(s) { siteStats[s.industry] = s; });
    }
    desc.textContent = totalSites + ' live sites across all industries. Click any type to edit its defaults or view its sites.';
    if (!data.ok) { main.textContent = 'Failed to load catalog'; return; }

    var types = data.data;
    types.sort(function(a,b) { return a.display_order - b.display_order; });

    // Category headers
    var freeTypes = types.filter(function(t) { return t.category === 'free'; });
    var bizTypes = types.filter(function(t) { return t.category === 'business'; });

    function renderCategory(label, items) {
        var section = document.createElement('div');
        section.style.marginBottom = '32px';
        var h3 = document.createElement('h3');
        h3.textContent = label;
        h3.style.cssText = 'font-size:14px;text-transform:uppercase;letter-spacing:.08em;color:var(--text-muted,#94a3b8);margin-bottom:12px;';
        section.appendChild(h3);

        var grid = document.createElement('div');
        grid.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:16px;';

        items.forEach(function(t) {
            var card = document.createElement('div');
            card.style.cssText = 'background:var(--surface,#1e293b);border:1px solid var(--border,#334155);border-radius:12px;padding:20px;cursor:pointer;transition:border-color .2s;';
            card.addEventListener('mouseenter', function() { card.style.borderColor = 'var(--primary,#3b82f6)'; });
            card.addEventListener('mouseleave', function() { card.style.borderColor = 'var(--border,#334155)'; });

            var top = document.createElement('div');
            top.style.cssText = 'display:flex;align-items:center;gap:10px;margin-bottom:8px;';
            var emoji = document.createElement('span');
            emoji.style.fontSize = '24px';
            emoji.textContent = t.emoji;
            top.appendChild(emoji);
            var name = document.createElement('strong');
            name.textContent = t.name;
            name.style.fontSize = '16px';
            top.appendChild(name);
            if (t.always_free) {
                var badge = document.createElement('span');
                badge.textContent = 'FREE';
                badge.style.cssText = 'font-size:10px;background:#16a34a;color:#fff;padding:2px 6px;border-radius:4px;font-weight:700;margin-left:auto;';
                top.appendChild(badge);
            }
            card.appendChild(top);

            var descEl = document.createElement('div');
            descEl.style.cssText = 'font-size:13px;color:var(--text-muted,#94a3b8);line-height:1.4;margin-bottom:12px;';
            descEl.textContent = t.description;
            card.appendChild(descEl);

            var stats = document.createElement('div');
            stats.style.cssText = 'font-size:11px;color:var(--text-muted,#64748b);display:flex;gap:12px;';
            var ss = siteStats[t.slug] || { total: 0, last_24h: 0 };
            stats.textContent = ss.total + ' sites';
            if (ss.last_24h > 0) stats.textContent += ' (' + ss.last_24h + ' new today)';
            stats.textContent += ' \u2022 ' + t.enabled_modules.length + ' modules \u2022 ' + t.default_pages.length + ' pages';
            card.appendChild(stats);

            card.addEventListener('click', function() {
                window._cat.edit(t.slug);
            });
            grid.appendChild(card);
        });

        section.appendChild(grid);
        return section;
    }

    if (freeTypes.length) main.appendChild(renderCategory('Free Personal Sites', freeTypes));
    if (bizTypes.length) main.appendChild(renderCategory('Business Sites ($69/mo \u2022 $499/yr \u2022 $999 lifetime)', bizTypes));
};

// Edit views are handled inline — no hash routing needed.
// Card click calls loadSiteTypeEditor() directly.

window._cat.edit = async function(slug) {
    var main = document.getElementById('adminMain');
    if (!main) return;
    while (main.firstChild) main.removeChild(main.firstChild);

    var resp = await fetch('/api/modules/site-catalog/types/' + encodeURIComponent(slug), { credentials: 'include' });
    var data = await resp.json();
    if (!data.ok) { main.textContent = 'Site type not found: ' + slug; return; }
    var t = data.data;

    // ── Back button + header ──
    var backBtn = document.createElement('button');
    backBtn.textContent = '\u2190 Back to Catalog';
    backBtn.style.cssText = 'background:none;border:none;color:var(--primary,#3b82f6);cursor:pointer;font-size:14px;margin-bottom:16px;padding:0;';
    backBtn.onclick = function() { window.moduleViews['site-catalog'](); };
    main.appendChild(backBtn);

    var hdr = document.createElement('div');
    hdr.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:24px;';
    var emoji = document.createElement('span');
    emoji.style.fontSize = '32px';
    emoji.textContent = t.emoji;
    hdr.appendChild(emoji);
    var title = document.createElement('h2');
    title.textContent = t.name;
    title.style.margin = '0';
    hdr.appendChild(title);
    if (t.always_free) {
        var badge = document.createElement('span');
        badge.textContent = 'FREE';
        badge.style.cssText = 'font-size:11px;background:#16a34a;color:#fff;padding:3px 8px;border-radius:4px;font-weight:700;';
        hdr.appendChild(badge);
    } else {
        var badge = document.createElement('span');
        badge.textContent = 'PRO';
        badge.style.cssText = 'font-size:11px;background:#3b82f6;color:#fff;padding:3px 8px;border-radius:4px;font-weight:700;';
        hdr.appendChild(badge);
    }
    main.appendChild(hdr);

    // ── Tabs ──
    var tabs = ['General', 'Theme Presets', 'Modules', 'Pages', 'Nav Menu', 'Pricing', 'Onboarding'];
    var tabBar = document.createElement('div');
    tabBar.style.cssText = 'display:flex;gap:4px;border-bottom:1px solid var(--border,#334155);margin-bottom:24px;overflow-x:auto;';
    var tabContent = document.createElement('div');
    var activeTab = 'General';

    function renderTab(name) {
        tabContent.textContent = '';
        activeTab = name;
        // Update tab button styles
        for (var i = 0; i < tabBar.children.length; i++) {
            var btn = tabBar.children[i];
            btn.style.borderBottom = btn.dataset.tab === name ? '2px solid var(--primary,#3b82f6)' : '2px solid transparent';
            btn.style.color = btn.dataset.tab === name ? 'var(--text,#f1f5f9)' : 'var(--text-muted,#64748b)';
        }

        if (name === 'General') window._cat.generalTab(tabContent, t, slug);
        else if (name === 'Theme Presets') window._cat.themeTab(tabContent, t);
        else if (name === 'Modules') window._cat.modulesTab(tabContent, t, slug);
        else if (name === 'Pages') window._cat.pagesTab(tabContent, t);
        else if (name === 'Nav Menu') window._cat.navTab(tabContent, t);
        else if (name === 'Pricing') window._cat.pricingTab(tabContent, t);
        else if (name === 'Onboarding') window._cat.onboardingTab(tabContent, t, slug);
    }

    tabs.forEach(function(name) {
        var btn = document.createElement('button');
        btn.textContent = name;
        btn.dataset.tab = name;
        btn.style.cssText = 'background:none;border:none;padding:10px 16px;cursor:pointer;font-size:13px;font-weight:600;white-space:nowrap;border-bottom:2px solid transparent;';
        btn.onclick = function() { renderTab(name); };
        tabBar.appendChild(btn);
    });

    main.appendChild(tabBar);
    main.appendChild(tabContent);
    renderTab('General');
}

// ── Tab renderers ───────────────────────────────────────────────────

window._cat.field = function(label, value, onSave) {
    var wrap = document.createElement('div');
    wrap.style.cssText = 'margin-bottom:16px;';
    var lbl = document.createElement('label');
    lbl.textContent = label;
    lbl.style.cssText = 'display:block;font-size:12px;font-weight:600;color:var(--text-muted,#94a3b8);margin-bottom:4px;';
    wrap.appendChild(lbl);
    var inp = document.createElement('input');
    inp.type = 'text';
    inp.value = value || '';
    inp.style.cssText = 'width:100%;padding:8px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:8px;font-size:14px;';
    if (onSave) inp.addEventListener('change', function() { onSave(inp.value); });
    wrap.appendChild(inp);
    return wrap;
}

window._cat.saveBtn = function(slug, updates) {
    var btn = document.createElement('button');
    btn.textContent = 'Save Changes';
    btn.style.cssText = 'padding:10px 24px;background:var(--primary,#3b82f6);color:#fff;border:none;border-radius:8px;font-weight:600;cursor:pointer;margin-top:16px;';
    btn.onclick = async function() {
        btn.disabled = true;
        btn.textContent = 'Saving...';
        var resp = await fetch('/api/modules/site-catalog/types/' + encodeURIComponent(slug), {
            method: 'PUT', credentials: 'include',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updates)
        });
        var d = await resp.json();
        btn.disabled = false;
        btn.textContent = d.ok ? 'Saved!' : 'Error: ' + (d.message || 'unknown');
        if (d.ok) setTimeout(function() { btn.textContent = 'Save Changes'; }, 2000);
    };
    return btn;
}

window._cat.generalTab = function(container, t, slug) {
    var updates = {};
    container.appendChild(window._cat.field('Name', t.name, function(v) { updates.name = v; }));
    container.appendChild(window._cat.field('Emoji', t.emoji, function(v) { updates.emoji = v; }));
    container.appendChild(window._cat.field('Description', t.description, function(v) { updates.description = v; }));
    container.appendChild(window._cat.field('Default Tagline', t.default_tagline, function(v) { updates.default_tagline = v; }));

    // Category dropdown
    var catWrap = document.createElement('div');
    catWrap.style.marginBottom = '16px';
    var catLabel = document.createElement('label');
    catLabel.textContent = 'Category';
    catLabel.style.cssText = 'display:block;font-size:12px;font-weight:600;color:var(--text-muted,#94a3b8);margin-bottom:4px;';
    catWrap.appendChild(catLabel);
    var catSel = document.createElement('select');
    catSel.style.cssText = 'width:100%;padding:8px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:8px;font-size:14px;';
    ['free', 'business'].forEach(function(c) {
        var opt = document.createElement('option');
        opt.value = c;
        opt.textContent = c === 'free' ? 'Free Personal' : 'Business (Pro)';
        if (t.category === c) opt.selected = true;
        catSel.appendChild(opt);
    });
    catSel.onchange = function() { updates.category = catSel.value; };
    catWrap.appendChild(catSel);
    container.appendChild(catWrap);

    // Always free toggle
    var freeWrap = document.createElement('label');
    freeWrap.style.cssText = 'display:flex;align-items:center;gap:8px;font-size:14px;margin-bottom:16px;cursor:pointer;';
    var freeCheck = document.createElement('input');
    freeCheck.type = 'checkbox';
    freeCheck.checked = t.always_free;
    freeCheck.onchange = function() { updates.always_free = freeCheck.checked; };
    freeWrap.appendChild(freeCheck);
    freeWrap.appendChild(document.createTextNode('Always free (no payment required)'));
    container.appendChild(freeWrap);

    // Publicly listed toggle
    var listWrap = document.createElement('label');
    listWrap.style.cssText = 'display:flex;align-items:center;gap:8px;font-size:14px;margin-bottom:16px;cursor:pointer;';
    var listCheck = document.createElement('input');
    listCheck.type = 'checkbox';
    listCheck.checked = t.publicly_listed;
    listCheck.onchange = function() { updates.publicly_listed = listCheck.checked; };
    listWrap.appendChild(listCheck);
    listWrap.appendChild(document.createTextNode('Publicly listed on get-started page'));
    container.appendChild(listWrap);

    container.appendChild(window._cat.saveBtn(slug, updates));
}

window._cat.themeTab = function(container, t) {
    var presets = t.theme_presets || [];
    var heading = document.createElement('p');
    heading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:16px;font-size:14px;';
    heading.textContent = presets.length + ' theme presets. These are the design options shown to new sites of this type.';
    container.appendChild(heading);

    presets.forEach(function(p) {
        var card = document.createElement('div');
        card.style.cssText = 'background:var(--surface,#1e293b);border:1px solid var(--border,#334155);border-radius:10px;padding:16px;margin-bottom:12px;';

        var top = document.createElement('div');
        top.style.cssText = 'display:flex;justify-content:space-between;align-items:center;margin-bottom:8px;';
        var name = document.createElement('strong');
        name.textContent = p.display_order + '. ' + p.name;
        top.appendChild(name);
        var navBadge = document.createElement('span');
        navBadge.textContent = (p.profile.nav_layout || p.nav_style || '').replace(/_/g, ' ');
        navBadge.style.cssText = 'font-size:10px;background:var(--bg,#0f172a);color:var(--text-muted,#94a3b8);padding:2px 8px;border-radius:4px;text-transform:uppercase;letter-spacing:.05em;';
        top.appendChild(navBadge);
        card.appendChild(top);

        var desc = document.createElement('div');
        desc.textContent = p.description;
        desc.style.cssText = 'font-size:13px;color:var(--text-muted,#64748b);margin-bottom:10px;';
        card.appendChild(desc);

        // Color swatches
        var tokens = p.profile.tokens || {};
        var swatches = document.createElement('div');
        swatches.style.cssText = 'display:flex;gap:6px;align-items:center;';
        ['primary', 'accent', 'header_bg', 'background'].forEach(function(key) {
            if (tokens[key]) {
                var sw = document.createElement('div');
                sw.style.cssText = 'width:24px;height:24px;border-radius:6px;border:1px solid var(--border,#334155);';
                sw.style.background = tokens[key];
                sw.title = key + ': ' + tokens[key];
                swatches.appendChild(sw);
            }
        });
        var fontLabel = document.createElement('span');
        fontLabel.textContent = tokens.body_font || '?';
        fontLabel.style.cssText = 'font-size:11px;color:var(--text-muted,#64748b);margin-left:8px;';
        swatches.appendChild(fontLabel);
        var radiusLabel = document.createElement('span');
        radiusLabel.textContent = 'r=' + (tokens.radius || 0);
        radiusLabel.style.cssText = 'font-size:11px;color:var(--text-muted,#64748b);';
        swatches.appendChild(radiusLabel);
        card.appendChild(swatches);

        container.appendChild(card);
    });

    // "Edit with Design Playground" hint
    var hint = document.createElement('p');
    hint.style.cssText = 'color:var(--text-muted,#64748b);font-size:12px;margin-top:16px;font-style:italic;';
    hint.textContent = 'To visually edit a preset, open the Design Playground on a site of this type, customize it, then click "Save as Catalog Default" (coming soon).';
    container.appendChild(hint);
}

window._cat.modulesTab = function(container, t, slug) {
    var heading = document.createElement('p');
    heading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:16px;font-size:14px;';
    heading.textContent = t.enabled_modules.length + ' modules enabled by default for new ' + t.name + ' sites.';
    container.appendChild(heading);

    var list = document.createElement('div');
    list.style.cssText = 'display:flex;flex-wrap:wrap;gap:6px;';
    t.enabled_modules.forEach(function(m) {
        var chip = document.createElement('span');
        chip.textContent = m;
        chip.style.cssText = 'font-size:12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);padding:4px 10px;border-radius:6px;border:1px solid var(--border,#334155);';
        list.appendChild(chip);
    });
    container.appendChild(list);
}

window._cat.pagesTab = function(container, t) {
    var heading = document.createElement('p');
    heading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:16px;font-size:14px;';
    heading.textContent = t.default_pages.length + ' pages created on new sites.';
    container.appendChild(heading);

    t.default_pages.forEach(function(p) {
        var row = document.createElement('div');
        row.style.cssText = 'display:flex;align-items:center;gap:12px;padding:10px 0;border-bottom:1px solid var(--border,#1e293b);';
        var slug = document.createElement('code');
        slug.textContent = '/' + p.slug;
        slug.style.cssText = 'font-size:13px;color:var(--primary,#3b82f6);min-width:120px;';
        row.appendChild(slug);
        var title = document.createElement('span');
        title.textContent = p.title;
        title.style.cssText = 'font-size:14px;flex:1;';
        row.appendChild(title);
        var seo = document.createElement('span');
        seo.textContent = p.seo_description || '';
        seo.style.cssText = 'font-size:11px;color:var(--text-muted,#64748b);max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        row.appendChild(seo);
        container.appendChild(row);
    });
}

window._cat.navTab = function(container, t) {
    var heading = document.createElement('p');
    heading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:16px;font-size:14px;';
    heading.textContent = t.default_nav_items.length + ' navigation links for new sites.';
    container.appendChild(heading);

    t.default_nav_items.forEach(function(item) {
        var row = document.createElement('div');
        row.style.cssText = 'display:flex;align-items:center;gap:10px;padding:8px 0;border-bottom:1px solid var(--border,#1e293b);';
        var em = document.createElement('span');
        em.textContent = item.emoji;
        em.style.fontSize = '18px';
        row.appendChild(em);
        var label = document.createElement('span');
        label.textContent = item.title;
        label.style.cssText = 'font-size:14px;min-width:100px;';
        row.appendChild(label);
        var url = document.createElement('code');
        url.textContent = item.url;
        url.style.cssText = 'font-size:12px;color:var(--text-muted,#64748b);';
        row.appendChild(url);
        if (item.children && item.children.length) {
            var sub = document.createElement('span');
            sub.textContent = item.children.length + ' sub-items';
            sub.style.cssText = 'font-size:11px;color:var(--primary,#3b82f6);margin-left:auto;';
            row.appendChild(sub);
        }
        container.appendChild(row);
    });
}

window._cat.pricingTab = function(container, t) {
    container.appendChild(window._cat.field('Default Tier', t.default_tier));
    var freeNote = document.createElement('p');
    freeNote.style.cssText = 'font-size:13px;color:var(--text-muted,#94a3b8);margin-bottom:16px;';
    freeNote.textContent = t.always_free
        ? 'This type is always free. Users get the "' + t.default_tier + '" tier automatically.'
        : 'Business tier. Users choose: $69/mo, $499/yr, or $999 lifetime.';
    container.appendChild(freeNote);

    if (t.limited_time_offer) {
        var offer = document.createElement('div');
        offer.style.cssText = 'background:var(--surface,#1e293b);border:1px solid var(--border,#334155);border-radius:10px;padding:16px;margin-bottom:16px;';
        var offerTitle = document.createElement('strong');
        offerTitle.textContent = t.limited_time_offer.label;
        offer.appendChild(offerTitle);
        var offerDesc = document.createElement('p');
        offerDesc.textContent = t.limited_time_offer.description;
        offerDesc.style.cssText = 'font-size:13px;color:var(--text-muted,#94a3b8);margin:8px 0 0;';
        offer.appendChild(offerDesc);
        container.appendChild(offer);
    }

    if (t.discount_codes && t.discount_codes.length) {
        var dcTitle = document.createElement('h4');
        dcTitle.textContent = 'Discount Codes';
        dcTitle.style.cssText = 'margin:16px 0 8px;font-size:14px;';
        container.appendChild(dcTitle);
        t.discount_codes.forEach(function(dc) {
            var row = document.createElement('div');
            row.style.cssText = 'font-size:13px;padding:6px 0;';
            row.textContent = dc.code + ' \u2014 ' + dc.discount_type + ' (' + dc.value + ') \u2022 ' + dc.uses + '/' + (dc.max_uses || '\u221E') + ' uses';
            container.appendChild(row);
        });
    }
}

window._cat.onboardingTab = function(container, t, slug) {
    var steps = JSON.parse(JSON.stringify(t.onboarding_steps || []));
    var updates = { onboarding_steps: steps };
    var loading = document.createElement('p');
    loading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:16px;font-size:14px;';
    loading.textContent = 'Loading onboarding details...';
    container.appendChild(loading);

    function paint(usageData) {
        while (container.firstChild) container.removeChild(container.firstChild);

        var usageByKey = {};
        (usageData || []).forEach(function(stepInfo) {
            (stepInfo.fields || []).forEach(function(fieldInfo) {
                usageByKey[(stepInfo.step_id || '') + '::' + (fieldInfo.key || '')] = fieldInfo;
            });
        });

        var heading = document.createElement('p');
        heading.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:12px;font-size:14px;';
        heading.textContent = steps.length
            ? steps.length + ' onboarding steps. Edit the customer help text, leave internal notes, and see where each answer is used.'
            : 'No custom onboarding steps. Uses generic wizard.';
        container.appendChild(heading);

        var explainer = document.createElement('div');
        explainer.style.cssText = 'background:var(--surface,#1e293b);border:1px solid var(--border,#334155);border-radius:10px;padding:14px 16px;margin-bottom:18px;';
        explainer.innerHTML =
            '<div style="font-weight:700;margin-bottom:6px;">Site-Type Setup Control Center</div>' +
            '<div style="font-size:13px;color:var(--text-muted,#94a3b8);line-height:1.5;">' +
            'This tab controls what customers are asked during onboarding. The <strong>Modules</strong>, <strong>Pages</strong>, <strong>Nav Menu</strong>, and <strong>Pricing</strong> tabs still control what gets applied by default after signup.' +
            '</div>';
        container.appendChild(explainer);

        if (!steps.length) {
            return;
        }

        steps.forEach(function(step, stepIndex) {
            var card = document.createElement('div');
            card.style.cssText = 'background:var(--surface,#1e293b);border:1px solid var(--border,#334155);border-radius:12px;padding:18px;margin-bottom:14px;';

            var top = document.createElement('div');
            top.style.cssText = 'display:flex;justify-content:space-between;align-items:flex-start;gap:12px;margin-bottom:12px;';
            var left = document.createElement('div');
            var stepTitle = document.createElement('strong');
            stepTitle.textContent = step.label;
            left.appendChild(stepTitle);
            var meta = document.createElement('div');
            meta.style.cssText = 'font-size:12px;color:var(--text-muted,#94a3b8);margin-top:4px;';
            meta.textContent = (step.fields || []).length + ' fields' + (step.skippable ? ' • skippable' : '');
            left.appendChild(meta);
            top.appendChild(left);
            var badge = document.createElement('span');
            badge.textContent = step.step_id;
            badge.style.cssText = 'font-size:10px;background:var(--bg,#0f172a);color:var(--text-muted,#94a3b8);padding:3px 8px;border-radius:999px;border:1px solid var(--border,#334155);';
            top.appendChild(badge);
            card.appendChild(top);

            (step.fields || []).forEach(function(field, fieldIndex) {
                var usage = usageByKey[(step.step_id || '') + '::' + (field.key || '')] || {};
                if (!field.help_text && usage.help_text) field.help_text = usage.help_text;
                if (!field.admin_notes && usage.admin_notes) field.admin_notes = usage.admin_notes;

                var fieldCard = document.createElement('div');
                fieldCard.style.cssText = 'border:1px solid var(--border,#334155);border-radius:10px;padding:14px;margin-top:12px;background:rgba(15,23,42,0.35);';

                var titleRow = document.createElement('div');
                titleRow.style.cssText = 'display:flex;justify-content:space-between;gap:12px;align-items:flex-start;';
                var titleWrap = document.createElement('div');
                var title = document.createElement('div');
                title.style.cssText = 'font-size:14px;font-weight:700;';
                title.textContent = field.label;
                titleWrap.appendChild(title);
                var sub = document.createElement('div');
                sub.style.cssText = 'font-size:12px;color:var(--text-muted,#94a3b8);margin-top:4px;';
                sub.textContent = field.key + ' • ' + field.field_type + (field.required ? ' • required' : ' • optional');
                if (field.options && field.options.length) sub.textContent += ' • ' + field.options.length + ' options';
                titleWrap.appendChild(sub);
                titleRow.appendChild(titleWrap);
                var qBadge = document.createElement('span');
                qBadge.textContent = '? help';
                qBadge.style.cssText = 'font-size:10px;background:#1d4ed8;color:#fff;padding:3px 8px;border-radius:999px;font-weight:700;text-transform:uppercase;letter-spacing:.04em;';
                titleRow.appendChild(qBadge);
                fieldCard.appendChild(titleRow);

                var usedTitle = document.createElement('div');
                usedTitle.style.cssText = 'font-size:12px;font-weight:700;color:var(--text-muted,#cbd5e1);margin:12px 0 8px;';
                usedTitle.textContent = 'Used In';
                fieldCard.appendChild(usedTitle);

                var usedWrap = document.createElement('div');
                usedWrap.style.cssText = 'display:flex;flex-wrap:wrap;gap:6px;margin-bottom:12px;';
                var hints = usage.used_in || [];
                hints.forEach(function(hint) {
                    var chip = document.createElement('span');
                    chip.textContent = hint;
                    chip.style.cssText = 'font-size:11px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);padding:4px 8px;border-radius:999px;border:1px solid var(--border,#334155);';
                    usedWrap.appendChild(chip);
                });
                if (!hints.length) {
                    var none = document.createElement('span');
                    none.textContent = 'Starter pages and feature defaults';
                    none.style.cssText = 'font-size:11px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);padding:4px 8px;border-radius:999px;border:1px solid var(--border,#334155);';
                    usedWrap.appendChild(none);
                }
                fieldCard.appendChild(usedWrap);

                var helpLabel = document.createElement('label');
                helpLabel.textContent = 'Customer Help Text (?)';
                helpLabel.style.cssText = 'display:block;font-size:12px;font-weight:700;color:var(--text-muted,#cbd5e1);margin-bottom:6px;';
                fieldCard.appendChild(helpLabel);
                var help = document.createElement('textarea');
                help.value = field.help_text || '';
                help.placeholder = 'Shown to customers when they click the ? help on this question.';
                help.rows = 3;
                help.style.cssText = 'width:100%;padding:10px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:8px;font-size:13px;resize:vertical;min-height:78px;margin-bottom:12px;';
                help.oninput = function() {
                    steps[stepIndex].fields[fieldIndex].help_text = help.value;
                };
                fieldCard.appendChild(help);

                var noteLabel = document.createElement('label');
                noteLabel.textContent = 'Internal Notes';
                noteLabel.style.cssText = 'display:block;font-size:12px;font-weight:700;color:var(--text-muted,#cbd5e1);margin-bottom:6px;';
                fieldCard.appendChild(noteLabel);
                var notes = document.createElement('textarea');
                notes.value = field.admin_notes || '';
                notes.placeholder = 'Only visible in Central. Use this for operator notes, reminders, or future automation ideas.';
                notes.rows = 2;
                notes.style.cssText = 'width:100%;padding:10px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:8px;font-size:13px;resize:vertical;min-height:64px;';
                notes.oninput = function() {
                    steps[stepIndex].fields[fieldIndex].admin_notes = notes.value;
                };
                fieldCard.appendChild(notes);

                card.appendChild(fieldCard);
            });

            container.appendChild(card);
        });

        container.appendChild(window._cat.saveBtn(slug, updates));
    }

    fetch('/api/modules/site-catalog/types/' + encodeURIComponent(slug) + '/onboarding-usage', { credentials: 'include' })
        .then(function(resp) { return resp.json(); })
        .then(function(data) { paint(data.ok ? (data.data || []) : []); })
        .catch(function() { paint([]); });
}

window._cat.fleetToast = function(message, kind) {
    if (typeof showToast === 'function') return showToast(message, kind || 'success');
    if (typeof toast === 'function') return toast(message);
    alert(message);
};

window._cat.fleetFetch = async function(path, opts) {
    var resp = await fetch(path, Object.assign({ credentials: 'include' }, opts || {}));
    var data = await resp.json().catch(function() { return { ok: false, message: 'Invalid JSON response' }; });
    if (!resp.ok || !data.ok) throw new Error(data.message || ('Request failed: ' + resp.status));
    return data;
};

window._cat.fleetActionBtn = function(label, tone, onclick) {
    var btn = document.createElement('button');
    btn.textContent = label;
    btn.style.cssText = 'border:1px solid ' + (tone === 'danger' ? '#ef4444' : tone === 'primary' ? 'var(--primary,#3b82f6)' : 'var(--border,#334155)') + ';background:' + (tone === 'danger' ? '#7f1d1d' : tone === 'primary' ? 'var(--primary,#3b82f6)' : 'transparent') + ';color:#fff;padding:6px 10px;border-radius:8px;font-size:12px;font-weight:600;cursor:pointer;';
    btn.onclick = onclick;
    return btn;
};

window._cat.fleetEsc = function(value) {
    return String(value == null ? '' : value)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
};

window._cat.fleetList = function(values, fallback) {
    if (!Array.isArray(values) || !values.length) return fallback || '-';
    return values.filter(Boolean).slice(0, 3).map(window._cat.fleetEsc).join(', ');
};

window._cat.fleetDate = function(unixSeconds) {
    if (!unixSeconds) return '-';
    var d = new Date(Number(unixSeconds) * 1000);
    if (isNaN(d.getTime())) return '-';
    return d.toISOString().slice(0, 10);
};

window._cat.backupSite = async function(domain) {
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/backup', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ domain: domain })
    });
    window._cat.fleetToast('Backup created: ' + (data.backup_dir || 'done'), 'success');
};

window._cat.runScheduledBackups = async function(dryRun) {
    var domain = prompt('Optional domain to limit this backup run. Leave blank for every due hosted site.', '');
    if (!dryRun && !confirm('Run due backups now? This can create many backup folders if many sites are due.')) return;
    var body = { dry_run: !!dryRun };
    if (domain && domain.trim()) body.domain = domain.trim();
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/scheduled-backups', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    });
    var msg = (dryRun ? 'Backup dry-run: ' : 'Due backups complete: ') + (data.count || 0) + ' site results';
    window._cat.fleetToast(msg, 'success');
    return data;
};

window._cat.loadFleetReconcile = async function() {
    return window._cat.fleetFetch('/api/modules/site-catalog/fleet/reconcile');
};

window._cat.exportSite = async function(domain) {
    var mode = prompt('Export mode: handoff redacts server secrets; internal is for LuperIQ recovery. Type handoff or internal.', 'handoff');
    if (!mode) return;
    mode = String(mode).trim().toLowerCase();
    if (mode !== 'handoff' && mode !== 'internal') {
        window._cat.fleetToast('Export cancelled: mode must be handoff or internal.', 'error');
        return;
    }
    var includeSecrets = false;
    if (mode === 'internal') {
        includeSecrets = confirm('Internal exports may include raw server secrets and should not be handed to clients. Continue?');
        if (!includeSecrets) return;
    }
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/export', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ domain: domain, mode: mode, include_secrets: includeSecrets })
    });
    var msg = 'Export ready: ' + (data.archive_file || data.archive_path || 'created');
    if (data.download_url) msg += ' • opening download';
    window._cat.fleetToast(msg, 'success');
    if (data.download_url) window.open(data.download_url, '_blank', 'noopener');
};

window._cat.importHandoff = async function() {
    var archivePath = prompt('Server archive path to import. Use a .tar.gz under /ai/backups/site-fleet/exports, /ai/backups, /mnt/server-bu, or /home/dave/backups.');
    if (!archivePath) return;
    var domain = prompt('Hosted domain to create, for example client-name.luperiq.com');
    if (!domain) return;
    var licenseKey = prompt('Central license key to attach to this returned hosted site');
    if (!licenseKey) return;
    var siteName = prompt('Optional site name override', '');
    var adminEmail = prompt('Optional admin email override', '');
    var dryRun = confirm('Run a dry-run first? OK = dry-run, Cancel = import now.');
    var body = {
        archive_path: archivePath.trim(),
        domain: domain.trim(),
        license_key: licenseKey.trim(),
        dry_run: dryRun
    };
    if (siteName && siteName.trim()) body.site_name = siteName.trim();
    if (adminEmail && adminEmail.trim()) body.admin_email = adminEmail.trim();
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/import-handoff', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    });
    var msg = dryRun ? 'Import dry-run ok: ' + data.domain : 'Hosted import ready: ' + data.domain;
    if (data.generated_bootstrap_admin_password && !dryRun && !data.existing_data_present) msg += ' • bootstrap password is in the JSON response';
    window._cat.fleetToast(msg, 'success');
    if (!dryRun) window.moduleViews['site-fleet']();
};

window._cat.cloneSite = async function(domain, mode) {
    var defaultTarget = domain.replace('.luperiq.com', mode === 'dev' ? '-dev.luperiq.com' : '-copy.luperiq.com');
    var targetDomain = prompt((mode === 'dev' ? 'Dev clone' : 'Live duplicate') + ' target domain', defaultTarget);
    if (!targetDomain) return;
    var siteName = prompt('Optional site name override', '');
    var body = {
        domain: domain,
        target_domain: targetDomain.trim(),
        mode: mode
    };
    if (siteName && siteName.trim()) body.site_name = siteName.trim();
    if (mode === 'clone') {
        var licenseKey = prompt('New license key for the live duplicate');
        if (!licenseKey) return;
        body.license_key = licenseKey.trim();
    }
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/clone', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    });
    window._cat.fleetToast((mode === 'dev' ? 'Dev clone ready: ' : 'Duplicate ready: ') + data.domain, 'success');
    window.moduleViews['site-fleet']();
};

window._cat.deleteSite = async function(domain) {
    if (!confirm('Archive this hosted copy? A backup will be taken first and the site directory will be archived. The Central license will be preserved unless you explicitly revoke it from Nexus Clients.')) return;
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/delete', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ domain: domain, revoke_license: false })
    });
    var msg = 'Site archived: ' + domain;
    if (data.backup_dir) msg += ' • backup: ' + data.backup_dir;
    if (data.license_preserved) msg += ' • license preserved';
    window._cat.fleetToast(msg, 'success');
    window.moduleViews['site-fleet']();
};

window._cat.markHostingMode = async function(domain, hostingMode, deploymentStatus) {
    var note = prompt('Optional note for Central history', '');
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/hosting-mode', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
            domain: domain,
            hosting_mode: hostingMode,
            deployment_status: deploymentStatus || (hostingMode === 'self_hosted' ? 'active' : hostingMode),
            note: note || null
        })
    });
    window._cat.fleetToast('Hosting mode updated: ' + (data.hosting_mode || hostingMode), 'success');
    window.moduleViews['site-fleet']();
};

window._cat.restoreSite = async function(domain) {
    if (!confirm('Restore this archived site back to live hosting? If it had a Central license, Site Fleet will try to reactivate it too.')) return;
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/restore', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ domain: domain })
    });
    var msg = 'Site restored: ' + domain;
    if (data.activation_ok) msg += ' • Central reactivated';
    window._cat.fleetToast(msg, 'success');
    window.moduleViews['site-fleet']();
};

window._cat.promoteSite = async function(domain) {
    var licenseKey = prompt('New license key for this dev site');
    if (!licenseKey) return;
    var data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet/promote', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ domain: domain, license_key: licenseKey.trim() })
    });
    var msg = 'Dev site promoted: ' + data.domain;
    if (data.activation_ok) msg += ' • Central activated';
    window._cat.fleetToast(msg, 'success');
    window.moduleViews['site-fleet']();
};

window.moduleViews['site-fleet'] = async function() {
    var main = document.getElementById('adminMain');
    if (!main) return;
    main.replaceChildren();

    var h = document.createElement('h2');
    h.textContent = 'Site Fleet';
    main.appendChild(h);

    var desc = document.createElement('p');
    desc.style.cssText = 'color:var(--text-muted,#94a3b8);margin-bottom:20px;';
    desc.textContent = 'Central view of every provisioned site and client deployment. Use this to back up or export a site, create dev clones, promote dev sites, duplicate licensed sites, mark hosted/self-hosted movement, restore archived sites, or archive a hosted copy with recovery data preserved.';
    main.appendChild(desc);

    var toolbar = document.createElement('div');
    toolbar.style.cssText = 'display:flex;gap:12px;align-items:center;flex-wrap:wrap;margin-bottom:16px;';
    var search = document.createElement('input');
    search.type = 'search';
    search.placeholder = 'Search domain, name, industry, email, license...';
    search.style.cssText = 'min-width:320px;flex:1;padding:10px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:10px;';
    toolbar.appendChild(search);
    var filter = document.createElement('select');
    filter.style.cssText = 'padding:10px 12px;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:10px;';
    ['all','active','deleted','dev','live','hosted','self_hosted','exported'].forEach(function(opt) {
        var o = document.createElement('option');
        o.value = opt; o.textContent = opt === 'all' ? 'All statuses' : opt.charAt(0).toUpperCase() + opt.slice(1);
        filter.appendChild(o);
    });
    toolbar.appendChild(filter);
    var dryRunBackupsBtn = window._cat.fleetActionBtn('Backup Dry Run', 'ghost', function() {
        window._cat.runScheduledBackups(true).catch(function(e) { window._cat.fleetToast(e.message || 'Backup dry-run failed', 'error'); });
    });
    toolbar.appendChild(dryRunBackupsBtn);
    var dueBackupsBtn = window._cat.fleetActionBtn('Run Due Backups', 'ghost', function() {
        window._cat.runScheduledBackups(false).catch(function(e) { window._cat.fleetToast(e.message || 'Backup run failed', 'error'); });
    });
    toolbar.appendChild(dueBackupsBtn);
    var auditBtn = window._cat.fleetActionBtn('License Audit', 'ghost', function() {
        auditBox.textContent = 'Checking Commerce, Nexus, and hosted registry matches...';
        auditBox.style.display = 'block';
        window._cat.loadFleetReconcile().then(function(data) {
            var issues = data.issues || {};
            var names = Object.keys(issues);
            var lines = [
                'Generated: ' + (data.generated_at || '-'),
                'Hosted: ' + ((data.summary || {}).active_hosted_domains || 0) + ' active / ' + ((data.summary || {}).hosted_registry_entries || 0) + ' registry entries',
                'Nexus clients: ' + ((data.summary || {}).nexus_clients || 0),
                'Commerce entitlements: ' + ((data.summary || {}).commerce_entitlements || 0),
                'Issue count: ' + ((data.summary || {}).issue_count || 0),
                ''
            ];
            names.forEach(function(name) {
                var group = issues[name] || [];
                lines.push(name.replace(/_/g, ' ') + ': ' + group.length);
                group.slice(0, 5).forEach(function(item) {
                    var label = item.domain || item.assigned_to_site || item.owner_email || item.license_key || item.client_id || item.entitlement_id || 'item';
                    lines.push('  - ' + label);
                });
                if (group.length > 5) lines.push('  ... ' + (group.length - 5) + ' more');
            });
            auditBox.textContent = lines.join('\\n');
            window._cat.fleetToast('License audit complete: ' + ((data.summary || {}).issue_count || 0) + ' items flagged', 'success');
        }).catch(function(e) {
            auditBox.textContent = 'License audit failed: ' + (e.message || e);
            window._cat.fleetToast(e.message || 'License audit failed', 'error');
        });
    });
    toolbar.appendChild(auditBtn);
    var importBtn = window._cat.fleetActionBtn('Import Handoff', 'primary', function() {
        window._cat.importHandoff().catch(function(e) { window._cat.fleetToast(e.message || 'Import failed', 'error'); });
    });
    toolbar.appendChild(importBtn);
    main.appendChild(toolbar);

    var statusLine = document.createElement('div');
    statusLine.style.cssText = 'font-size:12px;color:var(--text-muted,#94a3b8);margin-bottom:12px;';
    main.appendChild(statusLine);

    var auditBox = document.createElement('pre');
    auditBox.style.cssText = 'display:none;white-space:pre-wrap;max-height:320px;overflow:auto;background:var(--bg,#0f172a);color:var(--text,#f1f5f9);border:1px solid var(--border,#334155);border-radius:12px;padding:14px;margin:0 0 14px;font-size:12px;line-height:1.5;';
    main.appendChild(auditBox);

    var tableWrap = document.createElement('div');
    tableWrap.style.cssText = 'overflow:auto;border:1px solid var(--border,#334155);border-radius:12px;';
    main.appendChild(tableWrap);

    var data;
    try {
        data = await window._cat.fleetFetch('/api/modules/site-catalog/fleet');
    } catch (e) {
        tableWrap.textContent = 'Failed to load site fleet: ' + e.message;
        return;
    }
    var sites = data.data || [];

    function render() {
        tableWrap.replaceChildren();
        var q = (search.value || '').trim().toLowerCase();
        var mode = filter.value;
        var filtered = sites.filter(function(site) {
            if (mode === 'active' && (site.deleted || site.status === 'deleted')) return false;
            if (mode === 'deleted' && !(site.deleted || site.status === 'deleted')) return false;
            if (mode === 'dev' && site.mode !== 'dev') return false;
            if (mode === 'live' && site.mode === 'dev') return false;
            if (mode === 'hosted' && site.hosting_mode !== 'hosted') return false;
            if (mode === 'self_hosted' && site.hosting_mode !== 'self_hosted') return false;
            if (mode === 'exported' && site.hosting_mode !== 'exported') return false;
            if (!q) return true;
            var hay = [
                site.domain,
                site.site_name,
                site.industry,
                site.admin_email,
                site.license_key,
                site.service_name,
                site.hosting_mode,
                site.deployment_status
            ].join(' ').toLowerCase();
            return hay.indexOf(q) !== -1;
        });
        statusLine.textContent = filtered.length + ' of ' + sites.length + ' sites shown';

        var table = document.createElement('table');
        table.style.cssText = 'width:100%;border-collapse:collapse;font-size:13px;';
        var thead = document.createElement('thead');
        thead.innerHTML = '<tr style=\"background:var(--surface,#1e293b);text-align:left;\"><th style=\"padding:12px;\">Site</th><th style=\"padding:12px;\">Type</th><th style=\"padding:12px;\">License</th><th style=\"padding:12px;\">AI</th><th style=\"padding:12px;\">Service</th><th style=\"padding:12px;\">Actions</th></tr>';
        table.appendChild(thead);
        var tbody = document.createElement('tbody');

        filtered.forEach(function(site) {
            var tr = document.createElement('tr');
            tr.style.borderTop = '1px solid var(--border,#334155)';

            var siteTd = document.createElement('td');
            siteTd.style.padding = '12px';
            var siteTitle = document.createElement('div');
            siteTitle.style.cssText = 'display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:4px;';
            var titleStrong = document.createElement('strong');
            titleStrong.textContent = site.site_name || site.domain;
            siteTitle.appendChild(titleStrong);
            var modeBadge = document.createElement('span');
            modeBadge.textContent = site.deleted ? 'ARCHIVED' : ((site.hosting_mode || (site.mode === 'dev' ? 'dev' : 'hosted')).replace('_', ' ').toUpperCase());
            modeBadge.style.cssText = 'font-size:10px;padding:2px 6px;border-radius:999px;background:' + (site.deleted ? '#7f1d1d' : site.hosting_mode === 'self_hosted' ? '#0f766e' : site.mode === 'dev' ? '#312e81' : '#14532d') + ';color:#fff;font-weight:700;';
            siteTitle.appendChild(modeBadge);
            siteTd.appendChild(siteTitle);
            var domain = document.createElement('div');
            domain.innerHTML = '<a href=\"' + site.url + '\" target=\"_blank\" style=\"color:var(--primary,#3b82f6);text-decoration:none;\">' + site.domain + '</a> • <a href=\"' + site.admin_url + '\" target=\"_blank\" style=\"color:var(--primary,#3b82f6);text-decoration:none;\">Admin</a>';
            siteTd.appendChild(domain);
            var meta = document.createElement('div');
            meta.style.cssText = 'margin-top:4px;color:var(--text-muted,#94a3b8);font-size:12px;';
            meta.textContent = (site.admin_email || 'no admin email') + ' • created ' + (site.created || 'unknown') + ' • deployment ' + (site.deployment_status || 'active');
            siteTd.appendChild(meta);
            tbody.appendChild(tr);
            tr.appendChild(siteTd);

            var typeTd = document.createElement('td');
            typeTd.style.padding = '12px';
            typeTd.innerHTML = '<div><strong>' + (site.industry || 'unknown') + '</strong></div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;margin-top:4px;\">' + (site.module_count || 0) + ' modules • port ' + (site.port || '-') + '</div>';
            tr.appendChild(typeTd);

            var licenseTd = document.createElement('td');
            licenseTd.style.padding = '12px';
            var lic = site.nexus || {};
            var commerce = site.commerce || {};
            var licenseHtml = '';
            if (site.license_key) {
                licenseHtml += '<div><code>' + window._cat.fleetEsc(site.license_key) + '</code></div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;margin-top:4px;\">' + window._cat.fleetEsc((lic.license_tier || 'free') + ' • ' + (lic.license_status || 'unknown')) + '</div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;\">Credits: ' + Number((lic.credits_remaining || 0) + (lic.bundle_credits_remaining || 0)).toLocaleString() + '</div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;\">Update: ' + window._cat.fleetEsc(lic.update_channel || 'manual') + ' • Last seen: ' + window._cat.fleetEsc(lic.last_heartbeat_at || '-') + '</div>';
            } else {
                licenseHtml += '<div style=\"color:var(--text-muted,#94a3b8);\">No Central license</div>';
            }
            if (commerce && commerce.count) {
                licenseHtml += '<div style=\"margin-top:8px;padding-top:8px;border-top:1px solid var(--border,#334155);color:var(--text-muted,#94a3b8);font-size:12px;\">';
                licenseHtml += '<div><strong style=\"color:var(--text,#f1f5f9);\">Commerce:</strong> ' + Number(commerce.active_count || 0).toLocaleString() + '/' + Number(commerce.count || 0).toLocaleString() + ' active</div>';
                licenseHtml += '<div>Owner: ' + window._cat.fleetList(commerce.owner_emails, '-') + '</div>';
                licenseHtml += '<div>Tier: ' + window._cat.fleetList(commerce.tiers, '-') + ' • ' + window._cat.fleetList(commerce.billing_periods, '-') + '</div>';
                licenseHtml += '<div>Paid through: ' + window._cat.fleetEsc(window._cat.fleetDate(commerce.paid_through_max)) + ' • Stripe subs: ' + Number(commerce.stripe_subscription_count || 0).toLocaleString() + '</div>';
                licenseHtml += '</div>';
            }
            licenseTd.innerHTML = licenseHtml;
            tr.appendChild(licenseTd);

            var aiTd = document.createElement('td');
            aiTd.style.padding = '12px';
            var aiBits = [];
            if (site.has_ai_quick) aiBits.push('Quick');
            if (site.has_ai_content) aiBits.push('Content');
            if (site.has_ai_escalation) aiBits.push('Escalation');
            aiTd.innerHTML = '<div>' + (aiBits.length ? aiBits.join(', ') : 'None') + '</div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;margin-top:4px;\">WAL ' + Number(site.wal_bytes || 0).toLocaleString() + ' B</div>';
            tr.appendChild(aiTd);

            var svcTd = document.createElement('td');
            svcTd.style.padding = '12px';
            svcTd.innerHTML = '<div><strong>' + (site.service_state || site.status || 'unknown') + '</strong></div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;margin-top:4px;\">' + (site.service_name || '-') + '</div><div style=\"color:var(--text-muted,#94a3b8);font-size:12px;\">' + (site.dir || '-') + '</div>';
            tr.appendChild(svcTd);

            var actionsTd = document.createElement('td');
            actionsTd.style.padding = '12px';
            var actions = document.createElement('div');
            actions.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;';
            if (site.deleted) {
                actions.appendChild(window._cat.fleetActionBtn('Restore', 'primary', function() {
                    window._cat.restoreSite(site.domain).catch(function(e) { window._cat.fleetToast(e.message || 'Restore failed', 'error'); });
                }));
            } else {
                actions.appendChild(window._cat.fleetActionBtn('Backup', 'ghost', function() {
                    window._cat.backupSite(site.domain).catch(function(e) { window._cat.fleetToast(e.message || 'Backup failed', 'error'); });
                }));
                actions.appendChild(window._cat.fleetActionBtn('Export', 'ghost', function() {
                    window._cat.exportSite(site.domain).catch(function(e) { window._cat.fleetToast(e.message || 'Export failed', 'error'); });
                }));
                actions.appendChild(window._cat.fleetActionBtn('Mark Self-hosted', 'ghost', function() {
                    window._cat.markHostingMode(site.domain, 'self_hosted', 'active').catch(function(e) { window._cat.fleetToast(e.message || 'Hosting update failed', 'error'); });
                }));
                actions.appendChild(window._cat.fleetActionBtn('Mark Hosted', 'ghost', function() {
                    window._cat.markHostingMode(site.domain, 'hosted', 'active').catch(function(e) { window._cat.fleetToast(e.message || 'Hosting update failed', 'error'); });
                }));
                actions.appendChild(window._cat.fleetActionBtn('Dev Clone', 'ghost', function() {
                    window._cat.cloneSite(site.domain, 'dev').catch(function(e) { window._cat.fleetToast(e.message || 'Clone failed', 'error'); });
                }));
                if (site.mode === 'dev') {
                    actions.appendChild(window._cat.fleetActionBtn('Promote', 'primary', function() {
                        window._cat.promoteSite(site.domain).catch(function(e) { window._cat.fleetToast(e.message || 'Promote failed', 'error'); });
                    }));
                } else {
                    actions.appendChild(window._cat.fleetActionBtn('Duplicate', 'primary', function() {
                        window._cat.cloneSite(site.domain, 'clone').catch(function(e) { window._cat.fleetToast(e.message || 'Duplicate failed', 'error'); });
                    }));
                }
                actions.appendChild(window._cat.fleetActionBtn('Archive', 'danger', function() {
                    window._cat.deleteSite(site.domain).catch(function(e) { window._cat.fleetToast(e.message || 'Delete failed', 'error'); });
                }));
            }
            actionsTd.appendChild(actions);
            tr.appendChild(actionsTd);
        });

        table.appendChild(tbody);
        tableWrap.appendChild(table);
    }

    search.addEventListener('input', render);
    filter.addEventListener('change', render);
    render();
}
"##;
