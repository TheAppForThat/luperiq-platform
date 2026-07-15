//! GSC (Google Search Console) handlers — 7 endpoints that proxy through api.luperiq.com.

use axum::extract::{Query, State};
use axum::response::Json;
use serde::Deserialize;

use super::cache::{ttl_for_prefix, GoogleCacheManager};
use super::oauth::{make_client, GoogleState};
use super::{
    check_circuit_breaker, clamp_start_date, load_google_config, reset_circuit_breaker,
    save_google_config, trip_circuit_breaker, GResult, GoogleError,
};

// ── Query param struct for force_refresh only ────────────────────────

#[derive(Deserialize, Default)]
pub struct ForceRefreshQuery {
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

// ── 1. GET /gsc/sites ─────────────────────────────────────────────────

pub async fn gsc_sites(
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

    let cache_key = GoogleCacheManager::make_key(&["gsc", "sites"]);
    let force = q.force_refresh.unwrap_or(false);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Sites loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    match client.get("/oauth/google/gsc/sites", &[]).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("gsc:sites");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Sites loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC API error: HTTP {code}"),
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

// ── 2. POST /gsc/select ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GscSelectPayload {
    #[serde(default)]
    pub site_url: String,
    #[serde(default)]
    pub permission_level: String,
}

pub async fn gsc_select(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<GscSelectPayload>,
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
        "site_url": payload.site_url,
        "permission_level": payload.permission_level,
    });

    if let Err(e) = client
        .post("/oauth/google/gsc/select", &remote_payload)
        .await
    {
        eprintln!("Warning: failed to sync GSC selection to central: {e}");
    }

    let mut j = state.journal.lock().await;
    let mut config = load_google_config(&j);

    config.gsc_site_url = payload.site_url;
    config.gsc_permission_level = payload.permission_level;

    match save_google_config(&mut j, &config) {
        Ok(_) => Json(GResult {
            ok: true,
            message: "GSC site selected".into(),
            data: Some(serde_json::json!({
                "gsc_site_url": config.gsc_site_url,
                "gsc_permission_level": config.gsc_permission_level,
            })),
        }),
        Err(e) => Json(GResult {
            ok: false,
            message: format!("Save failed: {e}"),
            data: None,
        }),
    }
}

// ── 3. POST /gsc/verify ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GscVerifyPayload {
    #[serde(default)]
    pub site_url: String,
}

pub async fn gsc_verify(
    State(state): State<GoogleState>,
    axum::extract::Json(payload): axum::extract::Json<GscVerifyPayload>,
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

    let site_url = if payload.site_url.is_empty() {
        config.gsc_site_url.clone()
    } else {
        payload.site_url.clone()
    };

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No site_url provided and none saved in config".into(),
            data: None,
        });
    }

    // Step 1: get verification token
    let token_payload = serde_json::json!({ "site_url": site_url });
    let token_resp = match client
        .post("/oauth/google/gsc/verify-token", &token_payload)
        .await
    {
        Ok(v) => v,
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            return Json(GResult {
                ok: false,
                message: format!("Failed to get verification token: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            });
        }
        Err(e) => {
            return Json(GResult {
                ok: false,
                message: format!("Failed to get verification token: {e}"),
                data: None,
            })
        }
    };

    let token = token_resp
        .get("token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if token.is_empty() {
        return Json(GResult {
            ok: false,
            message: "Verification token response contained no token".into(),
            data: Some(token_resp),
        });
    }

    // Save the verification token
    config.gsc_verification_token = token.clone();
    let _ = save_google_config(&mut j, &config);

    // Step 2: verify the domain
    let verify_payload = serde_json::json!({
        "site_url": site_url,
        "token": token,
    });

    match client
        .post("/oauth/google/gsc/verify", &verify_payload)
        .await
    {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            config.gsc_site_url = site_url.clone();
            // Capture permission level if returned
            if let Some(perm) = data.get("permission_level").and_then(|v| v.as_str()) {
                config.gsc_permission_level = perm.to_string();
            }
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: true,
                message: "GSC site verified".into(),
                data: Some(serde_json::json!({
                    "site_url": site_url,
                    "token": token,
                    "verification": data,
                })),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC verify failed: HTTP {code}"),
                data: Some(serde_json::json!({ "body": body })),
            })
        }
        Err(e) => Json(GResult {
            ok: false,
            message: format!("GSC verify failed: {e}"),
            data: None,
        }),
    }
}

