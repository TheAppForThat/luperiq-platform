//! GA4 handlers — 9 endpoints that proxy through api.luperiq.com.

use axum::extract::{Query, State};
use axum::response::Json;

use serde::Deserialize;

use super::cache::{ttl_for_prefix, GoogleCacheManager};
use super::oauth::{make_client, GoogleState};
use super::{
    check_circuit_breaker, clamp_start_date, load_google_config, reset_circuit_breaker,
    save_google_config, trip_circuit_breaker, GResult, GoogleError,
};

// ── Query param structs ───────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct DateRangeQuery {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub row_limit: Option<u32>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct ForceRefreshQuery {
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

// ── 1. GET /ga4/properties ────────────────────────────────────────────

pub async fn ga4_properties(
    State(state): State<GoogleState>,
    Query(q): Query<ForceRefreshQuery>,
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

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    if let Err(e) = check_circuit_breaker(&config) {
        return Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        });
    }

    let cache_key = GoogleCacheManager::make_key(&["ga4", "properties"]);
    let force = q.force_refresh.unwrap_or(false);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Properties loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    match client.get("/oauth/google/ga4/properties", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("ga4:properties");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Properties loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GA4 API error: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── 2. POST /ga4/select ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct Ga4SelectPayload {
    #[serde(default)]
    pub property_id: String,
    #[serde(default)]
    pub property_display_name: String,
    #[serde(default)]
    pub account_display_name: String,
    #[serde(default)]
    pub measurement_id: String,
}

pub async fn ga4_select(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<Ga4SelectPayload>,
) -> Json<GResult> {
    // Forward selection to api.luperiq.com so data queries work
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
        "property_id": payload.property_id,
        "property_display_name": payload.property_display_name,
        "account_display_name": payload.account_display_name,
        "measurement_id": payload.measurement_id,
    });

    if let Err(e) = client
        .post("/oauth/google/ga4/select", &remote_payload)
        .await
    {
        eprintln!("Warning: failed to sync GA4 selection to central: {e}");
    }

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    config.ga4_property_id = payload.property_id;
    config.ga4_property_display_name = payload.property_display_name;
    config.ga4_account_display_name = payload.account_display_name;
    config.ga4_measurement_id = payload.measurement_id;

    match save_google_config(&mut j, &config) {
        Ok(_) => Json(GResult {
            ok: true,
            message: "GA4 property selected".into(),
            data: Some(serde_json::json!({
                "ga4_property_id": config.ga4_property_id,
                "ga4_measurement_id": config.ga4_measurement_id,
            })),
        }),
        Err(e) => Json(GResult {
            ok: false,
            message: format!("Save failed: {e}"),
            data: None,
        }),
    }
}

// ── 3. GET /ga4/accounts ──────────────────────────────────────────────

