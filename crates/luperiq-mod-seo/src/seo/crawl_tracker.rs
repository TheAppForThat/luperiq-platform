//! Crawl Tracker — detects search engine and AI bot visits, stores crawl events
//! in a capped SQLite side-store, and provides API endpoints for real-time
//! crawl analytics.
//!
//! Better than Google Search Console because:
//! - Real-time (no 2-day delay)
//! - Tracks ALL bots (Google, Bing, GPTBot, ClaudeBot, etc.)
//! - Captures 404s bots encounter (critical for SEO)
//! - Shows page discovery vs sitemap coverage
//!
//! Storage (2026-05-30 wal-shrink): bot hits are WRITTEN by the cms middleware
//! into `data/seo_crawl.sqlite` (`crate::seo_crawl_db` in `luperiq-cms`), NOT
//! the ForgeJournal — they were 88% of a 3.25 GB WAL. The historical `SeoCrawl`
//! aggregates are being dropped from the journal by a WAL compaction. The READ
//! side (`load_all_events`, which feeds every crawl-analytics handler below)
//! therefore reads from that same SQLite file. `luperiq-mod-seo` cannot call
//! `luperiq-cms` (cms already depends on mod-seo — a hard Cargo cycle), so the
//! ~15-line read helper + env path resolution are intentionally DUPLICATED here
//! from `seo_crawl_db.rs`; the schema and `LUPERIQ_SEO_CRAWL_DB` /
//! `data/seo_crawl.sqlite` path must stay in sync with the writer.
//!
//! Security notes:
//! - Admin UI uses DOM methods (no innerHTML) for XSS safety
//! - Bot detection is case-insensitive substring matching (no regex, fast path)

use axum::extract::{Query, State};
use axum::Json;
use chrono::Datelike;
use luperiq_forge::{ApexEvent, ForgeJournal};
use luperiq_module_api::SharedJournal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Aggregate type constants ─────────────────────────────────────────

pub const AGG_CRAWL_EVENT: &str = "SeoCrawl";

// ── Bot type classification ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BotType {
    Search,
    AI,
    Preview,
    Lighthouse,
    Other,
}

impl std::fmt::Display for BotType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BotType::Search => write!(f, "Search"),
            BotType::AI => write!(f, "AI"),
            BotType::Preview => write!(f, "Preview"),
            BotType::Lighthouse => write!(f, "Lighthouse"),
            BotType::Other => write!(f, "Other"),
        }
    }
}

impl BotType {
    /// Reverse of `Display` — maps the `bot_type` TEXT column (written as
    /// `BotType::to_string()` by the side-store writer) back to the enum.
    /// Unknown values fall back to `Other`.
    fn from_str_lossy(s: &str) -> BotType {
        match s {
            "Search" => BotType::Search,
            "AI" => BotType::AI,
            "Preview" => BotType::Preview,
            "Lighthouse" => BotType::Lighthouse,
            _ => BotType::Other,
        }
    }
}

// ── Known bot patterns ───────────────────────────────────────────────

struct BotPattern {
    /// Substring to match in User-Agent (case-insensitive)
    ua_contains: &'static str,
    /// Canonical bot name
    name: &'static str,
    /// Bot type classification
    bot_type: BotType,
}

const BOT_PATTERNS: &[BotPattern] = &[
    // Google family
    BotPattern {
        ua_contains: "googlebot-image",
        name: "Googlebot-Image",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "google-inspectiontool",
        name: "Google-InspectionTool",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "adsbot-google",
        name: "AdsBot-Google",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "googlebot",
        name: "Googlebot",
        bot_type: BotType::Search,
    },
    // Bing family
    BotPattern {
        ua_contains: "bingpreview",
        name: "BingPreview",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "bingbot",
        name: "Bingbot",
        bot_type: BotType::Search,
    },
    // AI bots
    BotPattern {
        ua_contains: "chatgpt-user",
        name: "ChatGPT-User",
        bot_type: BotType::AI,
    },
    BotPattern {
        ua_contains: "gptbot",
        name: "GPTBot",
        bot_type: BotType::AI,
    },
    BotPattern {
        ua_contains: "claudebot",
        name: "ClaudeBot",
        bot_type: BotType::AI,
    },
    BotPattern {
        ua_contains: "anthropic-ai",
        name: "Anthropic-AI",
        bot_type: BotType::AI,
    },
    BotPattern {
        ua_contains: "cohere-ai",
        name: "Cohere-AI",
        bot_type: BotType::AI,
    },
    BotPattern {
        ua_contains: "perplexitybot",
        name: "PerplexityBot",
        bot_type: BotType::AI,
    },
    // Other search bots
    BotPattern {
        ua_contains: "petalbot",
        name: "PetalBot",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "yandexbot",
        name: "YandexBot",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "baiduspider",
        name: "Baiduspider",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "duckduckbot",
        name: "DuckDuckBot",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "applebot",
        name: "Applebot",
        bot_type: BotType::Search,
    },
    BotPattern {
        ua_contains: "seznambot",
        name: "SeznamBot",
        bot_type: BotType::Search,
    },
    // Lighthouse / Performance
    BotPattern {
        ua_contains: "chrome-lighthouse",
        name: "Chrome-Lighthouse",
        bot_type: BotType::Lighthouse,
    },
    BotPattern {
        ua_contains: "pagespeed",
        name: "PageSpeed Insights",
        bot_type: BotType::Lighthouse,
    },
    // Social previews
    BotPattern {
        ua_contains: "twitterbot",
        name: "TwitterBot",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "facebookexternalhit",
        name: "Facebook",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "linkedinbot",
        name: "LinkedInBot",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "slackbot",
        name: "Slackbot",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "discordbot",
        name: "Discordbot",
        bot_type: BotType::Preview,
    },
    BotPattern {
        ua_contains: "telegrambot",
        name: "TelegramBot",
        bot_type: BotType::Preview,
    },
    // Other crawlers
    BotPattern {
        ua_contains: "semrushbot",
        name: "SemrushBot",
        bot_type: BotType::Other,
    },
    BotPattern {
        ua_contains: "ahrefsbot",
        name: "AhrefsBot",
        bot_type: BotType::Other,
    },
    BotPattern {
        ua_contains: "mj12bot",
        name: "MJ12Bot",
        bot_type: BotType::Other,
    },
    BotPattern {
        ua_contains: "dotbot",
        name: "DotBot",
        bot_type: BotType::Other,
    },
];