// ── 4. GET /gsc/queries ───────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct GscQueryParams {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub row_limit: Option<u32>,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub query_contains: Option<String>,
    #[serde(default)]
    pub query_not_contains: Option<String>,
    #[serde(default)]
    pub page_contains: Option<String>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub search_type: Option<String>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

pub async fn gsc_queries(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryParams>,
) -> Json<GResult> {
    run_gsc_query(&state, q, "/oauth/google/gsc/query-summary", 25).await
}

// ── 4b. GET /gsc/opportunities ───────────────────────────────────────

pub async fn gsc_opportunities(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryParams>,
) -> Json<GResult> {
    let Json(result) = run_gsc_query(&state, q, "/oauth/google/gsc/query-summary", 100).await;
    if !result.ok {
        return Json(result);
    }

    let data = result.data.unwrap_or(serde_json::Value::Null);
    let rows = extract_query_rows(&data);
    let mut items: Vec<serde_json::Value> =
        rows.into_iter().filter_map(gsc_query_opportunity).collect();
    items.sort_by(|a, b| {
        let a_score = a
            .get("priority_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let b_score = b
            .get("priority_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let summary = serde_json::json!({
        "total_queries": data
            .get("queries")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .or_else(|| data.get("items").and_then(|v| v.as_array()).map(|v| v.len()))
            .unwrap_or(0),
        "opportunity_count": items.len(),
        "page_two_pushes": items.iter().filter(|item| item.get("opportunity_type").and_then(|v| v.as_str()) == Some("page_two_push")).count(),
        "low_ctr_wins": items.iter().filter(|item| item.get("opportunity_type").and_then(|v| v.as_str()) == Some("low_ctr_snippet")).count(),
        "emerging_terms": items.iter().filter(|item| item.get("opportunity_type").and_then(|v| v.as_str()) == Some("emerging_term")).count(),
    });

    Json(GResult {
        ok: true,
        message: "SEO intelligence opportunities loaded".into(),
        data: Some(serde_json::json!({
            "summary": summary,
            "items": items.into_iter().take(25).collect::<Vec<_>>(),
        })),
    })
}

// ── 5. GET /gsc/pages ─────────────────────────────────────────────────

pub async fn gsc_pages(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryParams>,
) -> Json<GResult> {
    run_gsc_query(&state, q, "/oauth/google/gsc/page-summary", 25).await
}

// ── 5b. GET /gsc/query-timeseries ─────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct GscQueryTimeseriesParams {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub queries: Option<String>,
    #[serde(default)]
    pub search_type: Option<String>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

pub async fn gsc_query_timeseries(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryTimeseriesParams>,
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

    let site_url = q
        .site_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(config.gsc_site_url.as_str())
        .to_string();

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GSC site selected. Use /gsc/select first.".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_start = q.start_date.as_deref().unwrap_or("28daysAgo");
    let start_date = clamp_start_date(raw_start, nexus_role);
    let end_date = q.end_date.as_deref().unwrap_or("today");
    let queries = q
        .queries
        .clone()
        .or_else(|| q.query.clone())
        .unwrap_or_default();
    let search_type = q.search_type.as_deref().unwrap_or("web");
    let force = q.force_refresh.unwrap_or(false);

    let cache_key = GoogleCacheManager::make_key(&[
        "gsc",
        "query-timeseries",
        &start_date,
        end_date,
        &site_url,
        search_type,
        &queries,
    ]);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Query timeseries loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    let params: Vec<(&str, &str)> = vec![
        ("site_url", site_url.as_str()),
        ("start_date", start_date.as_str()),
        ("end_date", end_date),
        ("queries", queries.as_str()),
        ("search_type", search_type),
    ];

    match client
        .get("/oauth/google/gsc/query-timeseries", &params)
        .await
    {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("gsc:query-timeseries");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Query timeseries loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC API error: HTTP {code}"),
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

// ── 5c. GET /gsc/query-pages ─────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct GscQueryPagesParams {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub search_type: Option<String>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

pub async fn gsc_query_pages(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryPagesParams>,
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

    let site_url = q
        .site_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(config.gsc_site_url.as_str())
        .to_string();

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GSC site selected. Use /gsc/select first.".into(),
            data: None,
        });
    }

    let query = q.query.as_deref().unwrap_or("").trim().to_string();
    if query.is_empty() {
        return Json(GResult {
            ok: false,
            message: "query is required".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_start = q.start_date.as_deref().unwrap_or("28daysAgo");
    let start_date = clamp_start_date(raw_start, nexus_role);
    let end_date = q.end_date.as_deref().unwrap_or("today");
    let search_type = q.search_type.as_deref().unwrap_or("web");
    let force = q.force_refresh.unwrap_or(false);

    let cache_key = GoogleCacheManager::make_key(&[
        "gsc",
        "query-pages",
        &start_date,
        end_date,
        &site_url,
        search_type,
        &query,
    ]);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Query pages loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    let params: Vec<(&str, &str)> = vec![
        ("site_url", site_url.as_str()),
        ("start_date", start_date.as_str()),
        ("end_date", end_date),
        ("query", query.as_str()),
        ("search_type", search_type),
    ];

    match client.get("/oauth/google/gsc/query-pages", &params).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("gsc:query-pages");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Query pages loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC API error: HTTP {code}"),
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

// ── 5d. GET /gsc/page-timeseries ─────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct GscPageTimeseriesParams {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub pages: Option<String>,
    #[serde(default)]
    pub search_type: Option<String>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

pub async fn gsc_page_timeseries(
    State(state): State<GoogleState>,
    Query(q): Query<GscPageTimeseriesParams>,
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

    let site_url = q
        .site_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(config.gsc_site_url.as_str())
        .to_string();

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GSC site selected. Use /gsc/select first.".into(),
            data: None,
        });
    }

    let pages = q
        .pages
        .clone()
        .or_else(|| q.page.clone())
        .unwrap_or_default();
    if pages.trim().is_empty() {
        return Json(GResult {
            ok: false,
            message: "page or pages is required".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_start = q.start_date.as_deref().unwrap_or("28daysAgo");
    let start_date = clamp_start_date(raw_start, nexus_role);
    let end_date = q.end_date.as_deref().unwrap_or("today");
    let search_type = q.search_type.as_deref().unwrap_or("web");
    let force = q.force_refresh.unwrap_or(false);

    let cache_key = GoogleCacheManager::make_key(&[
        "gsc",
        "page-timeseries",
        &start_date,
        end_date,
        &site_url,
        search_type,
        &pages,
    ]);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Page timeseries loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    let params: Vec<(&str, &str)> = vec![
        ("site_url", site_url.as_str()),
        ("start_date", start_date.as_str()),
        ("end_date", end_date),
        ("pages", pages.as_str()),
        ("search_type", search_type),
    ];

    match client
        .get("/oauth/google/gsc/page-timeseries", &params)
        .await
    {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("gsc:page-timeseries");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Page timeseries loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC API error: HTTP {code}"),
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

// ── 6. GET /gsc/breakdown ─────────────────────────────────────────────

pub async fn gsc_breakdown(
    State(state): State<GoogleState>,
    Query(q): Query<GscQueryParams>,
) -> Json<GResult> {
    run_gsc_query(&state, q, "/oauth/google/gsc/breakdown", 25).await
}

// ── 7. GET /gsc/delta ─────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct GscDeltaParams {
    #[serde(default)]
    pub current_start: Option<String>,
    #[serde(default)]
    pub current_end: Option<String>,
    #[serde(default)]
    pub previous_start: Option<String>,
    #[serde(default)]
    pub previous_end: Option<String>,
    #[serde(default)]
    pub site_url: Option<String>,
    #[serde(default)]
    pub row_limit: Option<u32>,
    #[serde(default)]
    pub force_refresh: Option<bool>,
}

pub async fn gsc_delta(
    State(state): State<GoogleState>,
    Query(q): Query<GscDeltaParams>,
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

    let site_url = q
        .site_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(config.gsc_site_url.as_str())
        .to_string();

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GSC site selected. Use /gsc/select first.".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_current_start = q.current_start.as_deref().unwrap_or("28daysAgo");
    let current_start = clamp_start_date(raw_current_start, nexus_role);
    let current_end = q.current_end.as_deref().unwrap_or("today");
    let raw_prev_start = q.previous_start.as_deref().unwrap_or("56daysAgo");
    let previous_start = clamp_start_date(raw_prev_start, nexus_role);
    let previous_end = q.previous_end.as_deref().unwrap_or("29daysAgo");
    let row_limit = q.row_limit.unwrap_or(25).to_string();
    let force = q.force_refresh.unwrap_or(false);

    // Cache lookup
    let cache_key = GoogleCacheManager::make_key(&[
        "gsc",
        "delta",
        &current_start,
        current_end,
        &previous_start,
        previous_end,
        &site_url,
    ]);

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Delta data loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    let params: &[(&str, &str)] = &[
        ("site_url", site_url.as_str()),
        ("current_start", current_start.as_str()),
        ("current_end", current_end),
        ("previous_start", previous_start.as_str()),
        ("previous_end", previous_end),
        ("row_limit", row_limit.as_str()),
    ];

    match client.get("/oauth/google/gsc/query-delta", params).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix("gsc:delta");
            let _ = GoogleCacheManager::set(&mut j, &cache_key, &data, ttl);
            Json(GResult {
                ok: true,
                message: "Delta data loaded".into(),
                data: Some(data),
            })
        }
        Err(GoogleError::HttpError { code, body }) if code >= 500 => {
            trip_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            Json(GResult {
                ok: false,
                message: format!("GSC API error: HTTP {code}"),
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

// ── Internal helper: GSC query with date clamping and optional filters ─

/// Map a remote API path to a GSC cache prefix for TTL lookup and cache key.
fn gsc_cache_segment(path: &str) -> &str {
    if path.contains("query-summary") {
        "queries"
    } else if path.contains("page-summary") {
        "pages"
    } else if path.contains("breakdown") {
        "breakdown"
    } else {
        "queries"
    }
}

async fn run_gsc_query(
    state: &GoogleState,
    q: GscQueryParams,
    path: &str,
    default_row_limit: u32,
) -> Json<GResult> {
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

    let site_url = q
        .site_url
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or(config.gsc_site_url.as_str())
        .to_string();

    if site_url.is_empty() {
        return Json(GResult {
            ok: false,
            message: "No GSC site selected. Use /gsc/select first.".into(),
            data: None,
        });
    }

    let nexus_role = state
        .nexus_config
        .as_ref()
        .and_then(|nc| nc.role.as_deref())
        .unwrap_or("free");

    let raw_start = q.start_date.as_deref().unwrap_or("28daysAgo");
    let start_date = clamp_start_date(raw_start, nexus_role);
    let end_date = q.end_date.as_deref().unwrap_or("today");
    let row_limit = q.row_limit.unwrap_or(default_row_limit).to_string();
    let force = q.force_refresh.unwrap_or(false);

    // Build cache key from path type + all significant params
    let segment = gsc_cache_segment(path);
    let filter_suffix = {
        let mut parts = Vec::new();
        if let Some(v) = &q.query_contains {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if let Some(v) = &q.query_not_contains {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if let Some(v) = &q.page_contains {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if let Some(v) = &q.device {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if let Some(v) = &q.country {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        if let Some(v) = &q.search_type {
            if !v.is_empty() {
                parts.push(v.as_str());
            }
        }
        parts.join(":")
    };
    let cache_key = if filter_suffix.is_empty() {
        GoogleCacheManager::make_key(&["gsc", segment, &start_date, end_date, &site_url])
    } else {
        GoogleCacheManager::make_key(&[
            "gsc",
            segment,
            &start_date,
            end_date,
            &site_url,
            &filter_suffix,
        ])
    };
    let cache_prefix = format!("gsc:{segment}");

    if !force {
        if let Some(cached) = GoogleCacheManager::get(&j, &cache_key) {
            return Json(GResult {
                ok: true,
                message: "Data loaded (cached)".into(),
                data: Some(cached),
            });
        }
    }

    // Build params — only append optional filters when present
    let mut param_vec: Vec<(&str, String)> = vec![
        ("site_url", site_url.clone()),
        ("start_date", start_date),
        ("end_date", end_date.to_string()),
        ("row_limit", row_limit),
    ];

    if let Some(v) = &q.query_contains {
        if !v.is_empty() {
            param_vec.push(("query_contains", v.clone()));
        }
    }
    if let Some(v) = &q.query_not_contains {
        if !v.is_empty() {
            param_vec.push(("query_not_contains", v.clone()));
        }
    }
    if let Some(v) = &q.page_contains {
        if !v.is_empty() {
            param_vec.push(("page_contains", v.clone()));
        }
    }
    if let Some(v) = &q.device {
        if !v.is_empty() {
            param_vec.push(("device", v.clone()));
        }
    }
    if let Some(v) = &q.country {
        if !v.is_empty() {
            param_vec.push(("country", v.clone()));
        }
    }
    if let Some(v) = &q.search_type {
        if !v.is_empty() {
            param_vec.push(("search_type", v.clone()));
        }
    }

    // Convert to &[(&str, &str)] slice
    let params_ref: Vec<(&str, &str)> = param_vec.iter().map(|(k, v)| (*k, v.as_str())).collect();

    match client.get(path, &params_ref).await {
        Ok(data) => {
            reset_circuit_breaker(&mut config);
            let _ = save_google_config(&mut j, &config);
            let ttl = ttl_for_prefix(&cache_prefix);
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
                message: format!("GSC API error: HTTP {code}"),
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

fn extract_query_rows(data: &serde_json::Value) -> Vec<&serde_json::Value> {
    if let Some(arr) = data.get("queries").and_then(|v| v.as_array()) {
        return arr.iter().collect();
    }
    if let Some(arr) = data.get("items").and_then(|v| v.as_array()) {
        return arr.iter().collect();
    }
    if let Some(arr) = data.as_array() {
        return arr.iter().collect();
    }
    Vec::new()
}

fn row_query_text(row: &serde_json::Value) -> Option<String> {
    row.get("query")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or_else(|| {
            row.get("keys")
                .and_then(|v| v.as_array())
                .and_then(|keys| keys.first())
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
}

fn row_number(row: &serde_json::Value, key: &str) -> f64 {
    row.get(key)
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_u64().map(|n| n as f64))
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .unwrap_or(0.0)
}

fn normalized_ctr(row: &serde_json::Value) -> f64 {
    let ctr = row_number(row, "ctr");
    if ctr > 1.0 {
        ctr / 100.0
    } else {
        ctr
    }
}

fn gsc_query_opportunity(row: &serde_json::Value) -> Option<serde_json::Value> {
    let query = row_query_text(row)?;
    let impressions = row_number(row, "impressions");
    let clicks = row_number(row, "clicks");
    let position = row_number(row, "position");
    let ctr = normalized_ctr(row);

    if query.trim().is_empty() || impressions < 20.0 || position <= 0.0 {
        return None;
    }

    let (opportunity_type, label, recommendation, priority_score) = if position >= 8.0
        && position <= 20.0
        && impressions >= 30.0
    {
        (
                "page_two_push",
                "Page 2 push",
                "Refresh the best matching page and strengthen internal links so this query has a better shot at page-one movement.",
                (impressions * 1.4) + ((21.0 - position) * 6.0),
            )
    } else if position <= 10.0 && impressions >= 50.0 && ctr < 0.03 {
        (
                "low_ctr_snippet",
                "Low CTR win",
                "Rewrite the title and meta description for the current ranking page before spending time on a full content rewrite.",
                (impressions * 1.1) + (clicks * 2.0) + ((0.03 - ctr).max(0.0) * 900.0),
            )
    } else if position > 20.0 && position <= 40.0 && impressions >= 40.0 {
        (
                "emerging_term",
                "Emerging term",
                "Use this query in a dedicated brief or supporting section so the site can compete more directly for the term.",
                impressions + ((41.0 - position) * 3.0),
            )
    } else {
        return None;
    };

    Some(serde_json::json!({
        "query": query,
        "impressions": impressions.round() as u64,
        "clicks": clicks.round() as u64,
        "ctr": ctr,
        "position": position,
        "opportunity_type": opportunity_type,
        "label": label,
        "recommendation": recommendation,
        "priority_score": priority_score,
    }))
}
