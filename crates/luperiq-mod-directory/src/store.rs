//! `DirectoryStore` — SQLite-backed singleton for the pest-control company directory.
//!
//! ## Singleton model
//! Two `OnceLock<RwLock<…>>` globals (`EXCLUDE_CFG`, `COMPANY_OVERRIDES`) hold
//! process-wide display configuration and per-company overrides.  The store itself
//! is held in `lib::DIR_STORE` as an `Arc<DirectoryStore>`.
//!
//! ## Analytics dual-leg architecture
//! Page-view tracking writes to the raw `page_views` table.  A nightly rollup
//! script aggregates completed days into `page_views_daily`.  All stats queries
//! union both legs — rollup rows (`day < MAX(day)`) plus raw rows
//! (`ts >= midnight(MAX(day))`) — to avoid double-counting while keeping hot-path
//! writes to the small raw table only.

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, RwLock};

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryCompany {
    pub id: String,
    pub state_abbr: String,
    pub state_name: String,
    // Location
    pub city: Option<String>,
    pub city_slug: Option<String>,
    pub is_county_location: bool,
    pub county: Option<String>,
    // Names
    pub entity_name: String,
    pub dba: Option<String>,
    pub company_slug: String,
    // Contact
    pub phone: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    // Address
    pub address: Option<String>,
    // SOS data
    pub entity_type: Option<String>,
    pub formation_date: Option<String>,
    pub status: Option<String>,
    pub expiration_date: Option<String>,
    pub file_number: Option<String>,
    pub registered_agent: Option<String>,
    pub agent_address: Option<String>,
    // Pest license data
    pub pest_license_num: Option<String>,
    pub pest_license_type: Option<String>,
    pub pest_categories: Option<String>,
    pub pest_categories_decoded: Option<String>,
    pub pest_license_expires: Option<String>,
    pub pest_operator: Option<String>,
    pub pest_source_url: Option<String>,
    // Staff counts
    pub applicator_count: i64,
    pub technician_count: i64,
    pub apprentice_count: i64,
    // Source links
    pub source: String,
    pub source_url: Option<String>,
    pub sos_lookup_url: Option<String>,
    pub pest_lookup_url: Option<String>,
    // Enrichment / claiming
    pub listing_tier: i64,
    pub claimed_by: Option<String>,
    pub rating_count: i64,
    pub rating_sum: i64,
    // Computed fields for Tera
    pub is_active: bool,
    pub has_pest_license: bool,
    pub avg_rating: Option<f64>,
    pub formation_year: Option<String>,
    pub state_regs_url: Option<String>,
    pub staff_summary: Option<String>,
    // Human-readable date display variants (e.g. "August 31, 2026")
    pub pest_license_expires_display: Option<String>,
    pub formation_date_display: Option<String>,
    pub expiration_date_display: Option<String>,
    pub pest_license_issued: Option<String>,
    pub pest_license_issued_display: Option<String>,
    pub pest_license_renewed: Option<String>,
    pub pest_license_renewed_display: Option<String>,
    pub pest_insurance_expires: Option<String>,
    pub pest_insurance_expires_display: Option<String>,
    pub pest_responsible_applicator: Option<String>,
    pub pest_responsible_applicator_license: Option<String>,
    pub pest_spcb_id: Option<String>,
    // Phase 1 (directory hardening): canary/honeypot flag, additive
    pub is_canary: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryOfficer {
    pub role: Option<String>,
    pub full_name: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryApplicator {
    pub full_name: String,
    pub license_num: Option<String>,
    pub license_type: Option<String>,
    pub categories_decoded: Option<String>,
    pub expires: Option<String>,
    pub expires_display: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryCity {
    pub state_abbr: String,
    pub city_slug: String,
    pub city_name: String,
    pub is_county: bool,
    pub company_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryState {
    pub state_abbr: String,
    pub state_name: String,
    pub company_count: i64,
    pub city_count: i64,
}

#[derive(Debug, Serialize)]
pub struct DirStatRow {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct DirStats {
    pub total_views: i64,
    pub total_clicks: i64,
    pub by_page_type: Vec<DirStatRow>,
    pub by_state: Vec<DirStatRow>,
    pub top_cities: Vec<DirStatRow>,
    pub top_companies: Vec<DirStatRow>,
    pub by_click_type: Vec<DirStatRow>,
    pub top_clicked_companies: Vec<DirStatRow>,
    pub daily_views: Vec<DirStatRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryMiniSiteBlock {
    pub block_type: String,
    #[serde(default)]
    pub heading: Option<String>,
    #[serde(default)]
    pub subheading: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub cta_text: Option<String>,
    #[serde(default)]
    pub cta_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DirectoryMiniSitePage {
    pub page_slug: String,
    #[serde(default)]
    pub page_title: Option<String>,
    #[serde(default)]
    pub blocks: Vec<DirectoryMiniSiteBlock>,
}

/// One node of a company's recovered navigation (from the crawl). Renders as the
/// "your upgraded site's menu" preview on the directory listing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompanyNavItem {
    pub item_id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub position: u32,
}

const COMPANY_COLS: &str = "
    id, state_abbr, COALESCE(state_name,'') as state_name, city, city_slug,
    is_county_location, county, entity_name, dba, company_slug,
    phone, email, website, address, entity_type, formation_date, status,
    expiration_date, file_number, registered_agent, agent_address,
    pest_license_num, pest_license_type, pest_categories, pest_categories_decoded,
    pest_license_expires, pest_operator, pest_source_url,
    applicator_count, technician_count, apprentice_count,
    source, source_url, sos_lookup_url, pest_lookup_url,
    listing_tier, claimed_by, rating_count, rating_sum,
    pest_license_issued, pest_license_renewed, pest_insurance_expires,
    pest_responsible_applicator, pest_responsible_applicator_license, pest_spcb_id,
    is_canary";

// Phase 0 (2026-05-27): replaced with indexed is_chain=0 filter (populated at migration
// time from the same LIKE patterns originally listed here). To restore a chain to the
// directory now: UPDATE companies SET is_chain=0 WHERE id=...
// Original (kept for reference):
// const CHAIN_NAMES_FILTER: &str = " AND LOWER(entity_name) NOT LIKE '%terminix%' AND LOWER(entity_name) NOT LIKE '%orkin%' AND LOWER(entity_name) NOT LIKE '%rollins%' AND LOWER(entity_name) NOT LIKE '%rentokil%' AND LOWER(entity_name) NOT LIKE '%anticimex%' AND LOWER(entity_name) NOT LIKE '%truly nolen%' AND LOWER(entity_name) NOT LIKE '%home team%' AND LOWER(entity_name) NOT LIKE '%western pest%' AND LOWER(entity_name) NOT LIKE '%ehrlich%' AND LOWER(entity_name) NOT LIKE '%massey services%'";
const CHAIN_NAMES_FILTER: &str = " AND is_chain = 0";

const NONCOMMERCIAL_SQL: &str = " AND LOWER(COALESCE(pest_license_type,'')) NOT LIKE '%noncommercial%'";
const FOREIGN_ENTITY_SQL: &str = " AND UPPER(COALESCE(entity_type,'')) NOT LIKE '%FOREIGN%'";
const INDIVIDUAL_FILTER: &str = " AND (entity_type IS NOT NULL AND entity_type != '')";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DirectoryExcludeConfig {
    pub hide_noncommercial: bool,
    pub hide_foreign_entity: bool,
    pub hide_chain_names: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyOverride {
    pub hidden: bool,
    pub featured: bool,
    pub upgrade_type: Option<String>,
    pub tenant_slug: Option<String>,
    pub custom_name: Option<String>,
    pub custom_phone: Option<String>,
    pub custom_email: Option<String>,
    pub custom_website: Option<String>,
    pub custom_description: Option<String>,
    pub operator_notes: Option<String>,
    pub updated_at: i64,
}

static EXCLUDE_CFG: OnceLock<RwLock<HashMap<String, DirectoryExcludeConfig>>> = OnceLock::new();
static COMPANY_OVERRIDES: OnceLock<RwLock<HashMap<String, CompanyOverride>>> = OnceLock::new();

pub fn set_exclude_config(industry: &str, cfg: DirectoryExcludeConfig) {
    let lock = EXCLUDE_CFG.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut w) = lock.write() { w.insert(industry.to_string(), cfg); }
}

pub fn get_exclude_config_for(industry: &str) -> DirectoryExcludeConfig {
    EXCLUDE_CFG.get()
        .and_then(|l| l.read().ok().map(|r| r.get(industry).cloned().unwrap_or_default()))
        .unwrap_or_default()
}

pub fn get_exclude_config() -> DirectoryExcludeConfig {
    get_exclude_config_for("pest-control")
}

pub fn set_company_overrides(overrides: HashMap<String, CompanyOverride>) {
    let lock = COMPANY_OVERRIDES.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut w) = lock.write() { *w = overrides; }
}

pub fn update_company_override(id: String, ov: CompanyOverride) {
    let lock = COMPANY_OVERRIDES.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut w) = lock.write() { w.insert(id, ov); }
}

pub fn remove_company_override(id: &str) {
    if let Some(lock) = COMPANY_OVERRIDES.get() {
        if let Ok(mut w) = lock.write() { w.remove(id); }
    }
}

pub fn get_all_company_overrides() -> HashMap<String, CompanyOverride> {
    COMPANY_OVERRIDES.get()
        .and_then(|l| l.read().ok().map(|r| r.clone()))
        .unwrap_or_default()
}

fn build_display_filter() -> String {
    let cfg = get_exclude_config();
    let mut f = INDIVIDUAL_FILTER.to_string();
    if cfg.hide_chain_names { f.push_str(CHAIN_NAMES_FILTER); }
    if cfg.hide_noncommercial { f.push_str(NONCOMMERCIAL_SQL); }
    if cfg.hide_foreign_entity { f.push_str(FOREIGN_ENTITY_SQL); }
    f
}

fn apply_overrides(companies: Vec<DirectoryCompany>) -> Vec<DirectoryCompany> {
    let overrides = get_all_company_overrides();
    if overrides.is_empty() { return companies; }
    companies.into_iter().filter_map(|mut c| {
        if let Some(ov) = overrides.get(&c.id) {
            if ov.hidden { return None; }
            if let Some(ref v) = ov.custom_name { c.entity_name = v.clone(); }
            if let Some(ref v) = ov.custom_phone { c.phone = Some(v.clone()); }
            if let Some(ref v) = ov.custom_email { c.email = Some(v.clone()); }
            if let Some(ref v) = ov.custom_website { c.website = Some(v.clone()); }
            if ov.featured { c.listing_tier = c.listing_tier.max(100); }
            if ov.upgrade_type.is_some() { c.listing_tier = c.listing_tier.max(5); }
        }
        Some(c)
    }).collect()
}

fn addr_in_state(state: &str) -> String {
    format!(
        " AND (address IS NULL OR address = '' \
         OR UPPER(address) LIKE '%, {state}, %' \
         OR UPPER(address) LIKE '%, {state}')"
    )
}


/// A directory ownership claim row (Phase 4). `verified=true` rows unlock the
/// Owner viewer tier. `token_expiry` is the unix-secs deadline for the pending
/// verification link (None once verified). Serialized into the my-listings
/// dashboard context.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryClaim {
    pub company_id: String,
    pub user_id: String,
    pub email: String,
    pub attested_at: i64,
    pub verified: bool,
    pub token_expiry: Option<i64>,
    pub attest_legal: bool,
    pub newsletter: bool,
}

pub struct DirectoryStore {
    conn: Mutex<Connection>,
}

impl DirectoryStore {
    pub fn open(db_path: &str) -> Result<Self, String> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| format!("directory db open failed: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=3000;")
            .map_err(|e| format!("pragma failed: {e}"))?;
        // Phase 0 tuning (2026-05-27): larger cache, mmap, less-frequent autocheckpoint.
        // cache_size negative = KB; -262144 = 256 MB page cache.
        // mmap_size 1 GiB memory-maps the ~545 MB DB with growth headroom (right-sized from 4 GiB 2026-05-28).
        // wal_autocheckpoint 4000 pages (16 MB at 4 KB/page) reduces fsync churn on writes.
        conn.execute_batch(
            "PRAGMA cache_size = -262144;
             PRAGMA mmap_size = 1073741824;
             PRAGMA wal_autocheckpoint = 4000;"
        ).map_err(|e| format!("pragma tune: {e}"))?;
        // Create analytics tables if not present (v1 → v2 migration)
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS page_views (
                id INTEGER PRIMARY KEY, page_type TEXT NOT NULL, state_abbr TEXT,
                city_slug TEXT, company_slug TEXT, ip_hash TEXT, referer TEXT, ts INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS idx_pv_ts ON page_views(ts);
            CREATE TABLE IF NOT EXISTS click_events (
                id INTEGER PRIMARY KEY, event_type TEXT NOT NULL, company_id TEXT,
                state_abbr TEXT, city_slug TEXT, company_slug TEXT, ip_hash TEXT, ts INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS idx_ce_ts ON click_events(ts);",
        );
        // Create newsletter_signups table at open time (not on every signup call)
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS newsletter_signups (id INTEGER PRIMARY KEY AUTOINCREMENT,email TEXT NOT NULL UNIQUE,name TEXT,company TEXT,state TEXT,tier TEXT NOT NULL DEFAULT 'insider',listing_upgraded INTEGER NOT NULL DEFAULT 0,created_at INTEGER NOT NULL);",
        );
        // Phase 2 (directory hardening, 2026-06-12): engagement_events tracks
        // contact-reveal interactions (phone/email/website). Idempotent DDL,
        // additive — created here so a fresh DB and a re-run are both no-ops.
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS engagement_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                ip_hash TEXT NOT NULL,
                ts INTEGER NOT NULL,
                user_id TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_ee_company ON engagement_events(company_id);
            CREATE INDEX IF NOT EXISTS idx_ee_ts ON engagement_events(ts);",
        );
        // Phase 1 (directory hardening, 2026-06-12): additive is_canary flag.
        // SQLite lacks ADD COLUMN IF NOT EXISTS; check pragma table_info first so a
        // re-run (column already present) is a no-op rather than an error.
        {
            let has_is_canary = conn
                .prepare("PRAGMA table_info(companies)")
                .and_then(|mut stmt| {
                    let names: Vec<String> = stmt
                        .query_map([], |r| r.get::<_, String>(1))?
                        .filter_map(|r| r.ok())
                        .collect();
                    Ok(names.iter().any(|n| n == "is_canary"))
                })
                .unwrap_or(false);
            if !has_is_canary {
                if let Err(e) = conn.execute_batch(
                    "ALTER TABLE companies ADD COLUMN is_canary INTEGER NOT NULL DEFAULT 0;",
                ) {
                    tracing::warn!("[directory] is_canary migration skipped: {e}");
                }
            }
        }
        // Phase 3 (directory hardening, 2026-06-12): directory_claims backs the
        // Owner tier so a logged-in owner of a *verified* claim sees the full,
        // unmasked record. The claim *submission UI* lands in Phase 4 — this DDL
        // exists now so `is_verified_owner` resolves against a real table.
        // Idempotent + additive: a fresh DB and a re-run are both no-ops.
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS directory_claims (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id TEXT NOT NULL, user_id TEXT NOT NULL, email TEXT NOT NULL,
                attested_at INTEGER NOT NULL, verified INTEGER NOT NULL DEFAULT 0,
                verify_token TEXT, token_expiry INTEGER,
                attest_legal INTEGER NOT NULL DEFAULT 0, newsletter INTEGER NOT NULL DEFAULT 0,
                ip_hash TEXT, UNIQUE(company_id, user_id));
            CREATE INDEX IF NOT EXISTS idx_dc_user ON directory_claims(user_id);
            CREATE INDEX IF NOT EXISTS idx_dc_company ON directory_claims(company_id);",
        );
        // Mini-site pages (the per-company tabs: contact/services/about + custom).
        // Historically seeded directly into the DB with no app writer; this DDL makes a
        // fresh DB self-sufficient and the editor/import paths additive. Idempotent.
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS company_pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id TEXT NOT NULL,
                page_slug TEXT NOT NULL,
                page_title TEXT,
                blocks_json TEXT NOT NULL DEFAULT '[]');
            CREATE INDEX IF NOT EXISTS idx_cp_company ON company_pages(company_id);",
        );
        // Recovered company navigation (from the crawl) for the upgrade-menu preview.
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS company_nav (
                company_id TEXT PRIMARY KEY,
                nav_json TEXT NOT NULL,
                source TEXT);",
        );
        // Upgrade requests: an owner/operator asking to turn a listing into a full website.
        // Captures intent + carries the company_id; provisioning consumes the upgrade bundle.
        let _ = conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS upgrade_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                company_id TEXT NOT NULL,
                requested_by TEXT,
                source TEXT NOT NULL DEFAULT 'owner',
                status TEXT NOT NULL DEFAULT 'requested',
                note TEXT,
                requested_at INTEGER NOT NULL);
            CREATE INDEX IF NOT EXISTS idx_ur_company ON upgrade_requests(company_id);",
        );
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn now_secs() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }

    // ── Analytics ────────────────────────────────────────────────────────────

    pub fn track_view(
        &self,
        page_type: &str,
        state_abbr: Option<&str>,
        city_slug: Option<&str>,
        company_slug: Option<&str>,
        ip_hash: &str,
        referer: Option<&str>,
    ) {
        let conn = self.conn();
        let _ = conn.execute(
            "INSERT INTO page_views (page_type,state_abbr,city_slug,company_slug,ip_hash,referer,ts) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![page_type, state_abbr, city_slug, company_slug, ip_hash, referer, Self::now_secs()],
        );
    }

    pub fn track_click(
        &self,
        event_type: &str,
        company_id: Option<&str>,
        state_abbr: Option<&str>,
        city_slug: Option<&str>,
        company_slug: Option<&str>,
        ip_hash: &str,
    ) {
        let conn = self.conn();
        let _ = conn.execute(
            "INSERT INTO click_events (event_type,company_id,state_abbr,city_slug,company_slug,ip_hash,ts) \
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![event_type, company_id, state_abbr, city_slug, company_slug, ip_hash, Self::now_secs()],
        );
    }

    pub fn stats(&self) -> DirStats {
        let conn = self.conn();

        // page_views_daily holds aggregated rows for days strictly less than the
        // rollup cron's cutoff. The rollup script uses a sliding `ts < now-1day`
        // cutoff, so the MAX(day) row in the rollup is permanently partial —
        // any day with `day < MAX(day)` is fully rolled and complete.
        //
        // Boundary semantics (post-Phase 0 cutover, 2026-05-28):
        //   - rollup leg: page_views_daily WHERE day < MAX(day) in rollup
        //   - raw    leg: page_views       WHERE ts  >= midnight(MAX(day))
        // Together they cover every row in page_views exactly once.
        //
        // Edge case: empty rollup → MAX(day) is NULL → fall back to all-raw.
        let cutoff_day: Option<String> = conn
            .query_row("SELECT MAX(day) FROM page_views_daily", [], |r| r.get::<_, Option<String>>(0))
            .unwrap_or(None);

        let total_clicks: i64 = conn
            .query_row("SELECT COUNT(*) FROM click_events", [], |r| r.get(0))
            .unwrap_or(0);

        // Click events are not affected by the rollup — small table, same SQL.
        let by_click_type = query_rows(
            &conn,
            "SELECT event_type, COUNT(*) FROM click_events \
             GROUP BY event_type ORDER BY 2 DESC",
        );
        let top_clicked_companies = query_rows(
            &conn,
            "SELECT COALESCE(state_abbr,'')||'/'||COALESCE(city_slug,'')||'/'||COALESCE(company_slug,'')||' ('||event_type||')', \
             COUNT(*) FROM click_events \
             GROUP BY state_abbr,city_slug,company_slug,event_type \
             ORDER BY 2 DESC LIMIT 20",
        );

        // Helpers below assume the cutoff is a TEXT 'YYYY-MM-DD'. If absent
        // or malformed, we hand the legs a sentinel in the distant past:
        //   - rollup leg `day < '1970-01-01'` matches nothing (rollup is empty
        //     in this branch anyway, but stays correct if not).
        //   - raw    leg `ts >= strftime('%s','1970-01-01')` = `ts >= 0`
        //     matches every row, so stats fall back to raw-only behaviour
        //     equivalent to the pre-rollup implementation.
        // The strict shape check also forecloses any SQL-injection risk from
        // `format!`-interpolating the cutoff into the union queries below.
        let cutoff: &str = match cutoff_day.as_deref() {
            Some(d) if is_valid_day(d) => d,
            _ => "1970-01-01",
        };

        let total_views = query_count(&conn, &format!(
            "SELECT \
               (SELECT COALESCE(SUM(view_count),0) FROM page_views_daily WHERE day < '{c}') \
             + (SELECT COUNT(*)              FROM page_views WHERE ts >= strftime('%s','{c}'))",
            c = cutoff,
        ));

        let by_page_type = query_rows(&conn, &page_type_union_sql(cutoff));
        let by_state = query_rows(&conn, &by_state_union_sql(cutoff));
        let top_cities = query_rows(&conn, &top_cities_union_sql(cutoff));
        let top_companies = query_rows(&conn, &top_companies_union_sql(cutoff));
        let daily_views = query_rows(&conn, &daily_views_union_sql(cutoff));

        DirStats {
            total_views,
            total_clicks,
            by_page_type,
            by_state,
            top_cities,
            top_companies,
            by_click_type,
            top_clicked_companies,
            daily_views,
        }
    }

    // ── Directory queries ────────────────────────────────────────────────────

    pub fn all_states(&self) -> Vec<DirectoryState> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT c.state_abbr, COALESCE(c.state_name, c.state_abbr) as state_name, \
             COUNT(*) as company_count, COUNT(DISTINCT c.city_slug) as city_count \
             FROM companies c \
             WHERE entity_type IS NOT NULL AND entity_type != '' \
             GROUP BY c.state_abbr, c.state_name \
             ORDER BY company_count DESC",
        ).unwrap();
        stmt.query_map([], |row| {
            Ok(DirectoryState {
                state_abbr: row.get(0)?,
                state_name: row.get(1)?,
                company_count: row.get(2)?,
                city_count: row.get(3)?,
            })
        }).unwrap().flatten().collect()
    }

    pub fn total_counts(&self) -> (i64, i64) {
        let conn = self.conn();
        let companies: i64 = conn.query_row("SELECT COUNT(*) FROM companies WHERE entity_type IS NOT NULL AND entity_type != ''", [], |r| r.get(0)).unwrap_or(0);
        let cities: i64 = conn.query_row("SELECT COUNT(*) FROM cities", [], |r| r.get(0)).unwrap_or(0);
        (companies, cities)
    }

    pub fn cities_for_state(&self, state: &str) -> Vec<DirectoryCity> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT state_abbr, city_slug, city_name, is_county, company_count \
             FROM cities WHERE state_abbr=?1 ORDER BY company_count DESC",
        ).unwrap();
        stmt.query_map([state], |row| {
            Ok(DirectoryCity {
                state_abbr: row.get(0)?,
                city_slug: row.get(1)?,
                city_name: row.get(2)?,
                is_county: row.get::<_, i64>(3).map(|v| v != 0).unwrap_or(false),
                company_count: row.get(4)?,
            })
        }).unwrap().flatten().collect()
    }

    pub fn state_totals(&self, state: &str) -> i64 {
        let conn = self.conn();
        conn.query_row("SELECT COUNT(*) FROM companies WHERE state_abbr=?1 AND entity_type IS NOT NULL AND entity_type != ''", [state], |r| r.get(0)).unwrap_or(0)
    }

    pub fn companies_for_city(
        &self,
        state: &str,
        city_slug: &str,
        page: u32,
        per_page: u32,
    ) -> (Vec<DirectoryCompany>, i64) {
        let conn = self.conn();
        let offset = page * per_page;
        let addr = addr_in_state(state);
        let display_filter = build_display_filter();
        let count_sql = format!("SELECT COUNT(*) FROM companies WHERE state_abbr=?1 AND city_slug=?2 {display_filter}{addr}");
        let total: i64 = conn.query_row(&count_sql, [state, city_slug], |r| r.get(0)).unwrap_or(0);

        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE state_abbr=?1 AND city_slug=?2 {display_filter}{addr} \
             ORDER BY listing_tier DESC, \
               CASE WHEN pest_license_num IS NOT NULL THEN 1 ELSE 0 END DESC, \
               applicator_count DESC \
             LIMIT ?3 OFFSET ?4"
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        let raw: Vec<DirectoryCompany> = stmt.query_map(rusqlite::params![state, city_slug, per_page, offset], row_to_company)
            .unwrap().flatten().collect();
        (apply_overrides(raw), total)
    }

    pub fn company_by_slug(&self, state: &str, city_slug: &str, company_slug: &str) -> Option<DirectoryCompany> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE state_abbr=?1 AND city_slug=?2 AND company_slug=?3 LIMIT 1"
        );
        conn.query_row(&sql, rusqlite::params![state, city_slug, company_slug], row_to_company).ok()
    }

    /// Resolve a company that has no usable city (city_slug NULL/empty) by
    /// (state, company_slug). Backs the cityless fallback route
    /// `/directory/{state}/{company}` so location-less listings are reachable.
    /// Industry-agnostic — no pest-specific columns referenced.
    pub fn company_by_slug_statewide(&self, state: &str, company_slug: &str) -> Option<DirectoryCompany> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE state_abbr=?1 AND company_slug=?2 \
             AND (city_slug IS NULL OR city_slug = '') LIMIT 1"
        );
        conn.query_row(&sql, rusqlite::params![state, company_slug], row_to_company).ok()
    }

    /// Resolve a company by (state, company_slug) regardless of city. Used by
    /// the cityless route to detect a listing that has SINCE gained a city
    /// (via Phase 1 recovery) so the 2-segment URL can 301 to canonical.
    pub fn company_by_state_and_slug_any_city(&self, state: &str, company_slug: &str) -> Option<DirectoryCompany> {
        let conn = self.conn();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE state_abbr=?1 AND company_slug=?2 \
             AND city_slug IS NOT NULL AND city_slug != '' LIMIT 1"
        );
        conn.query_row(&sql, rusqlite::params![state, company_slug], row_to_company).ok()
    }

    /// Returns true if the given slug is a known city in the state (has at
    /// least one company with that city_slug). Used by the state/{slug}
    /// handler to disambiguate a city segment from a company segment.
    pub fn is_known_city_slug(&self, state: &str, slug: &str) -> bool {
        let conn = self.conn();
        conn.query_row(
            "SELECT 1 FROM cities WHERE state_abbr=?1 AND city_slug=?2 LIMIT 1",
            rusqlite::params![state, slug],
            |_| Ok(()),
        ).is_ok()
    }

    /// Companies in a state with no usable city — the "Statewide / Location
    /// not specified" bucket on the state page. Paged, honors display filters.
    pub fn cityless_companies_for_state(
        &self,
        state: &str,
        page: u32,
        per_page: u32,
    ) -> (Vec<DirectoryCompany>, i64) {
        let conn = self.conn();
        let offset = page * per_page;
        let display_filter = build_display_filter();
        let count_sql = format!(
            "SELECT COUNT(*) FROM companies WHERE state_abbr=?1 \
             AND (city_slug IS NULL OR city_slug = '') {display_filter}"
        );
        let total: i64 = conn.query_row(&count_sql, [state], |r| r.get(0)).unwrap_or(0);
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies WHERE state_abbr=?1 \
             AND (city_slug IS NULL OR city_slug = '') {display_filter} \
             ORDER BY listing_tier DESC, entity_name LIMIT ?2 OFFSET ?3"
        );
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return (vec![], total) };
        let raw: Vec<DirectoryCompany> = stmt
            .query_map(rusqlite::params![state, per_page, offset], row_to_company)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default();
        (apply_overrides(raw), total)
    }

    /// (state, company_slug) tuples for cityless companies — feeds the sitemap
    /// so search engines get the working `/directory/{state}/{company}` form.
    pub fn cityless_company_slug_tuples(&self) -> Vec<(String, String)> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT DISTINCT state_abbr, company_slug FROM companies \
             WHERE (city_slug IS NULL OR city_slug = '') \
             AND company_slug IS NOT NULL AND company_slug != '' \
             ORDER BY state_abbr, company_slug",
        ) { Ok(s) => s, Err(_) => return vec![] };
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default()
    }

    pub fn featured_companies(&self, state: &str, limit: u32) -> Vec<DirectoryCompany> {
        let conn = self.conn();
        let addr = addr_in_state(state);
        let display_filter = build_display_filter();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies WHERE state_abbr=?1 {display_filter}{addr} \
             ORDER BY listing_tier DESC, \
               CASE WHEN pest_license_num IS NOT NULL THEN 1 ELSE 0 END DESC, \
               applicator_count DESC \
             LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        let raw: Vec<DirectoryCompany> = stmt.query_map(rusqlite::params![state, limit], row_to_company)
            .unwrap().flatten().collect();
        apply_overrides(raw)
    }

    pub fn newest_for_state(&self, state: &str, limit: u32) -> Vec<DirectoryCompany> {
        let conn = self.conn();
        let addr = addr_in_state(state);
        let display_filter = build_display_filter();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies WHERE state_abbr=?1 AND formation_date IS NOT NULL AND length(formation_date) >= 4 {display_filter}{addr} ORDER BY formation_date DESC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        let raw: Vec<DirectoryCompany> = stmt.query_map(rusqlite::params![state, limit], row_to_company)
            .unwrap().flatten().collect();
        apply_overrides(raw)
    }

    pub fn oldest_for_state(&self, state: &str, limit: u32) -> Vec<DirectoryCompany> {
        let conn = self.conn();
        let addr = addr_in_state(state);
        let display_filter = build_display_filter();
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies WHERE state_abbr=?1 AND formation_date IS NOT NULL AND length(formation_date) >= 4 {display_filter}{addr} ORDER BY formation_date ASC LIMIT ?2"
        );
        let mut stmt = conn.prepare(&sql).unwrap();
        let raw: Vec<DirectoryCompany> = stmt.query_map(rusqlite::params![state, limit], row_to_company)
            .unwrap().flatten().collect();
        apply_overrides(raw)
    }

    pub fn officers_for_company(&self, company_id: &str) -> Vec<DirectoryOfficer> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT role, full_name, address FROM officers WHERE company_id=?1 ORDER BY role"
        ).unwrap();
        stmt.query_map([company_id], |row| {
            Ok(DirectoryOfficer {
                role: row.get(0)?,
                full_name: row.get(1)?,
                address: row.get(2)?,
            })
        }).unwrap().flatten().collect()
    }

    pub fn applicators_for_company(&self, company_id: &str) -> Vec<DirectoryApplicator> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT full_name, license_num, license_type, categories_decoded, expires \
             FROM applicators WHERE company_id=?1 \
             ORDER BY license_type, full_name"
        ).unwrap();
        stmt.query_map([company_id], |row| {
            Ok(DirectoryApplicator {
                full_name: row.get(0)?,
                license_num: row.get(1)?,
                license_type: row.get(2)?,
                categories_decoded: row.get(3)?,
                expires: row.get(4)?,
                expires_display: fmt_date(row.get::<_, Option<String>>(4).ok().flatten().as_deref()),
            })
        }).unwrap().flatten().collect()
    }

    pub fn company_nav_items(&self, company_id: &str) -> Vec<CompanyNavItem> {
        let conn = self.conn();
        let json: String = match conn.query_row(
            "SELECT nav_json FROM company_nav WHERE company_id=?1",
            [company_id],
            |row| row.get(0),
        ) {
            Ok(j) => j,
            Err(_) => return Vec::new(),
        };
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub fn mini_site_pages_for(&self, company_id: &str) -> Vec<DirectoryMiniSitePage> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT page_slug, page_title, blocks_json FROM company_pages WHERE company_id=?1 ORDER BY id"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([company_id], |row| {
            let blocks_json: String = row.get(2).unwrap_or_default();
            let blocks = parse_mini_site_blocks(&blocks_json);
            Ok(DirectoryMiniSitePage {
                page_slug: row.get(0)?,
                page_title: row.get(1)?,
                blocks,
            })
        }).map(|rows| rows.flatten().collect()).unwrap_or_default()
    }

    pub fn mini_site_page_for(&self, company_id: &str, page_slug: &str) -> Option<DirectoryMiniSitePage> {
        let conn = self.conn();
        conn.query_row(
            "SELECT page_slug, page_title, blocks_json FROM company_pages WHERE company_id=?1 AND page_slug=?2",
            rusqlite::params![company_id, page_slug],
            |row| {
                let blocks_json: String = row.get(2).unwrap_or_default();
                let blocks = parse_mini_site_blocks(&blocks_json);
                Ok(DirectoryMiniSitePage {
                    page_slug: row.get(0)?,
                    page_title: row.get(1)?,
                    blocks,
                })
            },
        ).ok()
    }

    // ── Mini-site page writers (the editor + import paths; previously missing) ──────────

    /// Create or replace a single mini-site page (tab) for a company. Keyed by
    /// (company_id, page_slug): an existing page with that slug is replaced, so this is a
    /// safe upsert without a unique index. `slug` is normalized to a url-safe token.
    pub fn upsert_mini_site_page(
        &self,
        company_id: &str,
        page_slug: &str,
        page_title: Option<&str>,
        blocks: &[DirectoryMiniSiteBlock],
    ) -> Result<(), String> {
        let slug = normalize_page_slug(page_slug);
        if slug.is_empty() {
            return Err("page_slug is empty after normalization".into());
        }
        let blocks_json = blocks_to_storage_json(blocks);
        let conn = self.conn();
        conn.execute(
            "DELETE FROM company_pages WHERE company_id=?1 AND page_slug=?2",
            rusqlite::params![company_id, slug],
        )
        .map_err(|e| format!("page delete-before-insert failed: {e}"))?;
        conn.execute(
            "INSERT INTO company_pages (company_id, page_slug, page_title, blocks_json) VALUES (?1,?2,?3,?4)",
            rusqlite::params![company_id, slug, page_title, blocks_json],
        )
        .map_err(|e| format!("page insert failed: {e}"))?;
        Ok(())
    }

    /// Delete one mini-site page (tab). Returns rows affected.
    pub fn delete_mini_site_page(&self, company_id: &str, page_slug: &str) -> usize {
        let conn = self.conn();
        conn.execute(
            "DELETE FROM company_pages WHERE company_id=?1 AND page_slug=?2",
            rusqlite::params![company_id, page_slug],
        )
        .unwrap_or(0)
    }

    /// All mini-site pages across the whole directory, as (company_id, page). Used by the
    /// site-wide export. Ordered by company then insertion so a re-export is stable.
    pub fn all_mini_site_pages(&self) -> Vec<(String, DirectoryMiniSitePage)> {
        let conn = self.conn();
        let mut stmt = match conn.prepare(
            "SELECT company_id, page_slug, page_title, blocks_json FROM company_pages ORDER BY company_id, id",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| {
            let blocks_json: String = row.get(3).unwrap_or_default();
            Ok((
                row.get::<_, String>(0)?,
                DirectoryMiniSitePage {
                    page_slug: row.get(1)?,
                    page_title: row.get(2)?,
                    blocks: parse_mini_site_blocks(&blocks_json),
                },
            ))
        })
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    /// Record an "upgrade to full website" request. Returns the new row id.
    pub fn record_upgrade_request(
        &self,
        company_id: &str,
        requested_by: Option<&str>,
        source: &str,
        note: Option<&str>,
    ) -> i64 {
        let conn = self.conn();
        let now = Self::now_secs();
        let _ = conn.execute(
            "INSERT INTO upgrade_requests (company_id, requested_by, source, status, note, requested_at)
             VALUES (?1,?2,?3,'requested',?4,?5)",
            rusqlite::params![company_id, requested_by, source, note, now],
        );
        conn.last_insert_rowid()
    }

    /// True if this company has any open (status='requested') upgrade request.
    pub fn has_open_upgrade_request(&self, company_id: &str) -> bool {
        let conn = self.conn();
        conn.query_row(
            "SELECT 1 FROM upgrade_requests WHERE company_id=?1 AND status='requested' LIMIT 1",
            rusqlite::params![company_id],
            |_| Ok(()),
        )
        .is_ok()
    }

    pub fn add_rating(&self, company_id: &str, rating: i64, review: Option<&str>, reviewer: Option<&str>) -> bool {
        let conn = self.conn();
        let ok = conn.execute(
            "INSERT INTO ratings (company_id,rating,review,reviewer_name,created_at) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![company_id, rating, review, reviewer, Self::now_secs()],
        ).is_ok();
        if ok {
            let _ = conn.execute(
                "UPDATE companies SET rating_count=rating_count+1, rating_sum=rating_sum+?1 WHERE id=?2",
                rusqlite::params![rating, company_id],
            );
        }
        ok
    }

    pub fn newsletter_signup(
        &self,
        email: &str,
        name: Option<&str>,
        company: Option<&str>,
        state_abbr: Option<&str>,
    ) -> bool {
        let conn = self.conn();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let ok = conn.execute(
            "INSERT OR IGNORE INTO newsletter_signups              (email,name,company,state,tier,created_at) VALUES(?1,?2,?3,?4,'insider',?5)",
            rusqlite::params![email, name, company, state_abbr, now],
        ).is_ok();
        if let Some(co) = company {
            let key: String = co.to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect();
            if !key.is_empty() {
                conn.execute(
                    "UPDATE companies SET listing_tier = MAX(listing_tier, 1) WHERE lower(replace(replace(entity_name,' ',''),'-','')) LIKE ?1",
                    rusqlite::params![format!("%{}%", &key[..key.len().min(20)])],
                ).ok();
            }
        }
        ok
    }

    /// Column names permitted in [`export_companies`] field lists.
    /// Only names present in this set are forwarded into the SQL SELECT;
    /// unrecognised names are silently dropped, preventing accidental breakage
    /// or future misuse of caller-supplied strings in raw SQL.
    pub const EXPORT_ALLOWED_COLS: &'static [&'static str] = &[
        "id", "state_abbr", "state_name", "city", "city_slug",
        "entity_name", "dba", "company_slug",
        "phone", "email", "website", "address",
        "entity_type", "formation_date", "status", "expiration_date",
        "file_number", "registered_agent", "agent_address",
        "pest_license_num", "pest_license_type", "pest_categories",
        "pest_license_expires", "pest_operator", "pest_source_url",
        "applicator_count", "technician_count", "apprentice_count",
        "source", "source_url", "sos_lookup_url", "pest_lookup_url",
        "listing_tier", "claimed_by", "rating_count", "rating_sum",
    ];

    pub fn export_companies(
        &self,
        fields: &[&str],
        states: &[String],
        require_phone: bool,
        require_email: bool,
        require_website: bool,
        require_license: bool,
        hide_noncommercial: bool,
        hide_foreign_entity: bool,
        hide_chain_names: bool,
        max_rows: i64,
    ) -> Vec<Vec<Option<String>>> {
        // Filter fields to the allowlist before injecting into SQL.
        let safe_fields: Vec<&str> = fields.iter()
            .copied()
            .filter(|f| Self::EXPORT_ALLOWED_COLS.contains(f))
            .collect();
        if safe_fields.is_empty() {
            return Vec::new();
        }
        let col_sql = safe_fields.join(", ");
        let mut wheres: Vec<String> = Vec::new();
        if !states.is_empty() {
            let phs: Vec<String> = (1..=states.len()).map(|i| format!("?{}", i)).collect();
            wheres.push(format!("state_abbr IN ({})", phs.join(",")));
        }
        if require_phone { wheres.push("phone IS NOT NULL AND phone != ''".to_string()); }
        if require_email { wheres.push("email IS NOT NULL AND email != ''".to_string()); }
        if require_website { wheres.push("website IS NOT NULL AND website != ''".to_string()); }
        if require_license { wheres.push("pest_license_num IS NOT NULL AND pest_license_num != ''".to_string()); }
        if hide_noncommercial { wheres.push("LOWER(COALESCE(pest_license_type,'')) NOT LIKE '%noncommercial%'".to_string()); }
        if hide_foreign_entity { wheres.push("UPPER(COALESCE(entity_type,'')) NOT LIKE '%FOREIGN%'".to_string()); }
        if hide_chain_names {
            for name in &["terminix","orkin","rollins","rentokil","anticimex","truly nolen","home team","western pest","ehrlich","massey services"] {
                wheres.push(format!("LOWER(entity_name) NOT LIKE '%{}%'", name));
            }
        }
        let where_clause = if wheres.is_empty() { String::new() }
                           else { format!(" WHERE {}", wheres.join(" AND ")) };
        let limit = if max_rows > 0 { format!(" LIMIT {}", max_rows) } else { String::new() };
        let sql = format!("SELECT {} FROM companies{}{}", col_sql, where_clause, limit);

        let conn = self.conn();
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return Vec::new() };
        let params_vec: Vec<rusqlite::types::Value> = states.iter()
            .map(|s| rusqlite::types::Value::Text(s.clone())).collect();
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter()
            .map(|v| v as &dyn rusqlite::ToSql).collect();
        let col_count = safe_fields.len();
        let mut out: Vec<Vec<Option<String>>> = Vec::new();
        if let Ok(mut rows) = stmt.query(param_refs.as_slice()) {
            loop {
                match rows.next() {
                    Ok(Some(row)) => {
                        let vals: Vec<Option<String>> = (0..col_count)
                            .map(|i| row.get::<usize, Option<String>>(i).ok().flatten())
                            .collect();
                        out.push(vals);
                    }
                    _ => break,
                }
            }
        }
        out
    }

    pub fn top_cities_per_state(&self, per_state: usize) -> std::collections::HashMap<String, Vec<DirectoryCity>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT state_abbr, city_slug, city_name, is_county, company_count              FROM cities WHERE company_count > 0 ORDER BY state_abbr, company_count DESC",
        ).unwrap();
        let all: Vec<(String, DirectoryCity)> = stmt.query_map([], |row| {
            let state: String = row.get(0)?;
            Ok((state.clone(), DirectoryCity {
                state_abbr: state,
                city_slug: row.get(1)?,
                city_name: row.get(2)?,
                is_county: row.get::<_, i64>(3).map(|v| v != 0).unwrap_or(false),
                company_count: row.get(4)?,
            }))
        }).unwrap().flatten().collect();
        let mut result: std::collections::HashMap<String, Vec<DirectoryCity>> = std::collections::HashMap::new();
        for (state, city) in all {
            let entry = result.entry(state).or_default();
            if entry.len() < per_state {
                entry.push(city);
            }
        }
        result
    }

    pub fn all_city_slugs(&self) -> Vec<DirectoryCity> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT state_abbr, city_slug, city_name, is_county, company_count              FROM cities WHERE company_count > 0 ORDER BY state_abbr, city_name",
        ).unwrap();
        stmt.query_map([], |row| {
            Ok(DirectoryCity {
                state_abbr: row.get(0)?,
                city_slug: row.get(1)?,
                city_name: row.get(2)?,
                is_county: row.get::<_, i64>(3).map(|v| v != 0).unwrap_or(false),
                company_count: row.get(4)?,
            })
        }).unwrap().flatten().collect()
    }

    pub fn all_company_slug_tuples(&self) -> Vec<(String, String, String)> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT state_abbr, city_slug, company_slug FROM companies              WHERE city_slug IS NOT NULL AND company_slug IS NOT NULL              ORDER BY state_abbr, city_slug, company_slug",
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).unwrap().flatten().collect()
    }

    pub fn empty_cities(&self) -> Vec<DirectoryCity> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT state_abbr, city_slug, city_name, is_county, company_count              FROM cities WHERE company_count = 0 ORDER BY state_abbr, city_name",
        ).unwrap();
        stmt.query_map([], |row| {
            Ok(DirectoryCity {
                state_abbr: row.get(0)?,
                city_slug: row.get(1)?,
                city_name: row.get(2)?,
                is_county: row.get::<_, i64>(3).map(|v| v != 0).unwrap_or(false),
                company_count: row.get(4)?,
            })
        }).unwrap().flatten().collect()
    }

    pub fn search_companies(&self, query: &str, limit: u32) -> Vec<DirectoryCompany> {
        let conn = self.conn();
        let pattern = sanitize_fts_query(query);
        if pattern.is_empty() {
            return vec![];
        }
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE rowid IN (SELECT rowid FROM companies_fts WHERE companies_fts MATCH ?1) \
             ORDER BY listing_tier DESC, \
               CASE WHEN pest_license_num IS NOT NULL THEN 1 ELSE 0 END DESC, \
               applicator_count DESC \
             LIMIT ?2"
        );
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return vec![] };
        stmt.query_map(rusqlite::params![pattern, limit], row_to_company)
            .map(|rows| rows.flatten().collect::<Vec<DirectoryCompany>>())
            .unwrap_or_default()
    }

    pub fn admin_search_companies(&self, query: &str, state_filter: &str, page: u32, per_page: u32) -> (Vec<DirectoryCompany>, i64) {
        let conn = self.conn();
        let offset = page * per_page;
        let pattern = sanitize_fts_query(query);
        if pattern.is_empty() {
            return (vec![], 0);
        }
        let state_clause = if state_filter.len() == 2 {
            // Length-checked (== 2) and uppercased before interpolation; low risk,
            // but a parameterised query would be cleaner — see audit note.
            format!(" AND state_abbr='{}'", state_filter.to_uppercase())
        } else { String::new() };
        let count_sql = format!(
            "SELECT COUNT(*) FROM companies WHERE rowid IN (SELECT rowid FROM companies_fts WHERE companies_fts MATCH ?1){state_clause}"
        );
        let total: i64 = conn.query_row(&count_sql, [&pattern], |r| r.get(0)).unwrap_or(0);
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE rowid IN (SELECT rowid FROM companies_fts WHERE companies_fts MATCH ?1){state_clause} \
             ORDER BY listing_tier DESC, entity_name \
             LIMIT ?2 OFFSET ?3"
        );
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return (vec![], 0) };
        let rows = stmt.query_map(rusqlite::params![pattern, per_page, offset], row_to_company)
            .map(|rows| rows.flatten().collect::<Vec<DirectoryCompany>>())
            .unwrap_or_default();
        (rows, total)
    }

    pub fn company_by_id(&self, id: &str) -> Option<DirectoryCompany> {
        let conn = self.conn();
        let sql = format!("SELECT {COMPANY_COLS} FROM companies WHERE id=?1 LIMIT 1");
        conn.query_row(&sql, [id], row_to_company).ok()
    }

    /// Companies whose `website` resolves to the given bare host (scheme/`www.`/
    /// path stripped, lowercased). Used by the AI-builder real-data autofill
    /// DOMAIN fallback. We pre-filter cheaply in SQL on a host substring, then
    /// confirm EXACT host equality in Rust (a substring match like `okpest.com`
    /// inside `notokpest.com` is rejected). The caller treats a result length of
    /// >1 as a non-unique (shared/franchise) host and declines to seed.
    pub fn companies_by_website_host(&self, host: &str) -> Vec<DirectoryCompany> {
        let host = host.trim().trim_start_matches("www.").to_ascii_lowercase();
        if host.is_empty() || !host.contains('.') {
            return vec![];
        }
        let conn = self.conn();
        let like = format!("%{host}%");
        let sql = format!(
            "SELECT {COMPANY_COLS} FROM companies \
             WHERE website IS NOT NULL AND website != '' AND lower(website) LIKE ?1 \
             LIMIT 50"
        );
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return vec![] };
        let candidates: Vec<DirectoryCompany> = stmt
            .query_map([&like], row_to_company)
            .map(|rows| rows.flatten().collect())
            .unwrap_or_default();
        candidates
            .into_iter()
            .filter(|c| {
                c.website
                    .as_deref()
                    .map(|w| website_host(w).as_deref() == Some(host.as_str()))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Best fuzzy match for a business name, optionally scoped to a state. Used
    /// by the AI-builder real-data autofill NAME+STATE last-resort fallback.
    /// Returns a company only if its normalized-name token-similarity to `query`
    /// is `>= min_similarity` (a strong bar, e.g. 0.86). Conservative by design:
    /// the cost of a false positive (wrong business's data) is high.
    pub fn best_company_by_name_state(
        &self,
        query: &str,
        state: Option<&str>,
        min_similarity: f64,
    ) -> Option<DirectoryCompany> {
        let q_norm = normalize_company_name(query);
        if q_norm.is_empty() {
            return None;
        }
        let conn = self.conn();
        // State-scope when we have one (cheap + avoids cross-state collisions),
        // otherwise scan all (bounded by the LIKE token prefilter below).
        let first_token = q_norm.split_whitespace().next().unwrap_or("");
        if first_token.len() < 3 {
            return None;
        }
        let like = format!("%{first_token}%");
        let (sql, have_state) = match state {
            Some(s) if s.len() == 2 => (
                format!(
                    "SELECT {COMPANY_COLS} FROM companies \
                     WHERE state_abbr=?1 AND (lower(entity_name) LIKE ?2 OR lower(dba) LIKE ?2) LIMIT 200"
                ),
                true,
            ),
            _ => (
                format!(
                    "SELECT {COMPANY_COLS} FROM companies \
                     WHERE lower(entity_name) LIKE ?1 OR lower(dba) LIKE ?1 LIMIT 200"
                ),
                false,
            ),
        };
        let mut stmt = match conn.prepare(&sql) { Ok(s) => s, Err(_) => return None };
        let candidates: Vec<DirectoryCompany> = if have_state {
            stmt.query_map(
                rusqlite::params![state.unwrap().to_uppercase(), like],
                row_to_company,
            )
        } else {
            stmt.query_map([&like], row_to_company)
        }
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default();

        let mut best: Option<(f64, DirectoryCompany)> = None;
        for c in candidates {
            let cand_name = c
                .dba
                .as_deref()
                .filter(|d| !d.trim().is_empty())
                .unwrap_or(&c.entity_name);
            let sim = name_similarity(&q_norm, &normalize_company_name(cand_name));
            if sim >= min_similarity && best.as_ref().map(|(b, _)| sim > *b).unwrap_or(true) {
                best = Some((sim, c));
            }
        }
        best.map(|(_, c)| c)
    }

    // -- Engagement events (Phase 2, directory hardening) --------------------

    /// Log a contact-reveal interaction. Mirrors `track_view`/`track_click`:
    /// locks the connection and swallows insert errors gracefully (never panics
    /// on a write failure -- analytics must not break the request).
    pub fn log_engagement_event(
        &self,
        company_id: &str,
        event_type: &str,
        ip_hash: &str,
        ts: i64,
        user_id: Option<&str>,
    ) {
        let conn = self.conn();
        let _ = conn.execute(
            "INSERT INTO engagement_events (company_id,event_type,ip_hash,ts,user_id) \
             VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![company_id, event_type, ip_hash, ts, user_id],
        );
    }

    /// Aggregate reveal counts by event_type for one company since `since_ts`.
    /// Used by the Phase 4 owner dashboard.
    pub fn engagement_counts_for_company(&self, company_id: &str, since_ts: i64) -> Vec<(String, i64)> {
        let conn = self.conn();
        let mut out = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT event_type, COUNT(*) FROM engagement_events \
             WHERE company_id=?1 AND ts>?2 GROUP BY event_type",
        ) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![company_id, since_ts], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
            }) {
                for row in rows.flatten() {
                    out.push(row);
                }
            }
        }
        out
    }

    /// Resolve a company's contact fields by id: (phone, email, website).
    /// Returns None if the company does not exist.
    pub fn company_contact(&self, company_id: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
        self.company_by_id(company_id).map(|c| (c.phone, c.email, c.website))
    }

    pub fn update_listing_tier(&self, id: &str, tier: i64) {
        let conn = self.conn();
        let _ = conn.execute("UPDATE companies SET listing_tier=?1 WHERE id=?2", rusqlite::params![tier, id]);
    }

    pub fn update_claimed_by(&self, id: &str, claimed_by: Option<&str>) {
        let conn = self.conn();
        let _ = conn.execute("UPDATE companies SET claimed_by=?1 WHERE id=?2", rusqlite::params![claimed_by, id]);
    }


    /// Release a company back to unclaimed state (admin tool).
    pub fn unclaim_company(&self, id: &str) -> Result<serde_json::Value, rusqlite::Error> {
        let conn = self.conn();

        let mut claim_info: Option<(String, Option<String>)> = None;
        let mut stmt = conn.prepare(
            "SELECT user_id, claimed_by FROM directory_claims WHERE company_id=?1 LIMIT 1"
        )?;
        let mut rows = stmt.query(rusqlite::params![id])?;
        if let Some(row) = rows.next()? {
            claim_info = Some((row.get(0)?, row.get(1)?));
        }

        let deleted = conn.execute(
            "DELETE FROM directory_claims WHERE company_id=?1",
            rusqlite::params![id],
        )?;

        let updated = conn.execute(
            "UPDATE companies SET claimed_by=NULL WHERE id=?1",
            rusqlite::params![id],
        )?;

        let mut map = serde_json::Map::new();
        map.insert("claims_deleted".into(), serde_json::json!(deleted));
        map.insert("company_updated".into(), serde_json::json!(updated > 0));

        if let Some((uid, cb)) = claim_info {
            map.insert("previous_user_id".into(), serde_json::json!(uid));
            map.insert("previous_claimed_by".into(), serde_json::json!(cb));
        }

        Ok(serde_json::Value::Object(map))
    }
    // -- Claims (Phase 3, directory hardening) -------------------------------

    /// True iff `user_id` holds a *verified* claim on `company_id`. Drives the
    /// Owner viewer tier. Until Phase 4 writes verified claims this is always
    /// false (the table is empty), so every viewer renders the Public mask —
    /// the intended pre-Phase-4 behavior.
    pub fn is_verified_owner(&self, company_id: &str, user_id: &str) -> bool {
        let conn = self.conn();
        conn.query_row(
            "SELECT 1 FROM directory_claims \
             WHERE company_id=?1 AND user_id=?2 AND verified=1 LIMIT 1",
            rusqlite::params![company_id, user_id],
            |_| Ok(()),
        )
        .is_ok()
    }

    // ── Claims write/read (Phase 4, directory hardening) ───────────────────
    //
    // The claim flow lets a logged-in member (ForgeJournal user_id, supplied by
    // main.rs via the DirViewer extension) assert ownership of a directory
    // listing, attest legally, optionally opt into the newsletter, and verify
    // via a one-time emailed token. Only a row with verified=1 unlocks the
    // Owner tier (see `is_verified_owner`). The directory crate never auto-
    // verifies: verified flips to 1 ONLY when the emailed token is presented.

    /// Generate a strong single-use token using SQLite's CSPRNG (randomblob,
    /// seeded from the OS entropy source). Avoids pulling a uuid/rand dep into
    /// this dependency-isolated crate. Returns a 32-hex-char (128-bit) token.
    pub fn new_claim_token(&self) -> String {
        let conn = self.conn();
        conn.query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get::<_, String>(0))
            .unwrap_or_else(|_| {
                // Fallback (should never hit): time-derived, still single-use.
                format!("{:032x}", Self::now_secs() as u128)
            })
    }

    /// Upsert a *pending* (verified=0) claim for (company_id, user_id). Stores
    /// the one-time token + expiry, the legal-attestation flag (must be true —
    /// the handler rejects otherwise), the optional newsletter opt-in, and the
    /// daily-salted ip_hash. UNIQUE(company_id,user_id) means a re-submit
    /// updates the existing row (new token/expiry), never duplicates. Never
    /// sets verified — that only happens in `verify_claim_by_token`.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_or_update_claim(
        &self,
        company_id: &str,
        user_id: &str,
        email: &str,
        token: &str,
        token_expiry: i64,
        attest_legal: bool,
        newsletter: bool,
        ip_hash: &str,
    ) -> Result<(), String> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO directory_claims \
                (company_id, user_id, email, attested_at, verified, \
                 verify_token, token_expiry, attest_legal, newsletter, ip_hash) \
             VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, ?9) \
             ON CONFLICT(company_id, user_id) DO UPDATE SET \
                email=excluded.email, attested_at=excluded.attested_at, \
                verify_token=excluded.verify_token, token_expiry=excluded.token_expiry, \
                attest_legal=excluded.attest_legal, newsletter=excluded.newsletter, \
                ip_hash=excluded.ip_hash \
             WHERE directory_claims.verified=0",
            rusqlite::params![
                company_id, user_id, email, Self::now_secs(),
                token, token_expiry,
                if attest_legal { 1 } else { 0 },
                if newsletter { 1 } else { 0 },
                ip_hash
            ],
        )
        .map(|_| ())
        .map_err(|e| format!("insert_or_update_claim: {e}"))
    }

    /// Fetch a claim row for (company_id, user_id), if any.
    pub fn get_claim(&self, company_id: &str, user_id: &str) -> Option<DirectoryClaim> {
        let conn = self.conn();
        conn.query_row(
            "SELECT company_id, user_id, email, attested_at, verified, \
                    token_expiry, attest_legal, newsletter \
             FROM directory_claims WHERE company_id=?1 AND user_id=?2",
            rusqlite::params![company_id, user_id],
            Self::map_claim_row,
        )
        .ok()
    }

    /// Verify a claim by its one-time token. On success: set verified=1, NULL
    /// the token + expiry (single-use), set companies.claimed_by, and return
    /// (company_id, user_id). Expired or unknown tokens return None and mutate
    /// nothing. Already-verified rows have a NULL token so they won't match.
    pub fn verify_claim_by_token(&self, token: &str) -> Option<(String, String)> {
        let conn = self.conn();
        let now = Self::now_secs();
        let (company_id, user_id): (String, String) = conn
            .query_row(
                "SELECT company_id, user_id FROM directory_claims \
                 WHERE verify_token=?1 AND verified=0 \
                   AND (token_expiry IS NULL OR token_expiry > ?2) LIMIT 1",
                rusqlite::params![token, now],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .ok()?;
        // Flip verified + clear the token (single-use).
        if conn
            .execute(
                "UPDATE directory_claims SET verified=1, verify_token=NULL, token_expiry=NULL \
                 WHERE company_id=?1 AND user_id=?2",
                rusqlite::params![company_id, user_id],
            )
            .is_err()
        {
            return None;
        }
        // Reflect ownership on the company record (best-effort; the owner tier
        // keys off directory_claims.verified, not this column).
        let _ = conn.execute(
            "UPDATE companies SET claimed_by=?1 WHERE id=?2",
            rusqlite::params![user_id, company_id],
        );
        Some((company_id, user_id))
    }

    /// All *verified* claims held by a user, newest first. Drives the
    /// `/directory/my-listings` dashboard.
    pub fn claims_for_user(&self, user_id: &str) -> Vec<DirectoryClaim> {
        let conn = self.conn();
        let mut out = Vec::new();
        if let Ok(mut stmt) = conn.prepare(
            "SELECT company_id, user_id, email, attested_at, verified, \
                    token_expiry, attest_legal, newsletter \
             FROM directory_claims WHERE user_id=?1 AND verified=1 \
             ORDER BY attested_at DESC",
        ) {
            if let Ok(rows) = stmt.query_map(rusqlite::params![user_id], Self::map_claim_row) {
                for row in rows.flatten() {
                    out.push(row);
                }
            }
        }
        out
    }

    fn map_claim_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DirectoryClaim> {
        Ok(DirectoryClaim {
            company_id: r.get(0)?,
            user_id: r.get(1)?,
            email: r.get(2)?,
            attested_at: r.get(3)?,
            verified: r.get::<_, i64>(4)? != 0,
            token_expiry: r.get(5)?,
            attest_legal: r.get::<_, i64>(6)? != 0,
            newsletter: r.get::<_, i64>(7)? != 0,
        })
    }
}

