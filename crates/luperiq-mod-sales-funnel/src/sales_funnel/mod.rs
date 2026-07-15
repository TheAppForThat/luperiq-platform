//! Sales Funnel module — trial flow, lead tracking, demo banner, and industry
//! CTA modal.
//!
//! **Aggregate namespaces:**
//! - `SalesPipeline:SiteTrial` — trial lifecycle (free/paid/expired/converted)
//! - `SalesPipeline:Lead` — lead tracking with UTM attribution and funnel stages
//!
//! All data persisted via ForgeJournal. No site-specific values are hardcoded;
//! everything comes from config or API parameters.

pub mod banner;
pub mod enrich;
pub mod leads;
pub mod trials;
pub mod promo;
pub mod directory_enrich;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use luperiq_forge::ApexEvent;
use luperiq_mod_smtp::send_email_internal;
use luperiq_module_api::{AdminView, AppContext, CmsModule, SharedJournal};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;

use leads::{AGG_LEAD, Lead};
use trials::{AGG_SITE_TRIAL, SiteTrial};

/// Global semaphore: only one site provision runs at a time.
/// Prevents port collisions when multiple trials start simultaneously.
static PROVISION_SEMAPHORE: std::sync::LazyLock<tokio::sync::Semaphore> =
    std::sync::LazyLock::new(|| tokio::sync::Semaphore::new(1));

// ── Rate Limiter (luper-guard sliding window) ────────────────────────

/// In-memory sliding-window rate limiter.
/// Mirrors the implementation in luperiq-cms/src/routes/auth.rs.
/// Each instance tracks max_requests per window_secs per IP address.
#[derive(Clone)]
struct RateLimiter {
    inner: Arc<TokioMutex<HashMap<String, Vec<Instant>>>>,
    max_requests: usize,
    window: std::time::Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            inner: Arc::new(TokioMutex::new(HashMap::new())),
            max_requests,
            window: std::time::Duration::from_secs(window_secs),
        }
    }

    async fn check(&self, ip: &str) -> bool {
        let mut map = self.inner.lock().await;
        let now = Instant::now();

        let entries = map.entry(ip.to_string()).or_default();
        entries.retain(|t| now.duration_since(*t) < self.window);

        if entries.len() >= self.max_requests {
            return false; // rate limited
        }
        entries.push(now);
        true
    }
}

/// Extract the client IP from request headers.
/// Uses the LAST X-Forwarded-For value — the one our trusted reverse proxy
/// (Apache) appends — to avoid attacker-controlled spoofing via the first value.
fn client_ip(headers: &HeaderMap) -> String {
    if let Some(val) = headers.get("x-forwarded-for") {
        if let Ok(s) = val.to_str() {
            if let Some(last_ip) = s.rsplit(',').next() {
                let ip = last_ip.trim();
                if !ip.is_empty() {
                    return ip.to_string();
                }
            }
        }
    }
    "unknown".to_string()
}

// ── Module State ─────────────────────────────────────────────────────

/// Axum router state for the sales funnel module.
/// Holds the shared WAL journal plus a rate limiter for cancel/deactivate
/// endpoints that accept a cancel token (to prevent brute-force guessing).
#[derive(Clone)]
struct SalesFunnelState {
    journal: SharedJournal,
    /// 3 requests per hour per IP on cancel/deactivate endpoints.
    cancel_limiter: RateLimiter,
}

// ── Helpers ─────────────────────────────────────────────────────────

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_id() -> String {
    ulid::Ulid::new().to_string().to_lowercase()
}

// ── API response ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ApiResult {
    pub ok: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ── Request payloads ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StartTrialPayload {
    pub email: String,
    pub industry_slug: String,
    pub business_name: Option<String>,
    pub phone: Option<String>,
    /// Optional admin password chosen at signup (app create-site). When present
    /// + valid (8..=128 chars) it becomes the new tenant's admin password
    /// instead of a generated one, so the user can log in immediately. The web
    /// signup omits it (keeps the generate-and-email flow). NEVER logged.
    #[serde(default)]
    pub admin_password: Option<String>,
    /// Optional custom subdomain (e.g. "pest" for pest.coderobot.net).
    /// If provided, skips auto-generation. Must be lowercase alphanumeric + hyphens.
    #[serde(default)]
    pub custom_domain: Option<String>,
    /// Pricing tier for business types (e.g. "pro-monthly", "pro-annual", "pro-lifetime").
    /// Required when the site type is not always_free.
    #[serde(default)]
    pub tier_slug: Option<String>,
    /// City of the business — used to derive city-based subdomains on
    /// vertical marketing domains (e.g. sanantonio.pestcontroller.org).
    #[serde(default)]
    pub city: Option<String>,
    /// State abbreviation for the business (e.g. "TX"). Used as a
    /// tiebreaker when the city subdomain is already taken.
    #[serde(default)]
    pub state: Option<String>,
    /// Optional referral code — usually the referrer's subdomain
    /// (e.g. "acme-pest-control" from ?ref=acme-pest-control).
    #[serde(default, alias = "ref")]
    pub referred_by: Option<String>,
    /// Pre-create wizard answers — every field the post-create wizard
    /// would have asked, gathered before provisioning so the new site
    /// lands fully populated. Shape is `{ <field_key>: <value>, ... }`.
    /// Forwarded verbatim to provision-site.sh and into the setup/apply
    /// call so the freshly provisioned WAL has CompanyProfile + industry
    /// aggregates already filled in.
    #[serde(default)]
    pub wizard_answers: Option<serde_json::Value>,
    /// Whether the visitor opted into the platform-operator "done-for-you"
    /// onboarding add-on. Only meaningful for field-service tiers that have
    /// a non-zero `setup_addon_price` in the effective tier table.
    #[serde(default)]
    pub setup_addon_requested: Option<bool>,
    /// Promo code entered at signup — stored for billing integration.
    #[serde(default)]
    pub promo_code: Option<String>,
}

fn truthy_wizard_flag(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    obj.get(key)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn sanitize_directory_wizard_answers(mut wizard_answers: serde_json::Value) -> serde_json::Value {
    let Some(obj) = wizard_answers.as_object_mut() else {
        return wizard_answers;
    };
    let confirmed = truthy_wizard_flag(obj, "_directory_selection_confirmed")
        || truthy_wizard_flag(obj, "_directory_claim_intent")
        || truthy_wizard_flag(obj, "_from_directory_claim");
    if !confirmed {
        obj.remove("_directory_company_id");
        obj.remove("_directory_company_slug");
        obj.remove("_directory_state");
        obj.remove("_directory_prefill_source");
        obj.remove("_from_directory_claim");
    }
    wizard_answers
}

#[derive(Debug, Deserialize)]
pub struct ExtendTrialPayload {
    pub email: String,
    pub stripe_session_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub email: Option<String>,
    pub all: Option<String>,
}

// ── Router ──────────────────────────────────────────────────────────

pub fn sales_funnel_router(journal: SharedJournal) -> Router {
    let state = SalesFunnelState {
        journal,
        // 3 cancel/deactivate attempts per hour per IP — prevents brute-force
        // token guessing on the cancel-site and deactivate endpoints.
        cancel_limiter: RateLimiter::new(3, 60 * 60),
    };
    Router::new()
        .route("/api/modules/sales-funnel/start", post(start_trial))
        .route("/api/modules/sales-funnel/status", get(trial_status))
        .route("/api/modules/sales-funnel/extend", post(extend_trial))
        .route("/api/modules/sales-funnel/banner", get(banner_data))
        .route("/api/modules/sales-funnel/leads", get(list_leads))
        .route("/api/modules/sales-funnel/stats", get(funnel_stats))
        .route(
            "/api/modules/sales-funnel/provision-timeout",
            post(provision_timeout_alert),
        )
        .route(
            "/api/modules/sales-funnel/provision-status",
            get(provision_status_check),
        )
        .route(
            "/api/modules/sales-funnel/deactivate",
            post(deactivate_site),
        )
        .route("/api/modules/sales-funnel/my-sites", get(my_sites))
        .route(
            "/api/modules/sales-funnel/lifetime-entitlements",
            get(list_lifetime_entitlements),
        )
        .route("/cancel-site", get(cancel_site_page))
        .with_state(state)
}

/// Admin-only — list every granted lifetime entitlement. Routes
/// under /api/modules/sales-funnel/ are auth-gated by the platform's
/// auth middleware, so unauthenticated visitors get 401.
async fn list_lifetime_entitlements(
    State(state): State<SalesFunnelState>,
) -> Json<serde_json::Value> {
    let journal = state.journal.lock().await;
    let mut items: Vec<serde_json::Value> = Vec::new();
    for evt in journal.latest_by_aggregate_type(trials::AGG_LIFETIME_ENTITLEMENT) {
        if evt.payload == b"__DELETED__" {
            continue;
        }
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&evt.payload) {
            items.push(v);
        }
    }
    // Newest first by granted_at.
    items.sort_by(|a, b| {
        b.get("granted_at")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .cmp(&a.get("granted_at").and_then(|v| v.as_u64()).unwrap_or(0))
    });

    let total_cents: i64 = items
        .iter()
        .map(|v| v.get("amount_cents").and_then(|c| c.as_i64()).unwrap_or(0))
        .sum();
    let by_tier = {
        let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for v in &items {
            let tier = v
                .get("tier")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            *counts.entry(tier).or_insert(0) += 1;
        }
        counts
    };

    Json(serde_json::json!({
        "ok": true,
        "count": items.len(),
        "total_revenue_cents": total_cents,
        "by_tier": by_tier,
        "entitlements": items,
    }))
}

// ── Handlers ────────────────────────────────────────────────────────

/// Path to the site provisioning script.
/// Reads `LUPERIQ_PROVISION_SCRIPT` env var at runtime; falls back to the
/// default path on the production server.
fn provision_script_path() -> String {
    std::env::var("LUPERIQ_PROVISION_SCRIPT")
        .unwrap_or_else(|_| "/home/dave/luperiq-apex-db/scripts/provision-site.sh".to_string())
}

/// Root of the on-disk queue consumed by `luperiq-provision-worker.service`.
/// The worker exists because `luperiq-cms.service` is hardened with
/// `NoNewPrivileges=yes` (2026-05-15 security sweep), which blocks sudo from
/// inside the main service. The main service writes a JSON job descriptor to
/// `<root>/pending/<trial_id>.json`; the worker (non-hardened, runs as dave with
/// NOPASSWD sudoers) processes it and writes a result to `done/` or `failed/`.
fn provision_queue_root() -> std::path::PathBuf {
    std::env::var("LUPERIQ_PROVISION_QUEUE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/var/spool/luperiq-provision"))
}

/// Write a job descriptor atomically (tmp + rename) so the worker never observes
/// a partial file.
async fn enqueue_provision_job(
    trial_id: &str,
    job: &serde_json::Value,
) -> Result<std::path::PathBuf, std::io::Error> {
    let pending = provision_queue_root().join("pending");
    tokio::fs::create_dir_all(&pending).await.ok();
    let target = pending.join(format!("{trial_id}.json"));
    let tmp = pending.join(format!("{trial_id}.json.tmp"));
    tokio::fs::write(&tmp, job.to_string()).await?;
    tokio::fs::rename(&tmp, &target).await?;
    Ok(target)
}

/// Wait for the worker to drop a result file (done/ or failed/) for this trial.
/// Returns `Ok((success, stdout, stderr, exit_code))` mirroring the previous
/// `Command::output()` semantics so the downstream code (sanitize, email,
/// WAL log) can stay unchanged.
async fn await_provision_result(
    trial_id: &str,
    timeout_secs: u64,
) -> Result<(bool, String, String, Option<i32>), String> {
    let done_path = provision_queue_root().join("done").join(format!("{trial_id}.json"));
    let failed_path = provision_queue_root().join("failed").join(format!("{trial_id}.json"));
    let mut elapsed: u64 = 0;
    let poll_ms: u64 = 1000;

    while elapsed * poll_ms / 1000 < timeout_secs {
        if let Ok(content) = tokio::fs::read_to_string(&done_path).await {
            let _ = tokio::fs::remove_file(&done_path).await;
            let v: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
            return Ok((
                true,
                v.get("stdout").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                v.get("stderr").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                v.get("exit_code").and_then(|i| i.as_i64()).map(|i| i as i32),
            ));
        }
        if let Ok(content) = tokio::fs::read_to_string(&failed_path).await {
            let _ = tokio::fs::remove_file(&failed_path).await;
            let v: serde_json::Value = serde_json::from_str(&content).unwrap_or(serde_json::json!({}));
            return Ok((
                false,
                v.get("stdout").and_then(|s| s.as_str()).unwrap_or("").to_string(),
                v.get("stderr").and_then(|s| s.as_str()).unwrap_or(
                    v.get("error").and_then(|s| s.as_str()).unwrap_or(""),
                ).to_string(),
                v.get("exit_code").and_then(|i| i.as_i64()).map(|i| i as i32),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
        elapsed += 1;
    }
    Err(format!("provision-worker timeout after {timeout_secs}s"))
}

/// Base domain for customer subdomains. Default is the LuperIQ-platform
/// catch-all `coderobot.net`; vertical marketing domains (pestcontroller.org)
/// route their signups to their own subdomain space.
const DEFAULT_PROVISION_BASE_DOMAIN: &str = "coderobot.net";

/// Pick the right subdomain base for a signup based on the request Host.
///
/// Visitors arriving on `pestcontroller.org` or `*.pestcontroller.org`
/// (not yet a real tenant) get `<slug>.pestcontroller.org` so the vertical
/// branding stays consistent end-to-end. Anyone else (coderobot.net,
/// luperiq.com, direct API) falls back to the platform default.
pub fn provision_base_domain_for_host(host_header: Option<&str>) -> &'static str {
    let host = host_header
        .map(|h| h.split(':').next().unwrap_or(h).to_ascii_lowercase())
        .unwrap_or_default();
    if host == "pestcontroller.org" || host.ends_with(".pestcontroller.org") {
        "pestcontroller.org"
    } else {
        DEFAULT_PROVISION_BASE_DOMAIN
    }
}

fn sites_dir_path() -> PathBuf {
    std::env::var("LUPERIQ_SITES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/home/dave/sites"))
}

fn site_dir_for_domain(domain: &str) -> PathBuf {
    let site_slug = domain
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>();
    sites_dir_path().join(site_slug)
}

fn domain_is_taken(journal: &luperiq_forge::ForgeJournal, domain: &str) -> bool {
    let domain_lower = domain.to_ascii_lowercase();
    site_dir_for_domain(&domain_lower).exists()
        || trials::list_trials(journal).into_iter().any(|trial| {
            trial
                .domain
                .as_deref()
                .map(|existing| existing.eq_ignore_ascii_case(&domain_lower))
                .unwrap_or(false)
                && trial.deactivated_at.is_none()
        })
}

/// Derive a subdomain base from a city name (and optional state abbreviation).
/// Used for vertical marketing domain signups (pestcontroller.org).
/// Returns just the city slug; state variants and digit suffixes are tried
/// by resolve_city_domain.
fn city_to_subdomain_base(city: &str, state: Option<&str>) -> (String, String, String) {
    let slugify = |s: &str| -> String {
        let clean: String = s
            .chars()
            .filter_map(|c| {
                if c.is_ascii_alphanumeric() {
                    Some(c.to_ascii_lowercase())
                } else if c == ' ' || c == '.' || c == '_' || c == '-' {
                    Some('-')
                } else {
                    None
                }
            })
            .collect();
        let mut collapsed = String::new();
        for ch in clean.chars() {
            if ch == '-' && collapsed.ends_with('-') {
                continue;
            }
            collapsed.push(ch);
        }
        collapsed.trim_matches('-').chars().take(28).collect()
    };
    let city_slug = slugify(city);
    let state_slug = state
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    // Candidate 1: just city (e.g. "sanantonio")
    let c1 = city_slug.clone();
    // Candidate 2: city + state abbrev (e.g. "sanantonio-tx")
    let c2 = if state_slug.len() <= 3 && !state_slug.is_empty() {
        format!("{}-{}", city_slug, state_slug)
    } else {
        format!("{}-{}", city_slug, &state_slug[..state_slug.len().min(6)])
    };
    // Candidate 3: city + full state slug (trimmed to avoid absurd lengths)
    let c3 = if state_slug.is_empty() {
        format!("{}-2", city_slug)
    } else {
        format!("{}-{}", city_slug, state_slug)
    };
    (c1, c2, c3)
}

/// Resolve a unique domain using city-based subdomain candidates.
fn resolve_city_domain(
    journal: &luperiq_forge::ForgeJournal,
    city: &str,
    state: Option<&str>,
    base_domain: &str,
) -> String {
    let (c1, c2, c3) = city_to_subdomain_base(city, state);
    for base in [&c1, &c2, &c3] {
        if base.is_empty() {
            continue;
        }
        let candidate = format!("{base}.{base_domain}");
        if !domain_is_taken(journal, &candidate) {
            return candidate;
        }
    }
    // Digit suffix fallback on the city slug
    for n in 2..=999 {
        let candidate = format!("{c1}-{n}.{base_domain}");
        if !domain_is_taken(journal, &candidate) {
            return candidate;
        }
    }
    // Last resort: random suffix
    let suffix: String = {
        let mut rng = rand::rng();
        (0..4)
            .map(|_| {
                let n: u8 = rng.random_range(0..16);
                format!("{n:x}").chars().next().unwrap()
            })
            .collect()
    };
    format!("{c1}-{suffix}.{base_domain}")
}

/// Derive a clean base subdomain from a business name (preferred) or email address.
///
/// Uses the business name if provided, otherwise falls back to the email
/// local part. Strips non-alphanumeric characters, lowercases, truncates
/// to 24 chars, and leaves uniqueness handling to the caller.
fn derive_subdomain_base(business_name: Option<&str>, email: &str) -> String {
    let source = match business_name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => email.split('@').next().unwrap_or("site").to_string(),
    };
    let clean: String = source
        .chars()
        .filter_map(|c| {
            if c.is_ascii_alphanumeric() {
                Some(c.to_ascii_lowercase())
            } else if c == ' ' || c == '.' || c == '_' || c == '-' {
                Some('-')
            } else {
                None
            }
        })
        .collect();
    // Collapse multiple hyphens
    let mut collapsed = String::with_capacity(clean.len());
    for ch in clean.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    let trimmed = collapsed.trim_matches('-');
    trimmed.chars().take(24).collect()
}

