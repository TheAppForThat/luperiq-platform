use serde::{Deserialize, Serialize};

/// A CMS content item (page, post, or custom type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeContent {
    pub content_id: String,
    pub content_type: String,
    pub title: String,
    pub slug: String,
    pub body_json: String,
    pub excerpt: Option<String>,
    pub author_id: String,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub published_at: Option<u64>,
}

/// Key-value metadata attached to a content item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeContentMeta {
    pub meta_id: String,
    pub content_id: String,
    pub meta_key: String,
    pub meta_value: String,
}

/// An immutable revision snapshot of content at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeContentRevision {
    pub revision_id: String,
    pub content_id: String,
    pub title: String,
    pub body_json: String,
    pub revision_number: u32,
    pub created_at: u64,
    pub author_id: String,
}
