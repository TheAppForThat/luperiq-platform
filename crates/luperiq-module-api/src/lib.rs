//! Shared module types for the LuperIQ CMS platform.
//!
//! This crate provides the core types used across all module crates:
//! - `AppContext` — shared state for route assembly
//! - `SharedJournal` — thread-safe journal handle with preview routing
//! - `CmsModule` — trait every module implements
//! - `ModuleRegistry` — collects modules, aggregates routes/JS/CSS
//! - `AdminView`, `ModuleMeta`, `ModulePricing`, `KNOWN_MODULES`
//! - `AiFeatureConfig`, `AiFeatureRegistry` — AI feature registration
//! - `IndustryHomepageProvider`, `CustomerPortalProvider` — provider traits
//! - `IndustryDefinition` — shared industry type (breaks circular deps)

pub mod context;
pub mod industry;
pub mod module_trait;
pub mod providers;
pub mod registry;

// Re-export context types at crate root for convenience.
pub use context::{
    new_ai_feature_registry, AiFeatureConfig, AiFeatureRegistry, AppContext, NexusNetworkConfig,
    OptService, PreviewHubSettings, PreviewJournalRouter, Service, SharedJournal,
    SharedJournalGuard,
};

// Re-export module trait types at crate root.
pub use module_trait::{
    find_module_meta, AdminView, CmsModule, ModuleMeta, ModulePricing, KNOWN_MODULES,
};

// Re-export registry at crate root.
pub use registry::{DepError, ModuleRegistry};

// Re-export provider traits at crate root.
pub use providers::{CustomerPortalProvider, IndustryHomepageProvider};

// Re-export shared industry type at crate root.
pub use industry::IndustryDefinition;

// Session extraction utility for extracted module crates.
pub mod session;
pub use session::extract_session;

// Re-export commonly used luperiq-forge types so downstream module crates
// can depend on luperiq-module-api alone instead of both crates.
pub use luperiq_forge::{
    ApexEvent, DurabilityMode, ForgeAuthManager, ForgeContentManager, ForgeError, ForgeJournal,
    ForgeMediaManager, ForgeMenuManager, ForgeSlugManager, ForgeUserProfile,
};
