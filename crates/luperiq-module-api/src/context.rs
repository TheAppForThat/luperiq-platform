//! AppContext and SharedJournal — core types shared across all modules.
//!
//! `AppContext` is the shared state passed to every CMS module during route
//! assembly. Universal fields (journal, jwt_secret, tera, etc.) are concrete
//! types. Module-specific services (AI client, Stripe gateway, embed client,
//! job registry, etc.) are stored as type-erased `Arc<dyn Any + Send + Sync>`
//! so that the API crate does not depend on module-specific code.

use luperiq_forge::ForgeJournal;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard};

// ── SharedJournal ──────────────────────────────────────────────────────────

/// Trait for routing journal access based on the current HTTP request host.
///
/// The preview-hub module implements this to map subdomain requests
/// (e.g., `my-site.preview.luperiq.com`) to per-preview ForgeJournal instances.
/// Modules never interact with this trait directly — they call
/// `SharedJournal::lock()` which handles the routing transparently.
pub trait PreviewJournalRouter: Send + Sync {
    /// Return the journal for the current request host, or `None` to fall
    /// through to the default journal.
    fn journal_for_current_host(
        &self,
    ) -> Pin<Box<dyn Future<Output = Option<Arc<Mutex<ForgeJournal>>>> + Send + '_>>;
}

/// Thread-safe shared journal handle with optional preview-hub routing.
///
/// Modules call `.lock().await` to obtain a `SharedJournalGuard` that
/// derefs to `ForgeJournal`. When a `PreviewJournalRouter` is configured
/// and matches the current request host, the lock targets the per-preview
/// journal instead of the default.
#[derive(Clone)]
pub struct SharedJournal {
    default: Arc<Mutex<ForgeJournal>>,
    preview_router: Option<Arc<dyn PreviewJournalRouter>>,
}

/// RAII guard returned by `SharedJournal::lock()`.
pub struct SharedJournalGuard(OwnedMutexGuard<ForgeJournal>);

