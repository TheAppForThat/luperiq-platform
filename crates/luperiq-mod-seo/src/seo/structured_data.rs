//! Structured data (JSON-LD) generators for schema.org types.
//! Shared SEO layer for quiz2, chemicals, directory, guide, pest-news.

use serde_json::json;

// ── PageType enum ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageType {
    Quiz,
    Chemical,
    ChemicalList,
    DirectoryCity,
    DirectoryCompany,
    NewsArticle,
    FieldGuide,
    WebPage,
}

impl PageType {
    pub fn og_type(&self) -> &'static str {
        match self {
            Self::Chemical | Self::NewsArticle | Self::FieldGuide => "article",
            _ => "website",
        }
    }

    pub fn changefreq(&self) -> &'static str {
        match self {
            Self::Quiz | Self::NewsArticle => "daily",
            Self::DirectoryCompany => "monthly",
            _ => "weekly",
        }
    }
}

// ── Helpers ──


fn script_block(json: &str) -> String {
    format!("<script type=\"application/ld+json\">{}</script>", json)
}

pub fn json_ld_quiz(name: &str, description: &str, url: &str, question_count: Option<usize>) -> String {
    let mut obj = json!({
        "@context": "https://schema.org",
        "@type": "Quiz",
        "name": name,
        "description": description,
        "url": url,
    });
    if let Some(n) = question_count {
        if n > 0 {
            obj["numberOfItems"] = json!(n);
        }
    }
    script_block(&obj.to_string())
}

pub fn json_ld_chemical(name: &str, description: &str, url: &str, manufacturer: &str, epa_number: Option<&str>) -> String {
    let mut creator = json!({
        "@type": "Organization",
        "name": manufacturer,
    });
    if let Some(n) = epa_number {
        if !n.is_empty() {
            creator["identifier"] = json!(n);
        }
    }
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "HowTo",
        "name": name,
        "description": description,
        "url": url,
        "creator": creator,
    });
    script_block(&obj.to_string())
}

pub fn json_ld_chemical_list(name: &str, description: &str, url: &str, item_count: usize) -> String {
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "ItemList",
        "name": name,
        "description": description,
        "url": url,
        "numberOfItems": item_count,
    });
    script_block(&obj.to_string())
}

pub fn json_ld_directory_city(city: &str, state: &str, state_abbr: &str, url: &str, company_count: usize) -> String {
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "City",
        "name": city,
        "containedInPlace": {
            "@type": "State",
            "name": state,
            "identifier": state_abbr,
        },
        "url": url,
        "numberOfItems": company_count,
    });
    script_block(&obj.to_string())
}

pub fn json_ld_directory_company(name: &str, description: &str, url: &str, city: &str, state: &str, phone: Option<&str>, email: Option<&str>, website: Option<&str>) -> String {
    let mut contact = json!({
        "@type": "ContactPoint",
        "contactType": "customer service",
    });
    if let Some(p) = phone {
        if !p.is_empty() {
            contact["telephone"] = json!(p);
        }
    }
    if let Some(e) = email {
        if !e.is_empty() {
            contact["email"] = json!(e);
        }
    }
    let mut obj = json!({
        "@context": "https://schema.org",
        "@type": "Organization",
        "name": name,
        "description": description,
        "url": url,
        "address": {
            "@type": "PostalAddress",
            "addressLocality": city,
            "addressRegion": state,
        },
        "contactPoint": contact,
    });
    if let Some(w) = website {
        if !w.is_empty() {
            obj["sameAs"] = json!(w);
        }
    }
    script_block(&obj.to_string())
}

pub fn json_ld_news_article(headline: &str, description: &str, url: &str, published: &str, scope: &str, source_count: usize) -> String {
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "NewsArticle",
        "headline": headline,
        "description": description,
        "url": url,
        "datePublished": published,
        "about": {
            "@type": "Thing",
            "name": scope,
            "numberOfSources": source_count,
        },
        "publisher": {
            "@type": "Organization",
            "name": "pestcontroller.org",
        },
        "author": {
            "@type": "Organization",
            "name": "pestcontroller.org AI Digest",
        },
    });
    script_block(&obj.to_string())
}

pub fn json_ld_field_guide(name: &str, description: &str, url: &str, questions: &[(&str, &str)]) -> String {
    let mut obj = json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "name": name,
        "description": description,
        "url": url,
    });
    if !questions.is_empty() {
        let items: Vec<_> = questions.iter().map(|(q, a)| {
            json!({
                "@type": "Question",
                "name": q,
                "acceptedAnswer": {
                    "@type": "AcceptedAnswer",
                    "text": a,
                },
            })
        }).collect();
        obj["mainEntity"] = json!(items);
    }
    script_block(&obj.to_string())
}