// ── CrawlEvent ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlEvent {
    pub bot_name: String,
    pub bot_type: BotType,
    pub url: String,
    pub status_code: u16,
    pub timestamp: String,
    pub user_agent: String,
    pub referrer: Option<String>,
}

// ── Bot detection (hot path — called from middleware) ─────────────────

/// Detect whether a User-Agent string belongs to a known bot.
/// Returns `Some((bot_name, bot_type))` if recognized, `None` otherwise.
/// This is designed to be lightweight — simple case-insensitive substring matching.
pub fn detect_bot(user_agent: &str) -> Option<(&'static str, BotType)> {
    let ua_lower = user_agent.to_ascii_lowercase();
    for pattern in BOT_PATTERNS {
        if ua_lower.contains(pattern.ua_contains) {
            return Some((pattern.name, pattern.bot_type.clone()));
        }
    }
    None
}

// ── WAL recording (called from middleware or dedicated handler) ───────

/// Record a crawl event in the journal. Uses a ULID-based aggregate_id
/// so every event is preserved (no overwrites).
pub fn record_crawl_event(
    journal: &mut ForgeJournal,
    event: &CrawlEvent,
) -> Result<String, String> {
    let id = ulid::Ulid::new().to_string();
    let payload = serde_json::to_vec(event).map_err(|e| format!("Serialize CrawlEvent: {e}"))?;
    let apex_event = ApexEvent::new(AGG_CRAWL_EVENT, &id, payload);
    journal
        .append(apex_event)
        .map_err(|e| format!("Journal append: {e}"))?;
    Ok(id)
}

/// Middleware hook: call this from the request pipeline after getting the response.
/// If the user-agent is a known bot, log the crawl event to the WAL.
/// Returns `true` if a bot was detected and logged.
pub async fn maybe_log_crawl(
    journal: &SharedJournal,
    url: &str,
    status_code: u16,
    user_agent: &str,
    referrer: Option<&str>,
) -> bool {
    let Some((bot_name, bot_type)) = detect_bot(user_agent) else {
        return false;
    };

    let event = CrawlEvent {
        bot_name: bot_name.to_string(),
        bot_type,
        url: url.to_string(),
        status_code,
        timestamp: chrono::Utc::now().to_rfc3339(),
        user_agent: user_agent.to_string(),
        referrer: referrer.map(|s| s.to_string()),
    };

    let mut j = journal.lock().await;
    if let Err(e) = record_crawl_event(&mut j, &event) {
        eprintln!("[crawl-tracker] Failed to log crawl event: {e}");
    }
    true
}

// ── Query helpers ────────────────────────────────────────────────────

/// Load all crawl events.
///
/// SOURCE (2026-05-30): the capped SQLite side-store `data/seo_crawl.sqlite`
/// (path overridable via `LUPERIQ_SEO_CRAWL_DB`), NOT the ForgeJournal — bot
/// hits no longer live there. The `journal` argument is retained only so the
/// six analytics handlers that call this primitive (`crawl_summary`,
/// `crawl_stats`, `crawl_log`, `crawl_events`, `missing_pages`,
/// `page_discovery`) keep an identical signature and need no edits.
///
/// Best-effort: a missing/locked/empty DB yields an empty Vec (never panics).
/// Returns events newest-first, matching the previous WAL-sourced contract.
pub(crate) fn load_all_events(journal: &ForgeJournal) -> Vec<CrawlEvent> {
    let _ = journal; // data source moved off the journal; arg kept for callers.
    load_all_events_from_side_store()
}

/// Resolve the side-store path the same way the cms writer does. Kept in sync
/// with `luperiq-cms/src/seo_crawl_db.rs` (duplicated to avoid a Cargo cycle).
fn side_store_path() -> String {
    std::env::var("LUPERIQ_SEO_CRAWL_DB").unwrap_or_else(|_| "data/seo_crawl.sqlite".to_string())
}

/// Read every retained bot hit from the SQLite side-store, newest-first,
/// reconstructing `CrawlEvent`. The side-store also carries a client `ip`
/// column the old journal event lacked; it is simply not surfaced in
/// `CrawlEvent` (callers never had it). All errors collapse to an empty Vec.
fn load_all_events_from_side_store() -> Vec<CrawlEvent> {
    let path = side_store_path();
    // Open READ_WRITE (no CREATE) rather than READ_ONLY: the side-store runs in
    // WAL journal mode, and a READ_ONLY connection to a WAL-mode DB needs write
    // access to the -shm/-wal sidecars (or a live in-process writer holding
    // them). After a restart, before the first bot hit re-opens the writer
    // connection, a READ_ONLY open can fail with SQLITE_CANTOPEN and silently
    // return empty for ALL six crawl handlers. READ_WRITE avoids that whole
    // class and matches how the cms writer opens the same file. We deliberately
    // omit CREATE so a not-yet-existing file still falls through to empty —
    // exactly the old "no events recorded yet" behaviour.
    let conn = match rusqlite::Connection::open_with_flags(
        &path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut stmt = match conn.prepare(
        "SELECT bot_name, bot_type, url, status_code, timestamp, user_agent, referrer
         FROM seo_crawl ORDER BY timestamp DESC",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([], |row| {
        let bot_type_str: String = row.get(1)?;
        let status_code: i64 = row.get(3)?;
        Ok(CrawlEvent {
            bot_name: row.get(0)?,
            bot_type: BotType::from_str_lossy(&bot_type_str),
            url: row.get(2)?,
            status_code: status_code as u16,
            timestamp: row.get(4)?,
            user_agent: row.get(5)?,
            referrer: row.get(6)?,
        })
    });
    match rows {
        Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

// ── API types ────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct CrawlLogQuery {
    #[serde(default)]
    pub bot: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub bot_type: Option<String>,
    #[serde(default)]
    pub status: Option<u16>,
    #[serde(default)]
    pub page: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Serialize)]
