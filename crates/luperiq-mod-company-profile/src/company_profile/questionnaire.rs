//! Structured questionnaire for guided company profile setup.
//!
//! Returns a predefined set of ~20 questions that map to CompanyProfile fields.
//! The admin fills in answers and they're stored as an import job in "review" status
//! for final confirmation before merging into the profile.

use serde::{Deserialize, Serialize};

/// A single questionnaire question with field mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionnaireQuestion {
    /// Unique ID for this question.
    pub id: String,
    /// The human-readable question text.
    pub question: String,
    /// Which CompanyProfile field this answer maps to.
    pub field_mapping: String,
    /// Input type: "text", "textarea", "select", "multi-select".
    pub input_type: String,
    /// Select/multi-select options (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Whether this question must be answered.
    pub required: bool,
}

/// Answer to a single question.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestionnaireAnswer {
    /// Question ID.
    pub id: String,
    /// Answer value (string for text/select, comma-separated for multi-select).
    pub value: String,
}

/// Get the full list of questionnaire questions.
pub fn get_questions() -> Vec<QuestionnaireQuestion> {
    vec![
        QuestionnaireQuestion {
            id: "q_name".into(),
            question: "What is your business name?".into(),
            field_mapping: "name".into(),
            input_type: "text".into(),
            options: None,
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_legal_name".into(),
            question: "What is your legal entity name (if different from business name)?".into(),
            field_mapping: "legal_name".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_tagline".into(),
            question: "What is your business tagline or slogan?".into(),
            field_mapping: "tagline".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_tone".into(),
            question: "What tone best describes your brand voice?".into(),
            field_mapping: "tone".into(),
            input_type: "select".into(),
            options: Some(vec![
                "professional".into(),
                "friendly".into(),
                "casual".into(),
                "authoritative".into(),
                "playful".into(),
            ]),
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_story".into(),
            question: "Tell us your company story. How did the business start? What is your mission?".into(),
            field_mapping: "story".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_service_philosophy".into(),
            question: "What is your service philosophy? How do you approach customer service?".into(),
            field_mapping: "service_philosophy".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_years_in_business".into(),
            question: "How many years has your business been operating?".into(),
            field_mapping: "years_in_business".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_usps".into(),
            question: "What makes your business unique? List your key selling points (one per line).".into(),
            field_mapping: "unique_selling_points".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_certifications".into(),
            question: "List any professional certifications your business holds (one per line).".into(),
            field_mapping: "certifications".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_license_numbers".into(),
            question: "List any license numbers (one per line).".into(),
            field_mapping: "license_numbers".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_owner_name".into(),
            question: "What is the owner/founder's name?".into(),
            field_mapping: "owner_name".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_owner_title".into(),
            question: "What is the owner's title (e.g. CEO, Founder, President)?".into(),
            field_mapping: "owner_title".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_phone".into(),
            question: "What is your primary business phone number?".into(),
            field_mapping: "phone".into(),
            input_type: "text".into(),
            options: None,
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_email".into(),
            question: "What is your primary business email address?".into(),
            field_mapping: "email".into(),
            input_type: "text".into(),
            options: None,
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_address".into(),
            question: "What is your business street address?".into(),
            field_mapping: "address".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_city".into(),
            question: "What city is your business located in?".into(),
            field_mapping: "city".into(),
            input_type: "text".into(),
            options: None,
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_state".into(),
            question: "What state?".into(),
            field_mapping: "state".into(),
            input_type: "text".into(),
            options: None,
            required: true,
        },
        QuestionnaireQuestion {
            id: "q_zip".into(),
            question: "What is your ZIP code?".into(),
            field_mapping: "zip".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_service_area".into(),
            question: "Describe the areas you serve (e.g. 'Greater Austin metro area, including Round Rock, Cedar Park, and Georgetown').".into(),
            field_mapping: "service_area_description".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_voice_notes".into(),
            question: "Any specific style preferences for how your business communicates? (e.g. 'We say folks not customers', 'Always mention our 24/7 availability'). One per line.".into(),
            field_mapping: "voice_notes".into(),
            input_type: "textarea".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_primary_color".into(),
            question: "What is your primary brand color? (hex code, e.g. #1a73e8)".into(),
            field_mapping: "brand_colors.primary".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_secondary_color".into(),
            question: "What is your secondary brand color? (hex code)".into(),
            field_mapping: "brand_colors.secondary".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
        QuestionnaireQuestion {
            id: "q_accent_color".into(),
            question: "What is your accent color? (hex code)".into(),
            field_mapping: "brand_colors.accent".into(),
            input_type: "text".into(),
            options: None,
            required: false,
        },
    ]
}

/// Build a partial CompanyProfile JSON from questionnaire answers.
///
/// Returns a `serde_json::Value` object with only the fields that were answered.
/// Multi-line answers for list fields (USPs, certifications, voice_notes, license_numbers)
/// are split into arrays.
pub fn build_profile_from_answers(answers: &[QuestionnaireAnswer]) -> serde_json::Value {
    let mut data = serde_json::Map::new();

    let questions = get_questions();
    let q_map: std::collections::HashMap<&str, &QuestionnaireQuestion> =
        questions.iter().map(|q| (q.id.as_str(), q)).collect();

    for answer in answers {
        let value = answer.value.trim();
        if value.is_empty() {
            continue;
        }

        let question = match q_map.get(answer.id.as_str()) {
            Some(q) => q,
            None => continue,
        };

        let field = &question.field_mapping;

        // Handle nested fields (brand_colors.primary, etc.)
        if field.starts_with("brand_colors.") {
            let sub_field = &field["brand_colors.".len()..];
            let colors = data
                .entry("brand_colors")
                .or_insert_with(|| serde_json::json!({}));
            if let Some(obj) = colors.as_object_mut() {
                obj.insert(sub_field.to_string(), serde_json::json!(value));
            }
            continue;
        }

        // Handle list fields (split multi-line textarea values into arrays)
        match field.as_str() {
            "unique_selling_points" | "certifications" | "license_numbers" | "voice_notes" => {
                let items: Vec<String> = value
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
                data.insert(field.clone(), serde_json::json!(items));
            }
            "years_in_business" => {
                if let Ok(years) = value.parse::<u32>() {
                    data.insert(field.clone(), serde_json::json!(years));
                }
            }
            _ => {
                data.insert(field.clone(), serde_json::json!(value));
            }
        }
    }

    serde_json::Value::Object(data)
}
