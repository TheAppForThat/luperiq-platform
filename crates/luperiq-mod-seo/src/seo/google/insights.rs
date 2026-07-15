//! Rule-based SEO insights engine — analyzes cached Google data to produce
//! actionable recommendations without any AI API calls (zero credits).
//!
//! Insight rules run on cached GSC/GA4 data and return structured findings.

use serde::{Deserialize, Serialize};

// ── CTR benchmarks by position ────────────────────────────────────

pub const CTR_BENCHMARKS: &[(u32, f64)] = &[
    (1, 31.7),
    (2, 24.7),
    (3, 18.6),
    (4, 13.6),
    (5, 9.5),
    (6, 6.2),
    (7, 4.2),
    (8, 3.1),
    (9, 2.8),
    (10, 2.5),
];

pub fn expected_ctr_for_position(position: f64) -> f64 {
    let pos = position.round() as u32;
    for &(p, ctr) in CTR_BENCHMARKS {
        if p == pos {
            return ctr;
        }
    }
    if pos > 10 {
        1.0
    } else {
        2.0
    }
}

// ── Insight types ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeoInsight {
    pub rule: String,
    pub severity: InsightSeverity,
    pub title: String,
    pub description: String,
    pub metric_label: String,
    pub metric_value: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_impact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InsightSeverity {
    Critical,
    Warning,
    Info,
    Opportunity,
}

// ── Query-level insight data ──────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct QueryMetrics {
    pub query: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
}

// ── Page-level insight data ───────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct PageMetrics {
    pub page: String,
    pub clicks: f64,
    pub impressions: f64,
    pub ctr: f64,
    pub position: f64,
    #[serde(default)]
    pub bounce_rate: Option<f64>,
    #[serde(default)]
    pub avg_duration: Option<f64>,
    #[serde(default)]
    pub sessions: Option<f64>,
    #[serde(default)]
    pub content_age_days: Option<u64>,
    #[serde(default)]
    pub word_count: Option<u64>,
}

// ── Rule engine ───────────────────────────────────────────────────

pub struct InsightsEngine;

impl InsightsEngine {
    /// Analyze a set of queries and produce insights.
    pub fn analyze_queries(queries: &[QueryMetrics]) -> Vec<SeoInsight> {
        let mut insights = Vec::new();

        for q in queries {
            // Rule 1: Low CTR with high position (appearing but not clicking)
            if q.position <= 10.0 && q.impressions > 20.0 {
                let expected = expected_ctr_for_position(q.position);
                if q.ctr < expected * 0.5 {
                    insights.push(SeoInsight {
                        rule: "low_ctr_high_position".into(),
                        severity: InsightSeverity::Warning,
                        title: format!("Low CTR for \"{}\"", q.query),
                        description: format!(
                            "Ranking at position {:.1} but CTR is {:.1}% (expected {:.1}%). \
                             Your title/description may not be compelling enough.",
                            q.position, q.ctr, expected
                        ),
                        metric_label: "CTR Gap".into(),
                        metric_value: format!("{:.1}% below expected", expected - q.ctr),
                        action: "Improve the title tag and meta description to be more compelling and relevant to the search query.".into(),
                        estimated_impact: Some(format!(
                            "Closing the CTR gap could add ~{:.0} clicks/period",
                            q.impressions * (expected - q.ctr) / 100.0
                        )),
                    });
                }
            }

            // Rule 2: Page 2 keywords (striking distance — positions 11-20)
            if q.position >= 11.0 && q.position <= 20.0 && q.impressions > 50.0 {
                insights.push(SeoInsight {
                    rule: "striking_distance".into(),
                    severity: InsightSeverity::Opportunity,
                    title: format!("Striking distance: \"{}\"", q.query),
                    description: format!(
                        "Ranking at position {:.1} with {} impressions. A small ranking improvement \
                         could move this to page 1.",
                        q.position, q.impressions as u64
                    ),
                    metric_label: "Current Position".into(),
                    metric_value: format!("{:.1}", q.position),
                    action: "Strengthen internal linking, add content depth, and optimize on-page SEO for this keyword.".into(),
                    estimated_impact: Some(format!(
                        "Moving to top 10 could generate ~{:.0} clicks/period",
                        q.impressions * 0.025 // ~2.5% CTR at position 10
                    )),
                });
            }

            // Rule 3: Zero clicks with impressions
            if q.clicks == 0.0 && q.impressions > 30.0 {
                insights.push(SeoInsight {
                    rule: "zero_clicks".into(),
                    severity: InsightSeverity::Warning,
                    title: format!("Zero clicks for \"{}\"", q.query),
                    description: format!(
                        "Appearing {} times but getting no clicks. Position: {:.1}.",
                        q.impressions as u64, q.position
                    ),
                    metric_label: "Impressions".into(),
                    metric_value: format!("{}", q.impressions as u64),
                    action: "Review whether the title and description match search intent. Consider if this query is relevant to your content.".into(),
                    estimated_impact: None,
                });
            }
        }

        // Rule 4: Keyword cannibalization (multiple queries ranking for same broad term)
        // Check for queries with very similar positions that might be competing
        let mut page_queries: std::collections::HashMap<String, Vec<&QueryMetrics>> =
            std::collections::HashMap::new();
        for q in queries {
            // Group by first two words for rough grouping
            let key: String = q
                .query
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            page_queries.entry(key).or_default().push(q);
        }
        for (group, qs) in &page_queries {
            if qs.len() >= 3 && !group.is_empty() {
                let total_impressions: f64 = qs.iter().map(|q| q.impressions).sum();
                if total_impressions > 100.0 {
                    insights.push(SeoInsight {
                        rule: "keyword_cluster".into(),
                        severity: InsightSeverity::Info,
                        title: format!("Keyword cluster: \"{}...\"", group),
                        description: format!(
                            "{} related queries with {} total impressions. Consider creating a dedicated page.",
                            qs.len(), total_impressions as u64
                        ),
                        metric_label: "Related Queries".into(),
                        metric_value: format!("{}", qs.len()),
                        action: "Group these related queries into a comprehensive content piece targeting the cluster.".into(),
                        estimated_impact: None,
                    });
                }
            }
        }

        // Sort by severity (critical first)
        insights.sort_by_key(|i| match i.severity {
            InsightSeverity::Critical => 0,
            InsightSeverity::Warning => 1,
            InsightSeverity::Opportunity => 2,
            InsightSeverity::Info => 3,
        });

        insights
    }

