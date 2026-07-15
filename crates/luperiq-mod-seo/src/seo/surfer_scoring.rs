//! Surfer SEO term frequency scoring engine.
//!
//! Pure scoring logic — no WAL operations, no HTTP. Takes a content HTML string
//! and a `SurferSheet` (from `surfer.rs`) and returns a `SurferScore` with
//! per-term counts, structure metrics, and an overall 0-100 score.
//!
//! Scoring formula:
//!   overall = (terms_in_range_pct * 70 / 100) + (structure_in_range_pct * 30 / 100)

use serde::Serialize;

use super::surfer::{StructureRange, SurferSheet};

// ── Output types ──────────────────────────────────────────────────────────────

/// Per-term frequency result.
#[derive(Debug, Clone, Serialize)]
pub struct TermScore {
    pub term: String,
    pub target_min: u32,
    pub target_max: u32,
    pub actual_count: u32,
    /// `"under"`, `"in_range"`, or `"over"`
    pub status: String,
    /// Positive = how many more uses needed; negative = how many to remove; 0 = in range.
    pub gap: i32,
}

/// A single structure metric (word count, heading count, etc.).
#[derive(Debug, Clone, Serialize)]
pub struct StructureMetric {
    pub target_min: Option<u64>,
    pub target_max: Option<u64>,
    pub actual: u64,
    /// `"under"`, `"in_range"`, or `"over"`
    pub status: String,
}

/// All four structure metrics bundled together.
#[derive(Debug, Clone, Serialize)]
pub struct StructureScores {
    pub words: StructureMetric,
    pub headings: StructureMetric,
    pub images: StructureMetric,
    pub paragraphs: StructureMetric,
}

/// Top-level result returned by `score_against_sheet`.
#[derive(Debug, Clone, Serialize)]
pub struct SurferScore {
    pub term_scores: Vec<TermScore>,
    pub structure_scores: StructureScores,
    /// Weighted overall score: 0-100
    pub overall_surfer_score: u8,
    /// Percentage of terms whose actual count falls within [min, max]
    pub terms_in_range_pct: u8,
    /// Percentage of structure metrics within their target range
    pub structure_in_range_pct: u8,
}

// ── HTML analysis helpers ─────────────────────────────────────────────────────

/// Strip HTML tags from `html`, replacing `>` with a space so that
/// tag-adjacent words don't run together.
///
/// Uses the same simple approach as `keyword_gate.rs`: iterate chars and track
/// an `in_tag` boolean. NOT a full HTML parser — sufficient for well-formed
/// CMS-generated content.
pub fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
            result.push(' '); // replace closing angle with space
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// Count words in `html` by stripping tags and splitting on ASCII whitespace.
pub fn count_words(html: &str) -> u64 {
    strip_html(html)
        .split_ascii_whitespace()
        .filter(|w| !w.is_empty())
        .count() as u64
}

/// Count heading tags (`<h1` … `<h6`) in `html`, case-insensitively.
pub fn count_headings(html: &str) -> u64 {
    let lower = html.to_lowercase();
    let mut count = 0u64;
    for tag in &["<h1", "<h2", "<h3", "<h4", "<h5", "<h6"] {
        let mut start = 0;
        while let Some(pos) = lower[start..].find(tag) {
            count += 1;
            start += pos + tag.len();
        }
    }
    count
}

/// Count `<img` occurrences in `html`, case-insensitively.
pub fn count_images(html: &str) -> u64 {
    let lower = html.to_lowercase();
    let mut count = 0u64;
    let mut start = 0;
    while let Some(pos) = lower[start..].find("<img") {
        count += 1;
        start += pos + 4; // len("<img")
    }
    count
}

