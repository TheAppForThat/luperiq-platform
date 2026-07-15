// luperiq-cms/src/modules/theme_studio/block_registry.rs
//! Block Registry — loads, validates, and serves block definitions.
//!
//! Block definitions describe a block's fields, template, CSS, and category.
//! Built-in definitions are embedded in the binary. Custom blocks load from disk.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// A block definition describing a reusable content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: String,
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<BlockField>,
    #[serde(default)]
    pub template: String,
    #[serde(default)]
    pub css: String,
    #[serde(default)]
    pub responsive: Option<BlockResponsive>,
    #[serde(default = "default_behavior")]
    pub behavior: String,
    #[serde(default)]
    pub render: String,
    #[serde(default)]
    pub industries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockField {
    pub key: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub fields: Vec<BlockField>, // for repeater
    #[serde(default)]
    pub options: Vec<String>, // for select
    #[serde(default)]
    pub min: Option<f64>, // for number
    #[serde(default)]
    pub max: Option<f64>, // for number
    #[serde(default)]
    pub step: Option<f64>, // for number
    #[serde(default)]
    pub source: Option<FieldSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldSource {
    #[serde(default)]
    pub read: String,
    #[serde(default)]
    pub write: String,
    #[serde(default)]
    pub field_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockResponsive {
    #[serde(default = "default_640")]
    pub stack_breakpoint: u16,
    #[serde(default)]
    pub mobile_css: String,
}

fn default_version() -> String {
    "1.0.0".into()
}
fn default_author() -> String {
    "LuperIQ".into()
}
fn default_category() -> String {
    "general".into()
}
fn default_behavior() -> String {
    "none".into()
}
fn default_640() -> u16 {
    640
}

/// The block registry: maps block ID to definition.
#[derive(Debug, Clone, Default)]
pub struct BlockRegistry {
    blocks: HashMap<String, BlockDefinition>,
}

impl BlockRegistry {
    /// Create a new registry from built-in definitions and optional custom blocks dir.
    pub fn load(builtin_jsons: &[&str], custom_dir: Option<&Path>) -> Self {
        let mut registry = Self::default();

        // Load built-in blocks
        for json_str in builtin_jsons {
            match serde_json::from_str::<BlockDefinition>(json_str) {
                Ok(def) => {
                    registry.blocks.insert(def.id.clone(), def);
                }
                Err(e) => {
                    eprintln!("[block_registry] Failed to parse built-in block: {e}");
                }
            }
        }

        // Load custom blocks from disk (overrides built-in by id)
        if let Some(dir) = custom_dir {
            if dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().map(|e| e == "json").unwrap_or(false) {
                            match std::fs::read_to_string(&path) {
                                Ok(contents) => {
                                    match serde_json::from_str::<BlockDefinition>(&contents) {
                                        Ok(def) => {
                                            if let Err(e) = validate_definition(&def) {
                                                eprintln!(
                                                    "[block_registry] Skipping {}: {e}",
                                                    path.display()
                                                );
                                            } else {
                                                registry.blocks.insert(def.id.clone(), def);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "[block_registry] Failed to parse {}: {e}",
                                                path.display()
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "[block_registry] Failed to read {}: {e}",
                                        path.display()
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        registry
    }

    /// Get a block definition by ID.
    pub fn get(&self, id: &str) -> Option<&BlockDefinition> {
        self.blocks.get(id)
    }

    /// Get all block definitions, sorted by category then name.
    pub fn all_sorted(&self) -> Vec<&BlockDefinition> {
        let mut defs: Vec<&BlockDefinition> = self.blocks.values().collect();
        defs.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        defs
    }

    /// Get the set of richtext field keys for a block definition.
    pub fn richtext_fields(&self, id: &str) -> HashSet<String> {
        let mut set = HashSet::new();
        if let Some(def) = self.get(id) {
            collect_richtext_fields(&def.fields, &mut set);
        }
        set
    }

    /// Render a custom block using the template engine.
    pub fn render_block(&self, id: &str, data: &serde_json::Value) -> Option<String> {
        let def = self.get(id)?;
        let rt_fields = self.richtext_fields(id);
        let html = super::template_engine::render_template(&def.template, data, &rt_fields);

        let mut output = String::new();
        // Wrap in block div with CSS class
        output.push_str(&format!(
            "<div class=\"liq-block liq-block--{}\">\n{}\n</div>\n",
            def.id, html
        ));
        // Append scoped CSS if present
        if !def.css.is_empty() {
            output.push_str(&format!("<style>{}</style>\n", def.css));
        }
        // Append responsive CSS if present
        if let Some(ref resp) = def.responsive {
            if !resp.mobile_css.is_empty() {
                output.push_str(&format!(
                    "<style>@media (max-width: {}px) {{ {} }}</style>\n",
                    resp.stack_breakpoint, resp.mobile_css
                ));
            }
        }
        Some(output)
    }

    /// Get block definitions relevant to a given industry, sorted by category then name.
    pub fn for_industry(&self, industry: &str) -> Vec<&BlockDefinition> {
        let mut defs: Vec<&BlockDefinition> = self
            .blocks
            .values()
            .filter(|d| d.industries.is_empty() || d.industries.iter().any(|i| i == industry))
            .collect();
        defs.sort_by(|a, b| a.category.cmp(&b.category).then(a.name.cmp(&b.name)));
        defs
    }

    /// Return all definitions as JSON for the editor palette API.
    pub fn to_palette_json(&self) -> String {
        let sorted = self.all_sorted();
        serde_json::to_string(&sorted).unwrap_or_else(|_| "[]".to_string())
    }
}

fn collect_richtext_fields(fields: &[BlockField], set: &mut HashSet<String>) {
    for field in fields {
        if field.field_type == "richtext" {
            set.insert(field.key.clone());
        }
        if !field.fields.is_empty() {
            collect_richtext_fields(&field.fields, set);
        }
    }
}

// ── Validation & Sanitization ──────────────────────────────────────────

/// Validate a block definition for safety.
pub fn validate_definition(def: &BlockDefinition) -> Result<(), String> {
    if def.id.is_empty() {
        return Err("Block id is required".into());
    }
    if def.name.is_empty() {
        return Err("Block name is required".into());
    }

    // Template sanitization
    let template_lower = def.template.to_lowercase();
    for dangerous in &[
        "<script",
        "onclick",
        "onerror",
        "onload",
        "javascript:",
        "<iframe",
        "onmouseover",
    ] {
        if template_lower.contains(dangerous) {
            return Err(format!(
                "Template contains forbidden content: {}",
                dangerous
            ));
        }
    }

    // CSS sanitization
    let css_lower = def.css.to_lowercase();
    if css_lower.contains("expression(") {
        return Err("CSS contains forbidden expression()".into());
    }
    // Block external URLs in CSS (data exfiltration risk)
    if css_lower.contains("url(http") || css_lower.contains("url(//") {
        return Err("CSS contains external url() — only relative paths allowed".into());
    }

    // Behavior validation (must be a known value or "none")
    if !super::behaviors::BlockBehavior::is_valid(&def.behavior) {
        return Err(format!("Unknown behavior: {}", def.behavior));
    }

    Ok(())
}

// ── File I/O ───────────────────────────────────────────────────────────

/// Save a validated block definition to a directory as a JSON file.
pub fn save_block_to_dir(def: &BlockDefinition, dir: &Path) -> Result<(), String> {
    validate_definition(def)?;
    std::fs::create_dir_all(dir).map_err(|e| format!("Failed to create blocks directory: {e}"))?;
    let filename = format!("{}.json", def.id);
    let path = dir.join(&filename);
    let json =
        serde_json::to_string_pretty(def).map_err(|e| format!("Failed to serialize block: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Failed to write {}: {e}", path.display()))?;
    Ok(())
}

/// Delete a custom block definition file from a directory.
pub fn delete_block_from_dir(id: &str, dir: &Path) -> Result<(), String> {
    let filename = format!("{}.json", id);
    let path = dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete {}: {e}", path.display()))?;
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_block_json() -> &'static str {
        r#"{
            "id": "test-block",
            "name": "Test Block",
            "category": "testing",
            "fields": [
                {"key": "title", "type": "text", "label": "Title"},
                {"key": "body", "type": "richtext", "label": "Body"}
            ],
            "template": "<h2>{{title}}</h2><div>{{{body}}}</div>",
            "css": ".test { color: red; }",
            "behavior": "none"
        }"#
    }

    #[test]
    fn parse_block_definition() {
        let def: BlockDefinition = serde_json::from_str(sample_block_json()).unwrap();
        assert_eq!(def.id, "test-block");
        assert_eq!(def.name, "Test Block");
        assert_eq!(def.category, "testing");
        assert_eq!(def.fields.len(), 2);
        assert_eq!(def.fields[0].field_type, "text");
        assert_eq!(def.fields[1].field_type, "richtext");
    }

    #[test]
    fn registry_loads_from_json_strings() {
        let registry = BlockRegistry::load(&[sample_block_json()], None);
        assert!(registry.get("test-block").is_some());
        assert_eq!(registry.all_sorted().len(), 1);
    }

    #[test]
    fn richtext_fields_collected() {
        let registry = BlockRegistry::load(&[sample_block_json()], None);
        let rt = registry.richtext_fields("test-block");
        assert!(rt.contains("body"));
        assert!(!rt.contains("title"));
    }

    #[test]
    fn render_custom_block() {
        let registry = BlockRegistry::load(&[sample_block_json()], None);
        let data = json!({"title": "Hello", "body": "<b>World</b>"});
        let html = registry.render_block("test-block", &data).unwrap();
        assert!(html.contains("<h2>Hello</h2>"));
        assert!(html.contains("<b>World</b>")); // richtext → raw
        assert!(html.contains("liq-block--test-block"));
        assert!(html.contains("<style>.test { color: red; }</style>"));
    }

    #[test]
    fn render_escapes_non_richtext() {
        let json = r#"{
            "id": "safe-block",
            "name": "Safe",
            "fields": [{"key": "text", "type": "text", "label": "T"}],
            "template": "{{text}}",
            "behavior": "none"
        }"#;
        let registry = BlockRegistry::load(&[json], None);
        let data = json!({"text": "<script>evil</script>"});
        let html = registry.render_block("safe-block", &data).unwrap();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn validation_rejects_script_in_template() {
        let def = BlockDefinition {
            id: "bad".into(),
            name: "Bad".into(),
            template: "<div><script>alert(1)</script></div>".into(),
            ..serde_json::from_str::<BlockDefinition>(sample_block_json()).unwrap()
        };
        assert!(validate_definition(&def).is_err());
    }

    #[test]
    fn validation_rejects_unknown_behavior() {
        let def = BlockDefinition {
            id: "bad".into(),
            name: "Bad".into(),
            behavior: "evil_behavior".into(),
            template: "<div></div>".into(),
            ..serde_json::from_str::<BlockDefinition>(sample_block_json()).unwrap()
        };
        assert!(validate_definition(&def).is_err());
    }

    #[test]
    fn custom_blocks_override_builtin() {
        let builtin = r#"{"id": "shared", "name": "Built-in", "behavior": "none"}"#;
        let custom = r#"{"id": "shared", "name": "Custom Override", "behavior": "none"}"#;
        // Use unique temp dir to avoid parallel test interference
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("shared.json"), custom).unwrap();

        let registry = BlockRegistry::load(&[builtin], Some(dir.path()));
        let def = registry.get("shared").unwrap();
        assert_eq!(def.name, "Custom Override");
        // dir auto-deletes on drop
    }

    #[test]
    fn palette_json_is_valid() {
        let registry = BlockRegistry::load(&[sample_block_json()], None);
        let json = registry.to_palette_json();
        let parsed: Vec<BlockDefinition> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "test-block");
    }

    #[test]
    fn save_and_load_block() {
        let dir = tempfile::tempdir().unwrap();
        let def: BlockDefinition = serde_json::from_str(sample_block_json()).unwrap();
        save_block_to_dir(&def, dir.path()).unwrap();
        assert!(dir.path().join("test-block.json").exists());
        let registry = BlockRegistry::load(&[], Some(dir.path()));
        assert_eq!(registry.get("test-block").unwrap().name, "Test Block");
    }

    #[test]
    fn save_rejects_invalid_block() {
        let dir = tempfile::tempdir().unwrap();
        let mut def: BlockDefinition = serde_json::from_str(sample_block_json()).unwrap();
        def.template = "<script>evil</script>".into();
        assert!(save_block_to_dir(&def, dir.path()).is_err());
    }

    #[test]
    fn delete_block_works() {
        let dir = tempfile::tempdir().unwrap();
        let def: BlockDefinition = serde_json::from_str(sample_block_json()).unwrap();
        save_block_to_dir(&def, dir.path()).unwrap();
        assert!(dir.path().join("test-block.json").exists());
        delete_block_from_dir("test-block", dir.path()).unwrap();
        assert!(!dir.path().join("test-block.json").exists());
    }

    #[test]
    fn block_field_with_source_deserializes() {
        let json = r#"{"key":"tagline","type":"text","label":"Headline","source":{"read":"/api/modules/company-profile/profile","write":"/api/modules/company-profile/profile","field_path":"tagline"}}"#;
        let field: BlockField = serde_json::from_str(json).unwrap();
        assert_eq!(field.key, "tagline");
        let src = field.source.unwrap();
        assert_eq!(src.field_path, "tagline");
    }

    #[test]
    fn block_definition_with_render_and_industries() {
        let json = r#"{"id":"company-hero","name":"Hero Section","render":"company_hero","industries":["pest-control","hvac"],"behavior":"none"}"#;
        let def: BlockDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.render, "company_hero");
        assert_eq!(def.industries.len(), 2);
    }

    #[test]
    fn block_definition_without_render_defaults_empty() {
        let def: BlockDefinition = serde_json::from_str(sample_block_json()).unwrap();
        assert!(def.render.is_empty());
        assert!(def.industries.is_empty());
    }
}
