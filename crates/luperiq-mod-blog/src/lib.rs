//! Public blog engine for LuperIQ CMS.
//!
//! Renders `/blog` (published-post listing) and `/blog/{slug}` (single post view).
//! Integrates Theme Studio tokens, Google Analytics/GTM tags, admin toolbar with
//! SEO score display, and a canonical URL per page. Toggled as a `CmsModule`.
pub mod blog;
pub use blog::BlogModule;