fn query_rows(conn: &Connection, sql: &str) -> Vec<DirStatRow> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| Ok(DirStatRow { label: row.get(0)?, count: row.get(1)? }))
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
}



fn query_count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap_or(0)
}

// Strict YYYY-MM-DD shape check. The rollup boundary value is interpolated
// directly into multiple SQL strings; this guards against any malformed or
// adversarial value sneaking in via the rollup table.
fn is_valid_day(s: &str) -> bool {
    if s.len() != 10 {
        return false;
    }
    let b = s.as_bytes();
    b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4] == b'-'
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && b[7] == b'-'
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit()
}

// ─── Rollup-aware stats SQL builders ────────────────────────────────────────
//
// All builders take a `cutoff` 'YYYY-MM-DD' string. They produce a UNION ALL
// query that reads aggregated rows from page_views_daily for days strictly
// less than cutoff, and raw rows from page_views with ts >= midnight(cutoff).
// The outer SELECT re-aggregates by the relevant dimension and applies the
// LIMIT/ORDER from the original query.

fn page_type_union_sql(cutoff: &str) -> String {
    format!(
        "SELECT label, SUM(c) AS cnt FROM (\n\
         \x20  SELECT page_type AS label, SUM(view_count) AS c FROM page_views_daily \
         WHERE day < '{c}' GROUP BY page_type\n\
         \x20  UNION ALL\n\
         \x20  SELECT COALESCE(page_type,'') AS label, COUNT(*) AS c FROM page_views \
         WHERE ts >= strftime('%s','{c}') GROUP BY page_type\n\
         ) GROUP BY label ORDER BY cnt DESC",
        c = cutoff,
    )
}

