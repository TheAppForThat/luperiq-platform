//! Two-stage AI content generation engine for service-business websites.
//!
//! Assembles a `GenerationContext` from CompanyProfile + IndustryProfile +
//! LocationProfile + SEO guidelines + FactPacks, renders Handlebars system/section
//! prompts, then drives an AI model (Anthropic, OpenAI/vLLM, or Ollama) to produce
//! HTML sections stitched into a full page. Supports single-page generation
//! (`/generate`), full-site batch runs (`/generate-site`), and job lifecycle
//! tracking (pending → generating → review → published).
pub mod content_pipeline;
pub use content_pipeline::ContentPipelineModule;
