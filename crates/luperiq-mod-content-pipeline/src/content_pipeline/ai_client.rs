// LuperIQ CMS — AI Content Generation Client
// Copyright 2026 Luper Industries. All rights reserved.
// Proprietary and confidential.
//
// Shared AI client with provider abstraction (Anthropic, OpenAI, Ollama).
// Any CMS module can use this to generate content.

use serde_json;

// ── Configuration ──────────────────────────────────────────────────────────

fn default_provider() -> String {
    "anthropic".into()
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}
fn default_local_context_window() -> u32 {
    4096
}

const APPROX_CHARS_PER_TOKEN: u32 = 3;
const CHAT_TOKEN_SAFETY_BUFFER: u32 = 128;
const MIN_COMPLETION_TOKENS: u32 = 64;
const MAX_LOCAL_COMPLETION_TOKENS: u32 = 1024;

/// AI provider configuration. Read from [ai_content] in cms.toml + env vars.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AiContentConfig {
    /// "anthropic", "openai", or "ollama"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model identifier. Defaults per provider:
    /// - anthropic: "claude-sonnet-4-20250514"
    /// - openai: "gpt-4o"
    /// - ollama: "llama3.1"
    #[serde(default)]
    pub model: Option<String>,
    /// API key — if not set, reads from LUPERIQ_AI_API_KEY env var
    #[serde(default)]
    pub api_key: Option<String>,
    /// Base URL override. Defaults:
    /// - anthropic: "https://api.anthropic.com"
    /// - openai: "https://api.openai.com"
    /// - ollama: "http://localhost:11434"
    #[serde(default)]
    pub base_url: Option<String>,
    /// Max tokens per generation. Default: 4096
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature (0.0-1.0). Default: 0.7
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

impl Default for AiContentConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: None,
            api_key: None,
            base_url: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
        }
    }
}

// ── Response ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiResponse {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Default)]
pub struct AiGenerateOptions {
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub max_tokens: Option<u32>,
}

// ── Error ──────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AiError {
    NoApiKey,
    UnsupportedProvider(String),
    HttpError(String),
    ApiError { status: u16, message: String },
    ParseError(String),
    InsufficientCredits { needed: u32, available: u64 },
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::NoApiKey => write!(
                f,
                "No AI API key configured. Set LUPERIQ_AI_API_KEY environment variable."
            ),
            AiError::UnsupportedProvider(p) => write!(f, "Unsupported AI provider: {p}"),
            AiError::HttpError(e) => write!(f, "HTTP error: {e}"),
            AiError::ApiError { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            AiError::ParseError(e) => write!(f, "Parse error: {e}"),
            AiError::InsufficientCredits { needed, available } => {
                write!(f, "Insufficient credits: need {needed}, have {available}")
            }
        }
    }
}

impl std::error::Error for AiError {}

// ── Client ─────────────────────────────────────────────────────────────────

pub struct AiClient {
    config: AiContentConfig,
    http: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl AiClient {
    /// Create from config. Resolves API key from config or LUPERIQ_AI_API_KEY env var.
    /// Falls back to ANTHROPIC_API_KEY or OPENAI_API_KEY env vars depending on provider.
    pub fn new(config: &AiContentConfig) -> Result<Self, AiError> {
        let provider = config.provider.as_str();

        // Resolve API key: config > LUPERIQ_AI_API_KEY > provider-specific env var
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("LUPERIQ_AI_API_KEY").ok())
            .or_else(|| match provider {
                "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
                "openai" => std::env::var("OPENAI_API_KEY").ok(),
                _ => None,
            })
            .unwrap_or_default();

        // Require API key for remote providers only; local servers (ollama, vllm) don't need one
        let is_local = provider == "ollama"
            || provider == "vllm"
            || config.base_url.as_ref().map_or(false, |u| {
                u.contains("127.0.0.1") || u.contains("localhost") || u.contains("://192.168.")
            });
        if api_key.is_empty() && !is_local {
            return Err(AiError::NoApiKey);
        }

        let base_url = config.base_url.clone().unwrap_or_else(|| match provider {
            "anthropic" => "https://api.anthropic.com".into(),
            "openai" => "https://api.openai.com".into(),
            "vllm" => "http://127.0.0.1:9200".into(),
            "ollama" => "http://localhost:11434".into(),
            _ => "https://api.anthropic.com".into(),
        });

        let model = config.model.clone().unwrap_or_else(|| match provider {
            "anthropic" => "claude-sonnet-4-20250514".into(),
            "openai" => "gpt-4o".into(),
            "vllm" => "luper-content".into(),
            "ollama" => "llama3.1".into(),
            _ => "claude-sonnet-4-20250514".into(),
        });

        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // AI calls can be slow
            .build()
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        Ok(Self {
            config: config.clone(),
            http,
            api_key,
            base_url,
            model,
        })
    }