fn by_state_union_sql(cutoff: &str) -> String {
    // Original behavior: filter out page_type='home', display '?' for null state.
    // Rollup stores '' for null page_type (COALESCE on backfill) — equivalent
    // because the original raw row's NULL page_type was treated as not-'home'.
    format!(
        "SELECT CASE WHEN state = '' THEN '?' ELSE state END AS label, SUM(c) AS cnt FROM (\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, SUM(view_count) AS c FROM page_views_daily \
         WHERE day < '{c}' AND COALESCE(page_type,'') != 'home' GROUP BY state_abbr\n\
         \x20  UNION ALL\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, COUNT(*) AS c FROM page_views \
         WHERE ts >= strftime('%s','{c}') AND COALESCE(page_type,'') != 'home' GROUP BY state_abbr\n\
         ) GROUP BY state ORDER BY cnt DESC LIMIT 20",
        c = cutoff,
    )
}

fn top_cities_union_sql(cutoff: &str) -> String {
    format!(
        "SELECT state||'/'||city AS label, SUM(c) AS cnt FROM (\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, COALESCE(city_slug,'') AS city, SUM(view_count) AS c FROM page_views_daily \
         WHERE day < '{c}' AND COALESCE(page_type,'') IN ('city','company') GROUP BY state_abbr, city_slug\n\
         \x20  UNION ALL\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, COALESCE(city_slug,'') AS city, COUNT(*) AS c FROM page_views \
         WHERE ts >= strftime('%s','{c}') AND COALESCE(page_type,'') IN ('city','company') GROUP BY state_abbr, city_slug\n\
         ) GROUP BY state, city ORDER BY cnt DESC LIMIT 20",
        c = cutoff,
    )
}

