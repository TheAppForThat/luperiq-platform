
//! Upload file parsing — converts text, markdown, and CSV into Vec<FactEntry>.
//!
//! ## Supported formats (V1)
//! - `.txt` / `.md` — parsed as "Key: Value" lines or sent to AI for extraction
//! - `.csv` — first column = key, second column = value
//!
//! ## Roadmap formats (documented, not built)
//! - `.docx` — Word document parsing (needs internal implementation, avoid external deps)
//! - `.pdf` — PDF text extraction (needs internal implementation)
//! - Images — OCR for scanned brochures (needs internal implementation)
//! - Google Docs — API import

use super::types::{FactConfidence, FactEntry};

/// Parse raw text content into FactEntry items.
///
/// Tries structured "Key: Value" format first. If the text doesn't appear
/// structured (less than 3 key-value pairs found), returns the whole text
/// as a single "content" fact entry — the AI prompt will use it as raw reference.
pub fn parse_text(content: &str) -> (Vec<FactEntry>, String) {
    let mut facts = Vec::new();
    let mut unstructured_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try "Key: Value" format
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            if !key.is_empty() && !value.is_empty() && key.len() < 60 {
                facts.push(FactEntry {
                    key: key.to_string(),
                    value: value.to_string(),
                    confidence: FactConfidence::CustomerStated,
                });
                continue;
            }
        }

        unstructured_lines.push(trimmed.to_string());
    }

    let raw = unstructured_lines.join("\n");

    // If we found very few structured facts, treat the whole thing as raw
    if facts.len() < 3 && !content.trim().is_empty() {
        facts.clear();
        facts.push(FactEntry {
            key: "content".to_string(),
            value: content.trim().to_string(),
            confidence: FactConfidence::CustomerStated,
        });
        return (facts, content.to_string());
    }

    (facts, raw)
}

/// Parse CSV content into FactEntry items.
///
/// Expected format: first column = key, second column = value.
/// Header row is auto-detected and skipped if first cell looks like "key" or "field".
pub fn parse_csv(content: &str) -> (Vec<FactEntry>, String) {
    let mut facts = Vec::new();
    let mut lines = content.lines();

    // Check for header row
    if let Some(first_line) = lines.next() {
        let lower = first_line.to_lowercase();
        if !lower.starts_with("key") && !lower.starts_with("field") && !lower.starts_with("name") {
            // Not a header, parse it
            if let Some(fact) = parse_csv_line(first_line) {
                facts.push(fact);
            }
        }
    }

    for line in lines {
        if let Some(fact) = parse_csv_line(line) {
            facts.push(fact);
        }
    }

    (facts, content.to_string())
}

fn parse_csv_line(line: &str) -> Option<FactEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Handle quoted CSV fields: "key","value with, commas"
    let (key, value) = if trimmed.starts_with('"') {
        // Find closing quote for first field
        let after_open = &trimmed[1..];
        let close = after_open.find('"')?;
        let k = &after_open[..close];
        // Skip comma separator after closing quote
        let rest = &after_open[close + 1..];
        let rest = rest.trim_start_matches(',').trim();
        // Strip quotes from value if present
        let v = rest.trim_matches('"');
        (k, v)
    } else {
        // Unquoted: simple split on first comma
        let parts: Vec<&str> = trimmed.splitn(2, ',').collect();
        if parts.len() < 2 {
            return None;
        }
        (parts[0].trim(), parts[1].trim().trim_matches('"'))
    };

    if !key.is_empty() && !value.is_empty() {
        Some(FactEntry {
            key: key.to_string(),
            value: value.to_string(),
            confidence: FactConfidence::CustomerStated,
        })
    } else {
        None
    }
}

/// Detect file format from filename extension.
pub fn detect_format(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".csv") {
        "csv"
    } else if lower.ends_with(".md") || lower.ends_with(".markdown") {
        "markdown"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_structured() {
        let input = "Description: A common household pest\nSeverity: High\nCategory: Insects\nPeak months: June, July";
        let (facts, _raw) = parse_text(input);
        assert_eq!(facts.len(), 4);
        assert_eq!(facts[0].key, "Description");
        assert_eq!(facts[0].value, "A common household pest");
        assert_eq!(facts[2].key, "Category");
    }

    #[test]
    fn parse_text_unstructured_falls_back() {
        let input = "This is just a paragraph about pests.\nNo structure here.";
        let (facts, raw) = parse_text(input);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].key, "content");
        assert!(!raw.is_empty());
    }

    #[test]
    fn parse_csv_with_header() {
        let input = "Key,Value\nPeak months,June July August\nSeverity,High";
        let (facts, _) = parse_csv(input);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].key, "Peak months");
        assert_eq!(facts[0].value, "June July August");
    }

    #[test]
    fn parse_csv_no_header() {
        let input = "Peak months,June July August\nSeverity,High";
        let (facts, _) = parse_csv(input);
        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn parse_csv_quoted_fields_with_commas() {
        let input = "Key,Value\n\"peak_months\",\"June, July, August\"\nSeverity,High";
        let (facts, _) = parse_csv(input);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts[0].key, "peak_months");
        assert_eq!(facts[0].value, "June, July, August");
        assert_eq!(facts[1].key, "Severity");
    }

    #[test]
    fn detect_format_csv() {
        assert_eq!(detect_format("data.csv"), "csv");
        assert_eq!(detect_format("notes.md"), "markdown");
        assert_eq!(detect_format("info.txt"), "text");
        assert_eq!(detect_format("readme"), "text");
    }
}