fn resolve_auto_domain(
    journal: &luperiq_forge::ForgeJournal,
    business_name: Option<&str>,
    email: &str,
    base_domain: &str,
) -> String {
    let base = {
        let candidate = derive_subdomain_base(business_name, email);
        if candidate.is_empty() {
            "site".to_string()
        } else {
            candidate
        }
    };

    let primary = format!("{base}.{base_domain}");
    if !domain_is_taken(journal, &primary) {
        return primary;
    }

    for n in 2..=999 {
        let candidate = format!("{base}-{n}.{base_domain}");
        if !domain_is_taken(journal, &candidate) {
            return candidate;
        }
    }

    let suffix: String = {
        let mut rng = rand::rng();
        (0..4)
            .map(|_| {
                let n: u8 = rng.random_range(0..16);
                format!("{n:x}").chars().next().unwrap()
            })
            .collect()
    };
    format!("{base}-{suffix}.{base_domain}")
}

fn sanitize_requested_domain(custom_domain: Option<&str>, base_domain: &str) -> Option<String> {
    let custom = custom_domain?.trim().to_lowercase();
    if custom.is_empty() {
        return None;
    }
    let clean: String = custom
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '.')
        .collect();
    if clean.is_empty() {
        None
    } else if clean.contains('.') {
        Some(clean)
    } else {
        Some(format!("{clean}.{base_domain}"))
    }
}

/// Generate a trial license key in the LIQ-TRIAL-XXXX-XXXX format.
fn generate_trial_license() -> String {
    let id = ulid::Ulid::new().to_string();
    let chars: String = id.chars().take(8).collect::<String>().to_uppercase();
    format!("LIQ-TRIAL-{}-{}", &chars[0..4], &chars[4..8])
}

/// Generate a random password of the given length.
fn generate_password(len: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    (0..len)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn sanitize_provision_log(text: &str, secrets: &[&str]) -> String {
    let mut out = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            out = out.replace(secret, "[REDACTED]");
        }
    }
    out
}

fn write_provision_password_file(trial_id: &str, password: &str) -> std::io::Result<PathBuf> {
    // Write into the queue passwords/ subdir (NOT /tmp). The main service
    // runs with PrivateTmp=yes, so /tmp is service-private and invisible to
    // the provision-worker. The queue dir is in ReadWritePaths and visible
    // to both processes.
    let pwd_dir = provision_queue_root().join("passwords");
    std::fs::create_dir_all(&pwd_dir)?;
    let path = pwd_dir.join(format!("{trial_id}-{}", ulid::Ulid::new()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)?;
    file.write_all(password.as_bytes())?;
    file.sync_all()?;
    Ok(path)
}

/// Log a structured funnel event to stderr/journald so Central's logs can be
/// grepped for conversion metrics. Format is intentionally simple + stable:
///
/// `[funnel] event=<name> ts=<unix> key1=value1 key2=value2`
///
/// Parse with:  journalctl -u luperiq-cms | grep '\[funnel\]'
fn log_funnel_event(event: &str, details: &[(&str, &str)]) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut line = format!("[funnel] event={event} ts={ts}");
    for (k, v) in details {
        // Keep values free of whitespace so greppers don't break on them.
        let clean: String = v
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '=')
            .take(64)
            .collect();
        line.push_str(&format!(" {k}={clean}"));
    }
    eprintln!("{line}");
}

/// Snapshot the done-for-you setup add-on price (in cents) for `tier_slug`
/// from the effective tier table. Returns `None` when the tier is unknown,
/// the price is zero, or the price is negative — i.e. when the add-on
/// should NOT be recorded on the trial.
fn snapshot_addon_cents(
    tiers: &[luperiq_forge::nexus::FieldServiceTierDef],
    tier_slug: &str,
) -> Option<u32> {
    let price = tiers.iter().find(|t| t.slug == tier_slug)?.setup_addon_price;
    if !price.is_finite() || price <= 0.0 {
        return None;
    }
    Some((price * 100.0).round() as u32)
}

/// POST /api/modules/sales-funnel/start
///
/// Start a free trial. Creates a SiteTrial with stage="free" and immediately
/// spawns background provisioning. Business sites use the cart checkout flow.
/// Map a public lifetime/pricing tier slug (carried in the lifetime
/// entitlement `tier` or the signup `tier_slug`) to the internal NexClient
/// `license_tier` value that the provisioner seeds on Central.
///
/// MIRRORS `luperiq-cms::routes::lifetime_checkout::lifetime_tier_to_license_tier`.
/// It is duplicated rather than imported because `luperiq-cms` depends on this
/// crate (not vice-versa); importing it would form a dependency cycle. Keep the
/// two in sync. Returns `None` for free / unknown slugs → the new site stays
/// free (unchanged behavior).
fn provision_tier_to_license_tier(tier_slug: &str) -> Option<&'static str> {
    match tier_slug {
        // Field-service (pest-control) lifetime → the pro-lifetime tier.
        "pest-1truck" | "pest-2to5trucks" | "pest-5plus" => Some("pro-lifetime"),
        // Creators & Pros / Local Service & Food lifetime → professional.
        "creators-pros" | "local-service-food" => Some("professional"),
        // Family & Community lifetime → starter.
        "family-community" => Some("starter"),
        // A bare "lifetime" entitlement (the generic lifetime claim) → the
        // internal pro-lifetime tier.
        "lifetime" | "pro-lifetime" => Some("pro-lifetime"),
        _ => None,
    }
}

async fn start_trial(
    State(state): State<SalesFunnelState>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(p): axum::extract::Json<StartTrialPayload>,
) -> Json<ApiResult> {
    // Host-aware subdomain base: pestcontroller.org-origin signups land at
    // <slug>.pestcontroller.org instead of <slug>.coderobot.net.
    let host_header = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok());
    let base_domain = provision_base_domain_for_host(host_header);

    // Industry lock: pestcontroller.org-family signups are pest-control only,
    // regardless of any client-side tampering with the wizard form.
    let force_pest_industry = host_header
        .map(|h| {
            let host = h.split(':').next().unwrap_or(h).to_ascii_lowercase();
            host == "pestcontroller.org" || host.ends_with(".pestcontroller.org")
        })
        .unwrap_or(false);

    let journal = state.journal;
    let email = p.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Json(ApiResult {
            ok: false,
            message: "A valid email address is required".into(),
            data: None,
        });
    }

    log_funnel_event(
        "start_free.submitted",
        &[
            ("industry", &p.industry_slug),
            (
                "has_ref",
                if p.referred_by.as_deref().unwrap_or("").is_empty() {
                    "no"
                } else {
                    "yes"
                },
            ),
        ],
    );

    let now = now_ts();
    let trial_id = new_id();
    let industry_slug = if force_pest_industry {
        "pest-control".to_string()
    } else {
        p.industry_slug.clone()
    };
    let mut business_name = p.business_name.clone().unwrap_or_default();
    let mut phone = p.phone.clone().unwrap_or_default();
    let wizard_answers = p
        .wizard_answers
        .clone()
        .map(sanitize_directory_wizard_answers);
    // For vertical marketing domains, prefer city-based subdomains.
    // City comes from the payload directly or from wizard_answers.
    let signup_city = p.city.clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            wizard_answers.as_ref()
                .and_then(|w| w.get("_tda_city").or_else(|| w.get("city")))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
        });
    let signup_state = p.state.clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            wizard_answers.as_ref()
                .and_then(|w| w.get("_tda_state").or_else(|| w.get("state")))
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty())
                .map(str::to_string)
        });

    // Enrich wizard_answers from directory when the visitor is claiming a listing.
    // Reads _directory_company_slug + _directory_state from wizard_answers and
    // merges the company's real services, description, address, etc. Best-effort;
    // never blocks provisioning on DB failure.
    let wizard_answers = wizard_answers.map(|wa| {
        directory_enrich::enrich_wizard_answers_from_directory(wa)
    });

    // If the enrichment populated business_name or phone (visitor left them blank
    // and we filled from directory), back-fill the outer variables so that
    // provision-site.sh gets the real company name and phone number.
    if business_name.is_empty() {
        if let Some(enriched_name) = wizard_answers.as_ref()
            .and_then(|w| w.get("business_name"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            business_name = enriched_name.to_string();
        }
    }
    if phone.is_empty() {
        if let Some(enriched_phone) = wizard_answers.as_ref()
            .and_then(|w| w.get("phone"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            phone = enriched_phone.to_string();
        }
    }

    // Done-for-you setup add-on: only honor when (a) the visitor checked the
    // box, (b) the chosen tier exists in FIELD_SERVICE_TIERS, and (c) the
    // operator has set a non-zero `setup_addon_price` for that tier. Snapshot
    // the price in cents so the agreed amount survives later operator edits.
    let (setup_addon_requested, setup_addon_price_cents) = if p
        .setup_addon_requested
        .unwrap_or(false)
    {
        let tier_slug = p.tier_slug.as_deref().unwrap_or("");
        let price_cents = {
            let j = journal.lock().await;
            let tiers = luperiq_forge::nexus::effective_field_service_tiers(&j);
            snapshot_addon_cents(&tiers, tier_slug)
        };
        match price_cents {
            Some(c) => (Some(true), Some(c)),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    // 7-day free trial. On day 8 the site switches to a private preview
    // until the owner upgrades. Content is never deleted.
    const TRIAL_SECS: u64 = 7 * 86400;
    let free_expiry = now + TRIAL_SECS;

    // One-click magic-login handoff: Central mints a raw token here, hands
    // the BLAKE3 hash to the new site via bootstrap config, and returns the
    // raw token in the API response. The new site seeds a single-use
    // ResetToken on first boot; the owner redeems it at /auth/magic?token=
    // to land directly in their admin.
    let (magic_raw_token, magic_token_hash, magic_expires_at) = {
        let mut rng = rand::rng();
        let raw: String = (0..32)
            .map(|_| {
                let idx = rng.random_range(0..16u8);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect();
        let hash = luperiq_forge::hash_token(&raw);
        // 30 minutes is plenty to finish provisioning + click through, but
        // short enough that a leaked link isn't long-lived.
        (raw, hash, now + 1800)
    };

    // Generate a random cancel token for email deactivation links
    let cancel_token: String = {
        let mut rng = rand::rng();
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..36u8);
                if idx < 10 {
                    (b'0' + idx) as char
                } else {
                    (b'a' + idx - 10) as char
                }
            })
            .collect()
    };

    let lead_id = new_id();

    // Resolve the domain and write WAL events under the same lock so we don't
    // hand out the same clean subdomain twice under concurrent signups.
    let (domain, trial, provision_license_tier, provision_trucks) = {
        let mut j = journal.lock().await;


        // Duplicate email check: if this email already has an active trial,
        // return an error instead of creating a duplicate tenant.
        if let Some(existing_trial) = trials::get_trial(&j, &email) {
            if existing_trial.deactivated_at.is_none() {
                if let Some(existing_domain) = existing_trial.domain {
                    return Json(ApiResult {
                        ok: false,
                        message: format!("You already have a site at {}. Please log in to manage it.", existing_domain),
                        data: None,
                    });
                }
            }
        }
        // Vertical marketing domains use city-based subdomains (no custom domain allowed).
        // coderobot.net signups still use business-name-based subdomains or custom_domain.
        let domain = if force_pest_industry {
            if let Some(ref city) = signup_city {
                resolve_city_domain(&j, city.trim(), signup_state.as_deref(), base_domain)
            } else {
                resolve_auto_domain(
                    &j,
                    if business_name.is_empty() { None } else { Some(&business_name) },
                    &email,
                    base_domain,
                )
            }
        } else if let Some(custom) = sanitize_requested_domain(p.custom_domain.as_deref(), base_domain) {
            if domain_is_taken(&j, &custom) {
                return Json(ApiResult {
                    ok: false,
                    message: "That domain is already in use".into(),
                    data: None,
                });
            }
            custom
        } else {
            resolve_auto_domain(
                &j,
                if business_name.is_empty() { None } else { Some(&business_name) },
                &email,
                base_domain,
            )
        };

        // Normalize the incoming ref code — only accept lowercase
        // alphanumerics + hyphens (i.e. valid subdomain format), trim
        // length, and drop it if it points at an unknown site.
        let referred_by = p.referred_by.as_deref().and_then(|raw| {
            let cleaned: String = raw
                .trim()
                .to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
                .take(63)
                .collect();
            if cleaned.is_empty() {
                return None;
            }
            // Accept only if we actually have a trial for that subdomain —
            // silently drops vanity / abusive refs.
            let full_domain = if cleaned.contains('.') {
                cleaned.clone()
            } else {
                format!("{cleaned}.{base_domain}")
            };
            if domain_is_taken(&j, &full_domain) {
                Some(cleaned)
            } else {
                None
            }
        });

        // If the visitor already paid lifetime via /lifetime/thank-you,
        // their entitlement is recorded under `LifetimeEntitlement:
        // Granted` keyed by email. Skip the 7-day trial entirely and
        // create the site already in the paid stage.
        let lifetime = trials::find_lifetime_entitlement(&j, &email);

        // ── Resolve the purchased tier so a PAID site comes up AT its tier ──
        // Prefer the verified lifetime entitlement's tier; fall back to the
        // signup `tier_slug` if it names a paid tier. The result (internal
        // NexClient license_tier, e.g. "pro-lifetime") + the purchased per-truck
        // count are threaded into provision_trial_site → the provision JOB →
        // provision-site.sh → the new site's NexClient on Central. When neither
        // resolves (free signup), both stay None/default → unchanged free flow.
        let provision_license_tier: Option<&'static str> = lifetime
            .as_ref()
            .and_then(|ent| provision_tier_to_license_tier(&ent.tier))
            .or_else(|| {
                p.tier_slug
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .and_then(provision_tier_to_license_tier)
            });
        let provision_trucks: u32 = lifetime
            .as_ref()
            .map(|ent| ent.trucks.max(1))
            .unwrap_or(1);

        let trial = if let Some(ref ent) = lifetime {
            log_funnel_event(
                "start_free.lifetime_claimed",
                &[
                    ("tier", &ent.tier),
                    ("amount_cents", &ent.amount_cents.to_string()),
                ],
            );
            SiteTrial {
                trial_id: trial_id.clone(),
                email: email.clone(),
                industry_slug: industry_slug.clone(),
                // Paid lifetime — never expires.
                stage: "paid".into(),
                free_started_at: now,
                free_expires_at: now, // already past free
                paid_started_at: Some(now),
                paid_expires_at: None, // None = lifetime
                stripe_session_id: if ent.stripe_session_id.is_empty() {
                    None
                } else {
                    Some(ent.stripe_session_id.clone())
                },
                converted_to: Some("lifetime".into()),
                converted_at: Some(now),
                created_at: now,
                domain: Some(domain.clone()),
                cancel_token: Some(cancel_token.clone()),
                deactivated_at: None,
                referred_by: referred_by.clone(),
                setup_addon_requested,
                setup_addon_price_cents,
                promo_code: p.promo_code.clone().filter(|s| !s.is_empty()),
                flash_offer_expires_at: None,
            }
        } else {
            SiteTrial {
                trial_id: trial_id.clone(),
                email: email.clone(),
                industry_slug: industry_slug.clone(),
                stage: "free".into(),
                free_started_at: now,
                free_expires_at: free_expiry,
                paid_started_at: None,
                paid_expires_at: None,
                stripe_session_id: None,
                converted_to: None,
                converted_at: None,
                created_at: now,
                domain: Some(domain.clone()),
                cancel_token: Some(cancel_token.clone()),
                deactivated_at: None,
                referred_by,
                setup_addon_requested,
                setup_addon_price_cents,
                promo_code: p.promo_code.clone().filter(|s| !s.is_empty()),
                flash_offer_expires_at: None,
            }
        };

        let lead = Lead {
            lead_id: lead_id.clone(),
            email: email.clone(),
            industry_slug: industry_slug.clone(),
            source_page: String::new(),
            referrer: String::new(),
            utm_source: String::new(),
            utm_medium: String::new(),
            utm_campaign: String::new(),
            stage: "trial_started".into(),
            stage_timestamps: serde_json::json!({ "trial_started": now }),
            created_at: now,
            updated_at: now,
        };

        // Write trial
        let trial_bytes = match serde_json::to_vec(&trial) {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Serialize error: {e}"),
                    data: None,
                });
            }
        };
        let trial_event = ApexEvent::new(AGG_SITE_TRIAL, &trial_id, trial_bytes);
        if let Err(e) = j.append(trial_event) {
            return Json(ApiResult {
                ok: false,
                message: format!("WAL write failed: {e}"),
                data: None,
            });
        }

        // Write lead
        let lead_bytes = match serde_json::to_vec(&lead) {
            Ok(b) => b,
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Serialize error: {e}"),
                    data: None,
                });
            }
        };
        let lead_event = ApexEvent::new(AGG_LEAD, &lead_id, lead_bytes);
        let _ = j.append(lead_event);
        (domain, trial, provision_license_tier, provision_trucks)
    }; // lock dropped here

    let site_url = format!("https://{domain}");
    let magic_url = format!(
        "{site_url}/auth/magic?token={magic_raw_token}&next=%2Fadmin"
    );

    // Spawn background provisioning
    let journal_clone = journal.clone();
    let email_clone = email.clone();
    let domain_clone = domain.clone();
    let industry_clone = industry_slug.clone();
    let trial_id_clone = trial_id.clone();
    let name_clone = business_name.clone();
    let phone_clone = phone.clone();
    let cancel_token_clone = cancel_token.clone();
    let magic_hash_clone = magic_token_hash.clone();

    let trial_started_at_clone = now;
    let trial_expires_at_clone = free_expiry;

    let wizard_answers_clone = wizard_answers.clone();
    let admin_password_clone = p.admin_password.clone();
    // Purchased tier + trucks threaded into provisioning (None = free flow).
    let provision_tier_clone: Option<String> =
        provision_license_tier.map(|t| t.to_string());
    let provision_trucks_clone: u32 = provision_trucks;
    tokio::spawn(async move {
        // Acquire semaphore — only one provision at a time to prevent port collisions
        let _permit = PROVISION_SEMAPHORE.acquire().await;
        provision_trial_site(
            &journal_clone,
            &trial_id_clone,
            &email_clone,
            &domain_clone,
            &industry_clone,
            &name_clone,
            &phone_clone,
            &cancel_token_clone,
            &magic_hash_clone,
            magic_expires_at,
            trial_started_at_clone,
            trial_expires_at_clone,
            wizard_answers_clone.as_ref(),
            admin_password_clone,
            provision_tier_clone.as_deref(),
            provision_trucks_clone,
        )
        .await;
        // _permit dropped here, releasing the semaphore for the next provision
    });

    log_funnel_event(
        "start_free.accepted",
        &[
            ("trial_id", &trial_id),
            ("industry", &industry_slug),
            ("domain", &domain),
        ],
    );

    Json(ApiResult {
        ok: true,
        message: "Free trial started".into(),
        data: Some(serde_json::json!({
            "trial_id": trial_id,
            "stage": "free",
            "expires_at": trial.free_expires_at,
            "site_url": site_url,
            "magic_url": magic_url,
            "magic_expires_at": magic_expires_at,
        })),
    })
}

