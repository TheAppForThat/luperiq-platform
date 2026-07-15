//! Competitor import for location profiles.
//!
//! Accepts a JSON array of competitors and replaces the existing
//! `local_competitors` list in a LocationProfile.

use super::profile::LocalCompetitor;
use serde::Deserialize;

/// Payload entry for a single competitor import.
#[derive(Debug, Deserialize)]
pub struct CompetitorImportEntry {
    pub name: String,
    pub website: Option<String>,
    pub rating: Option<f64>,
    pub review_count: Option<u32>,
    #[serde(default)]
    pub specialties: Vec<String>,
}

/// Validate and convert imported competitor entries into LocalCompetitor structs.
///
/// Returns an error if any entry has an empty name or invalid rating.
pub fn validate_competitors(
    entries: Vec<CompetitorImportEntry>,
) -> Result<Vec<LocalCompetitor>, String> {
    let mut competitors = Vec::with_capacity(entries.len());

    for (i, entry) in entries.into_iter().enumerate() {
        let name = entry.name.trim().to_string();
        if name.is_empty() {
            return Err(format!("Competitor at index {} has an empty name", i));
        }

        if let Some(rating) = entry.rating {
            if !(0.0..=5.0).contains(&rating) {
                return Err(format!(
                    "Competitor '{}' has invalid rating {} (must be 0.0 - 5.0)",
                    name, rating
                ));
            }
        }

        competitors.push(LocalCompetitor {
            name,
            website: entry
                .website
                .map(|w| w.trim().to_string())
                .filter(|w| !w.is_empty()),
            rating: entry.rating,
            review_count: entry.review_count,
            specialties: entry.specialties,
        });
    }

    Ok(competitors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_competitors() {
        let entries = vec![
            CompetitorImportEntry {
                name: "Acme HVAC".into(),
                website: Some("https://acmehvac.com".into()),
                rating: Some(4.5),
                review_count: Some(127),
                specialties: vec!["AC repair".into(), "furnace installation".into()],
            },
            CompetitorImportEntry {
                name: "Bob's Plumbing".into(),
                website: None,
                rating: Some(3.8),
                review_count: None,
                specialties: vec![],
            },
        ];
        let result = validate_competitors(entries).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "Acme HVAC");
        assert_eq!(result[0].rating, Some(4.5));
        assert_eq!(result[0].specialties.len(), 2);
        assert_eq!(result[1].website, None);
    }

    #[test]
    fn validate_empty_name_rejected() {
        let entries = vec![CompetitorImportEntry {
            name: "  ".into(),
            website: None,
            rating: None,
            review_count: None,
            specialties: vec![],
        }];
        assert!(validate_competitors(entries).is_err());
    }

    #[test]
    fn validate_invalid_rating_rejected() {
        let entries = vec![CompetitorImportEntry {
            name: "Bad Rating Co".into(),
            website: None,
            rating: Some(6.0),
            review_count: None,
            specialties: vec![],
        }];
        assert!(validate_competitors(entries).is_err());
    }

    #[test]
    fn validate_zero_rating_accepted() {
        let entries = vec![CompetitorImportEntry {
            name: "New Business".into(),
            website: None,
            rating: Some(0.0),
            review_count: Some(0),
            specialties: vec![],
        }];
        let result = validate_competitors(entries).unwrap();
        assert_eq!(result[0].rating, Some(0.0));
    }
}
