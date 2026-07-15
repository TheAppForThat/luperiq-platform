//! Google Data Cache — server-side caching layer for Google API responses.
//!
//! Uses ForgeJournal as the backing store with tiered TTLs. Eliminates
//! redundant API calls to api.luperiq.com during admin navigation.

use luperiq_forge::{ApexEvent, ForgeJournal};
use serde::{Deserialize, Serialize};

pub const AGG_GOOGLE_CACHE: &str = "GoogleDataCache";

// ── TTL constants (seconds) ────────────────────────────────────────

/// GA4 traffic summary — semi-real-time
pub const TTL_GA4_TRAFFIC: i64 = 900; // 15 min
/// GA4 timeseries and table data
pub const TTL_GA4_TABLES: i64 = 1800; // 30 min
/// GSC query/page/breakdown — GSC data is 2-3 days delayed anyway
pub const TTL_GSC_DATA: i64 = 21600; // 6 hours
/// Property and site lists — rarely changes
pub const TTL_PROPERTY_LISTS: i64 = 86400; // 24 hours
/// Per-page drill-down metrics
pub const TTL_PAGE_METRICS: i64 = 3600; // 1 hour

// ── Cache entry ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub data: serde_json::Value,
    pub fetched_at: i64,
    pub ttl_seconds: i64,
}

// ── Cache manager ──────────────────────────────────────────────────

pub struct GoogleCacheManager;

impl GoogleCacheManager {
    /// Get cached data if it exists and is still fresh.
    pub fn get(journal: &ForgeJournal, cache_key: &str) -> Option<serde_json::Value> {
        let event = journal.get_latest(AGG_GOOGLE_CACHE, cache_key)?;
        if event.payload == b"__DELETED__" {
            return None;
        }
        let entry: CacheEntry = serde_json::from_slice(&event.payload).ok()?;
        let now = super::now_epoch();
        if now - entry.fetched_at < entry.ttl_seconds {
            Some(entry.data)
        } else {
            None // stale
        }
    }

    /// Store data in cache with the given TTL.
    pub fn set(
        journal: &mut ForgeJournal,
        cache_key: &str,
        data: &serde_json::Value,
        ttl_seconds: i64,
    ) -> Result<(), String> {
        let entry = CacheEntry {
            data: data.clone(),
            fetched_at: super::now_epoch(),
            ttl_seconds,
        };
        let bytes = serde_json::to_vec(&entry).map_err(|e| e.to_string())?;
        let event = ApexEvent::new(AGG_GOOGLE_CACHE, cache_key, bytes);
        journal.append(event).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Invalidate all cache entries whose key starts with the given prefix.
    pub fn invalidate(journal: &mut ForgeJournal, prefix: &str) -> Result<usize, String> {
        let full_prefix = format!("{AGG_GOOGLE_CACHE}:{prefix}");
        let matching: Vec<String> = journal
            .latest_by_key_prefix(&full_prefix)
            .iter()
            .filter(|e| e.payload != b"__DELETED__")
            .filter_map(|e| {
                // key format is "GoogleDataCache:actual_cache_key"
                e.key()
                    .strip_prefix(&format!("{AGG_GOOGLE_CACHE}:"))
                    .map(|s| s.to_string())
            })
            .collect();

        let count = matching.len();
        for key in &matching {
            let event = ApexEvent::new(AGG_GOOGLE_CACHE, key, b"__DELETED__".to_vec());
            journal.append(event).map_err(|e| e.to_string())?;
        }
        Ok(count)
    }

    /// Build a cache key from components. Components are joined with `:`.
    pub fn make_key(parts: &[&str]) -> String {
        parts.join(":")
    }
}

/// Determine the appropriate TTL for a given data type prefix.
pub fn ttl_for_prefix(prefix: &str) -> i64 {
    match prefix {
        "ga4:traffic" => TTL_GA4_TRAFFIC,
        "ga4:timeseries" | "ga4:sources" | "ga4:pages" => TTL_GA4_TABLES,
        "gsc:queries"
        | "gsc:pages"
        | "gsc:breakdown"
        | "gsc:delta"
        | "gsc:query-timeseries"
        | "gsc:query-pages"
        | "gsc:page-timeseries" => TTL_GSC_DATA,
        "ga4:properties" | "gsc:sites" => TTL_PROPERTY_LISTS,
        "page" => TTL_PAGE_METRICS,
        _ => TTL_GA4_TABLES, // sensible default
    }
}