struct CrawlStatsResponse {
    ok: bool,
    total_crawls: usize,
    unique_pages: usize,
    unique_bots: usize,
    bots_seen: Vec<BotSummary>,
    by_bot: HashMap<String, usize>,
    by_type: HashMap<String, usize>,
    top_pages: Vec<PageCrawlCount>,
    error_pages: usize,
    // Time-based breakdown
    today: usize,
    yesterday: usize,
    this_week: usize,
    this_month: usize,
    // Legacy compat
    recent_24h: usize,
    recent_7d: usize,
}

#[derive(Serialize)]
struct BotSummary {
    name: String,
    bot_type: BotType,
    total_visits: usize,
    last_seen: String,
    pages_crawled: usize,
}

#[derive(Serialize)]
struct PageCrawlCount {
    url: String,
    count: usize,
}

#[derive(Serialize)]
struct CrawlLogResponse {
    ok: bool,
    events: Vec<CrawlEvent>,
    total: usize,
    page: u32,
    page_size: u32,
}

#[derive(Serialize)]
struct MissingPagesResponse {
    ok: bool,
    missing_pages: Vec<MissingPage>,
}

#[derive(Serialize)]
struct MissingPage {
    url: String,
    status_code: u16,
    bots_that_tried: Vec<String>,
    first_seen: String,
    last_seen: String,
    hit_count: usize,
}

#[derive(Serialize)]
struct PageDiscoveryResponse {
    ok: bool,
    discovered_pages: Vec<DiscoveredPage>,
}

#[derive(Serialize)]
struct DiscoveredPage {
    url: String,
    bots_that_found: Vec<String>,
    first_seen: String,
    last_seen: String,
    crawl_count: usize,
    status_code: u16,
}

// ── API handlers ─────────────────────────────────────────────────────

/// GET /api/modules/seo/crawl-stats — summary of all crawl activity.
pub(crate) async fn crawl_stats(State(state): State<super::SeoState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let events = load_all_events(&j);

    let total_crawls = events.len();

    // Unique pages and bots
    let unique_pages_set: std::collections::HashSet<&str> =
        events.iter().map(|e| e.url.as_str()).collect();
    let unique_bots_set: std::collections::HashSet<&str> =
        events.iter().map(|e| e.bot_name.as_str()).collect();

    // Per-bot stats
    let mut bot_map: HashMap<String, (BotType, Vec<&CrawlEvent>)> = HashMap::new();
    for ev in &events {
        bot_map
            .entry(ev.bot_name.clone())
            .or_insert_with(|| (ev.bot_type.clone(), Vec::new()))
            .1
            .push(ev);
    }

    // by_bot: simple name → count map
    let mut by_bot: HashMap<String, usize> = HashMap::new();
    for ev in &events {
        *by_bot.entry(ev.bot_name.clone()).or_default() += 1;
    }

    let mut bots_seen: Vec<BotSummary> = bot_map
        .into_iter()
        .map(|(name, (bot_type, visits))| {
            let pages: std::collections::HashSet<&str> =
                visits.iter().map(|v| v.url.as_str()).collect();
            let last_seen = visits
                .iter()
                .map(|v| v.timestamp.as_str())
                .max()
                .unwrap_or("")
                .to_string();
            BotSummary {
                name,
                bot_type,
                total_visits: visits.len(),
                last_seen,
                pages_crawled: pages.len(),
            }
        })
        .collect();
    bots_seen.sort_by(|a, b| b.total_visits.cmp(&a.total_visits));

    // By bot type
    let mut by_type: HashMap<String, usize> = HashMap::new();
    for ev in &events {
        *by_type.entry(ev.bot_type.to_string()).or_default() += 1;
    }

    // Time-based breakdown
    let now = chrono::Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let yesterday_start = today_start - chrono::Duration::days(1);
    let week_start =
        today_start - chrono::Duration::days(now.weekday().num_days_from_monday() as i64);
    let month_start = now
        .date_naive()
        .with_day(1)
        .unwrap_or(now.date_naive())
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut today = 0usize;
    let mut yesterday = 0usize;
    let mut this_week = 0usize;
    let mut this_month = 0usize;
    let mut recent_24h = 0usize;
    let mut recent_7d = 0usize;

    for ev in &events {
        if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&ev.timestamp) {
            let naive = t.naive_utc();
            let dur = now.signed_duration_since(t);
            if naive >= today_start.and_utc().naive_utc() {
                today += 1;
            }
            if naive >= yesterday_start.and_utc().naive_utc()
                && naive < today_start.and_utc().naive_utc()
            {
                yesterday += 1;
            }
            if naive >= week_start.and_utc().naive_utc() {
                this_week += 1;
            }
            if naive >= month_start.and_utc().naive_utc() {
                this_month += 1;
            }
            if dur.num_hours() < 24 {
                recent_24h += 1;
            }
            if dur.num_days() < 7 {
                recent_7d += 1;
            }
        }
    }

    // Top pages
    let mut page_counts: HashMap<&str, usize> = HashMap::new();
    for ev in &events {
        *page_counts.entry(&ev.url).or_default() += 1;
    }
    let mut top_pages: Vec<PageCrawlCount> = page_counts
        .into_iter()
        .map(|(url, count)| PageCrawlCount {
            url: url.to_string(),
            count,
        })
        .collect();
    top_pages.sort_by(|a, b| b.count.cmp(&a.count));
    top_pages.truncate(20);

    // Error pages (4xx, 5xx)
    let error_pages = events.iter().filter(|e| e.status_code >= 400).count();

    Json(
        serde_json::to_value(CrawlStatsResponse {
            ok: true,
            total_crawls,
            unique_pages: unique_pages_set.len(),
            unique_bots: unique_bots_set.len(),
            bots_seen,
            by_bot,
            by_type,
            top_pages,
            error_pages,
            today,
            yesterday,
            this_week,
            this_month,
            recent_24h,
            recent_7d,
        })
        .unwrap_or_default(),
    )
}

