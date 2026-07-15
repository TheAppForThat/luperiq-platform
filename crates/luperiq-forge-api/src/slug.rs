use serde::{Deserialize, Serialize};

/// A URL slug mapped to a content item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeSlug {
    pub slug_id: String,
    pub content_id: String,
    pub slug: String,
    pub content_type: String,
    pub is_current: bool,
    pub created_at: u64,
}

/// A URL redirect rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeRedirect {
    pub redirect_id: String,
    pub source_pattern: String,
    pub target_url: String,
    pub redirect_type: u16,
    pub pattern_type: String,
    pub is_active: bool,
    pub hit_count: u64,
    pub created_at: u64,
    pub updated_at: u64,
}