fn top_companies_union_sql(cutoff: &str) -> String {
    format!(
        "SELECT state||'/'||city||'/'||company AS label, SUM(c) AS cnt FROM (\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, COALESCE(city_slug,'') AS city, COALESCE(company_slug,'') AS company, SUM(view_count) AS c FROM page_views_daily \
         WHERE day < '{c}' AND COALESCE(page_type,'') = 'company' GROUP BY state_abbr, city_slug, company_slug\n\
         \x20  UNION ALL\n\
         \x20  SELECT COALESCE(state_abbr,'') AS state, COALESCE(city_slug,'') AS city, COALESCE(company_slug,'') AS company, COUNT(*) AS c FROM page_views \
         WHERE ts >= strftime('%s','{c}') AND COALESCE(page_type,'') = 'company' GROUP BY state_abbr, city_slug, company_slug\n\
         ) GROUP BY state, city, company ORDER BY cnt DESC LIMIT 20",
        c = cutoff,
    )
}

fn daily_views_union_sql(cutoff: &str) -> String {
    // 30-day chart. Restrict to last 30 days on both sides via a date math
    // expression on the rollup leg, and the existing ts comparator on raw.
    format!(
        "SELECT d AS label, SUM(c) AS cnt FROM (\n\
         \x20  SELECT day AS d, SUM(view_count) AS c FROM page_views_daily \
         WHERE day < '{c}' AND day > date('now','-30 days') GROUP BY day\n\
         \x20  UNION ALL\n\
         \x20  SELECT date(ts,'unixepoch') AS d, COUNT(*) AS c FROM page_views \
         WHERE ts >= strftime('%s','{c}') AND ts > strftime('%s','now') - 30*86400 GROUP BY d\n\
         ) GROUP BY d ORDER BY d DESC LIMIT 30",
        c = cutoff,
    )
}

