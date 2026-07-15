use axum::extract::State;
use axum::Json;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Semaphore;

use super::{ApiResult, SeoState};

#[derive(Deserialize)]
pub(crate) struct LinkCheckRequest {
    pub base_url: String,
    #[serde(default = "default_max_pages")]
    pub max_pages: usize,
    #[serde(default)]
    pub check_external: bool,
}

fn default_max_pages() -> usize {
    50
}

#[derive(Serialize, Clone)]
struct BrokenLink {
    source_page: String,
    target_url: String,
    status_code: Option<u16>,
    error: String,
}

pub(crate) async fn check_links(
    State(_state): State<SeoState>,
    Json(body): Json<LinkCheckRequest>,
) -> Json<ApiResult> {
    let base_url = body.base_url.trim_end_matches('/').to_string();
    let max_pages = body.max_pages.min(200).max(1);
    let check_external = body.check_external;

    let base_host = match extract_host(&base_url) {
        Some(h) => h,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Invalid base URL".into(),
                data: None,
            });
        }
    };

    let client = match reqwest::Client::builder()
        .user_agent("LuperIQ-LinkChecker/1.0")
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return Json(ApiResult {
                ok: false,
                message: format!("Failed to create HTTP client: {e}"),
                data: None,
            });
        }
    };

    let href_re = Regex::new(r#"href\s*=\s*["']([^"']+)["']"#).unwrap();
    let semaphore = Arc::new(Semaphore::new(10));

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(base_url.clone());
    visited.insert(base_url.clone());

    let mut all_links: HashMap<String, Vec<String>> = HashMap::new();
    let mut pages_crawled: usize = 0;

    while let Some(page_url) = queue.pop_front() {
        if pages_crawled >= max_pages {
            break;
        }

        let permit = semaphore.clone().acquire_owned().await;
        if permit.is_err() {
            break;
        }

        let resp = match client.get(&page_url).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        if !resp.status().is_success() {
            continue;
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.contains("text/html") {
            continue;
        }

        let html = match resp.text().await {
            Ok(t) => t,
            Err(_) => continue,
        };

        pages_crawled += 1;

        let mut page_links: Vec<String> = Vec::new();
        for cap in href_re.captures_iter(&html) {
            let raw_href = &cap[1];

            if should_skip_href(raw_href) {
                continue;
            }

            let resolved = resolve_url(&page_url, raw_href);
            let resolved = resolved.split('#').next().unwrap_or(&resolved).to_string();
            if resolved.is_empty() {
                continue;
            }

            let link_host = extract_host(&resolved);
            let is_internal = link_host.as_deref() == Some(&base_host);

            if is_internal {
                if !visited.contains(&resolved) && pages_crawled + queue.len() < max_pages {
                    visited.insert(resolved.clone());
                    queue.push_back(resolved.clone());
                }
                page_links.push(resolved);
            } else if check_external {
                page_links.push(resolved);
            }
        }

        all_links.insert(page_url, page_links);
    }

    let unique_targets: HashSet<String> = all_links.values().flatten().cloned().collect();

    let mut check_results: HashMap<String, (Option<u16>, String)> = HashMap::new();
    let mut handles = Vec::new();

    for target in unique_targets {
        let client = client.clone();
        let sem = semaphore.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let result = check_url(&client, &target).await;
            (target, result)
        }));
    }

    for handle in handles {
        if let Ok((url, (status, error))) = handle.await {
            check_results.insert(url, (status, error));
        }
    }

    let mut broken_links: Vec<BrokenLink> = Vec::new();
    for (source, targets) in &all_links {
        for target in targets {
            if let Some((status, error)) = check_results.get(target) {
                if !error.is_empty() || status.map_or(false, |s| s >= 400) {
                    broken_links.push(BrokenLink {
                        source_page: source.clone(),
                        target_url: target.clone(),
                        status_code: *status,
                        error: error.clone(),
                    });
                }
            }
        }
    }

    broken_links.sort_by(|a, b| {
        a.status_code
            .unwrap_or(0)
            .cmp(&b.status_code.unwrap_or(0))
            .reverse()
            .then_with(|| a.source_page.cmp(&b.source_page))
    });

    let checked_count = check_results.len();
    let broken_count = broken_links.len();

    Json(ApiResult {
        ok: true,
        message: format!(
            "Checked {checked_count} links across {pages_crawled} pages, found {broken_count} broken"
        ),
        data: Some(serde_json::json!({
            "broken_links": broken_links,
            "checked_count": checked_count,
            "broken_count": broken_count,
            "pages_crawled": pages_crawled,
        })),
    })
}

async fn check_url(client: &reqwest::Client, url: &str) -> (Option<u16>, String) {
    match client.head(url).send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            if status == 405 {
                match client.get(url).send().await {
                    Ok(r) => {
                        let s = r.status().as_u16();
                        if s >= 400 {
                            (Some(s), format!("HTTP {s}"))
                        } else {
                            (Some(s), String::new())
                        }
                    }
                    Err(e) => (None, format!("GET fallback failed: {e}")),
                }
            } else if status >= 400 {
                (Some(status), format!("HTTP {status}"))
            } else {
                (Some(status), String::new())
            }
        }
        Err(e) => {
            if e.is_timeout() {
                (None, "Connection timed out".into())
            } else if e.is_connect() {
                (None, "Connection failed".into())
            } else {
                (None, format!("{e}"))
            }
        }
    }
}

fn should_skip_href(href: &str) -> bool {
    let h = href.trim();
    h.is_empty()
        || h.starts_with('#')
        || h.starts_with("mailto:")
        || h.starts_with("tel:")
        || h.starts_with("javascript:")
        || h.starts_with("data:")
}

fn extract_host(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = after_scheme.split('/').next().unwrap_or(after_scheme);
    let host = host.split(':').next().unwrap_or(host);
    Some(host.to_lowercase())
}

fn resolve_url(base: &str, href: &str) -> String {
    let href = href.trim();
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        let scheme = if base.starts_with("https://") {
            "https:"
        } else {
            "http:"
        };
        return format!("{scheme}{href}");
    }
    if href.starts_with('/') {
        let origin = base_origin(base);
        return format!("{origin}{href}");
    }
    let base_dir = if let Some(pos) = base.rfind('/') {
        &base[..pos]
    } else {
        base
    };
    format!("{base_dir}/{href}")
}

fn base_origin(url: &str) -> &str {
    let after_scheme = if let Some(rest) = url.strip_prefix("https://") {
        rest
    } else if let Some(rest) = url.strip_prefix("http://") {
        rest
    } else {
        return url;
    };
    let scheme_len = url.len() - after_scheme.len();
    let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
    &url[..scheme_len + host_end]
}