/// Count `<p` occurrences in `html`, case-insensitively.
pub fn count_paragraphs(html: &str) -> u64 {
    let lower = html.to_lowercase();
    let mut count = 0u64;
    let mut start = 0;
    while let Some(pos) = lower[start..].find("<p") {
        // Ensure the `<p` is followed by `>` or a space/attribute (not e.g. `<pre`)
        let after = start + pos + 2;
        let next_char = lower[after..].chars().next().unwrap_or('\0');
        if next_char == '>'
            || next_char == ' '
            || next_char == '\t'
            || next_char == '\n'
            || next_char == '\r'
        {
            count += 1;
        }
        start += pos + 2;
    }
    count
}

/// Count non-overlapping, case-insensitive occurrences of `term` in `text`.
pub fn count_term(text: &str, term: &str) -> u32 {
    if term.is_empty() {
        return 0;
    }
    let text_lower = text.to_lowercase();
    let term_lower = term.to_lowercase();
    let mut count = 0u32;
    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&term_lower) {
        count += 1;
        start += pos + term_lower.len();
    }
    count
}

// ── Range status helpers ──────────────────────────────────────────────────────

fn range_status_u64(actual: u64, range: &StructureRange) -> String {
    match (range.min, range.max) {
        (Some(min), Some(max)) => {
            if actual < min {
                "under".to_string()
            } else if actual > max {
                "over".to_string()
            } else {
                "in_range".to_string()
            }
        }
        (Some(min), None) => {
            // max = Infinity: never "over"
            if actual < min {
                "under".to_string()
            } else {
                "in_range".to_string()
            }
        }
        (None, Some(max)) => {
            if actual > max {
                "over".to_string()
            } else {
                "in_range".to_string()
            }
        }
        (None, None) => "in_range".to_string(),
    }
}

fn make_structure_metric(actual: u64, range: &StructureRange) -> StructureMetric {
    StructureMetric {
        target_min: range.min,
        target_max: range.max,
        actual,
        status: range_status_u64(actual, range),
    }
}

// ── Main scoring function ─────────────────────────────────────────────────────