fn regs_url_for_state(state: &str) -> Option<String> {
    match state {
        "TX" => Some("https://www.tda.texas.gov/plants-pests-pesticides/structural-pest-control/".into()),
        "FL" => Some("https://www.freshfromflorida.com/Divisions-Offices/Agricultural-Environmental-Services/Pest-Control".into()),
        "NY" => Some("https://www.dec.ny.gov/chemical/23963.html".into()),
        "CT" => Some("https://portal.ct.gov/CAES/Pesticide-Management/Pesticide-Registration-and-Licensing".into()),
        "CO" => Some("https://agplants.colorado.gov/pest-management-program/pesticide-applicators".into()),
        "OR" => Some("https://www.oregon.gov/oda/programs/pesticides".into()),
        "IA" => Some("https://www.iowaagriculture.gov/pesticide-bureau".into()),
        "DE" => Some("https://agriculture.delaware.gov/pesticides".into()),
        "GA" => Some("https://www.caes.uga.edu/extension/county.html".into()),
        "NC" => Some("https://www.ncagr.gov/divisions/pesticides/license/index.htm".into()),
        "MN" => Some("https://www.mda.state.mn.us/plants-insects/pesticide-applicator-licensing".into()),
        "MO" => Some("https://agriculture.mo.gov/plants/pesticidemanagement/licensing.php".into()),
        "MI" => Some("https://www.michigan.gov/mdard/Agriculture/Pesticide-and-Plant-Pest-Management/Licensing".into()),
        "OH" => Some("https://apps.ohioagriculture.gov/plsv/pesticideapplic/pest_srch.PROCESS".into()),
        "CA" => Some("https://www.cdpr.ca.gov/docs/license/liclook.htm".into()),
        "IN" => Some("https://www.in.gov/oisc/pesticide-licensing/".into()),
        "KY" => Some("https://kyagr.com/regulatory/pesticides/licensing.html".into()),
        "VA" => Some("https://www.vdacs.virginia.gov/pesticides-licensing.shtml".into()),
        "WA" => Some("https://www.atlasbp.agr.wa.gov/".into()),
        "AR" => Some("https://agriculture.arkansas.gov/plants-insects/division-of-pesticides/licensing-certification/".into()),
        "KS" => Some("https://www.agriculture.ks.gov/divisions-programs/pmd/pesticide-applicators".into()),
        "LA" => Some("https://www.ldaf.state.la.us/pesticides/licensing-certification/".into()),
        "MT" => Some("https://agr.mt.gov/Portals/168/docs/docs/pesticides/".into()),
        "NM" => Some("https://www.nmda.nmsu.edu/pcd/licensing/".into()),
        "NV" => Some("https://agri.nv.gov/Plants/Pesticide_Control/".into()),
        "ME" => Some("https://www.maine.gov/dacf/php/pesticides/licensing.shtml".into()),
        "WV" => Some("https://agriculture.wv.gov/Divisions/Plant-Industries/Pesticides/Pages/default.aspx".into()),
        "AZ" => Some("https://oar.az.gov/".into()),
        "OK" => Some("https://www.ag.ok.gov/divisions/plant-industry-and-consumer-services/pesticide-division/".into()),
        _ => None,
    }
}