impl Deref for SharedJournalGuard {
    type Target = ForgeJournal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedJournalGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl SharedJournal {
    /// Create a new SharedJournal with an optional preview router.
    pub fn new(
        default: Arc<Mutex<ForgeJournal>>,
        preview_router: Option<Arc<dyn PreviewJournalRouter>>,
    ) -> Self {
        Self {
            default,
            preview_router,
        }
    }

    /// Lock the appropriate journal for the current request context.
    pub async fn lock(&self) -> SharedJournalGuard {
        if let Some(router) = &self.preview_router {
            if let Some(journal) = router.journal_for_current_host().await {
                return SharedJournalGuard(journal.lock_owned().await);
            }
        }

        SharedJournalGuard(self.default.clone().lock_owned().await)
    }
}

// ── AI Feature Registry ────────────────────────────────────────────────────

/// Configuration for a registered AI feature.
///
/// Modules register features (e.g., "theme", "seo", "content") with their
/// system prompts and result parsers. The AI router calls the feature's
/// parser to validate and transform the response.
pub struct AiFeatureConfig {
    pub system_prompt: String,
    pub max_input_len: usize,
    pub credit_cost: u32,
    pub escalation_credit_cost: u32,
    pub result_parser: fn(&str) -> Result<serde_json::Value, String>,
}

/// Thread-safe registry mapping feature names to their AI configurations.
pub type AiFeatureRegistry = Arc<Mutex<HashMap<String, AiFeatureConfig>>>;

/// Create a new empty AI feature registry.
pub fn new_ai_feature_registry() -> AiFeatureRegistry {
    Arc::new(Mutex::new(HashMap::new()))
}

// ── Config types ───────────────────────────────────────────────────────────

/// Nexus network configuration from cms.toml.
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct NexusNetworkConfig {
    /// "central" or "client". Omit for standalone.
    pub role: Option<String>,
    /// Central only: directory for module package ZIPs.
    pub packages_dir: Option<String>,
    /// Client only: URL of the central CMS (e.g., "https://luperiq.com").
    pub central_url: Option<String>,
    /// Client only: license key for authenticating with central.
    pub license_key: Option<String>,
    /// Client only: shared secret used to verify central Cortex tool callbacks.
    pub cortex_shared_secret: Option<String>,
    /// Client only: "hosted", "self_hosted", "dev", or "unreported".
    pub hosting_mode: Option<String>,
    /// Client only: stable deployment id reported to Central.
    pub install_id: Option<String>,
    /// Client only: stable site id preserved when moving hosted/self-hosted.
    pub canonical_site_id: Option<String>,
    /// Client only: "managed", "manual", "offline", or custom update channel.
    pub update_channel: Option<String>,
}

/// Resolved preview-hub settings (computed from toml config).
#[derive(Debug, Clone)]
pub struct PreviewHubSettings {
    pub enabled: bool,
    pub preview_dir: std::path::PathBuf,
    pub base_url: String,
}

// ── Type-erased service helpers ────────────────────────────────────────────

/// A type-erased service handle. Modules downcast to the concrete type.
pub type Service = Arc<dyn Any + Send + Sync>;
/// An optional type-erased service.
pub type OptService = Option<Service>;

// ── AppContext ──────────────────────────────────────────────────────────────

/// Shared context passed to all modules during route assembly.
///
/// Universal fields are concrete types. Module-specific services are stored
/// as `OptService` (`Option<Arc<dyn Any + Send + Sync>>`) and accessed via
/// typed helper methods that downcast to the expected concrete type.
#[derive(Clone)]
pub struct AppContext {
    // ── Universal fields (always available) ─────────────────────────────
    pub journal: SharedJournal,
    pub jwt_secret: String,
    pub base_url: String,
    pub tera: Option<Arc<tera::Tera>>,
    pub theme_css: Arc<String>,
    pub site_name: Arc<String>,
    /// Optional theme overlay slug. When set, base.html loads
    /// /static/css/themes/{slug}.css and applies body.theme-{slug}.
    pub theme_overlay: Arc<Option<String>>,

    /// Cached group terminology config (loaded from WAL or defaults)
    pub terminology: std::sync::Arc<luperiq_forge::GroupTerminology>,

    // ── AI feature registry (used by ~35 modules, kept concrete) ────────
    pub ai_features: AiFeatureRegistry,

    // ── Config types (simple data, kept concrete) ───────────────────────
    pub nexus_config: Option<NexusNetworkConfig>,
    pub preview_hub: Option<PreviewHubSettings>,

    // ── Type-erased services (modules downcast as needed) ───────────────
    /// AI quick client (concrete type: `Arc<AiClient>` from luperiq-cms)
    pub ai_quick_client: OptService,
    /// AI content client
    pub ai_client: OptService,
    /// AI escalation client
    pub escalation_client: OptService,
    /// Embedding client (feature-gated behind "cortex")
    pub embed_client: OptService,
    /// Vector store (`Arc<Mutex<ForgeVec>>`)
    pub vec_store: OptService,
    /// Job registry (feature-gated behind "jobs")
    pub job_registry: OptService,
    /// Availability registry
    pub availability_registry: OptService,
    /// Duration registry (feature-gated behind "field_ops")
    pub duration_registry: OptService,
    /// Stripe payment gateway
    pub stripe: OptService,
    /// Customer portal provider trait object
    pub portal_provider: OptService,
}

impl AppContext {
    /// Downcast a type-erased service to a concrete type.
    ///
    /// Returns `Some(Arc<T>)` if the service is present and is the expected
    /// type, or `None` if the service slot is empty or the type doesn't match.
    ///
    /// # Example
    /// ```ignore
    /// let ai: Option<Arc<AiClient>> = AppContext::service(&ctx.ai_client);
    /// ```
    pub fn service<T: Send + Sync + 'static>(svc: &OptService) -> Option<Arc<T>> {
        svc.as_ref()?.clone().downcast::<T>().ok()
    }
}
