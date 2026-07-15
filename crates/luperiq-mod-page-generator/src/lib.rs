//! Bulk SEO page generator crate — produces up to five page kinds
//! (ItemHub, CityHub, ItemxCity, CategoryHub, CategoryxCity) from an
//! industry item × city cross-product. Supports template mode (instant,
//! free) and AI mode (Ollama-backed, credit-gated). Industry-neutral via
//! the `IndustryPageGenProvider` trait; SEO meta and batch records are
//! persisted to the WAL.
pub mod page_generator;
pub use page_generator::PageGeneratorModule;