    /// Analyze page-level metrics and produce insights.
    pub fn analyze_pages(pages: &[PageMetrics]) -> Vec<SeoInsight> {
        let mut insights = Vec::new();

        for p in pages {
            // Rule 5: High bounce rate
            if let Some(bounce) = p.bounce_rate {
                if bounce > 70.0 && p.sessions.unwrap_or(0.0) > 10.0 {
                    insights.push(SeoInsight {
                        rule: "high_bounce".into(),
                        severity: InsightSeverity::Warning,
                        title: format!("High bounce rate on {}", short_path(&p.page)),
                        description: format!(
                            "Bounce rate is {:.1}% with {} sessions. Content may not match search intent \
                             or page may load slowly.",
                            bounce, p.sessions.unwrap_or(0.0) as u64
                        ),
                        metric_label: "Bounce Rate".into(),
                        metric_value: format!("{:.1}%", bounce),
                        action: "Review content relevance to search queries, improve page speed, and add clear calls-to-action.".into(),
                        estimated_impact: None,
                    });
                }
            }

            // Rule 6: Stale content
            if let Some(age) = p.content_age_days {
                if age > 90 && p.impressions > 50.0 {
                    let declining = p.clicks < p.impressions * 0.02;
                    let severity = if declining {
                        InsightSeverity::Warning
                    } else {
                        InsightSeverity::Info
                    };
                    insights.push(SeoInsight {
                        rule: "stale_content".into(),
                        severity,
                        title: format!("Stale content: {}", short_path(&p.page)),
                        description: format!(
                            "Last updated {} days ago with {} impressions. {}",
                            age, p.impressions as u64,
                            if declining { "Performance appears to be declining." } else { "Content refresh may help rankings." }
                        ),
                        metric_label: "Content Age".into(),
                        metric_value: format!("{} days", age),
                        action: "Update the content with fresh information, new data, and expanded sections.".into(),
                        estimated_impact: None,
                    });
                }
            }

            // Rule 7: Thin content
            if let Some(wc) = p.word_count {
                if wc < 300 && p.impressions > 20.0 {
                    insights.push(SeoInsight {
                        rule: "thin_content".into(),
                        severity: InsightSeverity::Warning,
                        title: format!("Thin content: {}", short_path(&p.page)),
                        description: format!(
                            "Only {} words. Search engines typically favor pages with 1,000+ words for informational queries.",
                            wc
                        ),
                        metric_label: "Word Count".into(),
                        metric_value: format!("{}", wc),
                        action: "Expand the content to 1,000+ words with relevant subtopics, FAQs, and detailed information.".into(),
                        estimated_impact: Some("Pages with 1,000+ words rank 2-3 positions higher on average.".into()),
                    });
                }
            }

            // Rule 8: Low CTR for page position
            if p.position <= 10.0 && p.impressions > 30.0 {
                let expected = expected_ctr_for_position(p.position);
                if p.ctr < expected * 0.4 {
                    insights.push(SeoInsight {
                        rule: "page_low_ctr".into(),
                        severity: InsightSeverity::Opportunity,
                        title: format!("Improve CTR for {}", short_path(&p.page)),
                        description: format!(
                            "Page ranks at position {:.1} but CTR is only {:.1}% (expected {:.1}%).",
                            p.position, p.ctr, expected
                        ),
                        metric_label: "CTR Gap".into(),
                        metric_value: format!("{:.1}% vs {:.1}%", p.ctr, expected),
                        action: "Rewrite the title and meta description to be more compelling and action-oriented.".into(),
                        estimated_impact: Some(format!(
                            "Closing the gap could add ~{:.0} clicks/period",
                            p.impressions * (expected - p.ctr) / 100.0
                        )),
                    });
                }
            }
        }

        insights.sort_by_key(|i| match i.severity {
            InsightSeverity::Critical => 0,
            InsightSeverity::Warning => 1,
            InsightSeverity::Opportunity => 2,
            InsightSeverity::Info => 3,
        });

        insights
    }
}

/// Shorten a URL path for display.
fn short_path(url: &str) -> String {
    if let Some(path) = url
        .strip_prefix("https://")
        .and_then(|s| s.split_once('/').map(|(_, p)| p))
    {
        if path.is_empty() { "/" } else { path }.to_string()
    } else if let Some(path) = url
        .strip_prefix("http://")
        .and_then(|s| s.split_once('/').map(|(_, p)| p))
    {
        if path.is_empty() { "/" } else { path }.to_string()
    } else {
        url.to_string()
    }
}
