//! Google OAuth handlers — proxy and direct flows.

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Json, Redirect, Response};
use axum::routing::{get, post};
use serde::Deserialize;

use luperiq_module_api::{NexusNetworkConfig, SharedJournal};

use super::{
    check_circuit_breaker, load_google_config, reset_circuit_breaker, save_google_config,
    trip_circuit_breaker, GoogleClient, GoogleConfig, GoogleError,
};

// ── Shared state for all Google handlers ─────────────────────────────

#[derive(Clone)]
pub struct GoogleState {
    pub journal: SharedJournal,
    pub nexus_config: Option<NexusNetworkConfig>,
}

// ── Response type ────────────────────────────────────────────────────

use super::GResult;

// ── Helper: build GoogleClient from state ────────────────────────────

// For the central role, the api_base should be "https://api.luperiq.com" not the central_url.
// Let's handle both: if role is "central", use api.luperiq.com. If client, use central_url.
pub fn get_api_base(nexus: &Option<NexusNetworkConfig>) -> String {
    if let Some(nc) = nexus {
        if nc.role.as_deref() == Some("central") {
            return "https://api.luperiq.com".to_string();
        }
        if let Some(url) = &nc.central_url {
            return url.clone();
        }
    }
    "https://api.luperiq.com".to_string()
}

pub fn get_license_key(nexus: &Option<NexusNetworkConfig>) -> Result<String, GoogleError> {
    let nc = nexus
        .as_ref()
        .ok_or_else(|| GoogleError::NotConfigured("No nexus config".into()))?;
    let key = nc.license_key.as_deref().unwrap_or("");
    if key.is_empty() {
        // For central role, use a placeholder
        if nc.role.as_deref() == Some("central") {
            return Ok("central".to_string());
        }
        return Err(GoogleError::NotConfigured("No license key".into()));
    }
    Ok(key.to_string())
}

pub(crate) fn make_client(state: &GoogleState) -> Result<GoogleClient, GoogleError> {
    let base = get_api_base(&state.nexus_config);
    let key = get_license_key(&state.nexus_config)?;
    Ok(GoogleClient::new(&base, &key))
}

// ── Scopes ───────────────────────────────────────────────────────────

const DEFAULT_SCOPES: &str = "https://www.googleapis.com/auth/analytics.readonly https://www.googleapis.com/auth/analytics.edit https://www.googleapis.com/auth/webmasters.readonly https://www.googleapis.com/auth/adwords https://www.googleapis.com/auth/siteverification";

// ── 1. GET /api/modules/seo/google/status ────────────────────────────

