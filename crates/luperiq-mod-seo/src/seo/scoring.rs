use super::SeoMeta;

/// Optional Google Search Console data for a page.
pub(crate) struct GscPageSignals {
    pub impressions: u64,
    pub _clicks: u64,
    pub avg_position: f64,
    pub ctr: f64,
}

/// Calculate Google signals score component (0-100).
pub(crate) fn google_signals_score(gsc: Option<&GscPageSignals>) -> u8 {
    let Some(data) = gsc else { return 0 };

    let mut score: u8 = 0;

    // Has any GSC data at all: +25
    score += 25;

    // Impressions > 100/month: +25
    if data.impressions > 100 {
        score += 25;
    }

    // Avg position < 20 (top 20 results): +25
    if data.avg_position > 0.0 && data.avg_position < 20.0 {
        score += 25;
    }

    // CTR > 3%: +25
    if data.ctr > 3.0 {
        score += 25;
    }

    score.min(100)
}

pub(crate) fn calculate_seo_score(
    meta: &SeoMeta,
    content_title: &str,
    content_body: &str,
    gsc: Option<&GscPageSignals>,
) -> u8 {
    let mut score: u8 = 0;

    // Title (0-25 points)
    if !meta.title.is_empty() {
        score += 10;
    }
    if (30..=60).contains(&meta.title.len()) {
        score += 10;
    }
    if !meta.focus_keyword.is_empty()
        && meta
            .title
            .to_lowercase()
            .contains(&meta.focus_keyword.to_lowercase())
    {
        score += 5;
    }

    // Description (0-25 points)
    if !meta.description.is_empty() {
        score += 10;
    }
    if (120..=160).contains(&meta.description.len()) {
        score += 10;
    }
    if !meta.focus_keyword.is_empty()
        && meta
            .description
            .to_lowercase()
            .contains(&meta.focus_keyword.to_lowercase())
    {
        score += 5;
    }

    // Content (0-25 points)
    let word_count = content_body.split_whitespace().count();
    if word_count >= 300 {
        score += 10;
    }
    if word_count >= 800 {
        score += 5;
    }
    if !meta.focus_keyword.is_empty()
        && content_body
            .to_lowercase()
            .contains(&meta.focus_keyword.to_lowercase())
    {
        score += 10;
    }

    // Technical (0-25 points)
    if !meta.og_image.is_empty() {
        score += 5;
    }
    if !meta.canonical_url.is_empty() {
        score += 5;
    }
    if !meta.schema_json.is_empty() {
        score += 5;
    }
    if !meta.robots.is_empty() {
        score += 5;
    }
    if content_title.len() <= 70 {
        score += 5;
    }

    let base_score = score.min(100);

    let g_score = google_signals_score(gsc);

    if g_score > 0 {
        // Weight: 80% base + 20% Google
        let combined = (base_score as f64 * 0.8 + g_score as f64 * 0.2).round() as u8;
        combined.min(100)
    } else {
        // No Google data — base score unchanged
        base_score
    }
}
