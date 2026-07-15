//! Page Intelligence Assembler — unified JSON endpoint for AI consumers.
//!
//! The `assemble()` function gathers page content, SEO metadata, keyword gate
//! results, Surfer SEO scoring, pipeline status, sibling pages, and platform
//! capabilities into a single `PageIntelligence` response.
//!
//! This is the integration layer: it calls existing sub-modules and merges
//! results. All I/O goes through the ForgeJournal; there are no HTTP calls
//! or file-system reads here.

use serde::Serialize;

use luperiq_forge::{ForgeContentManager, ForgeJournal};

use super::keyword_gate::KeywordCheck;
use super::surfer_scoring::SurferScore;
use super::{SeoMeta, AGG_SEO_META, TOMBSTONE};

// ── Output types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PageIntelligence {
    pub page: PageInfo,
    pub blocks: serde_json::Value,
    pub seo: SeoInfo,
    pub keyword_gate: KeywordGateInfo,
    pub surfer: Option<SurferInfo>,
    pub sibling_pages: Vec<SiblingPage>,
    pub platform_capabilities: serde_json::Value,
    pub insights: Vec<serde_json::Value>,
    pub pipeline_status: PipelineStatus,
}

#[derive(Debug, Serialize)]
pub struct PageInfo {
    pub content_id: String,
    pub slug: String,
    pub title: String,
    pub status: String,
    pub word_count: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize)]
pub struct SeoInfo {
    pub title: String,
    pub description: String,
    pub focus_keyword: String,
    pub canonical_url: String,
    pub og_image: String,
    pub schema_json: String,
    pub seo_score: u8,
}

#[derive(Debug, Serialize)]
pub struct KeywordGateInfo {
    pub checks: Vec<KeywordCheck>,
    pub score: String, // e.g. "4/7"
}

#[derive(Debug, Serialize)]
pub struct SurferInfo {
    pub primary_sheet: String,
    pub secondary_sheets: Vec<String>,
    pub score: SurferScore,
    pub facts_available: usize,
}

#[derive(Debug, Serialize)]
pub struct SiblingPage {
    pub slug: String,
    pub title: String,
    pub word_count: u64,
}

#[derive(Debug, Serialize)]
pub struct PipelineStatus {
    pub phase: String,
    pub content_ai_completed_at: Option<u64>,
    pub review_ai_completed_at: Option<u64>,
    pub needs_human_review: bool,
    pub notes: String,
}

// ── HTML helpers ──────────────────────────────────────────────────────────────