#[cfg(test)]
mod tests {
    use super::{derive_subdomain_base, snapshot_addon_cents};
    use luperiq_forge::nexus::FieldServiceTierDef;

    // ── flash-offer decision + email tests ──────────────────────────────
    use super::{
        should_send_flash10, should_send_flash10_remind, flash_subject_body,
        reminder_kind_for,
        FLASH10_START_SECS, FLASH10_END_SECS, FLASH_WINDOW_SECS,
        FLASH10_REMIND_DELAY_SECS,
    };

    #[test]
    fn existing_reminder_kinds_unchanged_by_flash_addition() {
        // hour4 band (>=4h, <24h) on signup day.
        assert_eq!(reminder_kind_for(4 * 3600, false), Some("hour4"));
        assert_eq!(reminder_kind_for(23 * 3600, false), Some("hour4"));
        // Gap between hour4 and day3 → None (this is the day-1 band flash10 now
        // occupies in the SCAN, but reminder_kind_for itself stays None here).
        assert_eq!(reminder_kind_for(26 * 3600, false), None);
        // day3 / day5 / day7 land on their day-of-trial.
        assert_eq!(reminder_kind_for(2 * 86400 + 10, false), Some("day3"));
        assert_eq!(reminder_kind_for(4 * 86400 + 10, false), Some("day5"));
        assert_eq!(reminder_kind_for(6 * 86400 + 10, false), Some("day7"));
        // expired wins over everything.
        assert_eq!(reminder_kind_for(0, true), Some("expired"));
        assert_eq!(reminder_kind_for(2 * 86400, true), Some("expired"));
    }

    #[test]
    fn flash10_fires_once_in_day1_band_only() {
        // Before the band (e.g. 12h in) → no.
        assert!(!should_send_flash10(12 * 3600, false));
        // Just before the lower edge.
        assert!(!should_send_flash10(FLASH10_START_SECS - 1, false));
        // Inside the band → yes.
        assert!(should_send_flash10(FLASH10_START_SECS, false));
        assert!(should_send_flash10(26 * 3600, false));
        assert!(should_send_flash10(FLASH10_END_SECS - 1, false));
        // At/after the upper edge → no (scan missed it; don't fire late).
        assert!(!should_send_flash10(FLASH10_END_SECS, false));
        assert!(!should_send_flash10(72 * 3600, false));
        // Idempotency: once recorded, never again even inside the band.
        assert!(!should_send_flash10(26 * 3600, true));
    }

    #[test]
    fn flash10_remind_only_30min_to_4h_after_send_and_once() {
        let sent = 1_000_000u64;
        // flash10 never sent → never.
        assert!(!should_send_flash10_remind(sent + 3600, None, false));
        // Too soon (< 30 min after send) → no.
        assert!(!should_send_flash10_remind(sent + 1799, Some(sent), false));
        // Exactly 30 min → yes.
        assert!(should_send_flash10_remind(
            sent + FLASH10_REMIND_DELAY_SECS,
            Some(sent),
            false
        ));
        // Mid-window (e.g. 2h) → yes.
        assert!(should_send_flash10_remind(sent + 2 * 3600, Some(sent), false));
        // Just before window close → yes.
        assert!(should_send_flash10_remind(
            sent + FLASH_WINDOW_SECS - 1,
            Some(sent),
            false
        ));
        // At/after window close (4h) → no (stale, skip).
        assert!(!should_send_flash10_remind(
            sent + FLASH_WINDOW_SECS,
            Some(sent),
            false
        ));
        assert!(!should_send_flash10_remind(sent + 5 * 3600, Some(sent), false));
        // Already sent → never again.
        assert!(!should_send_flash10_remind(sent + 2 * 3600, Some(sent), true));
    }

    #[test]
    fn flash_emails_carry_founding_price_wording_and_checkout_cta() {
        let (subj10, body10) = flash_subject_body("flash10", "acme.coderobot.net", FLASH_WINDOW_SECS);
        assert!(subj10.contains("founding price"));
        assert!(subj10.contains("10%"));
        assert!(body10.contains("founding"));
        assert!(body10.contains("locked for life"));
        assert!(body10.contains("isn't locked yet"));
        // CTA → the lifetime checkout entry the live page wires to.
        assert!(body10.contains("/start-free?plan=lifetime"));
        // No invented "10 years" framing.
        assert!(!body10.to_lowercase().contains("10 year"));

        // flash10_remind renders the time-left phrase from secs_remaining.
        let (subj_r, body_r) =
            flash_subject_body("flash10_remind", "acme.coderobot.net", 3 * 3600 + 600);
        assert!(subj_r.contains("10%"));
        assert!(body_r.contains("3h 10m"));
        assert!(body_r.contains("/start-free?plan=lifetime"));
        assert!(body_r.contains("keep the founding rate for life"));
        assert!(!body_r.to_lowercase().contains("10 year"));

        // Unknown kind → empty body (so the scan skips it).
        let (_s, b) = flash_subject_body("nope", "x.coderobot.net", 0);
        assert!(b.is_empty());
    }


    #[test]
    fn derive_subdomain_base_prefers_clean_business_name() {
        assert_eq!(
            derive_subdomain_base(Some("Acme Pest Control!!!"), "owner@example.com"),
            "acme-pest-control"
        );
    }

    #[test]
    fn derive_subdomain_base_falls_back_to_email_local_part() {
        assert_eq!(
            derive_subdomain_base(None, "hello.there@example.com"),
            "hello-there"
        );
    }

    fn tier(slug: &'static str, addon: f64) -> FieldServiceTierDef {
        FieldServiceTierDef {
            slug,
            name: slug,
            price_monthly: 0.0,
            trial_days: 0,
            trucks_included: 0,
            truck_addon_monthly: 0.0,
            monthly_credits: 0,
            modules_included: luperiq_forge::nexus::ModuleSet::Named("standard"),
            features: &[],
            priority_support: false,
            lifetime_truck_price: 0.0,
            setup_addon_price: addon,
        }
    }

    #[test]
    fn snapshot_addon_cents_unknown_slug_is_none() {
        let tiers = vec![tier("starter", 499.0)];
        assert_eq!(snapshot_addon_cents(&tiers, "professional"), None);
    }

    #[test]
    fn snapshot_addon_cents_zero_price_is_none() {
        let tiers = vec![tier("free", 0.0)];
        assert_eq!(snapshot_addon_cents(&tiers, "free"), None);
    }

    #[test]
    fn snapshot_addon_cents_whole_dollar_price_converts_cleanly() {
        let tiers = vec![tier("starter", 499.0)];
        assert_eq!(snapshot_addon_cents(&tiers, "starter"), Some(49900));
    }

    #[test]
    fn snapshot_addon_cents_fractional_price_rounds_half_up() {
        let tiers = vec![tier("professional", 499.99)];
        assert_eq!(snapshot_addon_cents(&tiers, "professional"), Some(49999));
    }

    #[test]
    fn snapshot_addon_cents_negative_price_is_none() {
        let tiers = vec![tier("broken", -50.0)];
        assert_eq!(snapshot_addon_cents(&tiers, "broken"), None);
    }

    #[test]
    fn snapshot_addon_cents_non_finite_price_is_none() {
        let tiers = vec![tier("nan", f64::NAN), tier("inf", f64::INFINITY)];
        assert_eq!(snapshot_addon_cents(&tiers, "nan"), None);
        assert_eq!(snapshot_addon_cents(&tiers, "inf"), None);
    }
}

// ---------------------------------------------------------------------------
// Trial site provisioning (runs in tokio::spawn)
// ---------------------------------------------------------------------------