/// GET /api/modules/seo/crawl-log — paginated crawl event log with filters.
pub(crate) async fn crawl_log(
    State(state): State<super::SeoState>,
    Query(q): Query<CrawlLogQuery>,
) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let mut events = load_all_events(&j);
    // Already sorted newest-first by load_all_events

    // Apply filters
    if let Some(ref bot) = q.bot {
        let bot_lower = bot.to_ascii_lowercase();
        events.retain(|e| e.bot_name.to_ascii_lowercase().contains(&bot_lower));
    }
    if let Some(ref url) = q.url {
        events.retain(|e| e.url.contains(url.as_str()));
    }
    if let Some(ref bt) = q.bot_type {
        let bt_lower = bt.to_ascii_lowercase();
        events.retain(|e| e.bot_type.to_string().to_ascii_lowercase() == bt_lower);
    }
    if let Some(status) = q.status {
        events.retain(|e| e.status_code == status);
    }

    let total = events.len();
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).min(200);
    let skip = ((page - 1) * limit) as usize;

    let page_events: Vec<CrawlEvent> = events.into_iter().skip(skip).take(limit as usize).collect();

    Json(
        serde_json::to_value(CrawlLogResponse {
            ok: true,
            events: page_events,
            total,
            page,
            page_size: limit,
        })
        .unwrap_or_default(),
    )
}

/// GET /api/modules/seo/crawl-events — raw recent crawl events (last 50).
///
/// Simple endpoint for the admin UI live feed — no pagination, no filters.
pub(crate) async fn crawl_events(State(state): State<super::SeoState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let events = load_all_events(&j);
    // Already sorted newest-first by load_all_events
    let recent: Vec<&CrawlEvent> = events.iter().take(50).collect();
    Json(serde_json::json!({
        "ok": true,
        "events": recent,
        "total": events.len(),
    }))
}

/// GET /api/modules/seo/missing-pages — URLs that returned 404 to bots.
pub(crate) async fn missing_pages(State(state): State<super::SeoState>) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let events = load_all_events(&j);

    // Group 404s by URL
    let mut missing: HashMap<String, Vec<&CrawlEvent>> = HashMap::new();
    for ev in &events {
        if ev.status_code == 404 {
            missing.entry(ev.url.clone()).or_default().push(ev);
        }
    }

    let mut missing_pages: Vec<MissingPage> = missing
        .into_iter()
        .map(|(url, hits)| {
            let bots: std::collections::HashSet<String> =
                hits.iter().map(|h| h.bot_name.clone()).collect();
            let first_seen = hits
                .iter()
                .map(|h| h.timestamp.as_str())
                .min()
                .unwrap_or("")
                .to_string();
            let last_seen = hits
                .iter()
                .map(|h| h.timestamp.as_str())
                .max()
                .unwrap_or("")
                .to_string();
            MissingPage {
                url,
                status_code: 404,
                bots_that_tried: bots.into_iter().collect(),
                first_seen,
                last_seen,
                hit_count: hits.len(),
            }
        })
        .collect();
    missing_pages.sort_by(|a, b| b.hit_count.cmp(&a.hit_count));

    Json(
        serde_json::to_value(MissingPagesResponse {
            ok: true,
            missing_pages,
        })
        .unwrap_or_default(),
    )
}

/// GET /api/modules/seo/page-discovery — pages found by bots.
pub(crate) async fn page_discovery(
    State(state): State<super::SeoState>,
) -> Json<serde_json::Value> {
    let j = state.journal.lock().await;
    let events = load_all_events(&j);

    // Group successful crawls by URL
    let mut pages: HashMap<String, Vec<&CrawlEvent>> = HashMap::new();
    for ev in &events {
        if ev.status_code < 400 {
            pages.entry(ev.url.clone()).or_default().push(ev);
        }
    }

    let mut discovered_pages: Vec<DiscoveredPage> = pages
        .into_iter()
        .map(|(url, hits)| {
            let bots: std::collections::HashSet<String> =
                hits.iter().map(|h| h.bot_name.clone()).collect();
            let first_seen = hits
                .iter()
                .map(|h| h.timestamp.as_str())
                .min()
                .unwrap_or("")
                .to_string();
            let last_seen = hits
                .iter()
                .map(|h| h.timestamp.as_str())
                .max()
                .unwrap_or("")
                .to_string();
            let status_code = hits.last().map(|h| h.status_code).unwrap_or(200);
            DiscoveredPage {
                url,
                bots_that_found: bots.into_iter().collect(),
                first_seen,
                last_seen,
                crawl_count: hits.len(),
                status_code,
            }
        })
        .collect();
    discovered_pages.sort_by(|a, b| b.crawl_count.cmp(&a.crawl_count));

    Json(
        serde_json::to_value(PageDiscoveryResponse {
            ok: true,
            discovered_pages,
        })
        .unwrap_or_default(),
    )
}

// ── Admin JS for Crawl Tracker panel ─────────────────────────────────

pub const CRAWL_TRACKER_ADMIN_JS: &str = r##"
// ── Crawl Tracker admin view ────────────────────────────────────────
// Security: all rendering uses DOM methods (createElement/textContent), no innerHTML.

