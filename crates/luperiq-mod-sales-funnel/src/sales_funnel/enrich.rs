//! Business online-presence enrichment — scrapes their existing website,
//! formats search links for Google Maps / BBB / Yelp, and returns a
//! structured preview the wizard can show before provisioning.

use axum::extract::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct EnrichRequest {
    pub business_name: String,
    pub city: Option<String>,
    pub state: Option<String>,
    pub website_url: Option<String>,
    pub facebook_url: Option<String>,
    pub instagram_handle: Option<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct EnrichResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<WebsiteData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub facebook: Option<FacebookData>,
    pub google_maps_url: String,
    pub bbb_url: String,
    pub yelp_url: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WebsiteData {
    pub title: String,
    pub description: String,
    pub services: Vec<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub social_links: SocialLinks,
}

#[derive(Debug, Serialize, Default)]
pub struct SocialLinks {
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub twitter: Option<String>,
    pub youtube: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FacebookData {
    pub name: String,
    pub description: String,
    pub page_url: String,
}

pub async fn enrich_handler(
    Json(req): Json<EnrichRequest>,
) -> impl IntoResponse {
    let result = enrich_business(req).await;
    Json(result)
}

async fn enrich_business(req: EnrichRequest) -> EnrichResponse {
    let city = req.city.as_deref().unwrap_or("").trim().to_string();
    let state_abbr = req.state.as_deref().unwrap_or("TX").trim().to_string();
    let biz = &req.business_name;

    let google_maps_url = format!(
        "https://www.google.com/maps/search/{}+{}+{}+pest+control",
        urlenc(biz),
        urlenc(&city),
        urlenc(&state_abbr)
    );
    let bbb_url = format!(
        "https://www.bbb.org/search?search_input={}&search_location={}+{}",
        urlenc(biz),
        urlenc(&city),
        urlenc(&state_abbr)
    );
    let yelp_url = format!(
        "https://www.yelp.com/search?find_desc={}&find_loc={}+{}",
        urlenc(biz),
        urlenc(&city),
        urlenc(&state_abbr)
    );

    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Mozilla/5.0 (compatible; LuperIQ-Enrichment/1.0)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return EnrichResponse {
                ok: false,
                google_maps_url,
                bbb_url,
                yelp_url,
                error: Some(format!("HTTP client error: {e}")),
                ..Default::default()
            };
        }
    };

    let mut resp = EnrichResponse {
        ok: true,
        google_maps_url,
        bbb_url,
        yelp_url,
        ..Default::default()
    };

    // Scrape their website if provided
    if let Some(url) = req.website_url.as_deref().filter(|u| !u.trim().is_empty()) {
        let url = normalize_url(url);
        match http.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                if let Ok(html) = r.text().await {
                    resp.website = Some(parse_website(&html, &url));
                    // Prefer website social links if wizard didn't provide them
                    if req.facebook_url.is_none() {
                        if let Some(ref ws) = resp.website {
                            if ws.social_links.facebook.is_some() {
                                resp.facebook = resp.facebook.or_else(|| {
                                    ws.social_links.facebook.as_deref().map(|u| FacebookData {
                                        name: biz.clone(),
                                        description: String::new(),
                                        page_url: u.to_string(),
                                    })
                                });
                            }
                        }
                    }
                }
            }
            _ => {} // silent — website scrape is best-effort
        }
    }

    // Try to fetch Facebook page if URL provided
    if let Some(fb_url) = req.facebook_url.as_deref().filter(|u| !u.trim().is_empty()) {
        let fb_url = normalize_facebook_url(fb_url);
        if let Ok(r) = http.get(&fb_url).send().await {
            if r.status().is_success() {
                if let Ok(html) = r.text().await {
                    resp.facebook = Some(parse_facebook(&html, biz, &fb_url));
                }
            }
        }
    }

    resp
}

fn parse_website(html: &str, url: &str) -> WebsiteData {
    let title = extract_meta(html, "og:title")
        .or_else(|| extract_title(html))
        .unwrap_or_default();

    let description = extract_meta(html, "og:description")
        .or_else(|| extract_meta(html, "description"))
        .unwrap_or_default();

    let phone = extract_phone(html);
    let address = extract_address(html);
    let services = extract_services(html);
    let social_links = extract_social_links(html, url);

    WebsiteData {
        title: clean_text(&title),
        description: clean_text(&description),
        services,
        phone,
        address,
        social_links,
    }
}

fn parse_facebook(html: &str, biz_name: &str, page_url: &str) -> FacebookData {
    let name = extract_meta(html, "og:title")
        .unwrap_or_else(|| biz_name.to_string());
    let description = extract_meta(html, "og:description")
        .unwrap_or_default();
    FacebookData {
        name: clean_text(&name),
        description: clean_text(&description),
        page_url: page_url.to_string(),
    }
}

