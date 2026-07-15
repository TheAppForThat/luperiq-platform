//! CSV import for SEO keywords.
//!
//! Accepts a simple CSV body with columns: keyword,search_volume,difficulty,intent
//! Parses and merges into an existing profile's seo_keywords (adds new, doesn't remove existing).
//!
//! No external CSV crate — just split by newlines and commas.

use super::profile::SeoKeyword;

/// Parse CSV text into a list of SeoKeyword entries.
///
/// Expected format (first line may be a header):
/// ```text
/// keyword,search_volume,difficulty,intent
/// hvac repair near me,12000,45,transactional
/// what is a seer rating,8000,22,informational
/// ```
///
/// Rules:
/// - Lines starting with "keyword" (case-insensitive) are treated as headers and skipped.
/// - Empty lines are skipped.
/// - search_volume and difficulty may be empty (parsed as None).
/// - intent defaults to "informational" if empty.
pub fn parse_seo_csv(csv_body: &str) -> Result<Vec<SeoKeyword>, String> {
    let mut keywords = Vec::new();

    for (line_num, raw_line) in csv_body.lines().enumerate() {
        let line = raw_line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Skip header row
        if line_num == 0 && line.to_lowercase().starts_with("keyword") {
            continue;
        }

        let cols: Vec<&str> = line.splitn(4, ',').collect();
        if cols.is_empty() {
            continue;
        }

        let keyword = cols[0].trim().to_string();
        if keyword.is_empty() {
            continue;
        }

        let search_volume = cols.get(1).and_then(|s| s.trim().parse::<u32>().ok());

        let difficulty = cols.get(2).and_then(|s| s.trim().parse::<u32>().ok());

        let intent = cols
            .get(3)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "informational".to_string());

        // Validate intent
        let valid_intents = [
            "informational",
            "commercial",
            "transactional",
            "navigational",
        ];
        if !valid_intents.contains(&intent.as_str()) {
            return Err(format!(
                "Line {}: invalid intent '{}'. Must be one of: {}",
                line_num + 1,
                intent,
                valid_intents.join(", ")
            ));
        }

        keywords.push(SeoKeyword {
            keyword,
            search_volume,
            difficulty,
            intent,
        });
    }

    Ok(keywords)
}

/// Merge new keywords into an existing keyword list.
///
/// Adds keywords that don't already exist (by keyword string, case-insensitive).
/// Returns the count of newly added keywords.
pub fn merge_keywords(existing: &mut Vec<SeoKeyword>, new_keywords: Vec<SeoKeyword>) -> usize {
    let existing_set: std::collections::HashSet<String> =
        existing.iter().map(|k| k.keyword.to_lowercase()).collect();

    let mut added = 0;
    for kw in new_keywords {
        if !existing_set.contains(&kw.keyword.to_lowercase()) {
            existing.push(kw);
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_with_header() {
        let csv = "keyword,search_volume,difficulty,intent\nhvac repair,12000,45,transactional\nac installation,8000,,commercial\n";
        let result = parse_seo_csv(csv).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].keyword, "hvac repair");
        assert_eq!(result[0].search_volume, Some(12000));
        assert_eq!(result[0].difficulty, Some(45));
        assert_eq!(result[0].intent, "transactional");
        assert_eq!(result[1].search_volume, Some(8000));
        assert_eq!(result[1].difficulty, None);
    }

    #[test]
    fn parse_csv_without_header() {
        let csv = "furnace tune-up,5000,30,commercial\n";
        let result = parse_seo_csv(csv).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].keyword, "furnace tune-up");
    }

    #[test]
    fn parse_csv_default_intent() {
        let csv = "hvac repair,1000,,\n";
        let result = parse_seo_csv(csv).unwrap();
        assert_eq!(result[0].intent, "informational");
    }

    #[test]
    fn parse_csv_invalid_intent() {
        let csv = "hvac repair,1000,30,invalid\n";
        assert!(parse_seo_csv(csv).is_err());
    }

    #[test]
    fn merge_deduplicates() {
        let mut existing = vec![SeoKeyword {
            keyword: "HVAC repair".into(),
            search_volume: Some(1000),
            difficulty: None,
            intent: "commercial".into(),
        }];
        let new = vec![
            SeoKeyword {
                keyword: "hvac repair".into(),
                search_volume: Some(2000),
                difficulty: None,
                intent: "commercial".into(),
            },
            SeoKeyword {
                keyword: "ac installation".into(),
                search_volume: None,
                difficulty: None,
                intent: "transactional".into(),
            },
        ];
        let added = merge_keywords(&mut existing, new);
        assert_eq!(added, 1);
        assert_eq!(existing.len(), 2);
    }
}