function load_seo_crawl_tracker() {
    var container = document.getElementById('adminMain') || document.getElementById('module-content');
    if (!container) return;
    while (container.firstChild) container.removeChild(container.firstChild);

    var header = document.createElement('div');
    header.style.cssText = 'display:flex;align-items:center;justify-content:space-between;margin-bottom:24px;flex-wrap:wrap;gap:12px;';
    var h2 = document.createElement('h2');
    h2.textContent = 'Crawl Tracker';
    h2.style.cssText = 'margin:0;font-size:1.4rem;';
    header.appendChild(h2);

    var headerRight = document.createElement('div');
    headerRight.style.cssText = 'display:flex;gap:8px;align-items:center;';
    var subtitle = document.createElement('span');
    subtitle.textContent = 'Real-time bot monitoring';
    subtitle.style.cssText = 'font-size:0.8rem;color:var(--text-secondary);';
    headerRight.appendChild(subtitle);
    var refreshBtn = document.createElement('button');
    refreshBtn.className = 'btn btn-secondary btn-sm';
    refreshBtn.textContent = 'Refresh';
    refreshBtn.onclick = function() { load_seo_crawl_tracker(); };
    headerRight.appendChild(refreshBtn);
    header.appendChild(headerRight);
    container.appendChild(header);

    // Stats cards row
    var statsRow = document.createElement('div');
    statsRow.style.cssText = 'display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:12px;margin-bottom:24px;';
    container.appendChild(statsRow);

    // Bot chart + top pages side-by-side
    var chartRow = document.createElement('div');
    chartRow.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-bottom:24px;';
    var botChartBox = document.createElement('div');
    botChartBox.style.cssText = 'background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:16px;';
    var topPagesBox = document.createElement('div');
    topPagesBox.style.cssText = 'background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:16px;';
    chartRow.appendChild(botChartBox);
    chartRow.appendChild(topPagesBox);
    container.appendChild(chartRow);

    // Tabs
    var tabs = document.createElement('div');
    tabs.style.cssText = 'display:flex;gap:4px;margin-bottom:16px;flex-wrap:wrap;';
    container.appendChild(tabs);

    // Filter bar
    var filterBar = document.createElement('div');
    filterBar.style.cssText = 'display:flex;gap:12px;margin-bottom:16px;flex-wrap:wrap;align-items:center;';
    filterBar.id = 'crawl-filter-bar';

    var botFilter = document.createElement('select');
    botFilter.style.cssText = 'padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--bg-primary);color:var(--text-primary);font-size:0.85rem;';
    var optAll = document.createElement('option');
    optAll.value = '';
    optAll.textContent = 'All Bots';
    botFilter.appendChild(optAll);

    var statusFilter = document.createElement('select');
    statusFilter.style.cssText = botFilter.style.cssText;
    [['', 'All Status'], ['200', '200 OK'], ['301', '301 Redirect'], ['404', '404 Not Found'], ['500', '500 Error']].forEach(function(pair) {
        var opt = document.createElement('option');
        opt.value = pair[0];
        opt.textContent = pair[1];
        statusFilter.appendChild(opt);
    });

    var urlInput = document.createElement('input');
    urlInput.type = 'text';
    urlInput.placeholder = 'Filter by URL...';
    urlInput.style.cssText = 'padding:8px 12px;border:1px solid var(--border);border-radius:6px;background:var(--bg-primary);color:var(--text-primary);font-size:0.85rem;min-width:200px;';

    var applyBtn = document.createElement('button');
    applyBtn.className = 'btn btn-primary btn-sm';
    applyBtn.textContent = 'Apply';
    applyBtn.onclick = function() { loadLog(); };

    filterBar.appendChild(botFilter);
    filterBar.appendChild(statusFilter);
    filterBar.appendChild(urlInput);
    filterBar.appendChild(applyBtn);
    container.appendChild(filterBar);

    // Content area
    var tableWrap = document.createElement('div');
    tableWrap.style.cssText = 'overflow-x:auto;';
    container.appendChild(tableWrap);

    function clearEl(el) { while (el.firstChild) el.removeChild(el.firstChild); }

    // Tab setup
    var tabNames = ['Recent Events', 'Crawl Log', 'Missing Pages (404)', 'Page Discovery'];
    var tabFns = [loadRecent, loadLog, loadMissing, loadDiscovery];
    var activeTab = 0;
    tabNames.forEach(function(name, i) {
        var btn = document.createElement('button');
        btn.textContent = name;
        btn.className = 'btn btn-sm ' + (i === 0 ? 'btn-primary' : 'btn-secondary');
        btn.dataset.tabIndex = String(i);
        btn.onclick = function() {
            activeTab = i;
            Array.prototype.forEach.call(tabs.querySelectorAll('button'), function(b) {
                b.className = 'btn btn-sm ' + (parseInt(b.dataset.tabIndex) === activeTab ? 'btn-primary' : 'btn-secondary');
            });
            // Show/hide filter bar (only for Crawl Log tab)
            filterBar.style.display = (i === 1) ? 'flex' : 'none';
            tabFns[i]();
        };
        tabs.appendChild(btn);
    });
    // Hide filter bar initially (Recent Events tab is default)
    filterBar.style.display = 'none';

    // ── Load stats and render charts ────────────────────────────────
    fetch('/api/modules/seo/crawl-stats').then(function(r) { return r.json(); }).then(function(data) {
        if (!data.ok) return;
        [
            { label: 'Total Crawls', value: data.total_crawls, color: '#3b82f6' },
            { label: 'Today', value: data.today, color: '#22c55e' },
            { label: 'Yesterday', value: data.yesterday, color: '#8b5cf6' },
            { label: 'This Week', value: data.this_week, color: '#f59e0b' },
            { label: 'This Month', value: data.this_month, color: '#06b6d4' },
            { label: 'Unique Pages', value: data.unique_pages, color: '#ec4899' },
            { label: 'Bots Seen', value: data.unique_bots, color: '#14b8a6' },
            { label: '404 Errors', value: data.error_pages, color: '#ef4444' }
        ].forEach(function(c) {
            var card = document.createElement('div');
            card.style.cssText = 'background:var(--bg-secondary);border:1px solid var(--border);border-radius:8px;padding:14px;text-align:center;';
            var val = document.createElement('div');
            val.textContent = (c.value || 0).toLocaleString();
            val.style.cssText = 'font-size:1.5rem;font-weight:700;color:' + c.color + ';';
            var lbl = document.createElement('div');
            lbl.textContent = c.label;
            lbl.style.cssText = 'font-size:0.75rem;color:var(--text-secondary);margin-top:4px;';
            card.appendChild(val);
            card.appendChild(lbl);
            statsRow.appendChild(card);
        });

        // Populate bot filter dropdown
        (data.bots_seen || []).forEach(function(bot) {
            var opt = document.createElement('option');
            opt.value = bot.name;
            opt.textContent = bot.name + ' (' + bot.total_visits + ')';
            botFilter.appendChild(opt);
        });

        // ── Bot visit chart (horizontal bar chart) ──────────────────
        clearEl(botChartBox);
        var chartTitle = document.createElement('div');
        chartTitle.textContent = 'Bot Visits';
        chartTitle.style.cssText = 'font-weight:600;font-size:0.95rem;margin-bottom:12px;color:var(--text-primary);';
        botChartBox.appendChild(chartTitle);

        var bots = (data.bots_seen || []).slice(0, 10);
        if (!bots.length) {
            var noBots = document.createElement('p');
            noBots.textContent = 'No bot visits recorded yet.';
            noBots.style.cssText = 'color:var(--text-secondary);font-size:0.85rem;';
            botChartBox.appendChild(noBots);
        } else {
            var maxVisits = bots[0].total_visits || 1;
            var botColors = { Search: '#22c55e', AI: '#8b5cf6', Preview: '#f59e0b', Lighthouse: '#3b82f6', Other: '#6b7280' };
            bots.forEach(function(bot) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;align-items:center;gap:8px;margin-bottom:6px;';
                var label = document.createElement('div');
                label.textContent = bot.name;
                label.style.cssText = 'width:120px;font-size:0.8rem;text-align:right;flex-shrink:0;color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
                var barWrap = document.createElement('div');
                barWrap.style.cssText = 'flex:1;background:var(--bg-primary);border-radius:4px;height:20px;overflow:hidden;';
                var bar = document.createElement('div');
                var pct = Math.round((bot.total_visits / maxVisits) * 100);
                var bc = botColors[bot.bot_type] || '#6b7280';
                bar.style.cssText = 'height:100%;border-radius:4px;background:' + bc + ';width:' + pct + '%;min-width:2px;transition:width 0.3s;';
                barWrap.appendChild(bar);
                var count = document.createElement('div');
                count.textContent = bot.total_visits.toLocaleString();
                count.style.cssText = 'width:50px;font-size:0.8rem;color:var(--text-secondary);text-align:right;';
                row.appendChild(label);
                row.appendChild(barWrap);
                row.appendChild(count);
                botChartBox.appendChild(row);
            });
        }

        // ── Top pages ───────────────────────────────────────────────
        clearEl(topPagesBox);
        var pagesTitle = document.createElement('div');
        pagesTitle.textContent = 'Top Crawled Pages';
        pagesTitle.style.cssText = 'font-weight:600;font-size:0.95rem;margin-bottom:12px;color:var(--text-primary);';
        topPagesBox.appendChild(pagesTitle);

        var pages = (data.top_pages || []).slice(0, 10);
        if (!pages.length) {
            var noPages = document.createElement('p');
            noPages.textContent = 'No pages crawled yet.';
            noPages.style.cssText = 'color:var(--text-secondary);font-size:0.85rem;';
            topPagesBox.appendChild(noPages);
        } else {
            pages.forEach(function(pg, idx) {
                var row = document.createElement('div');
                row.style.cssText = 'display:flex;justify-content:space-between;align-items:center;padding:6px 0;' + (idx < pages.length - 1 ? 'border-bottom:1px solid var(--border);' : '');
                var urlEl = document.createElement('div');
                urlEl.textContent = pg.url;
                urlEl.style.cssText = 'font-size:0.8rem;color:var(--text-primary);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:240px;';
                var countEl = document.createElement('span');
                countEl.className = 'status-badge';
                countEl.textContent = pg.count.toLocaleString();
                countEl.style.cssText = 'background:rgba(59,130,246,0.15);color:#3b82f6;font-size:0.75rem;';
                row.appendChild(urlEl);
                row.appendChild(countEl);
                topPagesBox.appendChild(row);
            });
        }
    });

    // ── Shared helpers ──────────────────────────────────────────────

    function makeTable(headers) {
        var table = document.createElement('table');
        table.className = 'data-table';
        table.style.cssText = 'width:100%;';
        var thead = document.createElement('thead');
        var tr = document.createElement('tr');
        headers.forEach(function(h) { var th = document.createElement('th'); th.textContent = h; tr.appendChild(th); });
        thead.appendChild(tr);
        table.appendChild(thead);
        var tbody = document.createElement('tbody');
        table.appendChild(tbody);
        return { table: table, tbody: tbody };
    }

    function statusBadge(code) {
        var b = document.createElement('span');
        b.className = 'status-badge';
        b.textContent = String(code);
        if (code >= 400) b.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;';
        else if (code >= 300) b.style.cssText = 'background:rgba(245,158,11,0.15);color:#f59e0b;';
        else b.style.cssText = 'background:rgba(34,197,94,0.15);color:#22c55e;';
        return b;
    }

    function typeBadge(botType) {
        var b = document.createElement('span');
        b.className = 'status-badge';
        b.textContent = botType;
        var colors = { Search: '#22c55e', AI: '#8b5cf6', Preview: '#f59e0b', Lighthouse: '#3b82f6', Other: '#6b7280' };
        var c = colors[botType] || '#6b7280';
        b.style.cssText = 'background:' + c + '22;color:' + c + ';';
        return b;
    }

    function renderEventRow(ev, tbody) {
        var row = document.createElement('tr');
        var tdBot = document.createElement('td'); tdBot.textContent = ev.bot_name; tdBot.style.fontWeight = '500';
        var tdType = document.createElement('td'); tdType.appendChild(typeBadge(ev.bot_type));
        var tdUrl = document.createElement('td'); tdUrl.textContent = ev.url; tdUrl.style.cssText = 'max-width:300px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        var tdStatus = document.createElement('td'); tdStatus.appendChild(statusBadge(ev.status_code));
        var tdTime = document.createElement('td'); tdTime.textContent = new Date(ev.timestamp).toLocaleString(); tdTime.style.fontSize = '0.8rem';
        var tdRef = document.createElement('td'); tdRef.textContent = ev.referrer || '-'; tdRef.style.fontSize = '0.8rem';
        row.appendChild(tdBot); row.appendChild(tdType); row.appendChild(tdUrl);
        row.appendChild(tdStatus); row.appendChild(tdTime); row.appendChild(tdRef);
        tbody.appendChild(row);
    }

    // ── Tab: Recent Events (live feed) ──────────────────────────────

    function loadRecent() {
        fetch('/api/modules/seo/crawl-events')
            .then(function(r) { return r.json(); })
            .then(function(data) {
                clearEl(tableWrap);
                if (!data.ok || !data.events || !data.events.length) {
                    var empty = document.createElement('p');
                    empty.textContent = 'No crawl events yet. Bot visits will appear here automatically.';
                    empty.style.cssText = 'color:var(--text-secondary);text-align:center;padding:32px;';
                    tableWrap.appendChild(empty);
                    return;
                }
                var t = makeTable(['Bot', 'Type', 'URL', 'Status', 'Time', 'Referrer']);
                data.events.forEach(function(ev) { renderEventRow(ev, t.tbody); });
                tableWrap.appendChild(t.table);
                var info = document.createElement('div');
                info.style.cssText = 'margin-top:12px;font-size:0.8rem;color:var(--text-secondary);';
                info.textContent = 'Showing latest ' + data.events.length + ' of ' + data.total + ' total events';
                tableWrap.appendChild(info);
            });
    }

    // ── Tab: Crawl Log (filtered, paginated) ────────────────────────

    function loadLog() {
        var params = new URLSearchParams();
        if (botFilter.value) params.set('bot', botFilter.value);
        if (statusFilter.value) params.set('status', statusFilter.value);
        if (urlInput.value) params.set('url', urlInput.value);
        params.set('limit', '50');
        fetch('/api/modules/seo/crawl-log?' + params.toString())
            .then(function(r) { return r.json(); })
            .then(function(data) {
                clearEl(tableWrap);
                if (!data.ok || !data.events.length) {
                    var empty = document.createElement('p');
                    empty.textContent = 'No crawl events match your filters.';
                    empty.style.cssText = 'color:var(--text-secondary);text-align:center;padding:32px;';
                    tableWrap.appendChild(empty);
                    return;
                }
                var t = makeTable(['Bot', 'Type', 'URL', 'Status', 'Time', 'Referrer']);
                data.events.forEach(function(ev) { renderEventRow(ev, t.tbody); });
                tableWrap.appendChild(t.table);
                var info = document.createElement('div');
                info.style.cssText = 'margin-top:12px;font-size:0.8rem;color:var(--text-secondary);';
                info.textContent = 'Showing ' + data.events.length + ' of ' + data.total + ' events (page ' + data.page + ')';
                tableWrap.appendChild(info);
            });
    }

    // ── Tab: Missing Pages (404s) ───────────────────────────────────

    function loadMissing() {
        fetch('/api/modules/seo/missing-pages')
            .then(function(r) { return r.json(); })
            .then(function(data) {
                clearEl(tableWrap);
                if (!data.ok || !data.missing_pages || !data.missing_pages.length) {
                    var empty = document.createElement('p');
                    empty.textContent = 'No 404 errors from bots. Great SEO health!';
                    empty.style.cssText = 'color:var(--text-secondary);text-align:center;padding:32px;';
                    tableWrap.appendChild(empty);
                    return;
                }
                var t = makeTable(['URL', 'Bots', 'Hits', 'First Seen', 'Last Seen']);
                data.missing_pages.forEach(function(pg) {
                    var row = document.createElement('tr');
                    var tdUrl = document.createElement('td'); tdUrl.textContent = pg.url; tdUrl.style.fontWeight = '600';
                    var tdBots = document.createElement('td'); tdBots.textContent = pg.bots_that_tried.join(', '); tdBots.style.fontSize = '0.8rem';
                    var tdHits = document.createElement('td');
                    var hb = document.createElement('span'); hb.className = 'status-badge'; hb.textContent = String(pg.hit_count);
                    hb.style.cssText = 'background:rgba(239,68,68,0.15);color:#ef4444;';
                    tdHits.appendChild(hb);
                    var tdFirst = document.createElement('td'); tdFirst.textContent = new Date(pg.first_seen).toLocaleDateString(); tdFirst.style.fontSize = '0.8rem';
                    var tdLast = document.createElement('td'); tdLast.textContent = new Date(pg.last_seen).toLocaleDateString(); tdLast.style.fontSize = '0.8rem';
                    row.appendChild(tdUrl); row.appendChild(tdBots); row.appendChild(tdHits);
                    row.appendChild(tdFirst); row.appendChild(tdLast);
                    t.tbody.appendChild(row);
                });
                tableWrap.appendChild(t.table);
            });
    }

    // ── Tab: Page Discovery ─────────────────────────────────────────

    function loadDiscovery() {
        fetch('/api/modules/seo/page-discovery')
            .then(function(r) { return r.json(); })
            .then(function(data) {
                clearEl(tableWrap);
                if (!data.ok || !data.discovered_pages || !data.discovered_pages.length) {
                    var empty = document.createElement('p');
                    empty.textContent = 'No pages discovered by bots yet.';
                    empty.style.cssText = 'color:var(--text-secondary);text-align:center;padding:32px;';
                    tableWrap.appendChild(empty);
                    return;
                }
                var t = makeTable(['URL', 'Status', 'Bots', 'Crawl Count', 'First Seen', 'Last Seen']);
                data.discovered_pages.forEach(function(pg) {
                    var row = document.createElement('tr');
                    var tdUrl = document.createElement('td'); tdUrl.textContent = pg.url; tdUrl.style.fontWeight = '600';
                    var tdStatus = document.createElement('td'); tdStatus.appendChild(statusBadge(pg.status_code));
                    var tdBots = document.createElement('td'); tdBots.textContent = pg.bots_that_found.join(', '); tdBots.style.fontSize = '0.8rem';
                    var tdCount = document.createElement('td'); tdCount.textContent = String(pg.crawl_count);
                    var tdFirst = document.createElement('td'); tdFirst.textContent = new Date(pg.first_seen).toLocaleDateString(); tdFirst.style.fontSize = '0.8rem';
                    var tdLast = document.createElement('td'); tdLast.textContent = new Date(pg.last_seen).toLocaleDateString(); tdLast.style.fontSize = '0.8rem';
                    row.appendChild(tdUrl); row.appendChild(tdStatus); row.appendChild(tdBots);
                    row.appendChild(tdCount); row.appendChild(tdFirst); row.appendChild(tdLast);
                    t.tbody.appendChild(row);
                });
                tableWrap.appendChild(t.table);
            });
    }

    // Load initial view (Recent Events)
    loadRecent();
}
"##;

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_googlebot() {
        let ua = "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "Googlebot");
        assert_eq!(bot_type, BotType::Search);
    }

    #[test]
    fn test_detect_googlebot_image() {
        let ua = "Googlebot-Image/1.0";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "Googlebot-Image");
    }

    #[test]
    fn test_detect_gptbot() {
        let ua = "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; GPTBot/1.0; +https://openai.com/gptbot)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "GPTBot");
        assert_eq!(bot_type, BotType::AI);
    }

    #[test]
    fn test_detect_claudebot() {
        let ua = "Mozilla/5.0 (compatible; ClaudeBot/1.0; +https://claudebot.ai)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "ClaudeBot");
        assert_eq!(bot_type, BotType::AI);
    }

    #[test]
    fn test_detect_bingbot() {
        let ua = "Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "Bingbot");
        assert_eq!(bot_type, BotType::Search);
    }

    #[test]
    fn test_detect_petalbot() {
        let ua = "Mozilla/5.0 (compatible; PetalBot; +https://webmaster.petalsearch.com/)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "PetalBot");
    }

    #[test]
    fn test_detect_chrome_lighthouse() {
        let ua = "Mozilla/5.0 Chrome-Lighthouse";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "Chrome-Lighthouse");
        assert_eq!(bot_type, BotType::Lighthouse);
    }

    #[test]
    fn test_detect_chatgpt_user() {
        let ua = "Mozilla/5.0 AppleWebKit/537.36 (KHTML, like Gecko; compatible; ChatGPT-User/1.0; +https://openai.com/bot)";
        let result = detect_bot(ua);
        assert!(result.is_some());
        let (name, bot_type) = result.unwrap();
        assert_eq!(name, "ChatGPT-User");
        assert_eq!(bot_type, BotType::AI);
    }

    #[test]
    fn test_detect_normal_browser_returns_none() {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
        assert!(detect_bot(ua).is_none());
    }

    #[test]
    fn test_detect_empty_ua_returns_none() {
        assert!(detect_bot("").is_none());
    }

    #[test]
    fn test_crawl_event_serialization_roundtrip() {
        let event = CrawlEvent {
            bot_name: "Googlebot".into(),
            bot_type: BotType::Search,
            url: "/pest-control".into(),
            status_code: 200,
            timestamp: "2026-03-20T10:00:00Z".into(),
            user_agent: "Googlebot/2.1".into(),
            referrer: None,
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        let decoded: CrawlEvent = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.bot_name, "Googlebot");
        assert_eq!(decoded.status_code, 200);
        assert_eq!(decoded.bot_type, BotType::Search);
    }

    #[test]
    fn test_crawl_event_with_referrer() {
        let event = CrawlEvent {
            bot_name: "GPTBot".into(),
            bot_type: BotType::AI,
            url: "/blog/travel-tips".into(),
            status_code: 200,
            timestamp: "2026-03-20T10:00:00Z".into(),
            user_agent: "GPTBot/1.0".into(),
            referrer: Some("https://chat.openai.com".into()),
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        let decoded: CrawlEvent = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            decoded.referrer,
            Some("https://chat.openai.com".to_string())
        );
    }

    #[test]
    fn test_bot_type_display() {
        assert_eq!(BotType::Search.to_string(), "Search");
        assert_eq!(BotType::AI.to_string(), "AI");
        assert_eq!(BotType::Preview.to_string(), "Preview");
        assert_eq!(BotType::Lighthouse.to_string(), "Lighthouse");
        assert_eq!(BotType::Other.to_string(), "Other");
    }

    #[test]
    fn test_detect_case_insensitive() {
        let ua = "GOOGLEBOT/2.1";
        let result = detect_bot(ua);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "Googlebot");
    }

    #[test]
    fn test_detect_social_bots() {
        assert_eq!(detect_bot("Twitterbot/1.0").unwrap().0, "TwitterBot");
        assert_eq!(detect_bot("facebookexternalhit/1.1").unwrap().0, "Facebook");
        assert_eq!(detect_bot("LinkedInBot/1.0").unwrap().0, "LinkedInBot");
        assert_eq!(
            detect_bot("Slackbot-LinkExpanding 1.0").unwrap().0,
            "Slackbot"
        );
        assert_eq!(detect_bot("Discordbot/2.0").unwrap().0, "Discordbot");
    }

    #[test]
    fn test_detect_seo_tool_bots() {
        assert_eq!(detect_bot("SemrushBot/7").unwrap().0, "SemrushBot");
        assert_eq!(detect_bot("AhrefsBot/7.0").unwrap().0, "AhrefsBot");
    }
}
