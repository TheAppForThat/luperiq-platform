// luperiq-cms/src/modules/theme_studio/template_engine.rs
//! Minimal Handlebars-like template engine for block registry templates.
//!
//! Supported syntax:
//! - {{field}} — HTML-escaped value
//! - {{{field}}} — raw value (only if field is declared richtext)
//! - {{#each items}}...{{/each}} — loop over array
//! - {{#if field}}...{{/if}} — conditional
//! - {{#unless field}}...{{/unless}} — inverse conditional
//! - {{@index}} — current loop index (0-based)

use serde_json::Value;
use std::collections::HashSet;

/// Render a template string with the given data context.
///
/// `richtext_fields` is the set of field keys declared as "richtext" in the
/// block definition. Only these fields are allowed to output raw HTML via
/// triple-brace syntax. All others are HTML-escaped regardless.
pub fn render_template(template: &str, data: &Value, richtext_fields: &HashSet<String>) -> String {
    let mut output = String::with_capacity(template.len() * 2);
    render_segment(template, data, richtext_fields, 0, &mut output);
    output
}

fn render_segment(
    template: &str,
    data: &Value,
    richtext_fields: &HashSet<String>,
    loop_index: usize,
    output: &mut String,
) {
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for {{ or {{{
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Check for triple brace (raw output)
            let is_triple = i + 2 < len && bytes[i + 2] == b'{';

            let start = if is_triple { i + 3 } else { i + 2 };
            let close_pattern = if is_triple { "}}}" } else { "}}" };

            if let Some(end_offset) = template[start..].find(close_pattern) {
                let expr = template[start..start + end_offset].trim();
                let close_len = close_pattern.len();

                if expr.starts_with("#each ") {
                    let var_name = expr[6..].trim();
                    let end_tag = "{{/each}}";
                    let rest_start = start + end_offset + close_len;
                    if let Some(body_end) = find_matching_close(&template[rest_start..], "each") {
                        let body = &template[rest_start..rest_start + body_end];
                        render_each(var_name, body, data, richtext_fields, output);
                        i = rest_start + body_end + end_tag.len();
                        continue;
                    }
                } else if expr.starts_with("#if ") {
                    let var_name = expr[4..].trim();
                    let end_tag = "{{/if}}";
                    let rest_start = start + end_offset + close_len;
                    if let Some(body_end) = find_matching_close(&template[rest_start..], "if") {
                        let body = &template[rest_start..rest_start + body_end];
                        if is_truthy(resolve_value(var_name, data)) {
                            render_segment(body, data, richtext_fields, loop_index, output);
                        }
                        i = rest_start + body_end + end_tag.len();
                        continue;
                    }
                } else if expr.starts_with("#unless ") {
                    let var_name = expr[8..].trim();
                    let end_tag = "{{/unless}}";
                    let rest_start = start + end_offset + close_len;
                    if let Some(body_end) = find_matching_close(&template[rest_start..], "unless") {
                        let body = &template[rest_start..rest_start + body_end];
                        if !is_truthy(resolve_value(var_name, data)) {
                            render_segment(body, data, richtext_fields, loop_index, output);
                        }
                        i = rest_start + body_end + end_tag.len();
                        continue;
                    }
                } else if expr == "@index" {
                    output.push_str(&loop_index.to_string());
                    i = start + end_offset + close_len;
                    continue;
                } else {
                    // Simple variable substitution
                    let val = resolve_value(expr, data);
                    let text = value_to_string(val);
                    if is_triple && richtext_fields.contains(expr) {
                        output.push_str(&text); // raw output for richtext
                    } else {
                        output.push_str(&escape_html(&text)); // always escape
                    }
                    i = start + end_offset + close_len;
                    continue;
                }
            }
        }

        // Regular character
        output.push(bytes[i] as char);
        i += 1;
    }
}

/// Find the matching closing tag for a block, respecting nesting depth.
/// `open_tag` is e.g. "each", `slice` is the template text after the opening tag.
/// Returns the byte offset of the start of the matching `{{/tag}}`.
fn find_matching_close(slice: &str, tag: &str) -> Option<usize> {
    let open_prefix = format!("{{{{#{} ", tag);
    let close_tag = format!("{{{{/{}}}}}", tag);
    let mut depth = 1usize;
    let mut pos = 0;
    while pos < slice.len() {
        // Check for nested open tag first
        if slice[pos..].starts_with(&open_prefix) {
            depth += 1;
            pos += open_prefix.len();
            continue;
        }
        // Check for close tag
        if slice[pos..].starts_with(&close_tag) {
            depth -= 1;
            if depth == 0 {
                return Some(pos);
            }
            pos += close_tag.len();
            continue;
        }
        pos += 1;
    }
    None
}

fn render_each(
    var_name: &str,
    body: &str,
    data: &Value,
    richtext_fields: &HashSet<String>,
    output: &mut String,
) {
    let arr = resolve_value(var_name, data);
    if let Value::Array(items) = arr {
        for (idx, item) in items.iter().enumerate() {
            render_segment(body, item, richtext_fields, idx, output);
        }
    }
}

/// Static null value for returning references to missing fields.
static JSON_NULL: Value = Value::Null;

fn resolve_value<'a>(path: &str, data: &'a Value) -> &'a Value {
    let mut current = data;
    for part in path.split('.') {
        match current.get(part) {
            Some(v) => current = v,
            None => return &JSON_NULL,
        }
    }
    current
}

fn value_to_string(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Number(_) => true,
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_variable_substitution() {
        let data = json!({"name": "Alice", "age": 30});
        let result = render_template("Hello {{name}}, age {{age}}", &data, &HashSet::new());
        assert_eq!(result, "Hello Alice, age 30");
    }

    #[test]
    fn html_escaping() {
        let data = json!({"text": "<script>alert('xss')</script>"});
        let result = render_template("{{text}}", &data, &HashSet::new());
        assert!(result.contains("&lt;script&gt;"));
        assert!(!result.contains("<script>"));
    }

    #[test]
    fn triple_brace_raw_only_for_richtext() {
        let data = json!({"body": "<b>bold</b>", "name": "<b>evil</b>"});
        let mut rt = HashSet::new();
        rt.insert("body".to_string());
        let result = render_template("{{{body}}} {{{name}}}", &data, &rt);
        // body is richtext → raw; name is not → escaped
        assert!(result.contains("<b>bold</b>"));
        assert!(result.contains("&lt;b&gt;evil&lt;/b&gt;"));
    }

    #[test]
    fn each_loop() {
        let data = json!({"items": [{"name": "A"}, {"name": "B"}]});
        let result = render_template("{{#each items}}[{{name}}]{{/each}}", &data, &HashSet::new());
        assert_eq!(result, "[A][B]");
    }

    #[test]
    fn if_conditional_truthy() {
        let data = json!({"show": true, "text": "visible"});
        let result = render_template("{{#if show}}{{text}}{{/if}}", &data, &HashSet::new());
        assert_eq!(result, "visible");
    }

    #[test]
    fn if_conditional_falsy() {
        let data = json!({"show": false, "text": "visible"});
        let result = render_template("{{#if show}}{{text}}{{/if}}", &data, &HashSet::new());
        assert_eq!(result, "");
    }

    #[test]
    fn unless_conditional() {
        let data = json!({"hidden": false});
        let result = render_template("{{#unless hidden}}shown{{/unless}}", &data, &HashSet::new());
        assert_eq!(result, "shown");
    }

    #[test]
    fn loop_index() {
        let data = json!({"items": [{"x": "a"}, {"x": "b"}, {"x": "c"}]});
        let result = render_template(
            "{{#each items}}{{@index}}:{{x}} {{/each}}",
            &data,
            &HashSet::new(),
        );
        assert_eq!(result, "0:a 1:b 2:c ");
    }

    #[test]
    fn nested_each() {
        let data =
            json!({"tiers": [{"name": "Basic", "features": [{"text": "F1"}, {"text": "F2"}]}]});
        let result = render_template(
            "{{#each tiers}}{{name}}:{{#each features}}{{text}},{{/each}}{{/each}}",
            &data,
            &HashSet::new(),
        );
        assert_eq!(result, "Basic:F1,F2,");
    }

    #[test]
    fn missing_variable_renders_empty() {
        let data = json!({"name": "Alice"});
        let result = render_template("{{name}} {{missing}}", &data, &HashSet::new());
        assert_eq!(result, "Alice ");
    }

    #[test]
    fn null_value_renders_empty() {
        let data = json!({"name": null});
        let result = render_template("{{name}}", &data, &HashSet::new());
        assert_eq!(result, "");
    }

    #[test]
    fn plain_text_passthrough() {
        let data = json!({});
        let result = render_template("Hello world", &data, &HashSet::new());
        assert_eq!(result, "Hello world");
    }
}