/// Provision a trial site and send welcome email. Best-effort — all errors
/// are logged but never propagated.
async fn provision_trial_site(
    journal: &SharedJournal,
    trial_id: &str,
    customer_email: &str,
    domain: &str,
    industry_slug: &str,
    business_name: &str,
    phone: &str,
    cancel_token: &str,
    magic_token_hash: &str,
    magic_token_expires_at: u64,
    trial_started_at: u64,
    trial_expires_at: u64,
    wizard_answers: Option<&serde_json::Value>,
    admin_password: Option<String>,
    // Internal NexClient license_tier to provision the new site AT (e.g.
    // "pro-lifetime"). `None` = free site (unchanged behavior). Forwarded to
    // provision-site.sh via the job descriptor's `tier` field.
    provision_tier: Option<&str>,
    // Purchased per-truck count (field-service). Only meaningful for a paid
    // provision; forwarded as the job's `trucks` field alongside `tier`.
    provision_trucks: u32,
) {
    let license_key = generate_trial_license();
    // Dev/test override: if LUPERIQ_DEV_ADMIN_PASSWORD is set in env, use it
    // as the admin password for the new tenant instead of generating a random
    // one. Set this in /etc/luperiq-customer.env on the testLuperIQ deployment;
    // unset on production deployments so each tenant gets unique random creds.
    // Admin password precedence: user-chosen at signup (app) → dev env override
    // → random. A valid user password lets them log in to the app immediately.
    // NEVER logged anywhere.
    let password = admin_password
        .filter(|v| v.len() >= 8 && v.len() <= 128)
        .or_else(|| {
            std::env::var("LUPERIQ_DEV_ADMIN_PASSWORD")
                .ok()
                .filter(|v| v.len() >= 10 && v.len() <= 128)
        })
        .unwrap_or_else(|| generate_password(16));
    let industry = if industry_slug.is_empty() {
        "general"
    } else {
        industry_slug
    };

    // Use business name for the site name, fall back to "My Business"
    let site_name = if business_name.is_empty() {
        "My Business".to_string()
    } else {
        business_name.to_string()
    };

    eprintln!(
        "[trial-provision] Starting site provisioning for {} — domain: {} name: {:?} (trial {})",
        customer_email, domain, site_name, trial_id
    );

    let password_file = match write_provision_password_file(trial_id, &password) {
        Ok(path) => path,
        Err(e) => {
            eprintln!(
                "[trial-provision] Failed to prepare password handoff for {} — domain: {} — error: {e}",
                customer_email, domain
            );
            notify_admin_trial_failure(
                journal,
                customer_email,
                domain,
                trial_id,
                &format!("Failed to prepare password handoff: {e}"),
            )
            .await;
            return;
        }
    };

    // Build the job descriptor for the provision-worker (see provision-worker.py).
    // Worker handles password_file cleanup after it reads the password, so we do
    // NOT remove it here.
    let mut job = serde_json::json!({
        "trial_id": trial_id,
        "domain": domain,
        "license": license_key,
        "industry": industry,
        "site_name": site_name,
        "admin_email": customer_email,
        "admin_password_file": password_file.to_string_lossy(),
    });
    // Paid provision: carry the purchased tier (+trucks) so provision-site.sh
    // brings the new site up AT its tier. Omitted entirely for free signups →
    // provision-worker passes no --tier → provision-site.sh stays on the free
    // path (byte-identical to today).
    if let Some(tier) = provision_tier.filter(|t| !t.is_empty()) {
        job["tier"] = serde_json::Value::String(tier.to_string());
        job["trucks"] = serde_json::Value::Number(provision_trucks.max(1).into());
    }
    if !phone.is_empty() {
        job["phone"] = serde_json::Value::String(phone.to_string());
    }
    if !magic_token_hash.is_empty() {
        job["magic_token_hash"] = serde_json::Value::String(magic_token_hash.to_string());
        job["magic_token_expires_at"] = serde_json::Value::Number(magic_token_expires_at.into());
    }
    if trial_expires_at > 0 {
        job["trial_started_at"] = serde_json::Value::Number(trial_started_at.into());
        job["trial_expires_at"] = serde_json::Value::Number(trial_expires_at.into());
    }
    if let Some(answers) = wizard_answers {
        if !answers.is_null() && !answers.as_object().map(|o| o.is_empty()).unwrap_or(true) {
            job["wizard_answers"] = answers.clone();
        }
    }

    // Enqueue + wait for the worker. Mirrors the previous `Command::output()`
    // shape so the success/failure handling below stays unchanged.
    if let Err(e) = enqueue_provision_job(trial_id, &job).await {
        log_funnel_event(
            "provision.enqueue_failed",
            &[("trial_id", trial_id), ("domain", domain), ("error", &e.to_string())],
        );
        eprintln!("[trial-provision] Failed to enqueue job for {customer_email} — domain: {domain} — error: {e}");
        let _ = tokio::fs::remove_file(&password_file).await;
        notify_admin_trial_failure(
            journal,
            customer_email,
            domain,
            trial_id,
            &format!("Failed to enqueue provision job: {e}"),
        )
        .await;
        return;
    }

    // Poll for completion. Worker timeout is 5 min; we add buffer for queue I/O.
    let result: Result<std::process::Output, std::io::Error> =
        match await_provision_result(trial_id, 360).await {
            Ok((success, stdout, stderr, exit_code)) => {
                // Synthesize a Command::Output so downstream sanitize/email/log
                // code keeps working without further refactor.
                use std::os::unix::process::ExitStatusExt;
                let raw_status = if success {
                    0i32
                } else {
                    // wait()-style status: low byte = signal, high byte = exit code
                    (exit_code.unwrap_or(1)) << 8
                };
                Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(raw_status),
                    stdout: stdout.into_bytes(),
                    stderr: stderr.into_bytes(),
                })
            }
            Err(msg) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, msg)),
        };

    match result {
        Ok(output) => {
            let stdout_raw = String::from_utf8_lossy(&output.stdout);
            let stderr_raw = String::from_utf8_lossy(&output.stderr);
            let stdout = sanitize_provision_log(&stdout_raw, &[&password, &license_key]);
            let stderr = sanitize_provision_log(&stderr_raw, &[&password, &license_key]);

            if output.status.success() {
                log_funnel_event(
                    "provision.completed",
                    &[("trial_id", trial_id), ("domain", domain)],
                );
                eprintln!(
                    "[trial-provision] Site provisioned successfully for {} — domain: {}",
                    customer_email, domain
                );
                if !stdout.is_empty() {
                    eprintln!("[trial-provision] stdout: {stdout}");
                }
                if !stderr.is_empty() {
                    eprintln!("[trial-provision] stderr: {stderr}");
                }

                // Send welcome email with credentials
                send_trial_welcome_email(
                    journal,
                    customer_email,
                    domain,
                    &password,
                    industry,
                    cancel_token,
                )
                .await;

                // Log the provisioning event to the WAL
                log_trial_provision_event(journal, trial_id, customer_email, domain, &license_key)
                    .await;
            } else {
                log_funnel_event(
                    "provision.failed",
                    &[
                        ("trial_id", trial_id),
                        ("domain", domain),
                        ("exit_code", &format!("{:?}", output.status.code())),
                    ],
                );
                eprintln!(
                    "[trial-provision] Script failed (exit code {:?}) for {} — domain: {}",
                    output.status.code(),
                    customer_email,
                    domain
                );
                if !stdout.is_empty() {
                    eprintln!("[trial-provision] stdout: {stdout}");
                }
                if !stderr.is_empty() {
                    eprintln!("[trial-provision] stderr: {stderr}");
                }

                // Send welcome email anyway — the site may still be running
                send_trial_welcome_email(
                    journal,
                    customer_email,
                    domain,
                    &password,
                    industry,
                    cancel_token,
                )
                .await;

                // Log provisioning event even on failure
                log_trial_provision_event(journal, trial_id, customer_email, domain, &license_key)
                    .await;

                // Notify admin of failure
                notify_admin_trial_failure(
                    journal,
                    customer_email,
                    domain,
                    trial_id,
                    &format!("Exit code: {:?}\n{stderr}", output.status.code()),
                )
                .await;
            }
        }
        Err(e) => {
            eprintln!(
                "[trial-provision] Failed to execute provision script for {} — domain: {} — error: {e}",
                customer_email, domain
            );
            notify_admin_trial_failure(
                journal,
                customer_email,
                domain,
                trial_id,
                &format!("Failed to execute: {e}"),
            )
            .await;
        }
    }
}

/// Send a welcome email to the trial customer with their site credentials.
async fn send_trial_welcome_email(
    journal: &SharedJournal,
    customer_email: &str,
    domain: &str,
    password: &str,
    _industry_slug: &str,
    cancel_token: &str,
) {
    let site_url = format!("https://{domain}");
    let admin_url = format!("https://{domain}/admin");
    // Referral code = subdomain portion of the user's domain. Unique per
    // site, readable, already validated at provision time.
    let referral_code = domain
        .strip_suffix(".coderobot.net")
        .unwrap_or(domain)
        .to_string();

    let headline = "Your 7-Day Free Trial Starts Now";
    let banner_html = r#"<div style="background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 8px; padding: 14px 18px; margin: 16px 0; text-align: center;">
    <p style="margin: 0 0 4px; font-size: 0.95rem; color: #166534; font-weight: 600;">
        Every feature unlocked for 7 days. No card required.
    </p>
    <p style="margin: 0; font-size: 0.85rem; color: #15803d;">
        On day 8 your site becomes a private preview until you upgrade — nothing is ever deleted.
    </p>
    <p style="margin: 8px 0 0; font-size: 0.82rem; color: #166534;">
        One honest note: your trial does <strong>not</strong> lock your price. Checkout locks the founding price for life &mdash; lock it in once, keep it forever.
    </p>
</div>"#;

    let html = format!(
        r##"<!DOCTYPE html>
<html>
<head><meta charset="utf-8"></head>
<body style="font-family: system-ui, -apple-system, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; color: #1e293b;">
<div style="text-align: center; margin-bottom: 24px;">
    <h1 style="color: #2563eb; margin: 0;">{headline}</h1>
    <p style="color: #64748b; margin: 8px 0 0;">Welcome to LuperIQ &mdash; your site is ready to explore.</p>
</div>

{banner_html}

<div style="background: #f8fafc; border: 1px solid #e2e8f0; border-radius: 8px; padding: 24px; margin: 20px 0;">
    <h2 style="margin: 0 0 16px; font-size: 1.1rem; color: #334155;">Your Login Credentials</h2>
    <table style="width: 100%; border-collapse: collapse;">
        <tr>
            <td style="padding: 8px 0; color: #64748b; width: 120px;">Site URL</td>
            <td style="padding: 8px 0; font-weight: 600;"><a href="{site_url}" style="color: #2563eb;">{domain}</a></td>
        </tr>
        <tr>
            <td style="padding: 8px 0; color: #64748b;">Admin Panel</td>
            <td style="padding: 8px 0;"><a href="{admin_url}" style="color: #2563eb;">{admin_url}</a></td>
        </tr>
        <tr>
            <td style="padding: 8px 0; color: #64748b;">Email</td>
            <td style="padding: 8px 0; font-weight: 600;">{customer_email}</td>
        </tr>
        <tr>
            <td style="padding: 8px 0; color: #64748b;">Password</td>
            <td style="padding: 8px 0; font-family: monospace; font-size: 1.05rem; font-weight: 700; color: #0f172a; background: #f1f5f9; padding: 8px 12px; border-radius: 4px;">{password}</td>
        </tr>
    </table>
</div>

<div style="text-align: center; margin-top: 24px;">
    <a href="{admin_url}" style="display: inline-block; padding: 14px 32px; background: #2563eb; color: white; text-decoration: none; border-radius: 8px; font-weight: 600; font-size: 1.05rem;">Open Admin Panel</a>
</div>

<div style="margin-top: 28px; padding: 18px 20px; background: #eff6ff; border: 1px solid #bfdbfe; border-radius: 8px;">
    <h3 style="font-size: 0.95rem; color: #1e40af; margin: 0 0 8px;">Know someone who&rsquo;d use this?</h3>
    <p style="color: #1e3a5f; font-size: 0.88rem; margin: 0 0 8px;">Share this link &mdash; anyone who signs up through it is credited to you:</p>
    <div style="background: #ffffff; padding: 10px 12px; border-radius: 4px; font-family: monospace; font-size: 0.82rem; color: #0f172a; border: 1px solid #dbeafe;">
        https://coderobot.net/start-free?ref={referral_code}
    </div>
</div>

<div style="margin-top: 32px; padding-top: 20px; border-top: 1px solid #e2e8f0;">
    <h3 style="font-size: 0.95rem; color: #334155; margin: 0 0 12px;">Quick Start Guide</h3>
    <ol style="color: #475569; line-height: 1.8; padding-left: 20px; margin: 0;">
        <li>Log in to your admin panel</li>
        <li>Update your company profile and logo</li>
        <li>Customize your site theme in Design Studio</li>
        <li>Add your services and content</li>
        <li>Connect your own domain (see below)</li>
    </ol>
</div>

<div style="margin-top: 24px; padding: 20px; background: #eff6ff; border: 1px solid #bfdbfe; border-radius: 8px;">
    <h3 style="font-size: 0.95rem; color: #1e40af; margin: 0 0 12px;">Want to use your own domain?</h3>
    <p style="color: #1e3a5f; font-size: 0.9rem; margin: 0 0 12px;">Your site is live right now at <strong>{domain}</strong>. To use your own domain, add an A record pointing to our server:</p>
    <div style="background: #f1f5f9; padding: 8px 12px; border-radius: 4px; margin: 6px 0; font-family: monospace; font-size: 0.85rem;">
        Type: <strong>A</strong> &nbsp; | &nbsp; Name: <strong>@</strong> &nbsp; | &nbsp; Value: <strong>24.178.222.168</strong>
    </div>
    <p style="color: #64748b; font-size: 0.8rem; margin: 8px 0 0;">Reply to this email with your domain name and we will connect it for you.</p>
</div>

<div style="text-align: center; margin-top: 32px; font-size: 0.85rem; color: #94a3b8;">
    <p>Need help? Reply to this email or visit our <a href="https://coderobot.net/support" style="color: #2563eb;">support page</a>.</p>
    <p>LuperIQ &mdash; Build something remarkable.</p>
    <p style="margin-top: 16px;"><a href="https://coderobot.net/cancel-site?token={cancel_token}" style="color: #94a3b8; text-decoration: underline;">Cancel my site</a></p>
</div>
</body>
</html>"##
    );

    let subject = format!("Your Free Site Is Live — {domain}");

    match send_email_internal(journal, customer_email, &subject, &html, true).await {
        Ok(()) => {
            eprintln!(
                "[trial-provision] Welcome email sent to {} for domain {}",
                customer_email, domain
            );
        }
        Err(e) => {
            eprintln!(
                "[trial-provision] Failed to send welcome email to {}: {e}",
                customer_email
            );
        }
    }
}

/// Log a trial provisioning event to the WAL for audit purposes.
async fn log_trial_provision_event(
    journal: &SharedJournal,
    trial_id: &str,
    customer_email: &str,
    domain: &str,
    license_key: &str,
) {
    let event_data = serde_json::json!({
        "trial_id": trial_id,
        "customer_email": customer_email,
        "domain": domain,
        "license_key": license_key,
        "provisioned_at": chrono::Utc::now().to_rfc3339(),
        "status": "completed",
        "type": "trial",
    });

    if let Ok(bytes) = serde_json::to_vec(&event_data) {
        let mut j = journal.lock().await;
        let agg_id = format!("prov-trial-{}", ulid::Ulid::new());
        if let Err(e) = j.append(ApexEvent::new(AGG_TRIAL_PROVISION, &agg_id, bytes)) {
            eprintln!("[trial-provision] Failed to log provision event: {e}");
        }
    }
}

/// Notify admin when trial site provisioning fails.
async fn notify_admin_trial_failure(
    journal: &SharedJournal,
    customer_email: &str,
    domain: &str,
    trial_id: &str,
    error_detail: &str,
) {
    let script_path = provision_script_path();
    let html = format!(
        r#"<!DOCTYPE html><html><body style="font-family:system-ui,sans-serif;max-width:600px;margin:0 auto;padding:20px;color:#1e293b;">
<h2 style="color:#dc2626;">Trial Site Provisioning Failed</h2>
<table style="width:100%;border-collapse:collapse;">
<tr><td style="padding:8px 0;color:#64748b;">Trial ID</td><td style="padding:8px 0;font-weight:600;">{trial_id}</td></tr>
<tr><td style="padding:8px 0;color:#64748b;">Customer</td><td style="padding:8px 0;">{customer_email}</td></tr>
<tr><td style="padding:8px 0;color:#64748b;">Domain</td><td style="padding:8px 0;">{domain}</td></tr>
</table>
<div style="margin-top:16px;padding:12px;background:#fef2f2;border:1px solid #fecaca;border-radius:6px;">
<pre style="margin:0;font-size:0.85rem;white-space:pre-wrap;color:#7f1d1d;">{error_detail}</pre>
</div>
<p style="margin-top:16px;color:#64748b;">Manual provisioning may be required. Run:</p>
<pre style="background:#f1f5f9;padding:12px;border-radius:6px;font-size:0.85rem;">sudo {script_path} \
  --domain {domain} \
  --license LIQ-TRIAL-XXXX-XXXX \
  --industry general \
  --admin-email {customer_email}</pre>
</body></html>"#
    );

    let subject = format!("ALERT: Trial Provisioning Failed — {domain}");
    let admin_email = {
        let j = journal.lock().await;
        luperiq_forge::get_notification_email(&j, "admin")
    };
    if admin_email.is_empty() {
        eprintln!("[trial-provision] No admin notification email configured — skipping alert");
    } else if let Err(e) = send_email_internal(journal, &admin_email, &subject, &html, true).await {
        eprintln!("[trial-provision] Failed to notify admin of provision failure: {e}");
    }
}

// ── Provision timeout alert (called from client after 4 minutes) ──

#[derive(Debug, Deserialize)]
struct ProvisionTimeoutPayload {
    site_url: String,
    email: String,
}

