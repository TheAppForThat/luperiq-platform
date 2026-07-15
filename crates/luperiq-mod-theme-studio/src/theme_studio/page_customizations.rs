// luperiq-cms/src/modules/theme_studio/page_customizations.rs
//! Page customization overrides for functional pages.
//!
//! Stores text/label overrides for pages that have fixed structure
//! but customizable text (booking, portal, cart, checkout, etc.).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const AGG_PAGE_CUSTOMIZATION: &str = "PageCustomization";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageCustomization {
    pub page: String,
    pub overrides: HashMap<String, String>,
}

/// Load customizations for a specific page from the journal.
pub fn load_customizations(
    journal: &luperiq_forge::ForgeJournal,
    page: &str,
) -> HashMap<String, String> {
    for event in journal.latest_by_aggregate_type(AGG_PAGE_CUSTOMIZATION) {
        if let Ok(custom) = serde_json::from_slice::<PageCustomization>(&event.payload) {
            if custom.page == page {
                return custom.overrides;
            }
        }
    }
    HashMap::new()
}

/// Get a customized text value, falling back to the default if not overridden.
pub fn get_text<'a>(
    overrides: &'a HashMap<String, String>,
    key: &str,
    default: &'a str,
) -> &'a str {
    overrides.get(key).map(|s| s.as_str()).unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_text_returns_override() {
        let mut overrides = HashMap::new();
        overrides.insert("heading".to_string(), "Custom Title".to_string());
        assert_eq!(get_text(&overrides, "heading", "Default"), "Custom Title");
    }

    #[test]
    fn get_text_returns_default() {
        let overrides = HashMap::new();
        assert_eq!(get_text(&overrides, "heading", "Default"), "Default");
    }
}
