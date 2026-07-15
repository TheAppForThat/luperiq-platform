// Items here are used by the main server binary but appear dead when compiled
// by bin/ tools that include this file via #[path] with a narrower scope.
#![allow(dead_code)]
//! CompanyProfile aggregate — business identity fact sheet (singleton per site).
//!
//! Stores brand identity, company story, team bios, certifications, trust signals,
//! contact info, social links, voice/tone preferences, and review highlights.
//!
//! The singleton instance uses `"global"` as the aggregate_id, so each site has
//! exactly one company profile.

use luperiq_forge::ApexEvent;
use serde::{Deserialize, Serialize};

/// Aggregate type for the company profile in the ForgeJournal.
pub const AGG_COMPANY: &str = "CompProf:Profile";

/// Tombstone marker for soft-deleted aggregates.
const TOMBSTONE: &[u8] = b"__DELETED__";

/// The fixed aggregate_id for the singleton company profile.
pub const SINGLETON_ID: &str = "global";

// ── Primary aggregate ────────────────────────────────────────────────

/// The business identity fact sheet — one per site.
///
/// Contains everything needed to generate branded content, answer questions
/// about the business, and maintain a consistent voice across all channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyProfile {
    /// Always "global" — singleton per site.
    pub id: String,
    /// Business name as used in branding and content.
    pub name: String,
    /// Legal entity name (if different from brand name).
    pub legal_name: Option<String>,
    /// Links to an IndustryProfile slug, e.g. "pest-control".
    pub industry_slug: String,
    /// Short tagline / slogan.
    pub tagline: String,
    /// Company origin/mission story, 2-3 paragraphs.
    pub story: String,
    /// Voice tone: "professional", "friendly", "casual", "authoritative", "playful".
    pub tone: String,
    /// Style notes, e.g. "We say 'folks' not 'customers'".
    pub voice_notes: Vec<String>,
    /// Brand color palette.
    pub brand_colors: BrandColors,
    /// URL to the logo image.
    pub logo_url: Option<String>,
    /// URL to the favicon.
    pub favicon_url: Option<String>,
    /// Owner / founder name.
    pub owner_name: Option<String>,
    /// Owner title, e.g. "CEO", "Founder".
    pub owner_title: Option<String>,
    /// Team members with bios.
    pub team_bios: Vec<TeamMember>,
    /// Professional certifications held.
    pub certifications: Vec<String>,
    /// Relevant license numbers.
    pub license_numbers: Vec<String>,
    /// How many years the business has been operating.
    pub years_in_business: Option<u32>,
    /// Service philosophy statement.
    pub service_philosophy: String,
    /// What sets this business apart.
    pub unique_selling_points: Vec<String>,
    /// Highlighted customer reviews.
    pub review_highlights: Vec<ReviewHighlight>,
    /// Social media and directory links.
    pub social_links: SocialLinks,
    /// Primary phone number.
    pub phone: String,
    /// Primary email address.
    pub email: String,
    /// Street address.
    pub address: String,
    /// City.
    pub city: String,
    /// State (abbreviation or full name).
    pub state: String,
    /// ZIP / postal code.
    pub zip: String,
    /// Human-readable service area description.
    pub service_area_description: String,
    /// Links to LocationProfile slugs.
    pub location_slugs: Vec<String>,
    /// Unix timestamp (seconds) when the profile was first created.
    pub created_at: u64,
    /// Unix timestamp (seconds) when the profile was last updated.
    pub updated_at: u64,
}

// ── Sub-types ────────────────────────────────────────────────────────

/// Brand color palette (hex values).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrandColors {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

/// A team member with bio and optional photo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub name: String,
    pub title: String,
    pub bio: String,
    pub photo_url: Option<String>,
}

/// A highlighted customer review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewHighlight {
    /// Source platform: "google", "yelp", "facebook", etc.
    pub source: String,
    /// Star rating (e.g. 5.0).
    pub rating: f64,
    /// Review text excerpt.
    pub text: String,
    /// Reviewer name/initials.
    pub author: String,
}

/// Social media and directory profile links.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocialLinks {
    pub google_business: Option<String>,
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub twitter: Option<String>,
    pub youtube: Option<String>,
    pub linkedin: Option<String>,
    pub yelp: Option<String>,
    pub nextdoor: Option<String>,
}

// ── Default for CompanyProfile ───────────────────────────────────────

impl Default for CompanyProfile {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: SINGLETON_ID.to_string(),
            name: String::new(),
            legal_name: None,
            industry_slug: String::new(),
            tagline: String::new(),
            story: String::new(),
            tone: "professional".to_string(),
            voice_notes: Vec::new(),
            brand_colors: BrandColors::default(),
            logo_url: None,
            favicon_url: None,
            owner_name: None,
            owner_title: None,
            team_bios: Vec::new(),
            certifications: Vec::new(),
            license_numbers: Vec::new(),
            years_in_business: None,
            service_philosophy: String::new(),
            unique_selling_points: Vec::new(),
            review_highlights: Vec::new(),
            social_links: SocialLinks::default(),
            phone: String::new(),
            email: String::new(),
            address: String::new(),
            city: String::new(),
            state: String::new(),
            zip: String::new(),
            service_area_description: String::new(),
            location_slugs: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ── Journal helpers ──────────────────────────────────────────────────

/// Load the singleton company profile (or None if it hasn't been created yet).
pub fn load_company_profile(j: &luperiq_forge::ForgeJournal) -> Option<CompanyProfile> {
    j.get_latest(AGG_COMPANY, SINGLETON_ID)
        .filter(|e| e.payload != TOMBSTONE)
        .and_then(|e| serde_json::from_slice::<CompanyProfile>(&e.payload).ok())
}

/// Persist the company profile to the journal.
///
/// Always uses `"global"` as the aggregate_id.
pub fn persist_company_profile(
    j: &mut luperiq_forge::ForgeJournal,
    profile: &CompanyProfile,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(profile).map_err(|e| e.to_string())?;
    let event = ApexEvent::new(AGG_COMPANY, SINGLETON_ID, bytes);
    j.append(event).map_err(|e| e.to_string())?;
    Ok(())
}