pub async fn ga4_accounts(State(state): State<GoogleState>) -> Json<GResult> {
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

    match client.get("/oauth/google/ga4/accounts", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: true,
                message: "Accounts loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GA4 API error: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── 4. POST /ga4/create ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct Ga4CreatePayload {
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub site_url: String,
}

pub async fn ga4_create(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<Ga4CreatePayload>,
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

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    if let Err(e) = check_circuit_breaker(&config) {
        return Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        });
    }

    // Step 1: create property
    let prop_payload = serde_json::json!({
        "account_id": payload.account_id,
        "display_name": payload.display_name,
    });

    let prop_resp = match client
        .post("/oauth/google/ga4/create-property", &prop_payload)
        .await
    {
        Ok(v) => v,
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            return Json(GResult {
                ok: false,
                message: format!("Create property failed: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            });
        }
        Err(e) => {
            return Json(GResult {
                ok: false,
                message: format!("Create property failed: {e}"),
                data: None,
            })
        }
    };

    let property_id = prop_resp
        .get("property_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if property_id.is_empty() {
        return Json(GResult {
            ok: false,
            message: "Create property returned no property_id".into(),
            data: Some(prop_resp),
        });
    }

    // Step 2: create stream
    let stream_payload = serde_json::json!({
        "property_id": property_id,
        "site_url": payload.site_url,
    });

    let stream_resp = match client
        .post("/oauth/google/ga4/create-stream", &stream_payload)
        .await
    {
        Ok(v) => v,
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            return Json(GResult {
                ok: false,
                message: format!("Create stream failed: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            });
        }
        Err(e) => {
            return Json(GResult {
                ok: false,
                message: format!("Create stream failed: {e}"),
                data: None,
            })
        }
    };

    let measurement_id = stream_resp
        .get("measurement_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Save results
    config.ga4_property_id = property_id.clone();
    config.ga4_property_display_name = payload.display_name.clone();
    config.ga4_measurement_id = measurement_id.clone();
    reset_circuit_breaker(&mut config);
    let _ = save_google_config(&mut j, &config);

    Json(GResult {
        ok: true,
        message: "GA4 property and stream created".into(),
        data: Some(serde_json::json!({
            "property_id": property_id,
            "measurement_id": measurement_id,
            "property": prop_resp,
            "stream": stream_resp,
        })),
    })
}

// ── 5. GET /ga4/traffic ───────────────────────────────────────────────

pub async fn ga4_traffic(
    State(state): State<GoogleState>,
    Query(q): Query<DateRangeQuery>,
) -> Json<GResult> {
    run_ga4_data_query(&state, q, "/oauth/google/ga4/traffic-summary").await
}

// ── 6. GET /ga4/timeseries ────────────────────────────────────────────

pub async fn ga4_timeseries(
    State(state): State<GoogleState>,
    Query(q): Query<DateRangeQuery>,
) -> Json<GResult> {
    run_ga4_data_query(&state, q, "/oauth/google/ga4/traffic-timeseries").await
}

// ── 7. GET /ga4/sources ───────────────────────────────────────────────

pub async fn ga4_sources(
    State(state): State<GoogleState>,
    Query(q): Query<DateRangeQuery>,
) -> Json<GResult> {
    run_ga4_data_query(&state, q, "/oauth/google/ga4/traffic-sources").await
}

// ── 8. GET /ga4/pages ─────────────────────────────────────────────────

pub async fn ga4_pages(
    State(state): State<GoogleState>,
    Query(q): Query<DateRangeQuery>,
) -> Json<GResult> {
    run_ga4_data_query(&state, q, "/oauth/google/ga4/top-pages").await
}

// ── 9. GET /ga4/status ────────────────────────────────────────────────

pub async fn ga4_status(State(state): State<GoogleState>) -> Json<GResult> {
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

    match client.get("/oauth/google/ga4/status", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: true,
                message: "GA4 status loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GA4 API error: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}

// ── Internal helper: data query with date clamping ────────────────────

/// Map a remote API path to a cache prefix for TTL lookup.
fn ga4_cache_prefix(path: &str) -> &str {
    if path.contains("traffic-summary") {
        "ga4:traffic"
    } else if path.contains("traffic-timeseries") {
        "ga4:timeseries"
    } else if path.contains("traffic-sources") {
        "ga4:sources"
    } else if path.contains("top-pages") {
        "ga4:pages"
    } else {
        "ga4:traffic"
    }
}

/// Extract short data type from API path for cache key component.
fn ga4_path_segment(path: &str) -> &str {
    if path.contains("traffic-summary") {
        "traffic"
    } else if path.contains("traffic-timeseries") {
        "timeseries"
    } else if path.contains("traffic-sources") {
        "sources"
    } else if path.contains("top-pages") {
        "pages"
    } else {
        "data"
    }
}

async fn run_ga4_data_query(state: &GoogleState, q: DateRangeQuery, path: &str) -> Json<GResult> {
    let client = match make_client(state) {
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

    if config.ga4_property_id.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GA4 property selected. Use /ga4/select first.".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_start = q.start_date.as_deref().unwrap_or("30daysAgo");
    let start_date = clamp_start_date(raw_start, nexus_role);
    let end_date = q.end_date.as_deref().unwrap_or("today");
    let row_limit = q.row_limit.unwrap_or(10).to_string();
    let force = q.force_refresh.unwrap_or(false);

    let property_id = config.ga4_property_id.clone();

    // Cache lookup
    let segment = ga4_path_segment(path);
    let cache_key =
        GoogleCacheManager::make_key(&["ga4", segment, &start_date, end_date, &property_id]);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Data loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    let params: &[(&str, &str)] = &[
        ("property_id", property_id.as_str()),
        ("start_date", start_date.as_str()),
        ("end_date", end_date),
        ("row_limit", row_limit.as_str()),
    ];

    match client.get(path, params).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix(ga4_cache_prefix(path));
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Data loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GA4 API error: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: e.to_string(),
            data: None,
        }),
    }
}