pub fn json_ld_web_page(name: &str, description: &str, url: &str) -> String {
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "WebPage",
        "name": name,
        "description": description,
        "url": url,
    });
    script_block(&obj.to_string())
}

pub fn json_ld_breadcrumbs(breadcrumbs: &[(&str, &str)]) -> String {
    if breadcrumbs.is_empty() {
        return String::new();
    }
    let items: Vec<_> = breadcrumbs.iter().enumerate().map(|(i, (name, url))| {
        if url.is_empty() {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "name": name,
            })
        } else {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "name": name,
                "item": url,
            })
        }
    }).collect();
    let obj = json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": items,
    });
    script_block(&obj.to_string())
}

pub fn render_seo_head(page_type: PageType, title: &str, description: &str, canonical_url: &str, json_ld: &str) -> String {
    let og = page_type.og_type();
    format!(
        "<title>{}</title>\n\
        <meta name=\"description\" content=\"{}\">\n\
        <link rel=\"canonical\" href=\"{}\">\n\
        <meta name=\"robots\" content=\"index,follow\">\n\
        <meta property=\"og:type\" content=\"{}\">\n\
        <meta property=\"og:title\" content=\"{}\">\n\
        <meta property=\"og:description\" content=\"{}\">\n\
        <meta property=\"og:url\" content=\"{}\">\n\
        {}",
        title, description, canonical_url, og, title, description, canonical_url, json_ld
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiz_has_correct_type() {
        let ld = json_ld_quiz("Texas License Exam", "Practice questions", "https://example.com/quiz", Some(100));
        assert!(ld.contains("\"@type\":\"Quiz\""));
        assert!(ld.contains("\"numberOfItems\":100"));
    }

    #[test]
    fn quiz_without_count() {
        let ld = json_ld_quiz("Test", "Desc", "https://example.com", None);
        assert!(ld.contains("\"@type\":\"Quiz\""));
        assert!(!ld.contains("numberOfItems"));
    }

    #[test]
    fn chemical_has_howto() {
        let ld = json_ld_chemical("BendiCap", "Termiticide", "https://example.com", "BASF", Some("EPA-123"));
        assert!(ld.contains("\"@type\":\"HowTo\""));
    }

    #[test]
    fn chemical_list_has_item_list() {
        let ld = json_ld_chemical_list("All Chemicals", "Catalog", "https://example.com", 50);
        assert!(ld.contains("\"@type\":\"ItemList\""));
    }

    #[test]
    fn directory_city_has_city() {
        let ld = json_ld_directory_city("Azle", "Texas", "TX", "https://example.com/dir/TX/azle", 10);
        assert!(ld.contains("\"@type\":\"City\""));
    }

    #[test]
    fn news_article_has_news_article() {
        let ld = json_ld_news_article("Headline", "Desc", "https://example.com/news", "2026-06-25", "national", 5);
        assert!(ld.contains("\"@type\":\"NewsArticle\""));
    }

    #[test]
    fn faq_has_faq_page() {
        let ld = json_ld_field_guide("Roach Guide", "How to", "https://example.com/guide/roach", &[("What kills roaches?", "BendiCap.")]);
        assert!(ld.contains("\"@type\":\"FAQPage\""));
        assert!(ld.contains("\"@type\":\"Question\""));
    }

    #[test]
    fn faq_without_questions() {
        let ld = json_ld_field_guide("Empty", "No Qs", "https://example.com", &[]);
        assert!(ld.contains("\"@type\":\"FAQPage\""));
        assert!(!ld.contains("mainEntity"));
    }

    #[test]
    fn seo_head_contains_all_parts() {
        let ld = json_ld_web_page("Test", "Desc", "https://example.com");
        let head = render_seo_head(PageType::WebPage, "Test", "Desc", "https://example.com", &ld);
        assert!(head.contains("<title>Test</title>"));
        assert!(head.contains("og:type"));
        assert!(head.contains("application/ld+json"));
    }

    #[test]
    fn page_type_og_types() {
        assert_eq!(PageType::NewsArticle.og_type(), "article");
        assert_eq!(PageType::Quiz.og_type(), "website");
    }

    #[test]
    fn page_type_changefreq() {
        assert_eq!(PageType::NewsArticle.changefreq(), "daily");
        assert_eq!(PageType::Quiz.changefreq(), "daily");
        assert_eq!(PageType::Chemical.changefreq(), "weekly");
    }
}