/// POST /api/modules/sales-funnel/provision-timeout
///
/// Client-side 4-minute timeout fires this to alert the team. Captures
/// journalctl logs for the provisioning and sends an email alert.
async fn provision_timeout_alert(
    State(state): State<SalesFunnelState>,
    axum::extract::Json(p): axum::extract::Json<ProvisionTimeoutPayload>,
) -> Json<ApiResult> {
    let journal = state.journal;
    // Sanitize user input for logging (prevent log injection via newlines/escapes)
    fn sanitize_log(s: &str) -> String {
        s.replace(['\n', '\r', '\x1b'], " ")
    }
    let site_url = sanitize_log(p.site_url.trim());
    let email = sanitize_log(&p.email.trim().to_lowercase());
    eprintln!(
        "[trial-provision] TIMEOUT ALERT: {} for {} — provisioning exceeded 4 minutes",
        site_url, email
    );

    // Capture recent provision logs
    let logs = match tokio::process::Command::new("journalctl")
        .args([
            "-u",
            "luperiq-cms",
            "--no-pager",
            "-n",
            "80",
            "--output",
            "short-iso",
        ])
        .output()
        .await
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
        Err(e) => format!("Failed to capture logs: {e}"),
    };

    let html = format!(
        r#"<!DOCTYPE html><html><body style="font-family:system-ui,sans-serif;max-width:600px;margin:0 auto;padding:20px;color:#1e293b;">
<h2 style="color:#f59e0b;">Provision Timeout — 4 Minutes Exceeded</h2>
<p>A customer has been waiting over 4 minutes for their site to build.</p>
<table style="width:100%;border-collapse:collapse;">
<tr><td style="padding:8px 0;color:#64748b;">Customer</td><td style="padding:8px 0;font-weight:600;">{email}</td></tr>
<tr><td style="padding:8px 0;color:#64748b;">Site URL</td><td style="padding:8px 0;">{site_url}</td></tr>
</table>
<h3 style="margin-top:20px;">Recent Server Logs</h3>
<div style="margin-top:8px;padding:12px;background:#f1f5f9;border:1px solid #e2e8f0;border-radius:6px;max-height:400px;overflow:auto;">
<pre style="margin:0;font-size:0.78rem;white-space:pre-wrap;color:#334155;">{logs}</pre>
</div>
</body></html>"#,
        email = email
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;"),
        site_url = site_url
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;"),
        logs = logs
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;"),
    );

    let subject = format!("SLOW PROVISION: {} waiting 4+ min — {}", email, site_url);
    let admin_email = {
        let j = journal.lock().await;
        luperiq_forge::get_notification_email(&j, "admin")
    };
    if admin_email.is_empty() {
        eprintln!("[trial-provision] No notification email configured — skipping timeout alert");
    } else if let Err(e) = send_email_internal(&journal, &admin_email, &subject, &html, true).await
    {
        eprintln!("[trial-provision] Failed to send timeout alert email: {e}");
    }

    Json(ApiResult {
        ok: true,
        message: "Alert sent".into(),
        data: None,
    })
}

/// GET /api/modules/sales-funnel/provision-status?site_url=...
///
/// Same-origin health check proxy. Returns whether the provisioned site
/// is up and responding to health checks.
async fn provision_status_check(
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let site_url = q.get("site_url").cloned().unwrap_or_default();
    let allowed_domain = site_url.contains(".coderobot.net") || site_url.contains(".pestcontroller.org") || site_url.contains(".generalpestco.com");
    if site_url.is_empty() || !allowed_domain {
        return Json(serde_json::json!({"ready": false, "error": "invalid site_url"}));
    }

    let health_url = format!("{}/health", site_url.trim_end_matches('/'));
    let ready = match tokio::process::Command::new("curl")
        .args(["-sf", "--max-time", "5", &health_url])
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    Json(serde_json::json!({"ready": ready}))
}

/// GET /api/modules/sales-funnel/status?email=...&all=true
///
/// Returns the current trial for the given email, or all trials if `all=true`.
async fn trial_status(
    State(state): State<SalesFunnelState>,
    Query(q): Query<StatusQuery>,
) -> Json<ApiResult> {
    let j = state.journal.lock().await;

    // If all=true, return all trials (admin use)
    if q.all.as_deref() == Some("true") {
        let all = trials::list_trials(&j);
        return Json(ApiResult {
            ok: true,
            message: format!("{} trials", all.len()),
            data: Some(serde_json::to_value(&all).unwrap_or_default()),
        });
    }

    let Some(email) = q.email.as_deref() else {
        return Json(ApiResult {
            ok: false,
            message: "email query parameter required".into(),
            data: None,
        });
    };

    match trials::get_trial(&j, email) {
        Some(trial) => Json(ApiResult {
            ok: true,
            message: format!("Trial found: stage={}", trial.stage),
            data: Some(serde_json::to_value(&trial).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: "No trial found for this email".into(),
            data: None,
        }),
    }
}

/// POST /api/modules/sales-funnel/extend
///
/// Extend a free trial to a paid trial (14 days).
async fn extend_trial(
    State(state): State<SalesFunnelState>,
    axum::extract::Json(p): axum::extract::Json<ExtendTrialPayload>,
) -> Json<ApiResult> {
    let email = p.email.trim().to_lowercase();
    let mut j = state.journal.lock().await;

    let existing = match trials::get_trial(&j, &email) {
        Some(t) => t,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "No trial found for this email".into(),
                data: None,
            });
        }
    };

    if existing.stage != "free" {
        return Json(ApiResult {
            ok: false,
            message: format!(
                "Trial is already in stage '{}', cannot extend",
                existing.stage
            ),
            data: None,
        });
    }

    let now = now_ts();
    let mut updated = existing;
    updated.stage = "paid".into();
    updated.paid_started_at = Some(now);
    updated.paid_expires_at = Some(now + 14 * 86400); // 14 days
    updated.stripe_session_id = p.stripe_session_id;

    if let Err(e) = trials::write_trial(&mut j, &updated) {
        return Json(ApiResult {
            ok: false,
            message: format!("WAL write failed: {e}"),
            data: None,
        });
    }

    // Also update the lead stage
    if let Some(mut lead) = leads::get_lead(&j, &email) {
        lead.stage = "trial_paid".into();
        lead.updated_at = now;
        if let Ok(mut ts) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            lead.stage_timestamps.clone(),
        ) {
            ts.insert("trial_paid".into(), serde_json::json!(now));
            lead.stage_timestamps = serde_json::Value::Object(ts);
        }
        let _ = leads::write_lead(&mut j, &lead);
    }

    Json(ApiResult {
        ok: true,
        message: "Trial extended to paid (14 days)".into(),
        data: Some(serde_json::json!({
            "stage": "paid",
            "paid_expires_at": updated.paid_expires_at,
        })),
    })
}

/// GET /api/modules/sales-funnel/banner?email=...
///
/// Returns trial banner data as JSON (stage, time remaining, show_upsell).
async fn banner_data(
    State(state): State<SalesFunnelState>,
    Query(q): Query<StatusQuery>,
) -> Json<ApiResult> {
    let email = q.email.as_deref().unwrap_or("");
    if email.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "email query parameter required".into(),
            data: None,
        });
    }

    let j = state.journal.lock().await;
    match trials::get_trial(&j, email) {
        Some(trial) => {
            let now = now_ts();
            let (expires_at, time_remaining) = match trial.stage.as_str() {
                "free" => {
                    let rem = trial.free_expires_at.saturating_sub(now);
                    (trial.free_expires_at, rem)
                }
                "paid" => {
                    let exp = trial.paid_expires_at.unwrap_or(0);
                    let rem = exp.saturating_sub(now);
                    (exp, rem)
                }
                _ => (0, 0),
            };
            let show_upsell = trial.stage == "free" && time_remaining < 43200; // < 12 hours

            Json(ApiResult {
                ok: true,
                message: "Banner data".into(),
                data: Some(serde_json::json!({
                    "stage": trial.stage,
                    "time_remaining_secs": time_remaining,
                    "expires_at": expires_at,
                    "show_upsell": show_upsell,
                    "industry_slug": trial.industry_slug,
                })),
            })
        }
        None => Json(ApiResult {
            ok: false,
            message: "No active trial".into(),
            data: None,
        }),
    }
}

/// GET /api/modules/sales-funnel/leads
///
/// Admin: list all leads.
async fn list_leads(State(state): State<SalesFunnelState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let all = leads::list_leads(&j);
    Json(ApiResult {
        ok: true,
        message: format!("{} leads", all.len()),
        data: Some(serde_json::to_value(&all).unwrap_or_default()),
    })
}

/// GET /api/modules/sales-funnel/stats
///
/// Admin: funnel stage counts.
async fn funnel_stats(State(state): State<SalesFunnelState>) -> Json<ApiResult> {
    let j = state.journal.lock().await;
    let stats = leads::funnel_stats(&j);
    Json(ApiResult {
        ok: true,
        message: format!("{} total leads", stats.total),
        data: Some(serde_json::to_value(&stats).unwrap_or_default()),
    })
}

// ── Site Deactivation (Kill Switch) ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeactivatePayload {
    pub token: String,
}

/// POST /api/modules/sales-funnel/deactivate — deactivate a site by cancel token.
/// Marks the trial as "deactivated" and stops the systemd service.
/// Rate limited to 3 requests per hour per IP to prevent token brute-forcing.
async fn deactivate_site(
    State(state): State<SalesFunnelState>,
    headers: HeaderMap,
    axum::extract::Json(p): axum::extract::Json<DeactivatePayload>,
) -> axum::response::Response {
    let ip = client_ip(&headers);
    if !state.cancel_limiter.check(&ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiResult {
                ok: false,
                message: "Too many attempts. Try again later.".into(),
                data: None,
            }),
        )
            .into_response();
    }

    let mut j = state.journal.lock().await;
    let token = p.token.trim();

    // Find the trial with this cancel token
    let all_trials = trials::list_trials(&j);
    let trial = all_trials
        .iter()
        .find(|t| t.cancel_token.as_deref() == Some(token) && t.stage != "deactivated");

    let trial = match trial {
        Some(t) => t.clone(),
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Invalid or expired cancel token.".into(),
                data: None,
            })
            .into_response();
        }
    };

    // Mark as deactivated
    let now = now_ts();
    let mut updated = trial.clone();
    updated.stage = "deactivated".into();
    updated.deactivated_at = Some(now);

    if let Err(e) = trials::write_trial(&mut j, &updated) {
        return Json(ApiResult {
            ok: false,
            message: format!("Failed to deactivate: {e}"),
            data: None,
        })
        .into_response();
    }

    // Stop the systemd service if we know the domain
    if let Some(ref domain) = updated.domain {
        let svc_name = format!("luperiq-{}.service", domain.replace('.', "-"));
        tokio::spawn(async move {
            let _ = tokio::process::Command::new("sudo")
                .args(["systemctl", "stop", &svc_name])
                .output()
                .await;
            let _ = tokio::process::Command::new("sudo")
                .args(["systemctl", "disable", &svc_name])
                .output()
                .await;
            eprintln!("[kill-switch] Service {svc_name} stopped and disabled");
        });
    }

    Json(ApiResult {
        ok: true,
        message: "Site deactivated.".into(),
        data: Some(serde_json::json!({
            "trial_id": updated.trial_id,
            "domain": updated.domain,
            "deactivated_at": now,
        })),
    })
    .into_response()
}

#[derive(Debug, Deserialize)]
pub struct CancelQuery {
    pub token: Option<String>,
}

/// GET /cancel-site?token=... — cancel confirmation page.
/// Rate limited to 3 requests per hour per IP to prevent token brute-forcing.
async fn cancel_site_page(
    State(state): State<SalesFunnelState>,
    headers: HeaderMap,
    Query(q): Query<CancelQuery>,
) -> axum::response::Response {
    let ip = client_ip(&headers);
    if !state.cancel_limiter.check(&ip).await {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            axum::response::Html(cancel_page_html(
                "Too Many Requests",
                "Too many attempts. Try again later.",
            )),
        )
            .into_response();
    }

    let token = q.token.unwrap_or_default();

    if token.is_empty() {
        return axum::response::Html(cancel_page_html(
            "Invalid Link",
            "This cancel link is missing a token. Please use the link from your welcome email.",
        ))
        .into_response();
    }

    let j = state.journal.lock().await;
    let all_trials = trials::list_trials(&j);
    let trial = all_trials
        .iter()
        .find(|t| t.cancel_token.as_deref() == Some(token.as_str()));

    match trial {
        Some(t) if t.stage == "deactivated" => axum::response::Html(cancel_page_html(
            "Already Deactivated",
            "This site has already been deactivated.",
        ))
        .into_response(),
        Some(t) => {
            let domain = t.domain.as_deref().unwrap_or("your site");
            axum::response::Html(cancel_page_html_confirm(domain, &token)).into_response()
        }
        None => axum::response::Html(cancel_page_html(
            "Invalid Token",
            "This cancel link is invalid or has expired. Please contact support if you need help.",
        ))
        .into_response(),
    }
}

