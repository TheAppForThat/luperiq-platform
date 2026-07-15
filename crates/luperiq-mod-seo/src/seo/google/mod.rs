//! Google API integration — HTTP client wrapper for api.luperiq.com proxy.

pub mod admin_js;
pub mod ads;
pub mod cache;
pub mod ga4;
pub mod gsc;
pub mod insights;
pub mod oauth;

use serde::{Deserialize, Serialize};

// ── Error type ──────────────────────────────────────────────────────

#[derive(Debug)]
pub enum GoogleError {
    CircuitBreakerOpen,
    HttpError { code: u16, body: String },
    NetworkError(String),
    NotConfigured(String),
    ParseError(String),
}

impl std::fmt::Display for GoogleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CircuitBreakerOpen => write!(f, "Google API temporarily unavailable"),
            Self::HttpError { code, body } => write!(f, "HTTP {code}: {body}"),
            Self::NetworkError(e) => write!(f, "Network error: {e}"),
            Self::NotConfigured(msg) => write!(f, "Not configured: {msg}"),
            Self::ParseError(e) => write!(f, "Parse error: {e}"),
        }
    }
}

// ── Config aggregate ────────────────────────────────────────────────

pub const AGG_GOOGLE_CONFIG: &str = "GoogleConfig";
pub const GOOGLE_CONFIG_ID: &str = "global";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoogleConfig {
    #[serde(default)]
    pub oauth_mode: String,
    #[serde(default)]
    pub authenticated: bool,
    #[serde(default)]
    pub direct_client_id: String,
    #[serde(default)]
    pub direct_client_secret: String,
    #[serde(default)]
    pub direct_refresh_token: String,
    #[serde(default)]
    pub direct_access_token: String,
    #[serde(default)]
    pub direct_token_expires_at: i64,
    #[serde(default)]
    pub ga4_property_id: String,
    #[serde(default)]
    pub ga4_property_display_name: String,
    #[serde(default)]
    pub ga4_account_display_name: String,
    #[serde(default)]
    pub ga4_measurement_id: String,
    #[serde(default)]
    pub gsc_site_url: String,
    #[serde(default)]
    pub gsc_permission_level: String,
    #[serde(default)]
    pub gsc_verification_token: String,
    #[serde(default)]
    pub ads_customer_id: String,
    #[serde(default)]
    pub ads_customer_display_name: String,
    #[serde(default)]
    pub cb_failures: u32,
    #[serde(default)]
    pub cb_down_until: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthStatus {
    pub authenticated: bool,
    #[serde(default)]
    pub ga4_property_id: String,
    #[serde(default)]
    pub gsc_site_url: String,
    #[serde(default)]
    pub ads_customer_id: String,
    #[serde(default)]
    pub callback_uri: String,
}

// ── GoogleClient ────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GoogleClient {
    http: reqwest::Client,
    api_base: String,
    license_key: String,
}

impl GoogleClient {
    pub fn new(api_base: &str, license_key: &str) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build reqwest TLS client"),
            api_base: api_base.trim_end_matches('/').to_string(),
            license_key: license_key.to_string(),
        }
    }

    pub async fn get(
        &self,
        path: &str,
        extra_params: &[(&str, &str)],
    ) -> Result<serde_json::Value, GoogleError> {
        let resp = self
            .http
            .get(format!("{}{}", self.api_base, path))
            .header("User-Agent", "LuperIQ-CMS/0.7.6")
            .header("X-License-Key", &self.license_key)
            .query(extra_params)
            .send()
            .await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(GoogleError::HttpError { code: status, body });
        }
        serde_json::from_str(&body).map_err(|e| GoogleError::ParseError(format!("{e}: {body}")))
    }

    pub async fn post(
        &self,
        path: &str,
        payload: &serde_json::Value,
    ) -> Result<serde_json::Value, GoogleError> {
        let url = format!("{}{}", self.api_base, path);
        let resp = self
            .http
            .post(&url)
            .header("User-Agent", "LuperIQ-CMS/0.7.6")
            .header("X-License-Key", &self.license_key)
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
            .map_err(|e| GoogleError::NetworkError(e.to_string()))?;
        let status = resp.status().as_u16();
        let resp_body = resp.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(GoogleError::HttpError {
                code: status,
                body: resp_body,
            });
        }
        serde_json::from_str(&resp_body)
            .map_err(|e| GoogleError::ParseError(format!("{e}: {resp_body}")))
    }
}

// ── Shared response type ─────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct GResult {
    pub(crate) ok: bool,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) data: Option<serde_json::Value>,
}

// ── Helpers ─────────────────────────────────────────────────────────

pub fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn load_google_config(journal: &luperiq_forge::ForgeJournal) -> GoogleConfig {
    match journal.get_latest(AGG_GOOGLE_CONFIG, GOOGLE_CONFIG_ID) {
        Some(event) if event.payload != crate::seo::TOMBSTONE => {
            serde_json::from_slice(&event.payload).unwrap_or_default()
        }
        _ => GoogleConfig::default(),
    }
}

pub fn save_google_config(
    journal: &mut luperiq_forge::ForgeJournal,
    config: &GoogleConfig,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(config).map_err(|e| e.to_string())?;
    let event = luperiq_forge::ApexEvent::new(AGG_GOOGLE_CONFIG, GOOGLE_CONFIG_ID, bytes);
    journal.append(event).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn check_circuit_breaker(config: &GoogleConfig) -> Result<(), GoogleError> {
    if config.cb_failures >= 2 && config.cb_down_until > now_epoch() {
        return Err(GoogleError::CircuitBreakerOpen);
    }
    Ok(())
}

pub fn trip_circuit_breaker(config: &mut GoogleConfig) {
    config.cb_failures += 1;
    if config.cb_failures >= 2 {
        config.cb_down_until = now_epoch() + 600;
    }
}

pub fn reset_circuit_breaker(config: &mut GoogleConfig) {
    config.cb_failures = 0;
    config.cb_down_until = 0;
}

pub fn max_lookback_days(nexus_role: &str) -> i64 {
    match nexus_role {
        "central" | "professional" | "enterprise" | "unlimited" => 540,
        _ => 14,
    }
}

pub fn clamp_start_date(start_date: &str, nexus_role: &str) -> String {
    let max_days = max_lookback_days(nexus_role);
    let earliest = chrono::Utc::now() - chrono::Duration::try_days(max_days).unwrap_or_default();
    let earliest_str = earliest.format("%Y-%m-%d").to_string();
    if start_date < earliest_str.as_str() {
        earliest_str
    } else {
        start_date.to_string()
    }
}