// ── HTML extraction helpers ──────────────────────────────────────────────────

fn extract_meta(html: &str, name: &str) -> Option<String> {
    // <meta name="..." content="..."> and <meta property="..." content="...">
    let patterns = [
        format!(r#"meta\s+name=["']{name}["']\s+content=["']([^"']+)["']"#),
        format!(r#"meta\s+content=["']([^"']+)["']\s+name=["']{name}["']"#),
        format!(r#"meta\s+property=["']{name}["']\s+content=["']([^"']+)["']"#),
        format!(r#"meta\s+content=["']([^"']+)["']\s+property=["']{name}["']"#),
    ];
    for pat in &patterns {
        if let Ok(re) = regex::Regex::new(pat) {
            if let Some(cap) = re.captures(html) {
                if let Some(val) = cap.get(1) {
                    let v = val.as_str().trim().to_string();
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

fn extract_title(html: &str) -> Option<String> {
    if let Ok(re) = regex::Regex::new(r"(?i)<title[^>]*>([^<]+)</title>") {
        re.captures(html)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

fn extract_phone(html: &str) -> Option<String> {
    // Look for tel: links first (most reliable)
    if let Ok(re) = regex::Regex::new(r#"href=["']tel:([+\d\s\-().]+)["']"#) {
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().trim().to_string());
            }
        }
    }
    None
}

fn extract_address(html: &str) -> Option<String> {
    // Look for schema.org address markup
    if let Ok(re) = regex::Regex::new(r#"itemprop=["']streetAddress["'][^>]*>([^<]+)<"#) {
        if let Some(cap) = re.captures(html) {
            if let Some(m) = cap.get(1) {
                return Some(m.as_str().trim().to_string());
            }
        }
    }
    None
}

fn extract_services(html: &str) -> Vec<String> {
    // Look for common service list patterns: <li> items near "services" heading
    let mut services: Vec<String> = Vec::new();
    if let Ok(re) = regex::Regex::new(r"(?i)<li[^>]*>\s*<a[^>]*>([^<]{5,60})</a>\s*</li>") {
        for cap in re.captures_iter(html).take(40) {
            if let Some(m) = cap.get(1) {
                let text = clean_text(m.as_str());
                if text.len() > 4 && text.len() < 60
                    && !text.contains("©")
                    && !text.contains("http")
                    && !is_nav_item(&text)
                {
                    services.push(text);
                }
            }
        }
    }
    services.dedup();
    services.truncate(12);
    services
}

fn extract_social_links(html: &str, _base_url: &str) -> SocialLinks {
    let mut links = SocialLinks::default();
    if let Ok(re) = regex::Regex::new(r#"href=["'](https?://(?:www\.)?(?:facebook\.com|fb\.com)/[^"'\s?#]+)"#) {
        if let Some(cap) = re.captures(html) {
            links.facebook = cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r#"href=["'](https?://(?:www\.)?instagram\.com/[^"'\s?#]+)"#) {
        if let Some(cap) = re.captures(html) {
            links.instagram = cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r#"href=["'](https?://(?:www\.)?(?:twitter\.com|x\.com)/[^"'\s?#]+)"#) {
        if let Some(cap) = re.captures(html) {
            links.twitter = cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    if let Ok(re) = regex::Regex::new(r#"href=["'](https?://(?:www\.)?youtube\.com/[^"'\s?#]+)"#) {
        if let Some(cap) = re.captures(html) {
            links.youtube = cap.get(1).map(|m| m.as_str().to_string());
        }
    }
    links
}

// ── Utilities ────────────────────────────────────────────────────────────────

fn urlenc(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

fn normalize_facebook_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http") {
        url.to_string()
    } else if url.contains('.') {
        format!("https://{url}")
    } else {
        // treat as a page name/handle
        format!("https://www.facebook.com/{url}")
    }
}

fn clean_text(s: &str) -> String {
    // Decode common HTML entities and strip tags
    let s = s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    // Strip any remaining tags
    let tag_re = regex::Regex::new(r"<[^>]+>").unwrap_or_else(|_| regex::Regex::new(".").unwrap());
    let s = tag_re.replace_all(&s, "");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_nav_item(text: &str) -> bool {
    let nav_words = [
        "home", "about", "contact", "blog", "news", "privacy", "terms",
        "login", "sign in", "register", "search", "menu", "sitemap",
        "faq", "help", "support", "cart", "checkout",
    ];
    let lower = text.to_lowercase();
    nav_words.iter().any(|w| lower == *w)
}