fn cancel_page_html(title: &str, message: &str) -> String {
    format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — LuperIQ</title>
<style>body{{font-family:system-ui,sans-serif;max-width:480px;margin:80px auto;padding:0 20px;color:#1e293b;text-align:center;}}
h1{{font-size:1.6rem;margin-bottom:12px;}}p{{color:#64748b;line-height:1.6;}}</style>
</head><body><h1>{title}</h1><p>{message}</p>
<p style="margin-top:32px;"><a href="https://coderobot.net" style="color:#2563eb;">Back to LuperIQ</a></p>
</body></html>"#
    )
}

fn cancel_page_html_confirm(domain: &str, token: &str) -> String {
    format!(
        r##"<!DOCTYPE html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Cancel Your Site — LuperIQ</title>
<style>
body{{font-family:system-ui,sans-serif;max-width:480px;margin:80px auto;padding:0 20px;color:#1e293b;text-align:center;}}
h1{{font-size:1.6rem;margin-bottom:8px;}}
.domain{{font-weight:700;color:#0f172a;font-size:1.1rem;}}
p{{color:#64748b;line-height:1.6;}}
.btn-cancel{{display:inline-block;padding:14px 32px;background:#dc2626;color:#fff;border:none;border-radius:10px;font-size:1rem;font-weight:600;cursor:pointer;margin-top:24px;text-decoration:none;}}
.btn-cancel:hover{{background:#b91c1c;}}
.btn-keep{{display:inline-block;padding:14px 32px;background:#22c55e;color:#fff;border-radius:10px;font-size:1rem;font-weight:600;text-decoration:none;margin-top:12px;}}
#result{{display:none;margin-top:24px;padding:16px;border-radius:10px;}}
</style>
</head><body>
<h1>Cancel Your Site?</h1>
<p>You're about to deactivate <span class="domain">{domain}</span>.</p>
<p>Your data will be preserved for 30 days in case you change your mind.</p>
<div id="buttons">
<button class="btn-cancel" onclick="deactivate()">Yes, Deactivate My Site</button><br>
<a href="https://{domain}" class="btn-keep">No, Keep My Site</a>
</div>
<div id="result"></div>
<script>
async function deactivate() {{
    var btn = document.querySelector('.btn-cancel');
    btn.textContent = 'Deactivating...';
    btn.disabled = true;
    try {{
        var res = await fetch('/api/modules/sales-funnel/deactivate', {{
            method: 'POST',
            headers: {{'Content-Type': 'application/json'}},
            body: JSON.stringify({{ token: '{token}' }})
        }});
        var data = await res.json();
        document.getElementById('buttons').style.display = 'none';
        var r = document.getElementById('result');
        r.style.display = 'block';
        if (data.ok) {{
            r.style.background = '#f0fdf4';
            r.style.color = '#166534';
            r.textContent = 'Site deactivated. Your data is preserved for 30 days. Contact us to reactivate.';
        }} else {{
            r.style.background = '#fef2f2';
            r.style.color = '#991b1b';
            r.textContent = 'Error: ' + (data.message || 'Something went wrong.');
            btn.textContent = 'Yes, Deactivate My Site';
            btn.disabled = false;
            document.getElementById('buttons').style.display = 'block';
        }}
    }} catch(e) {{
        var r = document.getElementById('result');
        r.style.display = 'block';
        r.style.background = '#fef2f2';
        r.style.color = '#991b1b';
        r.textContent = 'Network error. Please try again.';
    }}
}}
</script>
</body></html>"##
    )
}

// ── My Sites (SSO — returns all sites owned by a given email) ───────

#[derive(Debug, Deserialize)]
pub struct MySitesQuery {
    pub email: Option<String>,
}

/// GET /api/modules/sales-funnel/my-sites?email=...
/// Returns all active (non-deactivated) sites for the given email.
/// Used by the "My Sites" dashboard and site-switcher dropdown.
async fn my_sites(
    State(state): State<SalesFunnelState>,
    Query(q): Query<MySitesQuery>,
) -> Json<ApiResult> {
    let email = q.email.unwrap_or_default().to_lowercase();
    if email.is_empty() {
        return Json(ApiResult {
            ok: false,
            message: "Email parameter required".into(),
            data: None,
        });
    }

    let j = state.journal.lock().await;
    let all_trials = trials::list_trials(&j);
    let my_sites: Vec<serde_json::Value> = all_trials
        .iter()
        .filter(|t| t.email.to_lowercase() == email && t.stage != "deactivated")
        .filter_map(|t| {
            let domain = t.domain.as_deref()?;
            Some(serde_json::json!({
                "domain": domain,
                "url": format!("https://{}", domain),
                "industry_slug": t.industry_slug,
                "stage": t.stage,
                "created_at": t.created_at,
            }))
        })
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} sites", my_sites.len()),
        data: Some(serde_json::json!(my_sites)),
    })
}

// ── Admin JavaScript ────────────────────────────────────────────────

const ADMIN_JS: &str = r##"
(function() {
    // ── DOM helpers ───────────────────────────────────────────────
    function _sfEl(tag, className, text) {
        var el = document.createElement(tag);
        if (className) el.className = className;
        if (text) el.textContent = text;
        return el;
    }
    function _sfStyle(el, css) { el.style.cssText = css; return el; }
    function _sfFmtDate(ts) {
        if (!ts) return '';
        try {
            var d = new Date(ts * 1000);
            return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        } catch(e) { return ''; }
    }
    async function _sfFetch(path) {
        try {
            var r = await fetch(path);
            var j = await r.json();
            return j.ok ? j : { ok: false, data: null };
        } catch(e) { console.error('SF fetch error:', e); return { ok: false, data: null }; }
    }

    // ── Stat card ─────────────────────────────────────────────────
    function _sfStatCard(label, count, color) {
        var card = _sfEl('div');
        _sfStyle(card,
            'flex:1;padding:16px 20px;border-radius:10px;background:var(--surface);' +
            'border:1px solid var(--border);text-align:center;min-width:100px;'
        );
        var c = _sfEl('div', '', String(count));
        _sfStyle(c, 'font-size:28px;font-weight:700;color:' + (color || 'var(--accent)') + ';');
        card.appendChild(c);
        var l = _sfEl('div', '', label);
        _sfStyle(l, 'font-size:12px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;margin-top:4px;');
        card.appendChild(l);
        return card;
    }

    // ── Sales Pipeline view ───────────────────────────────────────
    window.load_sales_pipeline = async function load_sales_pipeline() {
        var main = document.getElementById('adminMain');
        var el = _sfEl('div');

        var toolbar = _sfEl('div', 'toolbar');
        toolbar.appendChild(_sfEl('h2', '', 'Sales Pipeline'));
        el.appendChild(toolbar);

        // Fetch data
        var leadsR, statsR;
        try {
            var results = await Promise.all([
                _sfFetch('/api/modules/sales-funnel/leads'),
                _sfFetch('/api/modules/sales-funnel/stats'),
            ]);
            leadsR = results[0];
            statsR = results[1];
        } catch(e) {
            leadsR = { data: [] };
            statsR = { data: {} };
        }
        var items = Array.isArray(leadsR.data) ? leadsR.data : [];
        var stats = (statsR.data && typeof statsR.data === 'object') ? statsR.data : {};

        // Stat cards
        var row = _sfEl('div');
        _sfStyle(row, 'display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;');
        row.appendChild(_sfStatCard('Total', stats.total || 0, 'var(--accent)'));
        row.appendChild(_sfStatCard('Discovered', stats.discovered || 0, '#6c757d'));
        row.appendChild(_sfStatCard('Trial Started', stats.trial_started || 0, '#e67e22'));
        row.appendChild(_sfStatCard('Trial Paid', stats.trial_paid || 0, '#2980b9'));
        row.appendChild(_sfStatCard('Converted', stats.converted || 0, '#27ae60'));
        row.appendChild(_sfStatCard('Churned', stats.churned || 0, '#e74c3c'));
        el.appendChild(row);

        // Lead table
        if (items.length === 0) {
            var empty = _sfEl('div', '', 'No leads yet. They will appear here as visitors start trials.');
            _sfStyle(empty, 'text-align:center;padding:40px 20px;color:var(--text-muted);font-size:14px;');
            el.appendChild(empty);
        } else {
            var table = _sfEl('table');
            _sfStyle(table, 'width:100%;border-collapse:collapse;font-size:13px;');

            var thead = _sfEl('thead');
            var headRow = _sfEl('tr');
            ['Email', 'Industry', 'Stage', 'Source', 'Created'].forEach(function(h) {
                var th = _sfEl('th', '', h);
                _sfStyle(th, 'text-align:left;padding:10px 12px;border-bottom:2px solid var(--border);font-weight:600;color:var(--text-muted);font-size:11px;text-transform:uppercase;letter-spacing:0.5px;');
                headRow.appendChild(th);
            });
            thead.appendChild(headRow);
            table.appendChild(thead);

            var tbody = _sfEl('tbody');
            items.sort(function(a, b) { return (b.created_at || 0) - (a.created_at || 0); });
            items.forEach(function(lead) {
                var tr = _sfEl('tr');
                tr.onmouseenter = function() { tr.style.background = 'var(--surface)'; };
                tr.onmouseleave = function() { tr.style.background = ''; };

                var emailTd = _sfEl('td', '', lead.email || '');
                _sfStyle(emailTd, 'padding:10px 12px;border-bottom:1px solid var(--border);font-weight:500;');
                tr.appendChild(emailTd);

                var indTd = _sfEl('td', '', lead.industry_slug || '');
                _sfStyle(indTd, 'padding:10px 12px;border-bottom:1px solid var(--border);');
                tr.appendChild(indTd);

                var stageTd = _sfEl('td');
                _sfStyle(stageTd, 'padding:10px 12px;border-bottom:1px solid var(--border);');
                var badge = _sfEl('span', '', lead.stage || '');
                var badgeColors = {
                    'discovered': 'background:#e9ecef;color:#495057;',
                    'trial_started': 'background:#fef3cd;color:#856404;',
                    'trial_paid': 'background:#cce5ff;color:#004085;',
                    'converted': 'background:#d4edda;color:#155724;',
                    'churned': 'background:#f8d7da;color:#721c24;',
                };
                _sfStyle(badge,
                    'font-size:11px;padding:2px 8px;border-radius:4px;font-weight:600;' +
                    (badgeColors[lead.stage] || 'background:var(--border);color:var(--text-muted);')
                );
                stageTd.appendChild(badge);
                tr.appendChild(stageTd);

                var srcTd = _sfEl('td', '', lead.utm_source || lead.source_page || '-');
                _sfStyle(srcTd, 'padding:10px 12px;border-bottom:1px solid var(--border);color:var(--text-muted);');
                tr.appendChild(srcTd);

                var dateTd = _sfEl('td', '', _sfFmtDate(lead.created_at));
                _sfStyle(dateTd, 'padding:10px 12px;border-bottom:1px solid var(--border);color:var(--text-muted);');
                tr.appendChild(dateTd);

                tbody.appendChild(tr);
            });
            table.appendChild(tbody);

            var wrap = _sfEl('div');
            _sfStyle(wrap, 'border:1px solid var(--border);border-radius:10px;overflow:hidden;');
            wrap.appendChild(table);
            el.appendChild(wrap);
        }

        main.replaceChildren(el);
    };

    // ── Trial Management view ─────────────────────────────────────
    window.load_trial_management = async function load_trial_management() {
        var main = document.getElementById('adminMain');
        var el = _sfEl('div');

        var toolbar = _sfEl('div', 'toolbar');
        toolbar.appendChild(_sfEl('h2', '', 'Trial Management'));
        el.appendChild(toolbar);

        var res = await _sfFetch('/api/modules/sales-funnel/status?all=true');
        var trials = Array.isArray(res.data) ? res.data : [];

        if (trials.length === 0) {
            var empty = _sfEl('div', '', 'No trials yet.');
            _sfStyle(empty, 'text-align:center;padding:40px 20px;color:var(--text-muted);font-size:14px;');
            el.appendChild(empty);
        } else {
            var now = Math.floor(Date.now() / 1000);

            // Summary cards
            var free = trials.filter(function(t) { return t.stage === 'free'; }).length;
            var paid = trials.filter(function(t) { return t.stage === 'paid'; }).length;
            var expired = trials.filter(function(t) { return t.stage === 'expired'; }).length;
            var converted = trials.filter(function(t) { return t.stage === 'converted'; }).length;

            var row = _sfEl('div');
            _sfStyle(row, 'display:flex;gap:12px;margin-bottom:20px;flex-wrap:wrap;');
            row.appendChild(_sfStatCard('Free', free, '#e67e22'));
            row.appendChild(_sfStatCard('Paid', paid, '#2980b9'));
            row.appendChild(_sfStatCard('Expired', expired, '#6c757d'));
            row.appendChild(_sfStatCard('Converted', converted, '#27ae60'));
            el.appendChild(row);

            // Trial list
            trials.sort(function(a, b) { return (b.created_at || 0) - (a.created_at || 0); });
            trials.forEach(function(trial) {
                var card = _sfEl('div');
                _sfStyle(card,
                    'padding:14px 18px;border:1px solid var(--border);border-radius:10px;' +
                    'margin-bottom:8px;background:var(--surface);display:flex;' +
                    'justify-content:space-between;align-items:center;gap:16px;'
                );

                var left = _sfEl('div');
                _sfStyle(left, 'flex:1;min-width:0;');

                var emailEl = _sfEl('strong', '', trial.email || '');
                left.appendChild(emailEl);

                var meta = _sfEl('div');
                _sfStyle(meta, 'font-size:12px;color:var(--text-muted);margin-top:4px;');
                var metaParts = [trial.industry_slug || ''];
                if (trial.created_at) metaParts.push('started ' + _sfFmtDate(trial.created_at));
                meta.textContent = metaParts.join(' \u2022 ');
                left.appendChild(meta);

                card.appendChild(left);

                var right = _sfEl('div');
                _sfStyle(right, 'display:flex;align-items:center;gap:8px;flex-shrink:0;');

                // Stage badge
                var badge = _sfEl('span', '', trial.stage || '');
                var stageColors = {
                    'free': 'background:#fef3cd;color:#856404;',
                    'paid': 'background:#cce5ff;color:#004085;',
                    'expired': 'background:#e9ecef;color:#495057;',
                    'converted': 'background:#d4edda;color:#155724;',
                };
                _sfStyle(badge,
                    'font-size:11px;padding:2px 8px;border-radius:4px;font-weight:600;' +
                    (stageColors[trial.stage] || 'background:var(--border);color:var(--text-muted);')
                );
                right.appendChild(badge);

                // Time remaining for active trials
                if (trial.stage === 'free' || trial.stage === 'paid') {
                    var exp = trial.stage === 'free' ? trial.free_expires_at : trial.paid_expires_at;
                    if (exp) {
                        var rem = exp - now;
                        var timeStr;
                        if (rem <= 0) {
                            timeStr = 'Expired';
                        } else if (rem < 3600) {
                            timeStr = Math.ceil(rem / 60) + 'm left';
                        } else if (rem < 86400) {
                            timeStr = Math.ceil(rem / 3600) + 'h left';
                        } else {
                            timeStr = Math.ceil(rem / 86400) + 'd left';
                        }
                        var timeEl = _sfEl('span', '', timeStr);
                        _sfStyle(timeEl, 'font-size:11px;color:var(--text-muted);');
                        right.appendChild(timeEl);
                    }
                }

                card.appendChild(right);
                el.appendChild(card);
            });
        }

        main.replaceChildren(el);
    };
})();
"##;

// ── Trial reminder emails (day 3, 5, 7, expired) ────────────────────
//
// Runs on Central via a tokio::spawn + 10-minute interval. Scans every
// live SiteTrial, computes days-since-start, and sends the matching
// reminder email if it hasn't been sent already. A SiteTrialReminder
// event is recorded per (trial_id, reminder_kind) so we never spam.

/// WAL aggregate type for trial provisioning audit events.
/// Matches the bare literal used in `log_trial_provision_event`.
const AGG_TRIAL_PROVISION: &str = "Platform:TrialProvision";
const AGG_TRIAL_REMINDER: &str = "SalesPipeline:SiteTrialReminder";
const REMINDER_SCAN_INTERVAL_SECS: u64 = 600; // 10 minutes
/// Length of the flash 10%-off window opened by the day-1 `flash10` email.
const FLASH_WINDOW_SECS: u64 = 4 * 3600; // 4 hours
/// Day-1 band in which `flash10` may first fire. Lower bound = ~24h after the
/// trial started; upper bound gives the 10-minute scan a few ticks to catch it
/// exactly once before `reminder_already_sent` suppresses repeats. Kept clear
/// of the day3 (>=2 days) reminder band so the two never collide.
const FLASH10_START_SECS: i64 = 24 * 3600;
const FLASH10_END_SECS: i64 = 28 * 3600;
/// Delay AFTER the `flash10` send before the `flash10_remind` may fire.
const FLASH10_REMIND_DELAY_SECS: u64 = 1800; // 30 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrialReminderRecord {
    trial_id: String,
    kind: String,
    sent_at: u64,
    email: String,
}

fn reminder_key(trial_id: &str, kind: &str) -> String {
    format!("{trial_id}:{kind}")
}

fn reminder_already_sent(
    journal: &luperiq_forge::ForgeJournal,
    trial_id: &str,
    kind: &str,
) -> bool {
    let key = reminder_key(trial_id, kind);
    journal
        .get_latest(AGG_TRIAL_REMINDER, &key)
        .map(|e| e.payload != b"__DELETED__")
        .unwrap_or(false)
}

/// Fetch the `sent_at` (unix seconds) of a previously recorded reminder of
/// `kind` for this trial, or `None` if it was never sent. Used for
/// RELATIVE-to-send timing (the `flash10_remind` reminder fires a fixed delay
/// after the `flash10` email actually went out, NOT after free_started_at).
fn reminder_sent_at(
    journal: &luperiq_forge::ForgeJournal,
    trial_id: &str,
    kind: &str,
) -> Option<u64> {
    let key = reminder_key(trial_id, kind);
    let ev = journal.get_latest(AGG_TRIAL_REMINDER, &key)?;
    if ev.payload == b"__DELETED__" {
        return None;
    }
    serde_json::from_slice::<TrialReminderRecord>(&ev.payload)
        .ok()
        .map(|r| r.sent_at)
}

async fn record_reminder_sent(
    journal: &SharedJournal,
    trial_id: &str,
    kind: &str,
    email: &str,
) {
    let now = now_ts();
    let record = TrialReminderRecord {
        trial_id: trial_id.to_string(),
        kind: kind.to_string(),
        sent_at: now,
        email: email.to_string(),
    };
    let Ok(payload) = serde_json::to_vec(&record) else {
        return;
    };
    let key = reminder_key(trial_id, kind);
    let mut j = journal.lock().await;
    let _ = j.append(ApexEvent::new(AGG_TRIAL_REMINDER, &key, payload));
}

/// The honest price-lock line carried in the admin banner AND every trial
/// conversion email: a running trial does NOT lock the founding price —
/// only checkout does. Matches the live site wording ("founding price",
/// "lock it in", "locked for life"). Single source of truth so the warning
/// reads identically everywhere.
const PRICE_LOCK_WARNING: &str =
    "Your trial is live, but your founding price isn't locked yet — only checkout locks it. Lock it in once and keep the founding rate for life.";

/// Subject + HTML body for the two flash-offer conversion emails.
///
/// `secs_remaining` is the time left in the 4-hour flash window
/// (`flash_offer_expires_at - now`); used by `flash10_remind` to render the
/// live countdown. For `flash10` itself the window is fresh (~4h) so we render
/// the headline "4 hours" framing. The CTA points at the lifetime checkout
/// (`/start-free?plan=lifetime`) on coderobot.net, matching the welcome email
/// and the other reminders' host.
fn flash_subject_body(
    kind: &str,
    domain: &str,
    secs_remaining: u64,
) -> (&'static str, String) {
    let admin_url = format!("https://{domain}/admin");
    // The lifetime checkout entry the live start-trial page wires to.
    let checkout_url = "https://coderobot.net/start-free?plan=lifetime";
    let warning = PRICE_LOCK_WARNING;
    // Render "Xh Ym" remaining, flooring to whole minutes.
    let hrs = secs_remaining / 3600;
    let mins = (secs_remaining % 3600) / 60;
    let remaining_phrase = if hrs > 0 {
        format!("{hrs}h {mins}m")
    } else {
        format!("{mins}m")
    };
    match kind {
        "flash10" => (
            "Your founding price isn't locked yet — here's 10% more off (4 hours)",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#0f172a;margin:0 0 10px;">One day in — here's an extra 10% off, just for the next 4 hours</h1>
  <p style="color:#475569;line-height:1.55;">You've had <strong><a href="{admin_url}" style="color:#2563eb;">{domain}</a></strong> running for a day now. Quick, honest heads-up: <strong>{warning}</strong></p>
  <p style="color:#475569;line-height:1.55;">To make locking it in easy, here's an <strong>extra 10% off the founding price</strong> — on top of the founding rate — if you lock it in within the next <strong>4 hours</strong>.</p>
  <div style="background:#fffbeb;border:1px solid #fde68a;border-radius:8px;padding:14px 18px;margin:18px 0;text-align:center;color:#92400e;font-weight:600;">
    Extra 10% off — expires 4 hours after this email
  </div>
  <div style="text-align:center;margin:28px 0;">
    <a href="{checkout_url}" style="display:inline-block;padding:14px 28px;background:#0f172a;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Lock in my founding price →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">Pay once, keep the founding rate forever — no recurring billing, locked for life. The extra 10% applies automatically at checkout while the window is open.</p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">LuperIQ — <a href="https://coderobot.net" style="color:#94a3b8;">coderobot.net</a></p>
</body></html>"##
            ),
        ),
        "flash10_remind" => (
            "Time's running out on your extra 10% off",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#b45309;margin:0 0 10px;">About {remaining_phrase} left on your extra 10% off</h1>
  <p style="color:#475569;line-height:1.55;">Just a nudge — the <strong>extra 10% off the founding price</strong> for <strong><a href="{admin_url}" style="color:#2563eb;">{domain}</a></strong> closes in about <strong>{remaining_phrase}</strong>.</p>
  <p style="color:#475569;line-height:1.55;">{warning}</p>
  <div style="text-align:center;margin:28px 0;">
    <a href="{checkout_url}" style="display:inline-block;padding:14px 28px;background:#b45309;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Lock it in now →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">Lock it in once, keep the founding rate for life. The extra 10% applies automatically at checkout until the window closes.</p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">LuperIQ — <a href="https://coderobot.net" style="color:#94a3b8;">coderobot.net</a></p>
</body></html>"##
            ),
        ),
        _ => ("LuperIQ", String::new()),
    }
}

fn reminder_subject_body(kind: &str, domain: &str) -> (&'static str, String) {
    let site_url = format!("https://{domain}");
    let admin_url = format!("https://{domain}/admin");
    let pricing_url = "https://coderobot.net/pricing";
    match kind {
        "hour4" => (
            "Your LuperIQ site is live — come finish setting it up",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#0f172a;margin:0 0 10px;">Your site is waiting for you</h1>
  <p style="color:#475569;line-height:1.55;">You signed up for <strong><a href="{site_url}" style="color:#2563eb;">{domain}</a></strong> a few hours ago. Your 7-day trial is running — every feature is unlocked, nothing is deleted if you come back later.</p>
  <p style="color:#475569;line-height:1.55;">A few quick wins when you log in:</p>
  <ul style="color:#475569;line-height:1.65;">
    <li>Add your business info so pages stop saying "Your Business"</li>
    <li>Pick a theme in Design Studio (takes ~30 seconds)</li>
    <li>Let the AI draft your first three pages</li>
  </ul>
  <div style="text-align:center;margin:28px 0;">
    <a href="{admin_url}" style="display:inline-block;padding:14px 28px;background:#2563eb;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Sign in and keep building →</a>
  </div>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">Lost? Reply to this email. We read every message.</p>
</body></html>"##
            ),
        ),
        "day3" => (
            "Day 3 of your LuperIQ trial — halfway there",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#0f172a;margin:0 0 10px;">You're halfway through your trial</h1>
  <p style="color:#475569;line-height:1.55;">Four days left on <strong><a href="{site_url}" style="color:#2563eb;">{domain}</a></strong>. Every feature is still unlocked — design, booking, SEO, customer portal, AI content. Pop back in and keep building.</p>
  <div style="text-align:center;margin:28px 0;">
    <a href="{admin_url}" style="display:inline-block;padding:14px 28px;background:#2563eb;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Continue building →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">If you're ready to lock it in, lifetime plans are 50% off during the founding window. <a href="{pricing_url}" style="color:#2563eb;">See plans →</a></p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">LuperIQ — <a href="https://coderobot.net" style="color:#94a3b8;">coderobot.net</a></p>
</body></html>"##
            ),
        ),
        "day5" => (
            "2 days left on your LuperIQ trial",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#0f172a;margin:0 0 10px;">Two days left on your trial</h1>
  <p style="color:#475569;line-height:1.55;">Your site <strong><a href="{site_url}" style="color:#2563eb;">{domain}</a></strong> stays fully live for two more days. After that it flips to a private preview until you upgrade — nothing gets deleted, but only you will be able to see it.</p>
  <div style="text-align:center;margin:28px 0;">
    <a href="{pricing_url}" style="display:inline-block;padding:14px 28px;background:#2563eb;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Pick a plan →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">Lifetime plans are still 50% off. Lock it in once and never pay again.</p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">LuperIQ — <a href="{admin_url}" style="color:#94a3b8;">back to your admin</a></p>
</body></html>"##
            ),
        ),
        "day7" => (
            "Last day of your LuperIQ trial",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#b91c1c;margin:0 0 10px;">Today is the last day of your trial</h1>
  <p style="color:#475569;line-height:1.55;">Tomorrow, <strong>{domain}</strong> flips to a private preview until you pick a plan. Your content, settings, and customers are safe — nothing is deleted — but the public site will be paused.</p>
  <div style="text-align:center;margin:28px 0;">
    <a href="{pricing_url}" style="display:inline-block;padding:14px 28px;background:#b91c1c;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Keep my site live →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">Lifetime plans are 50% off during the founding window. One click and your site stays online forever.</p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">Not ready yet? That's fine. Come back anytime — <a href="{admin_url}" style="color:#94a3b8;">sign in</a> to export, upgrade, or resume.</p>
</body></html>"##
            ),
        ),
        "expired" => (
            "Your LuperIQ site is in private preview",
            format!(
                r##"<!DOCTYPE html><html><head><meta charset="utf-8"></head>
<body style="font-family:system-ui,-apple-system,sans-serif;max-width:580px;margin:0 auto;padding:24px;color:#1e293b;">
  <h1 style="color:#0f172a;margin:0 0 10px;">Your site is ready to go live again</h1>
  <p style="color:#475569;line-height:1.55;">The 7-day trial on <strong>{domain}</strong> has ended, so the public site is temporarily in private preview. Everything you built is still there — nothing is deleted — it just needs an active plan to stay visible.</p>
  <div style="text-align:center;margin:28px 0;">
    <a href="{pricing_url}" style="display:inline-block;padding:14px 28px;background:#2563eb;color:#fff;text-decoration:none;border-radius:8px;font-weight:700;">Publish my site →</a>
  </div>
  <p style="color:#64748b;font-size:0.92rem;line-height:1.55;">If you'd rather take your content with you, <a href="{admin_url}" style="color:#2563eb;">sign in</a> and use Export. Everything is yours.</p>
  <p style="color:#94a3b8;font-size:0.82rem;margin-top:24px;">LuperIQ — <a href="https://coderobot.net" style="color:#94a3b8;">coderobot.net</a></p>
</body></html>"##
            ),
        ),
        _ => ("LuperIQ", String::new()),
    }
}

/// Decide which reminder kind matches a given elapsed time. Returns None
/// when the trial is in a gap between reminder moments, or the site has
/// already converted / been deactivated. Priority order matters: "expired"
/// wins over any day-based match so a trial that somehow missed earlier
/// reminders still gets a final email.
fn reminder_kind_for(elapsed_secs: i64, expired: bool) -> Option<&'static str> {
    if expired {
        return Some("expired");
    }
    let day_of_trial = (elapsed_secs / 86400) + 1;
    match day_of_trial {
        3 => return Some("day3"),
        5 => return Some("day5"),
        7 => return Some("day7"),
        _ => {}
    }
    // hour4 nudge on the signup day for folks who wandered off — fires
    // once the trial has been running at least 4 hours, up until 24h.
    if (4 * 3600..86400).contains(&elapsed_secs) {
        return Some("hour4");
    }
    None
}


/// Pure decision: should `flash10` (the day-1 extra-10%-off email) fire for a
/// trial right now? TRUE only when the trial has been running inside the day-1
/// band (24h..28h since free_started_at) AND it has not been sent yet. The
/// caller separately enforces stage=="free" / not-deactivated / not-converted
/// (the scan's candidate filter already does). Elapsed-only — no relative
/// timing needed here.
fn should_send_flash10(elapsed_secs: i64, flash10_already_sent: bool) -> bool {
    !flash10_already_sent && (FLASH10_START_SECS..FLASH10_END_SECS).contains(&elapsed_secs)
}

/// Pure decision: should `flash10_remind` fire right now? This is RELATIVE to
/// when `flash10` actually went out — NOT elapsed-since-start. Fires only when:
///   - flash10 was sent (we have its sent_at), AND
///   - at least 30 min have passed since that send, AND
///   - we are still INSIDE the 4-hour flash window (now < sent_at + 4h), AND
///   - flash10_remind hasn't already been sent.
/// If the 4h window already closed we return false (skip the stale reminder).
fn should_send_flash10_remind(
    now: u64,
    flash10_sent_at: Option<u64>,
    flash10_remind_already_sent: bool,
) -> bool {
    if flash10_remind_already_sent {
        return false;
    }
    let Some(sent_at) = flash10_sent_at else {
        return false; // flash10 never sent → no reminder
    };
    let open_at = sent_at + FLASH10_REMIND_DELAY_SECS;
    let close_at = sent_at + FLASH_WINDOW_SECS;
    now >= open_at && now < close_at
}

async fn scan_and_send_trial_reminders(journal: &SharedJournal) {
    let now = now_ts();
    let candidates: Vec<(SiteTrial, String)> = {
        let j = journal.lock().await;
        j.latest_by_aggregate_type(AGG_SITE_TRIAL)
            .into_iter()
            .filter(|e| e.payload != b"__DELETED__")
            .filter_map(|e| serde_json::from_slice::<SiteTrial>(&e.payload).ok())
            .filter(|t| t.stage == "free")
            .filter(|t| t.deactivated_at.is_none())
            .filter(|t| t.converted_to.is_none())
            .filter(|t| !t.email.is_empty() && t.email.contains('@'))
            .filter(|t| t.domain.as_deref().map(|d| !d.is_empty()).unwrap_or(false))
            .map(|t| {
                let domain = t.domain.clone().unwrap_or_default();
                (t, domain)
            })
            .collect()
    };

    for (trial, domain) in candidates {
        let elapsed = now.saturating_sub(trial.free_started_at) as i64;
        let expired = now > trial.free_expires_at;
        let trial_id = trial.trial_id.clone();
        let email = trial.email.clone();

        // ── (1) flash10: day-1 extra-10%-off email. On send we ALSO stamp
        //        flash_offer_expires_at = now + 4h onto the SiteTrial — that
        //        timestamp is the single source the lifetime checkout reads to
        //        decide the discount window. Evaluated explicitly (outside the
        //        elapsed-only reminder_kind_for model). ─────────────────────
        let flash10_already = {
            let j = journal.lock().await;
            reminder_already_sent(&j, &trial_id, "flash10")
        };
        if should_send_flash10(elapsed, flash10_already) {
            let (subject, html) = flash_subject_body("flash10", &domain, FLASH_WINDOW_SECS);
            if !html.is_empty() {
                match send_email_internal(journal, &email, subject, &html, true).await {
                    Ok(_) => {
                        // Stamp the flash window onto the trial BEFORE recording
                        // the reminder, so the checkout can never see a recorded
                        // flash10 without a live window.
                        {
                            let mut j = journal.lock().await;
                            if let Some(mut t) =
                                trials::get_trial_by_id(&j, &trial_id)
                            {
                                t.flash_offer_expires_at = Some(now + FLASH_WINDOW_SECS);
                                let _ = trials::write_trial(&mut j, &t);
                            }
                        }
                        eprintln!(
                            "[trial-reminder] Sent 'flash10' to {} ({domain}); flash window +4h",
                            sanitize_email_log(&email)
                        );
                        record_reminder_sent(journal, &trial_id, "flash10", &email).await;
                    }
                    Err(e) => {
                        eprintln!(
                            "[trial-reminder] FAILED to send 'flash10' to {}: {e}",
                            sanitize_email_log(&email)
                        );
                    }
                }
            }
            // flash10 is at most one send per trial; continue to next trial so
            // we don't also fire a day reminder in the same tick.
            continue;
        }

        // ── (2) flash10_remind: 30 min..4h AFTER the flash10 send (relative to
        //        the recorded TrialReminderRecord.sent_at, NOT elapsed). ─────
        let (remind_sent, flash10_sent_at) = {
            let j = journal.lock().await;
            (
                reminder_already_sent(&j, &trial_id, "flash10_remind"),
                reminder_sent_at(&j, &trial_id, "flash10"),
            )
        };
        if should_send_flash10_remind(now, flash10_sent_at, remind_sent) {
            // Time remaining = the live flash window end minus now. Prefer the
            // trial's stamped expiry; fall back to (flash10 sent_at + 4h).
            let expires_at = trial
                .flash_offer_expires_at
                .or_else(|| flash10_sent_at.map(|s| s + FLASH_WINDOW_SECS))
                .unwrap_or(now);
            let secs_remaining = expires_at.saturating_sub(now);
            let (subject, html) =
                flash_subject_body("flash10_remind", &domain, secs_remaining);
            if !html.is_empty() {
                match send_email_internal(journal, &email, subject, &html, true).await {
                    Ok(_) => {
                        eprintln!(
                            "[trial-reminder] Sent 'flash10_remind' to {} ({domain}); ~{}s left",
                            sanitize_email_log(&email),
                            secs_remaining
                        );
                        record_reminder_sent(journal, &trial_id, "flash10_remind", &email)
                            .await;
                    }
                    Err(e) => {
                        eprintln!(
                            "[trial-reminder] FAILED to send 'flash10_remind' to {}: {e}",
                            sanitize_email_log(&email)
                        );
                    }
                }
            }
            continue;
        }

        // ── (3) existing elapsed-based reminders (hour4/day3/day5/day7/expired)
        //        — unchanged logic. ─────────────────────────────────────────
        let Some(kind) = reminder_kind_for(elapsed, expired) else {
            continue;
        };
        let already = {
            let j = journal.lock().await;
            reminder_already_sent(&j, &trial_id, kind)
        };
        if already {
            continue;
        }
        let (subject, html) = reminder_subject_body(kind, &domain);
        if html.is_empty() {
            continue;
        }
        match send_email_internal(journal, &email, subject, &html, true).await {
            Ok(_) => {
                eprintln!(
                    "[trial-reminder] Sent '{kind}' to {} ({domain})",
                    sanitize_email_log(&email)
                );
                record_reminder_sent(journal, &trial_id, kind, &email).await;
            }
            Err(e) => {
                eprintln!(
                    "[trial-reminder] FAILED to send '{kind}' to {}: {e}",
                    sanitize_email_log(&email)
                );
            }
        }
    }

    // ── SWEEP: actually enforce the free-tier downgrade ──────────────────
    // The reminder loop above only emails. Expired never-paid PEST trials
    // also need their 10 business modules turned OFF and the tenant
    // restarted so the drop takes effect (module_enabled is read only at
    // boot). This runs AFTER reminders so a tick still emails first.
    sweep_expired_pest_downgrades(journal, now).await;
}

/// Max tenants this single 10-minute tick will downgrade+restart. A guard so
/// one bad tick can never churn the whole fleet at once; the NEXT tick
/// continues with the rest (each is idempotently skipped once swept).
const DOWNGRADE_SWEEP_MAX_PER_TICK: usize = 5;

/// Marker kind recorded in the APEX journal once a tenant has been swept, so a
/// restart-loop / next tick never re-downgrades + re-restarts the same tenant.
/// We rely on THIS apex-side marker, NOT the tenant's own `Downgraded` stage,
/// because apex cannot read the tenant's WAL.
const DOWNGRADE_SWEPT_KIND: &str = "downgrade_swept";

/// Grandfather clause (Dave, 2026-06-16): every trial created before this
/// cutover is a pre-existing demo/example/friend-with-free-use and is PRESERVED
/// -- never swept. Only trials created at/after this epoch are formal/enforceable.
/// Raising this value can never retroactively endanger an older site.
const DOWNGRADE_ENFORCEMENT_EPOCH: u64 = 1781638286;

/// Resolve a tenant unit's events.wal + snapshot.bin absolute paths by reading
/// the SAME source the tenant process consumes: its `LUPERIQ_CMS_CONFIG` toml's
/// `[database] wal_path`/`snapshot_path`, resolved relative to the unit's
/// `WorkingDirectory`. Both come from `systemctl show <unit>`. Returns None if
/// anything is missing/unparseable (we then SKIP the sweep for that unit rather
/// than guess a path).
async fn tenant_journal_paths(unit: &str) -> Option<(String, String, Option<u16>)> {
    let out = tokio::process::Command::new("systemctl")
        .args(["show", unit, "-p", "Environment", "-p", "WorkingDirectory"])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut working_dir: Option<String> = None;
    let mut config_path: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("WorkingDirectory=") {
            if !rest.is_empty() {
                working_dir = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("Environment=") {
            // Environment is a space-separated list of KEY=VALUE pairs.
            for tok in rest.split_whitespace() {
                if let Some(v) = tok.strip_prefix("LUPERIQ_CMS_CONFIG=") {
                    config_path = Some(v.to_string());
                }
            }
        }
    }
    let config_path = config_path?;
    let toml_text = tokio::fs::read_to_string(&config_path).await.ok()?;
    let val: toml::Value = toml_text.parse().ok()?;
    let db = val.get("database")?;
    let wal = db.get("wal_path")?.as_str()?.to_string();
    let snap = db.get("snapshot_path")?.as_str()?.to_string();
    // Resolve relative paths against the unit's WorkingDirectory (the tenant
    // toml uses relative `data/events.wal`). main.rs runs with that cwd.
    let resolve = |pth: &str| -> Option<String> {
        let pb = std::path::Path::new(pth);
        if pb.is_absolute() {
            Some(pth.to_string())
        } else {
            let wd = working_dir.as_ref()?;
            Some(std::path::Path::new(wd).join(pth).to_string_lossy().into_owned())
        }
    };
    // Bind port (tenants bind 127.0.0.1:30xx). Used for a REAL listener check
    // after restart (is-active alone is the crashloop trap — a tenant can be
    // `active` yet never bind on MerkleRootMismatch). Optional: fall back to
    // is-active if [server] bind is absent/unparseable.
    let port: Option<u16> = val
        .get("server")
        .and_then(|sv| sv.get("bind"))
        .and_then(|b| b.as_str())
        .and_then(|b| b.rsplit(':').next())
        .and_then(|p| p.parse::<u16>().ok());
    Some((resolve(&wal)?, resolve(&snap)?, port))
}

/// Run `systemctl is-active <unit>` and return true iff it prints "active".
async fn unit_is_active(unit: &str) -> bool {
    tokio::process::Command::new("systemctl")
        .args(["is-active", unit])
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false)
}

/// True iff something is LISTENING on 127.0.0.1:<port>. This is the real
/// health signal for a tenant restart — `systemctl is-active` can report
/// `active` while the process crashloops on MerkleRootMismatch and never binds.
async fn port_is_listening(port: u16) -> bool {
    let out = tokio::process::Command::new("ss")
        .args(["-ltn"])
        .output()
        .await;
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let needle = format!(":{port} ");
            text.lines().any(|l| l.contains(&needle))
        }
        Err(_) => false,
    }
}

/// THE downgrade sweep. For each expired, never-paid PEST funnel SiteTrial that
/// has NOT already been swept (apex-side marker), stop the tenant, run the
/// `downgrade_tenant` bin against its journal, ALWAYS restart it, confirm it
/// came back active, and only then record the apex-side guard marker. Capped at
/// `DOWNGRADE_SWEEP_MAX_PER_TICK` per tick and serialized behind
/// `PROVISION_SEMAPHORE` so a sweep never overlaps a provision.
///
/// SAFETY INVARIANTS:
///   * never leaves a tenant stopped (start is unconditional, even on bin fail);
///   * never re-downgrades (the apex marker is the restart-loop guard);
///   * the bin itself re-checks the ever-paid guard inside the tenant journal;
///   * NO `LUPERIQ_WAL_KEY`/`LUPERIQ_SIGNING_KEY` is passed — the bin opens the
///     journal exactly as the tenant does (plaintext, demo key). See the bin.
async fn sweep_expired_pest_downgrades(journal: &SharedJournal, now: u64) {
    // Path to the downgrade_tenant bin — colocated with this binary
    // (target/release/luperiq-cms-apex -> target/release/downgrade_tenant). Fall
    // back to the canonical build tree if current_exe can't be resolved.
    let bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("downgrade_tenant")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            std::path::PathBuf::from(
                "/home/dave/luperiq-apex-db/target/release/downgrade_tenant",
            )
        });

    // Build the candidate set: expired, never-paid, pest, not-yet-swept.
    // `stage == "free"` + not-deactivated + not-converted is already the funnel
    // shape; we add the pest / expired / never-paid / not-swept gates here.
    let candidates: Vec<(String, String)> = {
        let j = journal.lock().await;
        j.latest_by_aggregate_type(AGG_SITE_TRIAL)
            .into_iter()
            .filter(|e| e.payload != b"__DELETED__")
            .filter_map(|e| serde_json::from_slice::<SiteTrial>(&e.payload).ok())
            .filter(|t| t.stage == "free")
            // Grandfather gate: only trials created at/after the enforcement
            // epoch are enforceable; everything older is a preserved demo/friend
            // site. This makes the blast radius zero by design.
            .filter(|t| t.created_at >= DOWNGRADE_ENFORCEMENT_EPOCH)
            .filter(|t| t.deactivated_at.is_none())
            .filter(|t| t.converted_to.is_none())
            // Pest only — exact match, same spelling the gate/signup use.
            .filter(|t| t.industry_slug == "pest-control")
            // Expired free window.
            .filter(|t| now > t.free_expires_at)
            // Never paid (funnel-side view; the bin re-checks in the tenant WAL).
            .filter(|t| t.paid_started_at.is_none())
            .filter(|t| t.domain.as_deref().map(|d| !d.is_empty()).unwrap_or(false))
            // Not already swept (apex-side restart-loop guard).
            .filter(|t| !reminder_already_sent(&j, &t.trial_id, DOWNGRADE_SWEPT_KIND))
            .map(|t| (t.trial_id.clone(), t.domain.clone().unwrap_or_default()))
            .collect()
    };

    if candidates.is_empty() {
        return;
    }

    let mut done = 0usize;
    for (trial_id, domain) in candidates {
        if done >= DOWNGRADE_SWEEP_MAX_PER_TICK {
            eprintln!(
                "[downgrade-sweep] tick cap ({DOWNGRADE_SWEEP_MAX_PER_TICK}) reached; remaining sites continue next tick"
            );
            break;
        }
        // Same domain->unit transform used by the kill-switch (mod.rs).
        let unit = format!("luperiq-{}.service", domain.replace('.', "-"));

        // Serialize behind the provision semaphore so a sweep never overlaps a
        // provision (port/journal contention). Held for the whole stop/run/start
        // of this ONE tenant, then released before the next.
        let _permit = match PROVISION_SEMAPHORE.acquire().await {
            Ok(p) => p,
            Err(_) => {
                eprintln!("[downgrade-sweep] semaphore closed; aborting sweep");
                return;
            }
        };

        let Some((wal_path, snapshot_path, bind_port)) = tenant_journal_paths(&unit).await else {
            eprintln!(
                "[downgrade-sweep] {unit}: could not resolve journal paths from systemd/cms.toml — SKIP (not swept)"
            );
            continue;
        };

        // 1) Stop the tenant (WAL must be free for the bin).
        let stop = tokio::process::Command::new("sudo")
            .args(["systemctl", "stop", &unit])
            .output()
            .await;
        if stop.as_ref().map(|o| !o.status.success()).unwrap_or(true) {
            eprintln!("[downgrade-sweep] {unit}: stop failed — SKIP (not swept)");
            // Best-effort restart in case it half-stopped; never leave it down.
            let _ = tokio::process::Command::new("sudo")
                .args(["systemctl", "start", &unit])
                .output()
                .await;
            continue;
        }

        // 2) Run the downgrade bin against the tenant journal. We pass NO
        //    LUPERIQ_WAL_KEY / LUPERIQ_SIGNING_KEY: the bin opens the journal
        //    exactly as the tenant does (plaintext, demo key). Inherit env but
        //    explicitly strip those two so an apex-side value can never leak in.
        let bin_result = tokio::process::Command::new(&bin)
            .arg(&wal_path)
            .arg(&snapshot_path)
            .env_remove("LUPERIQ_WAL_KEY")
            .env_remove("LUPERIQ_SIGNING_KEY")
            .output()
            .await;
        let bin_ok = match &bin_result {
            Ok(o) => {
                if !o.stdout.is_empty() {
                    eprintln!(
                        "[downgrade-sweep] {unit}: {}",
                        String::from_utf8_lossy(&o.stdout).trim()
                    );
                }
                if !o.status.success() && !o.stderr.is_empty() {
                    eprintln!(
                        "[downgrade-sweep] {unit}: bin stderr: {}",
                        String::from_utf8_lossy(&o.stderr).trim()
                    );
                }
                o.status.success()
            }
            Err(e) => {
                eprintln!("[downgrade-sweep] {unit}: failed to exec downgrade bin: {e}");
                false
            }
        };

        // 3a) Catch the snapshot up to the WAL tail OURSELVES before start.
        //     The bin just appended while the tenant was stopped, so snapshot.bin
        //     is now behind events.wal. Most tenants have an ExecStartPre
        //     refresh_snapshot drop-in, but ~13 do NOT — relying on it would
        //     crashloop those on MerkleRootMismatch. refresh_snapshot is lossless
        //     and idempotent (harmless if the drop-in also runs it), so we run it
        //     unconditionally — this mirrors the documented manual deploy dance.
        let refresh_bin = bin
            .parent()
            .map(|d| d.join("refresh_snapshot"))
            .filter(|p| p.exists())
            .unwrap_or_else(|| {
                std::path::PathBuf::from(
                    "/home/dave/luperiq-apex-db/target/release/refresh_snapshot",
                )
            });
        let _ = tokio::process::Command::new(&refresh_bin)
            .arg(&wal_path)
            .env_remove("LUPERIQ_WAL_KEY")
            .env_remove("LUPERIQ_SIGNING_KEY")
            .output()
            .await;

        // 3b) ALWAYS restart — even if the bin failed. Never leave a tenant down.
        let _ = tokio::process::Command::new("sudo")
            .args(["systemctl", "start", &unit])
            .output()
            .await;

        // 4) Confirm it actually came BACK UP. Prefer a real listener check on
        //    the tenant's bind port (is-active is the crashloop trap — a tenant
        //    can be `active` yet never bind on MerkleRootMismatch). Fall back to
        //    is-active only when the port is unknown. Poll ~15s (replay of a
        //    tenant WAL is well under apex's ~25s).
        let mut healthy = false;
        for _ in 0..15 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let up = match bind_port {
                Some(port) => port_is_listening(port).await,
                None => unit_is_active(&unit).await,
            };
            if up {
                healthy = true;
                break;
            }
        }
        if !healthy {
            eprintln!(
                "[downgrade-sweep] {unit}: did NOT come back up (no listener on bind port) after restart — NOT marking swept (will retry next tick)"
            );
            done += 1;
            continue;
        }

        // 5) Only on confirmed success (bin succeeded AND tenant active) write
        //    the apex-side guard marker. On bin-fail-but-active we deliberately
        //    do NOT mark, so the next tick retries the downgrade.
        if bin_ok {
            record_reminder_sent(journal, &trial_id, DOWNGRADE_SWEPT_KIND, "").await;
            eprintln!("[downgrade-sweep] {unit}: downgraded + restarted OK; marked swept");
        } else {
            eprintln!(
                "[downgrade-sweep] {unit}: tenant is back up but downgrade bin failed — NOT marked swept (retry next tick)"
            );
        }
        done += 1;
    }
}