fn staff_summary(applicators: i64, technicians: i64, apprentices: i64) -> Option<String> {
    let mut parts = Vec::new();
    if applicators > 0 {
        parts.push(format!("{applicators} Certified Applicator{}", if applicators == 1 { "" } else { "s" }));
    }
    if technicians > 0 {
        parts.push(format!("{technicians} Technician{}", if technicians == 1 { "" } else { "s" }));
    }
    if apprentices > 0 {
        parts.push(format!("{apprentices} Apprentice{}", if apprentices == 1 { "" } else { "s" }));
    }
    if parts.is_empty() { None } else { Some(parts.join(" · ")) }
}

fn fmt_date(s: Option<&str>) -> Option<String> {
    let s = s?;
    let months = ["January","February","March","April","May","June",
                  "July","August","September","October","November","December"];
    // ISO: YYYY-MM-DD
    let parts: Vec<&str> = s.splitn(4, |c| c == '-' || c == '/').collect();
    match parts.as_slice() {
        [y, m, d] if y.len() == 4 => {
            // YYYY-MM-DD
            if let (Ok(month), Ok(day)) = (m.parse::<usize>(), d.parse::<u32>()) {
                if month >= 1 && month <= 12 {
                    return Some(format!("{} {}, {}", months[month - 1], day, y));
                }
            }
            None
        }
        [m, d, y] => {
            // M/D/YYYY or MM/DD/YYYY
            if let (Ok(month), Ok(day), Ok(year)) =
                (m.trim().parse::<usize>(), d.trim().parse::<u32>(), y.trim().parse::<u32>())
            {
                if month >= 1 && month <= 12 && year > 1900 {
                    return Some(format!("{} {}, {}", months[month - 1], day, year));
                }
            }
            None
        }
        _ => None,
    }
}