pub async fn get_oauth_status(State(state): State<GoogleState>) -> Json<GResult> {
    let client = match make_client(&state) {
        Ok(c) => c,
        Err(e) => {
            return Json(GResult {
                ok: false,
                message: e.to_string(),
                data: None,
            })
        }
    };

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    if let Err(e) = check_circuit_breaker(&config) {
        return Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        });
    }

    match client.get("/oauth/google/status", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            // Only sync+save config when remote confirms authenticated.
            // When remote says false (token expired, network hiccup), don't
            // overwrite local authenticated state — just reset circuit breaker.
            let remote_auth = data
                .get("authenticated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if remote_auth {
                config.authenticated = true;
                if let Some(v) = data.get("ga4_property_id").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        config.ga4_property_id = v.to_string();
                    }
                }
                if let Some(v) = data.get("gsc_site_url").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        config.gsc_site_url = v.to_string();
                    }
                }
                if let Some(v) = data.get("ads_customer_id").and_then(|v| v.as_str()) {
                    if !v.is_empty() {
                        config.ads_customer_id = v.to_string();
                    }
                }
                let _ = save_google_config(&mut j, &config);
            }
            Json(GResult {
                ok: true,
                message: if config.authenticated {
                    "Connected".into()
                } else {
                    "Not connected".into()
                },
                data: Some(serde_json::json!({
                    "authenticated": config.authenticated,
                    "ga4_property_id": config.ga4_property_id,
                    "gsc_site_url": config.gsc_site_url,
                    "ads_customer_id": config.ads_customer_id,
                    "oauth_mode": config.oauth_mode,
                })),
            })
        }
        Err(GoogleError::HttpError { code, .. }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: "Google API temporarily unavailable".into(),
                data: None,
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── 2. GET /api/modules/seo/google/auth-url ──────────────────────────

#[derive(Deserialize)]
pub struct AuthUrlQuery {
    #[serde(default)]
    scopes: Option<String>,
    #[serde(default)]
    redirect_uri: Option<String>,
}

pub async fn get_auth_url(
    State(state): State<GoogleState>,
    Query(q): Query<AuthUrlQuery>,
) -> Json<GResult> {
    let client = match make_client(&state) {
        Ok(c) => c,
        Err(e) => {
            return Json(GResult {
                ok: false,
                message: e.to_string(),
                data: None,
            })
        }
    };

    let scopes = q.scopes.as_deref().unwrap_or(DEFAULT_SCOPES);
    let base = get_api_base(&state.nexus_config);
    // redirect_uri is required by api.luperiq.com — default to the admin panel
    let default_redirect = format!("{}/admin?google_status=connected", base);
    let redirect = q.redirect_uri.as_deref().unwrap_or(&default_redirect);

    match client
        .get(
            "/oauth/google/auth-url",
            &[("scopes", scopes), ("redirect_uri", redirect)],
        )
        .await
    {
        Ok(data) => {
            let url = data
                .get("auth_url")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if url.is_empty() {
                return Json(GResult {
                    ok: false,
                    message: "Google auth URL was missing from central response".into(),
                    data: Some(data),
                });
            }

            Json(GResult {
                ok: true,
                message: "Auth URL generated".into(),
                data: Some(serde_json::json!({ "url": url })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── 3. GET /api/modules/seo/google/config ────────────────────────────

pub async fn get_config(State(state): State<GoogleState>) -> Json<GResult> {
    let j = state.journal.lock().await;
    let config = load_google_config(&j);
    // Don't expose secrets in response
    Json(GResult {
        ok: true,
        message: "Config loaded".into(),
        data: Some(serde_json::json!({
            "oauth_mode": config.oauth_mode,
            "authenticated": config.authenticated,
            "ga4_property_id": config.ga4_property_id,
            "ga4_property_display_name": config.ga4_property_display_name,
            "ga4_measurement_id": config.ga4_measurement_id,
            "gsc_site_url": config.gsc_site_url,
            "gsc_permission_level": config.gsc_permission_level,
            "ads_customer_id": config.ads_customer_id,
            "ads_customer_display_name": config.ads_customer_display_name,
            "has_direct_credentials": !config.direct_client_id.is_empty(),
        })),
    })
}

// ── 4. PUT /api/modules/seo/google/config ────────────────────────────

#[derive(Deserialize)]
pub struct UpdateConfigPayload {
    #[serde(default)]
    pub direct_client_id: Option<String>,
    #[serde(default)]
    pub direct_client_secret: Option<String>,
}

pub async fn update_config(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<UpdateConfigPayload>,
) -> Json<GResult> {
    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    if let Some(id) = payload.direct_client_id {
        config.direct_client_id = id;
    }
    if let Some(secret) = payload.direct_client_secret {
        config.direct_client_secret = secret;
    }

    match save_google_config(&mut j, &config) {
        Ok(_) => Json(GResult {
            ok: true,
            message: "Config updated".into(),
            data: None,
        }),
        Err(e) => Json(GResult {
            ok: false,
            message: format!("Save failed: {e}"),
            data: None,
        }),
    }
}

// ── 5. POST /api/modules/seo/google/disconnect ───────────────────────

pub async fn disconnect(State(state): State<GoogleState>) -> Json<GResult> {
    let mut j = state.journal.lock().await;
    let config = GoogleConfig::default(); // Reset everything
    match save_google_config(&mut j, &config) {
        Ok(_) => Json(GResult {
            ok: true,
            message: "Disconnected from Google".into(),
            data: None,
        }),
        Err(e) => Json(GResult {
            ok: false,
            message: format!("Failed: {e}"),
            data: None,
        }),
    }
}

// ── 6. GET /api/modules/seo/google/oauth/callback ────────────────────

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    #[serde(default)]
    code: String,
    #[serde(default)]
    error: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<GoogleState>,
    Query(q): Query<OAuthCallbackQuery>,
) -> Response {
    if let Some(err) = q.error {
        return Redirect::to(&format!(
            "/admin?google_error={}",
            urlencoding::encode(&err)
        ))
        .into_response();
    }
    if q.code.is_empty() {
        return Redirect::to("/admin?google_error=no_code").into_response();
    }

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    if config.direct_client_id.is_empty() || config.direct_client_secret.is_empty() {
        return Redirect::to("/admin?google_error=no_direct_credentials").into_response();
    }

    // Exchange code for tokens
    let http = reqwest::Client::new();
    let resp = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code", q.code.as_str()),
            ("client_id", config.direct_client_id.as_str()),
            ("client_secret", config.direct_client_secret.as_str()),
            (
                "redirect_uri",
                &format!(
                    "{}/api/modules/seo/google/oauth/callback",
                    get_api_base(&state.nexus_config)
                ),
            ),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            if let Ok(body) = r.json::<serde_json::Value>().await {
                if let Some(rt) = body.get("refresh_token").and_then(|v| v.as_str()) {
                    config.direct_refresh_token = rt.to_string();
                }
                if let Some(at) = body.get("access_token").and_then(|v| v.as_str()) {
                    config.direct_access_token = at.to_string();
                }
                if let Some(exp) = body.get("expires_in").and_then(|v| v.as_i64()) {
                    config.direct_token_expires_at = super::now_epoch() + exp;
                }
                config.oauth_mode = "direct".to_string();
                config.authenticated = true;
                let _ = save_google_config(&mut j, &config);
                Redirect::to("/admin?google_status=connected").into_response()
            } else {
                Redirect::to("/admin?google_error=parse_failed").into_response()
            }
        }
        Ok(r) => {
            let code = r.status().as_u16();
            let body = r.text().await.unwrap_or_default();
            eprintln!("Google token exchange failed: HTTP {code}: {body}");
            Redirect::to(&format!("/admin?google_error=token_exchange_failed_{code}"))
                .into_response()
        }
        Err(e) => {
            eprintln!("Google token exchange network error: {e}");
            Redirect::to("/admin?google_error=network_error").into_response()
        }
    }
}

// ── Router (used by seo/mod.rs) ──────────────────────────────────────

pub fn google_router(state: GoogleState) -> axum::Router {
    axum::Router::new()
        // OAuth
        .route("/api/modules/seo/google/status", get(get_oauth_status))
        .route("/api/modules/seo/google/auth-url", get(get_auth_url))
        .route(
            "/api/modules/seo/google/config",
            get(get_config).put(update_config),
        )
        .route("/api/modules/seo/google/disconnect", post(disconnect))
        .route(
            "/api/modules/seo/google/oauth/callback",
            get(oauth_callback),
        )
        // GA4
        .route(
            "/api/modules/seo/google/ga4/properties",
            get(super::ga4::ga4_properties),
        )
        .route(
            "/api/modules/seo/google/ga4/select",
            post(super::ga4::ga4_select),
        )
        .route(
            "/api/modules/seo/google/ga4/accounts",
            get(super::ga4::ga4_accounts),
        )
        .route(
            "/api/modules/seo/google/ga4/create",
            post(super::ga4::ga4_create),
        )
        .route(
            "/api/modules/seo/google/ga4/traffic",
            get(super::ga4::ga4_traffic),
        )
        .route(
            "/api/modules/seo/google/ga4/timeseries",
            get(super::ga4::ga4_timeseries),
        )
        .route(
            "/api/modules/seo/google/ga4/sources",
            get(super::ga4::ga4_sources),
        )
        .route(
            "/api/modules/seo/google/ga4/pages",
            get(super::ga4::ga4_pages),
        )
        .route(
            "/api/modules/seo/google/ga4/status",
            get(super::ga4::ga4_status),
        )
        // GSC
        .route(
            "/api/modules/seo/google/gsc/sites",
            get(super::gsc::gsc_sites),
        )
        .route(
            "/api/modules/seo/google/gsc/select",
            post(super::gsc::gsc_select),
        )
        .route(
            "/api/modules/seo/google/gsc/verify",
            post(super::gsc::gsc_verify),
        )
        .route(
            "/api/modules/seo/google/gsc/queries",
            get(super::gsc::gsc_queries),
        )
        .route(
            "/api/modules/seo/google/gsc/opportunities",
            get(super::gsc::gsc_opportunities),
        )
        .route(
            "/api/modules/seo/google/gsc/query-timeseries",
            get(super::gsc::gsc_query_timeseries),
        )
        .route(
            "/api/modules/seo/google/gsc/query-pages",
            get(super::gsc::gsc_query_pages),
        )
        .route(
            "/api/modules/seo/google/gsc/pages",
            get(super::gsc::gsc_pages),
        )
        .route(
            "/api/modules/seo/google/gsc/page-timeseries",
            get(super::gsc::gsc_page_timeseries),
        )
        .route(
            "/api/modules/seo/google/gsc/breakdown",
            get(super::gsc::gsc_breakdown),
        )
        .route(
            "/api/modules/seo/google/gsc/delta",
            get(super::gsc::gsc_delta),
        )
        // Ads
        .route(
            "/api/modules/seo/google/ads/customers",
            get(super::ads::ads_customers),
        )
        .route(
            "/api/modules/seo/google/ads/select",
            post(super::ads::ads_select),
        )
        .with_state(state)
}
