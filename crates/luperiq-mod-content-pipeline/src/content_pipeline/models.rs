//! Model selection — local vs cloud AI model routing.
//!
//! "quick_draft" quality uses the local model (vLLM / Ollama) for fast iteration.
//! "premium" quality uses a cloud model (Anthropic Claude) for production-grade content.

use super::ai_client::{AiClient, AiContentConfig};
use luperiq_module_api::AppContext;
use std::sync::Arc;

/// Get the local AI client from the app context.
///
/// Returns `None` if no AI client is configured.
pub fn local_client(app_ctx: &AppContext) -> Option<Arc<AiClient>> {
    AppContext::service::<AiClient>(&app_ctx.ai_client)
}

/// Create a cloud AI client configured for premium content generation.
///
/// Uses Anthropic Claude for high-quality, production-grade content.
/// The API key is read from `ANTHROPIC_API_KEY` or `LUPERIQ_AI_API_KEY` env vars.
pub fn cloud_client() -> Result<AiClient, String> {
    let config = AiContentConfig {
        provider: "anthropic".into(),
        model: Some("claude-sonnet-4-20250514".into()),
        ..Default::default()
    };
    AiClient::new(&config).map_err(|e| format!("Failed to create cloud AI client: {}", e))
}

/// Select the appropriate AI client based on quality level.
pub fn select_client(app_ctx: &AppContext, quality: &str) -> Result<ClientRef, String> {
    select_client_from_opt(&local_client(app_ctx), quality)
}

/// Select AI client from an optional local client reference.
/// Used when we don't have a full AppContext (e.g., from PipelineState).
pub fn select_client_from_opt(
    local: &Option<Arc<AiClient>>,
    quality: &str,
) -> Result<ClientRef, String> {
    match quality {
        "premium" => match cloud_client() {
            Ok(client) => Ok(ClientRef::Owned(client)),
            Err(e) => match local {
                Some(client) => Ok(ClientRef::Shared(Arc::clone(client))),
                None => Err(format!("No AI client available for premium quality: {}", e)),
            },
        },
        _ => match local {
            Some(client) => Ok(ClientRef::Shared(Arc::clone(client))),
            None => match cloud_client() {
                Ok(client) => Ok(ClientRef::Owned(client)),
                Err(e) => Err(format!("No AI client available: {}", e)),
            },
        },
    }
}

/// A reference to an AI client, either shared (from AppContext) or owned (newly created).
pub enum ClientRef {
    /// Shared reference from AppContext (local client).
    Shared(Arc<AiClient>),
    /// Owned instance (cloud client, created on demand).
    Owned(AiClient),
}

impl ClientRef {
    /// Get a reference to the underlying AiClient.
    pub fn as_ref(&self) -> &AiClient {
        match self {
            ClientRef::Shared(arc) => arc.as_ref(),
            ClientRef::Owned(client) => client,
        }
    }
}