fn sanitize_email_log(email: &str) -> String {
    match email.find('@') {
        Some(idx) if idx > 2 => format!("{}…{}", &email[..2], &email[idx..]),
        _ => "[email]".to_string(),
    }
}

/// Start the background trial-reminder scanner. Safe to call once at
/// Central boot time — only one scanner is needed per process.
pub fn spawn_trial_reminders(journal: SharedJournal) {
    tokio::spawn(async move {
        // Small warm-up so the rest of the service can finish starting
        // before we begin hitting SMTP.
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(REMINDER_SCAN_INTERVAL_SECS));
        loop {
            interval.tick().await;
            scan_and_send_trial_reminders(&journal).await;
        }
    });
}

// ── CmsModule implementation ────────────────────────────────────────

pub struct SalesFunnelModule;

impl CmsModule for SalesFunnelModule {
    fn slug(&self) -> &str {
        "sales-funnel"
    }

    fn name(&self) -> &str {
        "Sales Funnel"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    fn description(&self) -> &str {
        "Sales pipeline with trial flow, lead tracking, demo banner, and industry CTA modal."
    }

    fn category(&self) -> &str {
        "Commerce"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(sales_funnel_router(ctx.journal.clone()))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![
            AdminView {
                id: "sales-pipeline".into(),
                label: "Sales Pipeline".into(),
                section: "Commerce".into(),
            },
            AdminView {
                id: "trial-management".into(),
                label: "Trial Management".into(),
                section: "Commerce".into(),
            },
        ]
    }

    fn admin_js(&self) -> Option<String> {
        Some(ADMIN_JS.to_string())
    }
}
