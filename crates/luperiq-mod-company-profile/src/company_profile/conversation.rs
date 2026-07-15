//! AI-powered extraction of company profile data from free-form conversation text.
//!
//! Uses the shared AI provider to parse an owner's description of their business
//! and extract structured CompanyProfile fields. The result is stored as an import
//! job in "review" status for admin confirmation.

use super::CompanyAiProvider;

/// System prompt for the AI extraction task.
const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a business data extraction assistant. Your job is to extract structured business identity information from a conversation transcript or free-form text provided by a business owner.

Return ONLY a valid JSON object with these possible fields (only include fields you can confidently extract):

- "name" (string): Business name
- "legal_name" (string): Legal entity name if mentioned
- "tagline" (string): Business tagline or slogan
- "story" (string): Company origin story or mission (2-3 paragraphs)
- "tone" (string): One of "professional", "friendly", "casual", "authoritative", "playful"
- "voice_notes" (array of strings): Communication style preferences
- "certifications" (array of strings): Professional certifications
- "license_numbers" (array of strings): License numbers
- "years_in_business" (number): How many years in business
- "service_philosophy" (string): Their approach to customer service
- "unique_selling_points" (array of strings): What makes them unique
- "owner_name" (string): Owner/founder name
- "owner_title" (string): Owner's title
- "phone" (string): Business phone number
- "email" (string): Business email
- "address" (string): Street address
- "city" (string): City
- "state" (string): State
- "zip" (string): ZIP code
- "service_area_description" (string): Areas they serve

Do NOT include fields that aren't mentioned or can't be reasonably inferred.
Do NOT make up information. Only extract what is clearly stated or strongly implied.
Return ONLY the JSON object, no explanation or markdown formatting."#;

/// Extract structured CompanyProfile fields from a free-form conversation transcript.
///
/// Returns a JSON object with whatever fields could be confidently extracted.
/// Returns Err if AI provider is unavailable or if the extraction fails.
pub async fn extract_from_conversation(
    ai_provider: &dyn CompanyAiProvider,
    transcript: &str,
) -> Result<serde_json::Value, String> {
    if transcript.trim().is_empty() {
        return Err("Transcript is empty".into());
    }

    let user_message = format!(
        "Extract business identity information from this conversation/text:\n\n{}",
        transcript
    );

    let response = ai_provider
        .generate(EXTRACTION_SYSTEM_PROMPT, &user_message)
        .await
        .map_err(|e| format!("AI extraction failed: {e}"))?;

    // Parse the AI response as JSON
    let content = response.content.trim();

    // Try to extract JSON from the response (handle markdown code blocks)
    let json_str = if content.starts_with("```") {
        // Strip markdown code block
        let inner = content
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        inner
    } else {
        content
    };

    let extracted: serde_json::Value = serde_json::from_str(json_str).map_err(|e| {
        format!(
            "Failed to parse AI response as JSON: {e}. Response was: {}",
            &content[..content.len().min(200)]
        )
    })?;

    if !extracted.is_object() {
        return Err("AI returned non-object JSON".into());
    }

    // Filter out null and empty values
    let mut filtered = serde_json::Map::new();
    if let Some(obj) = extracted.as_object() {
        for (key, val) in obj {
            match val {
                serde_json::Value::Null => continue,
                serde_json::Value::String(s) if s.is_empty() => continue,
                serde_json::Value::Array(a) if a.is_empty() => continue,
                _ => {
                    filtered.insert(key.clone(), val.clone());
                }
            }
        }
    }

    if filtered.is_empty() {
        return Err("AI could not extract any business information from the transcript".into());
    }

    Ok(serde_json::Value::Object(filtered))
}