/// Strip HTML tags from `html` for word-count purposes.
/// Replaces `>` with a space so tag-adjacent words don't run together.
fn strip_html_for_counting(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            result.push(' ');
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// Count words in an HTML string by stripping tags then splitting on whitespace.
fn count_words(html: &str) -> u64 {
    strip_html_for_counting(html).split_whitespace().count() as u64
}

// ── Main assembler ────────────────────────────────────────────────────────────

/// Assemble all page intelligence for `content_id` from the WAL.
///
/// Returns `Err(String)` if the page is not found or cannot be deserialized.
/// All other sub-components (SEO meta, surfer map, queue item, siblings) fall
/// back to defaults or `None` if absent — a missing component is never fatal.
pub fn assemble(journal: &mut ForgeJournal, content_id: &str) -> Result<PageIntelligence, String> {
    // ── 1. Load page content ──────────────────────────────────────────────────
    // Create manager, load the page, then drop the manager to release the
    // mutable borrow before we call other sub-modules.
    let page_content = {
        let mgr = ForgeContentManager::new(journal);
        mgr.get_content(content_id)
            .map_err(|e| format!("get_content error: {e}"))?
            .ok_or_else(|| format!("page not found: {content_id}"))?
    };

    // ── 2. Parse blocks ───────────────────────────────────────────────────────
    let body_json = &page_content.body_json;
    let blocks: serde_json::Value = if body_json.trim_start().starts_with('[') {
        serde_json::from_str(body_json).unwrap_or(serde_json::Value::Array(vec![]))
    } else {
        serde_json::json!([{
            "type": "html",
            "data": { "content": body_json }
        }])
    };

    // ── 3. Word count ─────────────────────────────────────────────────────────
    let word_count = count_words(body_json);

    // ── 4. Load SEO meta ──────────────────────────────────────────────────────
    let seo_meta: Option<SeoMeta> = {
        let event = journal.get_latest(AGG_SEO_META, content_id);
        match event {
            Some(e) if e.payload == TOMBSTONE => None,
            Some(e) => serde_json::from_slice(&e.payload).ok(),
            None => None,
        }
    };

    let seo_info = match seo_meta {
        Some(ref m) => SeoInfo {
            title: m.title.clone(),
            description: m.description.clone(),
            focus_keyword: m.focus_keyword.clone(),
            canonical_url: m.canonical_url.clone(),
            og_image: m.og_image.clone(),
            schema_json: m.schema_json.clone(),
            seo_score: m.seo_score,
        },
        None => SeoInfo {
            title: String::new(),
            description: String::new(),
            focus_keyword: String::new(),
            canonical_url: String::new(),
            og_image: String::new(),
            schema_json: String::new(),
            seo_score: 0,
        },
    };

    // ── 5. Keyword gate ───────────────────────────────────────────────────────
    let focus_keyword = seo_meta
        .as_ref()
        .map(|m| m.focus_keyword.as_str())
        .unwrap_or("");
    let seo_title = seo_meta.as_ref().map(|m| m.title.as_str()).unwrap_or("");
    let seo_description = seo_meta
        .as_ref()
        .map(|m| m.description.as_str())
        .unwrap_or("");

    let kw_checks = super::keyword_gate::keyword_consistency_check(
        focus_keyword,
        &page_content.slug,
        seo_title,
        seo_description,
        body_json,
    );
    let (passed, total) = super::keyword_gate::keyword_score(&kw_checks);
    let kw_score_str = format!("{}/{}", passed, total);

    let keyword_gate = KeywordGateInfo {
        checks: kw_checks,
        score: kw_score_str,
    };

    // ── 6. Surfer map + sheet + scoring ───────────────────────────────────────
    let surfer_info: Option<SurferInfo> = {
        let surfer_map = super::surfer_map::load_map(journal, content_id);
        match surfer_map {
            None => None,
            Some(map) => {
                let primary_sheet_id = &map.primary_sheet_id;
                match super::surfer::load_sheet(journal, primary_sheet_id) {
                    None => None,
                    Some(sheet) => {
                        let score = super::surfer_scoring::score_against_sheet(body_json, &sheet);

                        // Count total facts across all groups
                        let facts_available: usize =
                            sheet.facts.iter().map(|fg| fg.items.len()).sum();

                        // Secondary sheet IDs = all except the primary
                        let secondary_sheets: Vec<String> = map
                            .sheet_ids
                            .iter()
                            .filter(|id| id.as_str() != primary_sheet_id)
                            .cloned()
                            .collect();

                        Some(SurferInfo {
                            primary_sheet: primary_sheet_id.clone(),
                            secondary_sheets,
                            score,
                            facts_available,
                        })
                    }
                }
            }
        }
    };

    // ── 7. Sibling pages ──────────────────────────────────────────────────────
    let current_page_type = super::content_queue::detect_page_type(&page_content.slug);

    let sibling_pages: Vec<SiblingPage> = {
        let mgr = ForgeContentManager::new(journal);
        // Load up to 200 published pages to find siblings
        let (all_published, _) = mgr
            .list_content(None, Some("published"), None, 200, 0, None, None)
            .unwrap_or_default();

        all_published
            .into_iter()
            .filter(|c| {
                c.content_id != content_id
                    && super::content_queue::detect_page_type(&c.slug) == current_page_type
            })
            .take(20)
            .map(|c| {
                let wc = count_words(&c.body_json);
                SiblingPage {
                    slug: c.slug,
                    title: c.title,
                    word_count: wc,
                }
            })
            .collect()
    };

    // ── 8. Pipeline / queue status ────────────────────────────────────────────
    let pipeline_status = match super::content_queue::load_item(journal, content_id) {
        Some(item) => {
            let phase = format!("{:?}", item.phase);
            PipelineStatus {
                phase,
                content_ai_completed_at: item.content_ai_completed_at,
                review_ai_completed_at: item.review_ai_completed_at,
                needs_human_review: item.needs_human_review,
                notes: item.notes,
            }
        }
        None => PipelineStatus {
            phase: "NotQueued".to_string(),
            content_ai_completed_at: None,
            review_ai_completed_at: None,
            needs_human_review: false,
            notes: String::new(),
        },
    };

    // ── 9. Platform capabilities (hardcoded feature map) ──────────────────────
    let platform_capabilities = serde_json::json!({
        "scheduling": true,
        "invoicing": true,
        "customer_portal": true,
        "chemical_tracking": true,
        "service_areas": true,
        "booking": true,
        "seo": true,
        "ai_content": true,
        "orbit_social": true,
        "commerce": true,
        "email_marketing": true,
        "blog": true,
        "family_hub": true,
        "notifications": true,
        "analytics": true
    });

    // ── Assemble final struct ─────────────────────────────────────────────────
    Ok(PageIntelligence {
        page: PageInfo {
            content_id: page_content.content_id,
            slug: page_content.slug,
            title: page_content.title,
            status: page_content.status,
            word_count,
            updated_at: page_content.updated_at,
        },
        blocks,
        seo: seo_info,
        keyword_gate,
        surfer: surfer_info,
        sibling_pages,
        platform_capabilities,
        insights: vec![],
        pipeline_status,
    })
}
