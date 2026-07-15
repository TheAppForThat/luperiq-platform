//! Industry provider trait for the generic page generator.
//!
//! Each industry module can implement `IndustryPageGenProvider` to supply
//! items (pest types, service types, menu categories, etc.) that the page
//! generator uses to create cross-product SEO pages.

use serde::{Deserialize, Serialize};

/// An item from any industry that can be used for page generation.
/// For pest control: a pest type. For HVAC: an equipment/service type.
/// For restaurant: a menu category. For plumbing: a service type, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryItem {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub category: String,
    pub description: String,
    pub active: bool,
    /// Pre-formatted fact sheet text combining all item data (description,
    /// signs/symptoms, treatment notes, severity, peak months, etc.).
    /// Fed into AI prompts so generated pages use real data, not guesses.
    #[serde(default)]
    pub fact_sheet: String,
}

/// Configuration that describes how an industry uses the page generator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndustryPageGenConfig {
    /// Industry name (e.g. "Pest Control", "HVAC", "Bakery")
    pub industry_name: String,
    /// Singular noun for items (e.g. "pest", "service", "product")
    pub item_singular: String,
    /// Plural noun (e.g. "pests", "services", "products")
    pub item_plural: String,
    /// Service verb used in templates (e.g. "control", "repair", "service")
    pub service_verb: String,
    /// Slug prefix for city hub pages (e.g. "pest-control", "hvac-service")
    pub city_hub_prefix: String,
}

/// Trait for industry modules to provide items to the page generator.
pub trait IndustryPageGenProvider: Send + Sync {
    /// The industry slug (e.g. "pest-control", "hvac")
    fn industry_slug(&self) -> &str;
    /// Configuration for page generation
    fn page_gen_config(&self) -> IndustryPageGenConfig;
    /// Load items from the journal
    fn load_items(&self, journal: &luperiq_forge::ForgeJournal) -> Vec<IndustryItem>;
}

/// Registry of industry page generation providers.
pub struct PageGenProviderRegistry {
    providers: Vec<Box<dyn IndustryPageGenProvider>>,
}

impl PageGenProviderRegistry {
    pub fn new() -> Self {
        Self { providers: vec![] }
    }

    pub fn register(&mut self, provider: Box<dyn IndustryPageGenProvider>) {
        self.providers.push(provider);
    }

    pub fn get(&self, slug: &str) -> Option<&dyn IndustryPageGenProvider> {
        self.providers
            .iter()
            .find(|p| p.industry_slug() == slug)
            .map(|p| p.as_ref())
    }

    pub fn list(&self) -> Vec<&dyn IndustryPageGenProvider> {
        self.providers.iter().map(|p| p.as_ref()).collect()
    }

    /// Returns the first registered provider, or None if the registry is empty.
    pub fn first(&self) -> Option<&dyn IndustryPageGenProvider> {
        self.providers.first().map(|p| p.as_ref())
    }
}