    /// Check if the client is configured (has API key or is local provider).
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
            || self.config.provider == "ollama"
            || self.config.provider == "vllm"
            || self.base_url.contains("127.0.0.1")
            || self.base_url.contains("localhost")
            || self.base_url.contains("://192.168.")
    }

    /// Get the provider name.
    pub fn provider(&self) -> &str {
        &self.config.provider
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get status info (provider, model, configured, base_url).
    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "provider": self.config.provider,
            "model": self.model,
            "base_url": self.base_url,
            "configured": self.is_configured(),
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        })
    }

    // ── Public generation API ──────────────────────────────────────────────

    /// Generate content from a system prompt and user message.
    /// Returns the generated text content.
    pub async fn generate(&self, system: &str, user_message: &str) -> Result<AiResponse, AiError> {
        self.generate_with_options(system, user_message, &AiGenerateOptions::default())
            .await
    }

    /// Generate content with per-request sampling overrides.
    pub async fn generate_with_options(
        &self,
        system: &str,
        user_message: &str,
        options: &AiGenerateOptions,
    ) -> Result<AiResponse, AiError> {
        let result = match self.config.provider.as_str() {
            "anthropic" => self.generate_anthropic(system, user_message, options).await,
            "openai" | "vllm" => self.generate_openai(system, user_message, options).await,
            "ollama" => self.generate_ollama(system, user_message, options).await,
            other => Err(AiError::UnsupportedProvider(other.to_string())),
        };

        // Strip <think>...</think> blocks from models that use reasoning (e.g. Qwen3)
        result.map(|mut r| {
            r.content = strip_thinking(&r.content);
            r
        })
    }

    fn effective_max_tokens(&self, requested: u32, system: &str, user_message: &str) -> u32 {
        match self.config.provider.as_str() {
            "vllm" | "ollama" => clamp_completion_tokens_for_context(
                requested,
                system,
                user_message,
                default_local_context_window(),
            ),
            _ => requested,
        }
    }

    /// Test the AI connection with a simple prompt. Returns Ok with model info on success.
    pub async fn test_connection(&self) -> Result<AiResponse, AiError> {
        self.generate(
            "You are a helpful assistant.",
            "Respond with exactly: CONNECTED. Include the current date if you know it.",
        )
        .await
    }

    // ── Provider implementations ───────────────────────────────────────────

    async fn generate_anthropic(
        &self,
        system: &str,
        user_message: &str,
        options: &AiGenerateOptions,
    ) -> Result<AiResponse, AiError> {
        let url = format!("{}/v1/messages", self.base_url);
        let max_tokens = self.effective_max_tokens(
            options.max_tokens.unwrap_or(self.config.max_tokens),
            system,
            user_message,
        );
        let temperature = options.temperature.unwrap_or(self.config.temperature);

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "system": system,
            "messages": [
                {"role": "user", "content": user_message}
            ]
        });
        if let Some(top_p) = options.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        let status = resp.status();
        let resp_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        if !status.is_success() {
            let msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: msg.to_string(),
            });
        }

        // Extract content from Anthropic response
        let content = resp_body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("No content in response".into()))?
            .to_string();

        let input_tokens = resp_body["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = resp_body["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;

        Ok(AiResponse {
            content,
            input_tokens,
            output_tokens,
            model: self.model.clone(),
            provider: "anthropic".into(),
        })
    }

    async fn generate_openai(
        &self,
        system: &str,
        user_message: &str,
        options: &AiGenerateOptions,
    ) -> Result<AiResponse, AiError> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let max_tokens = self.effective_max_tokens(
            options.max_tokens.unwrap_or(self.config.max_tokens),
            system,
            user_message,
        );
        let temperature = options.temperature.unwrap_or(self.config.temperature);

        let mut body = serde_json::json!({
            "model": self.model,
            "temperature": temperature,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_message}
            ]
        });
        if self.uses_max_completion_tokens() {
            body["max_completion_tokens"] = serde_json::json!(max_tokens);
        } else {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(top_p) = options.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        // Disable Qwen3 thinking mode for vLLM — we want direct content, not chain-of-thought
        if self.config.provider == "vllm" {
            body["chat_template_kwargs"] = serde_json::json!({"enable_thinking": false});
            if let Some(top_k) = options.top_k {
                body["extra_body"] = serde_json::json!({"top_k": top_k});
            }
        }

        let mut req = self
            .http
            .post(&url)
            .header("content-type", "application/json");

        // Only add auth header for remote providers
        if !self.api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let resp = req
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        let status = resp.status();
        let resp_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        if !status.is_success() {
            let msg = resp_body["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: msg.to_string(),
            });
        }

        let content = resp_body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("No content in response".into()))?
            .to_string();

        let input_tokens = resp_body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32;
        let output_tokens = resp_body["usage"]["completion_tokens"]
            .as_u64()
            .unwrap_or(0) as u32;

        Ok(AiResponse {
            content,
            input_tokens,
            output_tokens,
            model: self.model.clone(),
            provider: self.provider_label().into(),
        })
    }

    /// Get the provider label for responses (show "vllm" as the provider, not "openai").
    fn provider_label(&self) -> &str {
        &self.config.provider
    }

    fn uses_max_completion_tokens(&self) -> bool {
        self.config.provider == "openai" && self.model.starts_with("gpt-5")
    }

    async fn generate_ollama(
        &self,
        system: &str,
        user_message: &str,
        options: &AiGenerateOptions,
    ) -> Result<AiResponse, AiError> {
        let url = format!("{}/api/chat", self.base_url);
        let max_tokens = self.effective_max_tokens(
            options.max_tokens.unwrap_or(self.config.max_tokens),
            system,
            user_message,
        );
        let temperature = options.temperature.unwrap_or(self.config.temperature);

        let mut options_body = serde_json::json!({
            "temperature": temperature,
            "num_predict": max_tokens,
        });
        if let Some(top_p) = options.top_p {
            options_body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(top_k) = options.top_k {
            options_body["top_k"] = serde_json::json!(top_k);
        }

        let body = serde_json::json!({
            "model": self.model,
            "stream": false,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_message}
            ],
            "options": options_body
        });

        let resp = self
            .http
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        let status = resp.status();
        let resp_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AiError::HttpError(e.to_string()))?;

        if !status.is_success() {
            return Err(AiError::ApiError {
                status: status.as_u16(),
                message: resp_body.to_string(),
            });
        }

        let content = resp_body["message"]["content"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("No content in response".into()))?
            .to_string();

        // Ollama doesn't always report tokens
        let input_tokens = resp_body["prompt_eval_count"].as_u64().unwrap_or(0) as u32;
        let output_tokens = resp_body["eval_count"].as_u64().unwrap_or(0) as u32;

        Ok(AiResponse {
            content,
            input_tokens,
            output_tokens,
            model: self.model.clone(),
            provider: "ollama".into(),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

/// Strip `<think>...</think>` blocks from model output.
/// Qwen3 and other reasoning models may emit chain-of-thought in these tags.
fn strip_thinking(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut remaining = text;
    while let Some(start) = remaining.find("<think>") {
        result.push_str(&remaining[..start]);
        if let Some(end) = remaining[start..].find("</think>") {
            remaining = &remaining[start + end + 8..]; // 8 = "</think>".len()
        } else {
            // Unclosed <think> tag — skip everything after it
            return result.trim().to_string();
        }
    }
    result.push_str(remaining);
    result.trim().to_string()
}

fn approximate_token_count(text: &str) -> u32 {
    let chars = text.chars().count() as u32;
    (chars + (APPROX_CHARS_PER_TOKEN - 1)) / APPROX_CHARS_PER_TOKEN
}

fn clamp_completion_tokens_for_context(
    requested: u32,
    system: &str,
    user_message: &str,
    context_window: u32,
) -> u32 {
    let estimated_input_tokens = approximate_token_count(system)
        .saturating_add(approximate_token_count(user_message))
        .saturating_add(CHAT_TOKEN_SAFETY_BUFFER);
    let available_completion_tokens = context_window.saturating_sub(estimated_input_tokens);
    requested
        .min(available_completion_tokens.max(MIN_COMPLETION_TOKENS))
        .min(MAX_LOCAL_COMPLETION_TOKENS)
}

#[cfg(test)]
mod tests {
    use super::{clamp_completion_tokens_for_context, strip_thinking};

    #[test]
    fn strips_thinking_blocks() {
        assert_eq!(
            strip_thinking("hello <think>secret</think> world"),
            "hello  world"
        );
    }

    #[test]
    fn clamps_local_completion_budget_below_context_window() {
        let max_tokens = clamp_completion_tokens_for_context(
            4096,
            "System prompt with a little guidance.",
            "User prompt with some detail.",
            4096,
        );
        assert!(max_tokens < 4096);
        assert!(max_tokens >= 64);
    }

    #[test]
    fn keeps_smaller_requested_completion_budget() {
        let max_tokens =
            clamp_completion_tokens_for_context(512, "Short system", "Short user prompt", 4096);
        assert_eq!(max_tokens, 512);
    }

    #[test]
    fn caps_local_completion_budget_to_reasonable_ceiling() {
        let max_tokens =
            clamp_completion_tokens_for_context(4096, "Short system", "Short user prompt", 4096);
        assert_eq!(max_tokens, 1024);
    }
}
