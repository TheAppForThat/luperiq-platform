//! Census data import for location profiles.
//!
//! Accepts a JSON body with demographic fields and merges them into
//! an existing LocationProfile. Only provided fields are updated;
//! missing fields retain their current values.

use serde::Deserialize;

use super::profile::LocationProfile;

/// Payload for census data import. All fields are optional — only
/// provided fields will be merged into the existing profile.
#[derive(Debug, Deserialize)]
pub struct CensusImportPayload {
    pub population: Option<u64>,
    pub median_income: Option<u64>,
    pub housing_units: Option<u64>,
    pub owner_occupied_pct: Option<f64>,
    pub median_home_age: Option<u32>,
    pub cost_of_living_index: Option<f64>,
}

/// Merge census data into an existing location profile.
///
/// Only updates fields that are `Some` in the payload. Returns the
/// count of fields that were actually updated.
pub fn merge_census(profile: &mut LocationProfile, data: &CensusImportPayload) -> usize {
    let mut updated = 0;

    if let Some(v) = data.population {
        profile.population = Some(v);
        updated += 1;
    }
    if let Some(v) = data.median_income {
        profile.median_income = Some(v);
        updated += 1;
    }
    if let Some(v) = data.housing_units {
        profile.housing_units = Some(v);
        updated += 1;
    }
    if let Some(v) = data.owner_occupied_pct {
        profile.owner_occupied_pct = Some(v);
        updated += 1;
    }
    if let Some(v) = data.median_home_age {
        profile.median_home_age = Some(v);
        updated += 1;
    }
    if let Some(v) = data.cost_of_living_index {
        profile.cost_of_living_index = Some(v);
        updated += 1;
    }

    updated
}

#[cfg(test)]
mod tests {
    use super::super::profile::LocationProfile;
    use super::*;

    fn test_profile() -> LocationProfile {
        LocationProfile {
            id: "test".into(),
            slug: "test-city-tx".into(),
            city: "Test City".into(),
            state: "TX".into(),
            county: None,
            zip_codes: vec![],
            metro_area: None,
            population: None,
            median_income: None,
            housing_units: None,
            owner_occupied_pct: None,
            median_home_age: None,
            climate_zone: None,
            weather_patterns: vec![],
            local_keywords: vec![],
            local_competitors: vec![],
            local_regulations: vec![],
            cost_of_living_index: None,
            area_description: String::new(),
            neighborhoods: vec![],
            active: true,
        }
    }

    #[test]
    fn merge_all_fields() {
        let mut p = test_profile();
        let data = CensusImportPayload {
            population: Some(1_000_000),
            median_income: Some(65_000),
            housing_units: Some(400_000),
            owner_occupied_pct: Some(55.0),
            median_home_age: Some(25),
            cost_of_living_index: Some(95.0),
        };
        let count = merge_census(&mut p, &data);
        assert_eq!(count, 6);
        assert_eq!(p.population, Some(1_000_000));
        assert_eq!(p.median_income, Some(65_000));
        assert_eq!(p.housing_units, Some(400_000));
        assert_eq!(p.owner_occupied_pct, Some(55.0));
        assert_eq!(p.median_home_age, Some(25));
        assert_eq!(p.cost_of_living_index, Some(95.0));
    }

    #[test]
    fn merge_partial_fields() {
        let mut p = test_profile();
        p.population = Some(500_000);
        p.median_income = Some(50_000);

        let data = CensusImportPayload {
            population: Some(600_000),
            median_income: None,
            housing_units: Some(200_000),
            owner_occupied_pct: None,
            median_home_age: None,
            cost_of_living_index: None,
        };
        let count = merge_census(&mut p, &data);
        assert_eq!(count, 2);
        assert_eq!(p.population, Some(600_000));
        assert_eq!(p.median_income, Some(50_000)); // unchanged
        assert_eq!(p.housing_units, Some(200_000));
    }

    #[test]
    fn merge_no_fields() {
        let mut p = test_profile();
        p.population = Some(100_000);

        let data = CensusImportPayload {
            population: None,
            median_income: None,
            housing_units: None,
            owner_occupied_pct: None,
            median_home_age: None,
            cost_of_living_index: None,
        };
        let count = merge_census(&mut p, &data);
        assert_eq!(count, 0);
        assert_eq!(p.population, Some(100_000)); // unchanged
    }
}
