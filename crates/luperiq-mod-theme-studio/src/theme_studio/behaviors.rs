// luperiq-cms/src/modules/theme_studio/behaviors.rs
//! Block behavior system — defines valid interactive behaviors and serves behaviors.js.
//!
//! Behaviors are interactive patterns (accordion, tabs, carousel, etc.) that blocks
//! can opt into. The JS implementation is compiled into the binary via include_str!()
//! and served through a dedicated route. No custom JS is allowed in block definitions.

use serde::{Deserialize, Serialize};

/// The JS source for all behavior implementations.
/// Compiled into the binary at build time.
const BEHAVIORS_JS: &str = include_str!("behaviors.js");

/// All valid interactive behaviors a block can declare.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlockBehavior {
    #[default]
    None,
    Accordion,
    Tabs,
    Carousel,
    Lightbox,
    Counter,
    Toggle,
    Dropdown,
    CopyCode,
    BeforeAfter,
    FormValidation,
    LazyLoad,
    SmoothScroll,
}

impl BlockBehavior {
    /// Returns the behavior name as a string for data-behavior attribute.
    /// Returns None for BlockBehavior::None (no attribute needed).
    pub fn as_data_attr(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Accordion => Some("accordion"),
            Self::Tabs => Some("tabs"),
            Self::Carousel => Some("carousel"),
            Self::Lightbox => Some("lightbox"),
            Self::Counter => Some("counter"),
            Self::Toggle => Some("toggle"),
            Self::Dropdown => Some("dropdown"),
            Self::CopyCode => Some("copy_code"),
            Self::BeforeAfter => Some("before_after"),
            Self::FormValidation => Some("form_validation"),
            Self::LazyLoad => Some("lazy_load"),
            Self::SmoothScroll => Some("smooth_scroll"),
        }
    }

    /// Check if a string is a valid behavior name.
    pub fn is_valid(name: &str) -> bool {
        matches!(
            name,
            "none"
                | "accordion"
                | "tabs"
                | "carousel"
                | "lightbox"
                | "counter"
                | "toggle"
                | "dropdown"
                | "copy_code"
                | "before_after"
                | "form_validation"
                | "lazy_load"
                | "smooth_scroll"
        )
    }
}

/// Axum handler: serves behaviors.js with proper content type and caching.
pub async fn serve_behaviors_js() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 2],
    &'static str,
) {
    (
        axum::http::StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (
                axum::http::header::CACHE_CONTROL,
                "public, max-age=86400, immutable",
            ),
        ],
        BEHAVIORS_JS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behaviors_js_compiles_and_is_nonempty() {
        assert!(!BEHAVIORS_JS.is_empty());
        assert!(BEHAVIORS_JS.contains("behaviors"));
        assert!(BEHAVIORS_JS.contains("accordion"));
        assert!(BEHAVIORS_JS.contains("DOMContentLoaded"));
    }

    #[test]
    fn block_behavior_serializes_snake_case() {
        let b = BlockBehavior::CopyCode;
        let json = serde_json::to_string(&b).unwrap();
        assert_eq!(json, "\"copy_code\"");
    }

    #[test]
    fn block_behavior_deserializes_snake_case() {
        let b: BlockBehavior = serde_json::from_str("\"before_after\"").unwrap();
        assert_eq!(b, BlockBehavior::BeforeAfter);
    }

    #[test]
    fn block_behavior_default_is_none() {
        let b = BlockBehavior::default();
        assert_eq!(b, BlockBehavior::None);
        assert!(b.as_data_attr().is_none());
    }

    #[test]
    fn block_behavior_data_attr() {
        assert_eq!(BlockBehavior::Accordion.as_data_attr(), Some("accordion"));
        assert_eq!(BlockBehavior::Tabs.as_data_attr(), Some("tabs"));
        assert_eq!(BlockBehavior::None.as_data_attr(), None);
    }

    #[test]
    fn is_valid_accepts_known_behaviors() {
        assert!(BlockBehavior::is_valid("accordion"));
        assert!(BlockBehavior::is_valid("none"));
        assert!(BlockBehavior::is_valid("copy_code"));
    }

    #[test]
    fn is_valid_rejects_unknown() {
        assert!(!BlockBehavior::is_valid("evil_behavior"));
        assert!(!BlockBehavior::is_valid(""));
        assert!(!BlockBehavior::is_valid("script_injection"));
    }
}