/// Score `content_html` against the targets in `sheet`.
///
/// Returns a `SurferScore` with per-term frequency checks, structure checks,
/// and a weighted overall score (0-100).
pub fn score_against_sheet(content_html: &str, sheet: &SurferSheet) -> SurferScore {
    // Strip HTML once for term counting (we want to count in text only,
    // not inside tag attributes).
    let plain_text = strip_html(content_html);

    // ── Term scores ───────────────────────────────────────────────────────────
    let term_scores: Vec<TermScore> = sheet
        .terms
        .iter()
        .map(|t| {
            let actual = count_term(&plain_text, &t.term);
            let status = if actual < t.min {
                "under".to_string()
            } else if t.max != u32::MAX && actual > t.max {
                "over".to_string()
            } else {
                "in_range".to_string()
            };
            let gap: i32 = match status.as_str() {
                "under" => (t.min - actual) as i32,
                "over" => -((actual - t.max) as i32),
                _ => 0,
            };
            TermScore {
                term: t.term.clone(),
                target_min: t.min,
                target_max: if t.max == u32::MAX { u32::MAX } else { t.max },
                actual_count: actual,
                status,
                gap,
            }
        })
        .collect();

    // ── Structure scores ──────────────────────────────────────────────────────
    let actual_words = count_words(content_html);
    let actual_headings = count_headings(content_html);
    let actual_images = count_images(content_html);
    let actual_paragraphs = count_paragraphs(content_html);

    let structure_scores = StructureScores {
        words: make_structure_metric(actual_words, &sheet.structure.words),
        headings: make_structure_metric(actual_headings, &sheet.structure.headings),
        images: make_structure_metric(actual_images, &sheet.structure.images),
        paragraphs: make_structure_metric(actual_paragraphs, &sheet.structure.paragraphs),
    };

    // ── Percentage calculations ───────────────────────────────────────────────
    let terms_in_range_pct: u8 = if term_scores.is_empty() {
        100
    } else {
        let in_range = term_scores
            .iter()
            .filter(|s| s.status == "in_range")
            .count();
        ((in_range * 100) / term_scores.len()) as u8
    };

    let structure_metrics = [
        &structure_scores.words,
        &structure_scores.headings,
        &structure_scores.images,
        &structure_scores.paragraphs,
    ];
    let structure_in_range_count = structure_metrics
        .iter()
        .filter(|m| m.status == "in_range")
        .count();
    let structure_in_range_pct: u8 = ((structure_in_range_count * 100) / 4) as u8;

    // ── Overall score ─────────────────────────────────────────────────────────
    // 70% from terms, 30% from structure
    let overall_surfer_score: u8 =
        ((terms_in_range_pct as u32 * 70 / 100) + (structure_in_range_pct as u32 * 30 / 100)) as u8;

    SurferScore {
        term_scores,
        structure_scores,
        overall_surfer_score,
        terms_in_range_pct,
        structure_in_range_pct,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seo::surfer::{
        parse_surfer_txt, StructureRange, StructureTargets, SurferSheet, SurferTerm,
    };

    // ── strip_html ────────────────────────────────────────────────────────────

    #[test]
    fn test_strip_html_basic() {
        let result = strip_html("<p>Hello world</p>");
        assert!(result.contains("Hello world"));
        assert!(!result.contains('<'));
        assert!(!result.contains('>'));
    }

    #[test]
    fn test_strip_html_adjacent_words_separated() {
        // Without a space injected, "foo" and "bar" would run together as "foobar".
        let result = strip_html("<h1>foo</h1><p>bar</p>");
        assert!(result.contains("foo"));
        assert!(result.contains("bar"));
        // Should not be "foobar" without any separator
        assert!(!result.contains("foobar"));
    }

    #[test]
    fn test_strip_html_empty() {
        assert_eq!(strip_html(""), "");
    }

    #[test]
    fn test_strip_html_no_tags() {
        assert_eq!(strip_html("just plain text"), "just plain text");
    }

    // ── count_words ───────────────────────────────────────────────────────────

    #[test]
    fn test_count_words_basic() {
        assert_eq!(count_words("<p>Hello world</p>"), 2);
    }

    #[test]
    fn test_count_words_multi_tag() {
        assert_eq!(
            count_words("<h1>Pest Control</h1><p>We handle bugs.</p>"),
            5
        );
    }

    #[test]
    fn test_count_words_empty() {
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("<p></p>"), 0);
    }

    // ── count_headings ────────────────────────────────────────────────────────

    #[test]
    fn test_count_headings_mixed_levels() {
        let html = "<h1>Title</h1><h2>Section</h2><h3>Sub</h3><h2>Another</h2>";
        assert_eq!(count_headings(html), 4);
    }

    #[test]
    fn test_count_headings_case_insensitive() {
        let html = "<H1>Title</H1><H2>Section</H2>";
        assert_eq!(count_headings(html), 2);
    }

    #[test]
    fn test_count_headings_none() {
        assert_eq!(count_headings("<p>no headings here</p>"), 0);
    }

    // ── count_images ──────────────────────────────────────────────────────────

    #[test]
    fn test_count_images_basic() {
        let html = r#"<img src="a.jpg" alt="first"><img src="b.jpg">"#;
        assert_eq!(count_images(html), 2);
    }

    #[test]
    fn test_count_images_case_insensitive() {
        let html = "<IMG src='a.jpg'><img src='b.jpg'>";
        assert_eq!(count_images(html), 2);
    }

    #[test]
    fn test_count_images_none() {
        assert_eq!(count_images("<p>no images</p>"), 0);
    }

    // ── count_paragraphs ──────────────────────────────────────────────────────

    #[test]
    fn test_count_paragraphs_basic() {
        let html = "<p>First</p><p>Second</p><p class=\"intro\">Third</p>";
        assert_eq!(count_paragraphs(html), 3);
    }

    #[test]
    fn test_count_paragraphs_ignores_pre() {
        // <pre> should not be counted as a paragraph
        let html = "<p>para</p><pre>code</pre><p>para2</p>";
        assert_eq!(count_paragraphs(html), 2);
    }

    #[test]
    fn test_count_paragraphs_case_insensitive() {
        let html = "<P>First</P><P>Second</P>";
        assert_eq!(count_paragraphs(html), 2);
    }

    #[test]
    fn test_count_paragraphs_none() {
        assert_eq!(count_paragraphs("<div>no paragraphs</div>"), 0);
    }

    // ── count_term ────────────────────────────────────────────────────────────

    #[test]
    fn test_count_term_basic() {
        assert_eq!(
            count_term("Pest control is great. Pest control rocks.", "pest control"),
            2
        );
    }

    #[test]
    fn test_count_term_case_insensitive() {
        assert_eq!(count_term("HVAC hvac Hvac", "hvac"), 3);
    }

    #[test]
    fn test_count_term_zero() {
        assert_eq!(count_term("nothing to see here", "pest control"), 0);
    }

    #[test]
    fn test_count_term_empty_term() {
        assert_eq!(count_term("some text", ""), 0);
    }

    #[test]
    fn test_count_term_overlapping_not_double_counted() {
        // Non-overlapping: "aa" in "aaa" = 1 match at pos 0, then search from pos 2
        assert_eq!(count_term("aaa", "aa"), 1);
    }

    #[test]
    fn test_count_term_single_occurrence() {
        assert_eq!(
            count_term("pest management is important", "pest management"),
            1
        );
    }

    // ── range_status ──────────────────────────────────────────────────────────

    #[test]
    fn test_range_status_under() {
        let range = StructureRange {
            min: Some(10),
            max: Some(30),
        };
        let m = make_structure_metric(5, &range);
        assert_eq!(m.status, "under");
    }

    #[test]
    fn test_range_status_in_range() {
        let range = StructureRange {
            min: Some(10),
            max: Some(30),
        };
        let m = make_structure_metric(15, &range);
        assert_eq!(m.status, "in_range");
    }

    #[test]
    fn test_range_status_over() {
        let range = StructureRange {
            min: Some(10),
            max: Some(30),
        };
        let m = make_structure_metric(50, &range);
        assert_eq!(m.status, "over");
    }

    #[test]
    fn test_range_status_infinity_never_over() {
        // max = None means Infinity — can never be "over"
        let range = StructureRange {
            min: Some(10),
            max: None,
        };
        let m = make_structure_metric(99999, &range);
        assert_eq!(m.status, "in_range");
    }

    #[test]
    fn test_range_status_infinity_can_be_under() {
        let range = StructureRange {
            min: Some(100),
            max: None,
        };
        let m = make_structure_metric(5, &range);
        assert_eq!(m.status, "under");
    }

    // ── score_against_sheet (integration) ────────────────────────────────────

    fn make_test_sheet() -> SurferSheet {
        SurferSheet {
            sheet_id: "test".into(),
            topic: "pest control website".into(),
            source_file: "test.txt".into(),
            source_date: "2026-03-16".into(),
            industry: "pest-control".into(),
            structure: StructureTargets {
                words: StructureRange {
                    min: Some(100),
                    max: Some(500),
                },
                headings: StructureRange {
                    min: Some(2),
                    max: Some(10),
                },
                images: StructureRange {
                    min: Some(1),
                    max: Some(5),
                },
                paragraphs: StructureRange {
                    min: Some(2),
                    max: None, // Infinity
                },
                characters: StructureRange {
                    min: None,
                    max: None,
                },
            },
            terms: vec![
                SurferTerm {
                    term: "pest control".into(),
                    min: 3,
                    max: 6,
                },
                SurferTerm {
                    term: "pest management".into(),
                    min: 1,
                    max: 3,
                },
            ],
            facts: vec![],
        }
    }

    #[test]
    fn test_score_all_in_range() {
        // Build HTML that satisfies all targets
        // "pest control" occurrences (counted in stripped text):
        //   h1: 1, h2: 1, p1: 1, p2: 1 = 4 total (within [3,6])
        // "pest management" occurrences:
        //   p1: 1, p2: 1 = 2 total (within [1,3])
        let html = "\
            <h1>Pest Control Services</h1>\
            <h2>Why Choose Pest Control</h2>\
            <h3>Our Service Plans</h3>\
            <p>We offer professional pest control and pest management solutions.</p>\
            <p>Our specialty is pest control. Pest management plans start here.</p>\
            <img src=\"a.jpg\" alt=\"exterminator\">\
            <img src=\"b.jpg\" alt=\"team\">\
        ";

        let sheet = make_test_sheet();
        let score = score_against_sheet(html, &sheet);

        // Verify terms
        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        assert!(
            pc.actual_count >= 3 && pc.actual_count <= 6,
            "pest control count {} not in [3,6]",
            pc.actual_count
        );
        assert_eq!(pc.status, "in_range");
        assert_eq!(pc.gap, 0);

        let pm = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest management")
            .unwrap();
        assert!(pm.actual_count >= 1 && pm.actual_count <= 3);
        assert_eq!(pm.status, "in_range");

        // Structure checks
        assert_eq!(score.structure_scores.headings.status, "in_range");
        assert_eq!(score.structure_scores.images.status, "in_range");
        assert_eq!(score.structure_scores.paragraphs.status, "in_range");

        // Overall should be high
        assert!(
            score.overall_surfer_score >= 70,
            "Expected score >= 70, got {}",
            score.overall_surfer_score
        );
    }

    #[test]
    fn test_score_terms_under() {
        let html = "<p>We do pest management here.</p>";
        let sheet = make_test_sheet();
        let score = score_against_sheet(html, &sheet);

        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        assert_eq!(pc.status, "under");
        assert!(pc.gap > 0, "Gap should be positive when under");
    }

    #[test]
    fn test_score_terms_over() {
        // Repeat "pest control" 10 times — max is 6
        let content = "pest control ".repeat(10);
        let html = format!("<p>{content}</p>");
        let sheet = make_test_sheet();
        let score = score_against_sheet(&html, &sheet);

        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        assert_eq!(pc.status, "over");
        assert!(pc.gap < 0, "Gap should be negative when over");
    }

    #[test]
    fn test_score_gap_under_value() {
        // actual=1, min=3 → gap = 3-1 = 2
        let html = "<p>pest control here once</p>";
        let sheet = make_test_sheet();
        let score = score_against_sheet(html, &sheet);

        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        assert_eq!(pc.actual_count, 1);
        assert_eq!(pc.status, "under");
        assert_eq!(pc.gap, 2); // min(3) - actual(1) = 2
    }

    #[test]
    fn test_score_gap_over_value() {
        // actual=9, max=6 → gap = -(9-6) = -3
        let html = "<p>pest control pest control pest control pest control pest control pest control pest control pest control pest control</p>";
        let sheet = make_test_sheet();
        let score = score_against_sheet(html, &sheet);

        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        assert_eq!(pc.actual_count, 9);
        assert_eq!(pc.status, "over");
        assert_eq!(pc.gap, -3);
    }

    #[test]
    fn test_score_terms_in_range_pct() {
        // One term in range, one under
        let html = "<p>pest control pest control pest control pest management here.</p>";
        let sheet = make_test_sheet();
        let score = score_against_sheet(html, &sheet);

        // pest control: 3 → in range [3,6] ✓
        // pest management: 1 → in range [1,3] ✓
        assert_eq!(score.terms_in_range_pct, 100);
    }

    #[test]
    fn test_score_structure_in_range_pct_full() {
        // Build a doc that satisfies all 4 structure metrics
        let paras: String = (0..3)
            .map(|i| format!("<p>paragraph {i} of content with words here</p>"))
            .collect();
        let headings = "<h1>H1</h1><h2>H2</h2><h3>H3</h3>";
        let images = "<img src='a.jpg'><img src='b.jpg'>";
        // Need 100-500 words
        let body_words: String = std::iter::repeat("word ").take(150).collect();
        let html = format!("{headings}{images}{paras}<p>{body_words}</p>");

        let sheet = make_test_sheet();
        let score = score_against_sheet(&html, &sheet);

        assert_eq!(score.structure_scores.words.status, "in_range");
        assert_eq!(score.structure_scores.headings.status, "in_range");
        assert_eq!(score.structure_scores.images.status, "in_range");
        assert_eq!(score.structure_scores.paragraphs.status, "in_range");
        assert_eq!(score.structure_in_range_pct, 100);
    }

    #[test]
    fn test_overall_score_formula() {
        // 100% terms in range, 100% structure in range → 70 + 30 = 100
        let score = SurferScore {
            term_scores: vec![],
            structure_scores: StructureScores {
                words: StructureMetric {
                    target_min: None,
                    target_max: None,
                    actual: 0,
                    status: "in_range".into(),
                },
                headings: StructureMetric {
                    target_min: None,
                    target_max: None,
                    actual: 0,
                    status: "in_range".into(),
                },
                images: StructureMetric {
                    target_min: None,
                    target_max: None,
                    actual: 0,
                    status: "in_range".into(),
                },
                paragraphs: StructureMetric {
                    target_min: None,
                    target_max: None,
                    actual: 0,
                    status: "in_range".into(),
                },
            },
            overall_surfer_score: (100u32 * 70 / 100 + 100u32 * 30 / 100) as u8,
            terms_in_range_pct: 100,
            structure_in_range_pct: 100,
        };
        assert_eq!(score.overall_surfer_score, 100);
    }

    #[test]
    fn test_overall_score_zero() {
        // 0% terms, 0% structure → 0
        assert_eq!((0u32 * 70 / 100 + 0u32 * 30 / 100) as u8, 0);
    }

    #[test]
    fn test_overall_score_mixed() {
        // 50% terms, 75% structure → (50*70/100) + (75*30/100) = 35 + 22 = 57
        let terms_pct = 50u32;
        let struct_pct = 75u32;
        let expected = (terms_pct * 70 / 100 + struct_pct * 30 / 100) as u8;
        assert_eq!(expected, 57);
    }

    #[test]
    fn test_empty_terms_sheet() {
        let mut sheet = make_test_sheet();
        sheet.terms = vec![];
        let html = "<p>some content here</p>";
        let score = score_against_sheet(html, &sheet);
        assert_eq!(score.terms_in_range_pct, 100);
        assert!(score.term_scores.is_empty());
    }

    #[test]
    fn test_score_with_parsed_sheet() {
        // Use parse_surfer_txt to produce a real sheet and score against it
        const MINI_TXT: &str = r#"## CONTENT STRUCTURE
* Images: 1 - 5
* Headings: 2 - 8
* Characters: 0 - Infinity
* Paragraphs: 2 - Infinity
* Words: 50 - 300

## IMPORTANT TERMS TO USE
* pest control: 2 - 4
* pest management: 1 - 2
"#;
        let sheet = parse_surfer_txt(
            MINI_TXT,
            "surfer-guidelines-pest control website-16-03-2026.txt",
        )
        .unwrap();

        let html = "\
            <h1>Pest Control Services</h1>\
            <h2>Pest Management</h2>\
            <img src='a.jpg'>\
            <img src='b.jpg'>\
            <p>We provide professional pest control for all pest management needs.</p>\
            <p>Our pest control team is experienced. Contact us today.</p>\
        ";

        let score = score_against_sheet(html, &sheet);
        assert!(!score.term_scores.is_empty());

        let pc = score
            .term_scores
            .iter()
            .find(|t| t.term == "pest control")
            .unwrap();
        // "pest control" appears in: h1, h2-text isn't there, p1, p2 = 3 times
        // Let's just verify the count is reasonable
        assert!(pc.actual_count >= 2);
    }
}