/// Normalize a website URL to its bare host (scheme/`www.`/path stripped,
/// lowercased). Mirrors the AI-builder caller's `normalize_host` so DB-side and
/// app-side host comparison agree. Returns `None` if there's no plausible host.
fn website_host(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let no_scheme = s.split_once("://").map(|(_, r)| r).unwrap_or(s);
    let host = no_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(no_scheme)
        .split('@')
        .last()
        .unwrap_or(no_scheme)
        .split(':')
        .next()
        .unwrap_or(no_scheme)
        .trim_end_matches('.')
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    if host.is_empty() || !host.contains('.') {
        None
    } else {
        Some(host)
    }
}

/// Normalize a company name for fuzzy comparison: lowercase, drop punctuation,
/// collapse whitespace, and strip common legal/entity suffixes so
/// "ACME PEST CONTROL, INC." and "Acme Pest Control" compare equal.
fn normalize_company_name(raw: &str) -> String {
    const STOP: &[&str] = &[
        "inc", "llc", "ltd", "co", "corp", "company", "incorporated", "lp", "llp",
        "pllc", "the", "and", "of",
    ];
    let lowered: String = raw
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect();
    lowered
        .split_whitespace()
        .filter(|t| !STOP.contains(t))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Token-set Jaccard similarity of two already-normalized names, in `[0,1]`.
/// Simple, dependency-free, and order-insensitive — adequate for the
/// high-threshold (>=0.86) last-resort fallback.
fn name_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    if a == b {
        return 1.0;
    }
    let ta: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let tb: std::collections::HashSet<&str> = b.split_whitespace().collect();
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    inter / union
}

