// luperiq-cms/src/modules/theme_studio/smart_block_defaults.rs
//! Industry-aware default block generation for new pages.
//!
//! Given an industry slug and page type, produces a sensible set of smart
//! blocks so newly created pages are never blank.

use serde_json::{json, Value};

/// Generate a default block list for a new page.
///
/// Returns a Vec of JSON objects, each with at least a `"type"` key matching
/// a smart block type string.
pub fn generate_default_blocks(industry: &str, page_type: &str) -> Vec<Value> {
    match page_type {
        "home" => generate_home_blocks(industry),
        "services" => vec![block("service-grid"), block("cta-bar")],
        "booking" => vec![block("booking-form")],
        "account" => vec![block("account-tabs")],
        _ => Vec::new(),
    }
}

fn generate_home_blocks(industry: &str) -> Vec<Value> {
    let mut blocks = vec![block("company-hero")];

    match industry {
        "pest-control" | "hvac" | "electrician" | "plumbing" | "landscaping" => {
            blocks.push(block("service-grid"));
            blocks.push(block("trust-badges"));
        }
        "restaurant" => {
            blocks.push(block("featured-menu-items"));
            blocks.push(block("menu-categories"));
        }
        "bakery" | "coffee" => {
            blocks.push(block("product-showcase"));
        }
        "salon" => {
            blocks.push(block("salon-services-preview"));
            blocks.push(block("salon-team-grid"));
        }
        "brooke-grace" => {
            blocks.push(block("bg-featured-products"));
            blocks.push(block("bg-category-nav"));
            blocks.push(block("bg-social-feed"));
            blocks.push(block("bg-blog-preview"));
            blocks.push(block("bg-rewards-banner"));
        }
        _ => {
            // Default for unknown/unset industries: include service grid + trust badges
            blocks.push(block("service-grid"));
            blocks.push(block("trust-badges"));
        }
    }

    blocks.push(block("about-section"));

    match industry {
        "restaurant" | "bakery" | "coffee" | "salon" => {
            blocks.push(block("hours-location"));
        }
        _ => {}
    }

    blocks.push(block("cta-bar"));
    blocks.push(block("contact-info"));

    blocks
}

fn block(block_type: &str) -> Value {
    json!({ "type": block_type })
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn block_types(blocks: &[Value]) -> Vec<&str> {
        blocks.iter().map(|b| b["type"].as_str().unwrap()).collect()
    }

    #[test]
    fn pest_control_home() {
        let blocks = generate_default_blocks("pest-control", "home");
        let types = block_types(&blocks);
        assert!(types.contains(&"company-hero"));
        assert!(types.contains(&"service-grid"));
        assert!(types.contains(&"trust-badges"));
        assert!(types.contains(&"cta-bar"));
        assert!(types.contains(&"contact-info"));
        assert!(!types.contains(&"hours-location"));
    }

    #[test]
    fn restaurant_home() {
        let blocks = generate_default_blocks("restaurant", "home");
        let types = block_types(&blocks);
        assert!(types.contains(&"featured-menu-items"));
        assert!(types.contains(&"menu-categories"));
        assert!(types.contains(&"hours-location"));
        assert!(!types.contains(&"service-grid"));
    }

    #[test]
    fn brooke_grace_home() {
        let blocks = generate_default_blocks("brooke-grace", "home");
        let types = block_types(&blocks);
        assert!(types.contains(&"bg-featured-products"));
        assert!(types.contains(&"bg-category-nav"));
        assert!(types.contains(&"bg-social-feed"));
        assert!(types.contains(&"bg-blog-preview"));
        assert!(types.contains(&"bg-rewards-banner"));
        assert!(!types.contains(&"hours-location"));
    }

    #[test]
    fn salon_home() {
        let blocks = generate_default_blocks("salon", "home");
        let types = block_types(&blocks);
        assert!(types.contains(&"salon-services-preview"));
        assert!(types.contains(&"salon-team-grid"));
        assert!(types.contains(&"hours-location"));
    }

    #[test]
    fn unknown_page_type_returns_empty() {
        let blocks = generate_default_blocks("pest-control", "foobar");
        assert!(blocks.is_empty());
    }

    #[test]
    fn every_home_starts_with_hero() {
        for industry in &[
            "pest-control",
            "restaurant",
            "salon",
            "brooke-grace",
            "bakery",
            "hvac",
            "unknown-ind",
        ] {
            let blocks = generate_default_blocks(industry, "home");
            assert_eq!(
                blocks[0]["type"].as_str().unwrap(),
                "company-hero",
                "Home for '{}' should start with company-hero",
                industry
            );
        }
    }
}
