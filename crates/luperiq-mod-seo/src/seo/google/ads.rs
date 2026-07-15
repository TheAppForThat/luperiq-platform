//! Google Ads handlers — 2 endpoints with special optional-service error handling.

use axum::extract::State;
use axum::response::Json;
use serde::Deserialize;

use super::oauth::{make_client, GoogleState};
use super::{
    check_circuit_breaker, load_google_config, reset_circuit_breaker, save_google_config,
    trip_circuit_breaker, GResult, GoogleError,
};

// ── Known non-retriable Ads error reasons ─────────────────────────────

const ADS_SOFT_ERRORS: &[&str] = &[
    "developer_token_missing",
    "scope_missing",
    "forbidden",
    "ads_not_enabled",
    "permission_denied",
];

fn is_soft_ads_error(body: &str) -> Option<&'static str> {
    let lower = body.to_lowercase();
    for reason in ADS_SOFT_ERRORS {
        if lower.contains(reason) {
            return Some(reason);
        }
    }
    None
}

// ── 1. GET /ads/customers ─────────────────────────────────────────────

pub async fn ads_customers(State(state): State<GoogleState>) -> Json<GResult> {
    let client = match make_client(&state) {
        Ok(c) => c,
        Err(e) => {
            // Even a client build failure is non-fatal for Ads — it's optional
            return Json(GResult {
                ok: true,
                message: "Google Ads not configured".into(),
                data: Some(serde_json::json!({
                    "customers": [],
                    "optional": true,
                    "reason": "not_configured",
                    "detail": e.to_string(),
                })),
            });
        }
    };

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    // Circuit breaker open → graceful degradation (Ads is optional)
    if let Err(_) = check_circuit_breaker(&config) {
        return Json(GResult {
            ok: true,
            message: "Google Ads temporarily unavailable".into(),
            data: Some(serde_json::json!({
                "customers": [],
                "optional": true,
                "reason": "google_ads_temporarily_unavailable",
            })),
        });
    }

    match client.get("/oauth/google/ads/customers", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: true,
                message: "Customers loaded".into(),
                data: Some(data),
            })
        }
        // 403 or known soft errors → optional graceful response, do NOT trip CB
        Err(GoogleError::HttpError { code: 403, body }) => {
            let reason = is_soft_ads_error(&body).unwrap_or("forbidden");
            Json(GResult {
                ok: true,
                message: "Google Ads not available".into(),
                data: Some(serde_json::json!({
                    "customers": [],
                    "optional": true,
                    "reason": reason,
                })),
            })
        }
        // Other 4xx where body matches a soft error → also graceful
        Err(GoogleError::HttpError { code, body }) if code < 500 => {
            if let Some(reason) = is_soft_ads_error(&body) {
                Json(GResult {
                    ok: true,
                    message: "Google Ads not available".into(),
                    data: Some(serde_json::json!({
                        "customers": [],
                        "optional": true,
                        "reason": reason,
                    })),
                })
            } else {
                Json(GResult {
                    ok: false,
                    message: format!("Ads API error: HTTP {code}"),
                    data: Some(serde_json::json!({ "body": body })),
                })
            }
        }
        // 5xx → trip circuit breaker (but still return graceful optional response)
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: true,
                message: "Google Ads temporarily unavailable".into(),
                data: Some(serde_json::json!({
                    "customers": [],
                    "optional": true,
                    "reason": "google_ads_temporarily_unavailable",
                    "detail": format!("HTTP {code}: {body}"),
                })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── 2. POST /ads/select ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AdsSelectPayload {
    #[serde(default)]
    pub customer_id: String,
    #[serde(default)]
    pub customer_display_name: String,
}

pub async fn ads_select(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<AdsSelectPayload>,
) -> Json<GResult> {
    // Forward selection to api.luperiq.com
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

    let remote_payload = serde_json::json!({
        "customer_id": payload.customer_id,
        "customer_display_name": payload.customer_display_name,
    });

    if let Err(e) = client
        .post("/oauth/google/ads/select", &remote_payload)
        .await
    {
        eprintln!("Warning: failed to sync Ads selection to central: {e}");
    }

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    config.ads_customer_id = payload.customer_id;
    config.ads_customer_display_name = payload.customer_display_name;

    match save_google_config(&mut j, &config) {
        Ok(_) => Json(GResult {
            ok: true,
            message: "Google Ads customer selected".into(),
            data: Some(serde_json::json!({
                "ads_customer_id": config.ads_customer_id,
                "ads_customer_display_name": config.ads_customer_display_name,
            })),
        }),
        Err(e) => Json(GResult {
            ok: false,
            message: format!("Save failed: {e}"),
            data: None,
        }),
    }
}