fn row_to_company(row: &rusqlite::Row<'_>) -> rusqlite::Result<DirectoryCompany> {
    let state_abbr: String = row.get(1)?;
    let status: Option<String> = row.get(16)?;
    let formation_date: Option<String> = row.get(15)?;
    let pest_license_num: Option<String> = row.get(21)?;
    let applicator_count: i64 = row.get(28)?;
    let technician_count: i64 = row.get(29)?;
    let apprentice_count: i64 = row.get(30)?;
    let rating_count: i64 = row.get(37)?;
    let rating_sum: i64 = row.get(38)?;

    let is_active = status.as_deref()
        .map(|s| s.eq_ignore_ascii_case("active") || s.eq_ignore_ascii_case("initial/active"))
        .unwrap_or(true); // pest-license-only records don't have status — assume active
    let has_pest_license = pest_license_num.is_some();
    let avg_rating = if rating_count > 0 { Some(rating_sum as f64 / rating_count as f64) } else { None };
    let formation_year = formation_date.as_deref().and_then(|d| d.get(..4)).map(String::from);
    let state_regs_url = regs_url_for_state(&state_abbr);
    let ssum = staff_summary(applicator_count, technician_count, apprentice_count);

    Ok(DirectoryCompany {
        id: row.get(0)?,
        state_abbr,
        state_name: row.get(2)?,
        city: row.get(3)?,
        city_slug: row.get(4)?,
        is_county_location: row.get::<_, i64>(5).map(|v| v != 0).unwrap_or(false),
        county: row.get(6)?,
        entity_name: row.get(7)?,
        dba: row.get(8)?,
        company_slug: row.get(9)?,
        phone: row.get(10)?,
        email: row.get(11)?,
        website: row.get(12)?,
        address: row.get(13)?,
        entity_type: row.get(14)?,
        formation_date_display: fmt_date(formation_date.as_deref()),
        formation_date,
        status,
        expiration_date: row.get(17)?,
        file_number: row.get(18)?,
        registered_agent: row.get(19)?,
        agent_address: row.get(20)?,
        pest_license_num,
        pest_license_type: row.get(22)?,
        pest_categories: row.get(23)?,
        pest_categories_decoded: row.get(24)?,
        pest_license_expires: row.get(25)?,
        pest_license_expires_display: fmt_date(row.get::<_, Option<String>>(25).ok().flatten().as_deref()),
        pest_operator: row.get(26)?,
        pest_source_url: row.get(27)?,
        applicator_count,
        technician_count,
        apprentice_count,
        source: row.get(31)?,
        source_url: row.get(32)?,
        sos_lookup_url: row.get(33)?,
        pest_lookup_url: row.get(34)?,
        listing_tier: row.get(35)?,
        claimed_by: row.get(36)?,
        rating_count,
        rating_sum,
        is_active,
        has_pest_license,
        avg_rating,
        formation_year,
        state_regs_url,
        staff_summary: ssum,
        expiration_date_display: fmt_date(row.get::<_, Option<String>>(17).ok().flatten().as_deref()),
        pest_license_issued: row.get(39)?,
        pest_license_issued_display: fmt_date(row.get::<_, Option<String>>(39).ok().flatten().as_deref()),
        pest_license_renewed: row.get(40)?,
        pest_license_renewed_display: fmt_date(row.get::<_, Option<String>>(40).ok().flatten().as_deref()),
        pest_insurance_expires: row.get(41)?,
        pest_insurance_expires_display: fmt_date(row.get::<_, Option<String>>(41).ok().flatten().as_deref()),
        pest_responsible_applicator: row.get(42)?,
        pest_responsible_applicator_license: row.get(43)?,
        pest_spcb_id: row.get(44)?,
        is_canary: row.get::<_, i64>(45).map(|v| v != 0).unwrap_or(false),
    })
}

/// Serialize blocks into the ON-DISK `blocks_json` shape that [`parse_mini_site_blocks`]
/// reads (key `"type"`, not the struct's `block_type`) so writes stay byte-compatible with
/// the historically-seeded rows. Export/import use the struct's own serde shape separately.
fn blocks_to_storage_json(blocks: &[DirectoryMiniSiteBlock]) -> String {
    let arr: Vec<serde_json::Value> = blocks
        .iter()
        .map(|b| {
            serde_json::json!({
                "type": b.block_type,
                "heading": b.heading,
                "subheading": b.subheading,
                "body": b.body,
                "cta_text": b.cta_text,
                "cta_url": b.cta_url,
                "image_url": b.image_url,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// Normalize a page slug to a url-safe token (lowercase, alnum + dashes, collapsed).
pub fn normalize_page_slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if (ch == ' ' || ch == '-' || ch == '_' || ch == '/') && !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn parse_mini_site_blocks(json: &str) -> Vec<DirectoryMiniSiteBlock> {
    serde_json::from_str::<Vec<serde_json::Value>>(json)
        .unwrap_or_default()
        .into_iter()
        .map(|v| DirectoryMiniSiteBlock {
            block_type: v.get("type").and_then(|t| t.as_str()).unwrap_or("content").to_string(),
            heading: v.get("heading").and_then(|t| t.as_str()).map(String::from),
            subheading: v.get("subheading").and_then(|t| t.as_str()).map(String::from),
            body: v.get("body").and_then(|t| t.as_str()).map(String::from),
            cta_text: v.get("cta_text").and_then(|t| t.as_str()).map(String::from),
            cta_url: v.get("cta_url").and_then(|t| t.as_str()).map(String::from),
            image_url: v.get("image_url").and_then(|t| t.as_str()).map(String::from),
        })
        .collect()
}

/// Phase 0 (2026-05-27): sanitize a user-supplied query for FTS5 MATCH.
/// Splits on whitespace, strips embedded double quotes, wraps each token
/// in quotes + appends '*' for prefix matching. Multi-word queries become
/// implicit-AND across tokens. Returns empty string when no usable tokens.
fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter_map(|tok| {
            let cleaned: String = tok.replace('"', "");
            if cleaned.is_empty() { None } else { Some(format!(r#""{}"*"#, cleaned)) }
        })
        .collect::<Vec<_>>()
        .join(" ")
}



/// Self-contained "what your upgraded site's menu looks like" preview — renders the
/// company's recovered nav as a top bar with hover dropdown columns. Emits its own CSS;
/// safe to inject into the listing page. Returns "" when there is no recovered menu.
pub fn render_company_mega_menu(items: &[CompanyNavItem], business_name: &str) -> String {
    fn esc(v: &str) -> String {
        v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
    }
    let mut tops: Vec<&CompanyNavItem> = items.iter().filter(|i| i.parent_id.is_none()).collect();
    tops.sort_by_key(|i| i.position);
    tops.truncate(12);
    if tops.is_empty() {
        return String::new();
    }
    let mut h = String::with_capacity(4096);
    h.push_str("<style>\n");
    h.push_str(".cmm-wrap{margin:0 0 22px;border:1px solid #e2e8f0;border-radius:14px;background:#fff;position:relative;z-index:5}\n");
    h.push_str(".cmm-tag{display:flex;align-items:center;gap:8px;padding:8px 16px;background:#f8fafc;border-bottom:1px solid #e2e8f0;border-radius:13px 13px 0 0;font-size:12px;font-weight:700;color:#64748b;text-transform:uppercase;letter-spacing:.4px}\n");
    h.push_str(".cmm-tag .dot{width:8px;height:8px;border-radius:50%;background:var(--pc-accent,#e8752a);flex:0 0 auto}\n");
    h.push_str(".cmm-bar{display:flex;flex-wrap:wrap;background:linear-gradient(135deg,#1a3c5e,#0f2a44);border-radius:0 0 13px 13px}\n");
    h.push_str(".cmm-item{position:relative;flex:0 0 auto}\n");
    h.push_str(".cmm-link{display:block;padding:14px 18px;color:#e6eef7;text-decoration:none;font-size:14px;font-weight:600;white-space:nowrap;cursor:pointer;border:0;background:none;font-family:inherit}\n");
    h.push_str(".cmm-item:hover>.cmm-link{background:rgba(255,255,255,.08);color:#fff}\n");
    h.push_str(".cmm-caret{font-size:9px;opacity:.7;margin-left:4px}\n");
    h.push_str(".cmm-panel{display:none;position:absolute;left:0;top:100%;min-width:230px;background:#fff;border:1px solid #e2e8f0;border-top:3px solid var(--pc-accent,#e8752a);border-radius:0 0 10px 10px;box-shadow:0 12px 30px rgba(0,0,0,.16);z-index:40;padding:8px}\n");
    h.push_str(".cmm-item:hover>.cmm-panel{display:block}\n");
    h.push_str(".cmm-sub{display:block;padding:9px 14px;color:#1a2230;text-decoration:none;font-size:14px;border-radius:7px;white-space:nowrap}\n");
    h.push_str(".cmm-sub:hover{background:#f1f5f9;color:var(--pc-accent,#e8752a)}\n");
    h.push_str("@media(max-width:768px){.cmm-bar{flex-wrap:wrap}.cmm-item{flex:1 1 100%}.cmm-panel{position:static;display:block;box-shadow:none;border:0;border-radius:0;padding:0 0 8px 16px;min-width:0}}\n");
    h.push_str("</style>\n");
    h.push_str("<div class=\"cmm-wrap\">\n");
    h.push_str(&format!("<div class=\"cmm-tag\"><span class=\"dot\"></span>Preview · {}’s website menu, rebuilt on PestController.org</div>\n", esc(business_name)));
    h.push_str("<nav class=\"cmm-bar\">\n");
    for top in &tops {
        let mut kids: Vec<&CompanyNavItem> =
            items.iter().filter(|c| c.parent_id.as_deref() == Some(top.item_id.as_str())).collect();
        kids.sort_by_key(|c| c.position);
        if kids.is_empty() {
            h.push_str(&format!(
                "<div class=\"cmm-item\"><a class=\"cmm-link\" href=\"{}\">{}</a></div>\n",
                esc(&top.url), esc(&top.title)
            ));
        } else {
            h.push_str("<div class=\"cmm-item\">");
            h.push_str(&format!(
                "<button class=\"cmm-link\" type=\"button\">{} <span class=\"cmm-caret\">▼</span></button>",
                esc(&top.title)
            ));
            h.push_str("<div class=\"cmm-panel\">");
            let shown = kids.len().min(8);
            for k in kids.iter().take(shown) {
                h.push_str(&format!(
                    "<a class=\"cmm-sub\" href=\"{}\">{}</a>",
                    esc(&k.url), esc(&k.title)
                ));
            }
            if kids.len() > shown {
                h.push_str(&format!(
                    "<a class=\"cmm-sub\" href=\"{}\" style=\"color:#64748b;font-style:italic\">+ {} more</a>",
                    esc(&top.url), kids.len() - shown
                ));
            }
            h.push_str("</div></div>\n");
        }
    }
    h.push_str("</nav>\n</div>\n");
    h
}
