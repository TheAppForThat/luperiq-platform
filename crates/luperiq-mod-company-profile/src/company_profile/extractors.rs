//! URL scrapers for semi-automated company profile import.
//!
//! These are *best-effort* extractors that use simple regex and HTML text parsing
//! to pull structured data from public web pages. They will NOT work perfectly on
//! every website — that's by design. The extracted data goes into a "review" state
//! where the admin can correct it before applying to the CompanyProfile.
//!
//! Supported sources:
//! - Google Business Profile pages
//! - Facebook Business pages
//! - Any website (generic extraction)

use serde_json::json;

/// Best-effort extraction from a Google Business Profile URL.
///
/// Attempts to extract: name, address, phone, rating, review_count, categories, hours.
/// Returns a partial JSON object with whatever fields it can find.
pub async fn extract_google_business(url: &str) -> Result<serde_json::Value, String> {
    let html = fetch_page(url).await?;
    let mut data = serde_json::Map::new();

    // Extract business name from <title> tag
    if let Some(title) = extract_between(&html, "<title>", "</title>") {
        // Google Business titles are typically "Business Name - Google Maps" or similar
        let name = title
            .split(" - ")
            .next()
            .unwrap_or(&title)
            .trim()
            .to_string();
        if !name.is_empty() && name != "Google Maps" {
            data.insert("name".into(), json!(name));
        }
    }

    // Extract phone numbers using regex pattern
    for phone in extract_phone_numbers(&html) {
        data.insert("phone".into(), json!(phone));
        break; // Take the first phone number found
    }

    // Extract star rating patterns like "4.8 stars" or "4.8/5" or data attributes
    if let Some(rating) = extract_rating(&html) {
        data.insert("_google_rating".into(), json!(rating));
    }

    // Extract review count patterns like "123 reviews" or "(123)"
    if let Some(count) = extract_review_count(&html) {
        data.insert("_google_review_count".into(), json!(count));
    }

    // Look for address patterns (street number + street name + city/state/zip)
    if let Some(addr) = extract_address_pattern(&html) {
        data.insert("address".into(), json!(addr));
    }

    // Extract meta description for potential tagline
    if let Some(desc) = extract_meta_description(&html) {
        data.insert("_meta_description".into(), json!(desc));
    }

    if data.is_empty() {
        return Err("Could not extract any data from the Google Business page. The page structure may have changed.".into());
    }

    Ok(serde_json::Value::Object(data))
}

/// Best-effort extraction from a Facebook Business page URL.
///
/// Attempts to extract: about text, cover photo URL, story, name.
/// Returns a partial JSON object with whatever fields it can find.
pub async fn extract_facebook(url: &str) -> Result<serde_json::Value, String> {
    let html = fetch_page(url).await?;
    let mut data = serde_json::Map::new();

    // Extract page title (business name)
    if let Some(title) = extract_between(&html, "<title>", "</title>") {
        // Facebook page titles are typically "Business Name | Facebook" or "Business Name - Home | Facebook"
        let name = title
            .split('|')
            .next()
            .unwrap_or(&title)
            .split(" - ")
            .next()
            .unwrap_or(&title)
            .trim()
            .to_string();
        if !name.is_empty() && name != "Facebook" {
            data.insert("name".into(), json!(name));
        }
    }

    // Extract og:description meta tag (often contains the "About" text)
    if let Some(desc) = extract_og_meta(&html, "og:description") {
        if !desc.is_empty() {
            data.insert("_about".into(), json!(desc));
        }
    }

    // Extract og:image for cover photo
    if let Some(img) = extract_og_meta(&html, "og:image") {
        if !img.is_empty() {
            data.insert("_cover_photo_url".into(), json!(img));
        }
    }

    // Extract phone numbers
    for phone in extract_phone_numbers(&html) {
        data.insert("phone".into(), json!(phone));
        break;
    }

    // Extract email addresses
    for email in extract_emails(&html) {
        data.insert("email".into(), json!(email));
        break;
    }

    // Extract meta description
    if let Some(desc) = extract_meta_description(&html) {
        if !desc.is_empty() {
            data.insert("_meta_description".into(), json!(desc));
        }
    }

    if data.is_empty() {
        return Err("Could not extract any data from the Facebook page. The page may require authentication or the structure has changed.".into());
    }

    Ok(serde_json::Value::Object(data))
}

/// Best-effort extraction from any website URL.
///
/// Attempts to extract: title, description, phone numbers, emails.
/// Returns a partial JSON object with whatever fields it can find.
pub async fn extract_website(url: &str) -> Result<serde_json::Value, String> {
    let html = fetch_page(url).await?;
    let mut data = serde_json::Map::new();

    // Extract page title
    if let Some(title) = extract_between(&html, "<title>", "</title>") {
        let title = title.trim().to_string();
        if !title.is_empty() {
            data.insert("name".into(), json!(title));
        }
    }

    // Extract meta description
    if let Some(desc) = extract_meta_description(&html) {
        if !desc.is_empty() {
            data.insert("tagline".into(), json!(desc));
        }
    }

    // Extract all unique phone numbers found on the page
    let phones = extract_phone_numbers(&html);
    if !phones.is_empty() {
        data.insert("phone".into(), json!(phones[0]));
        if phones.len() > 1 {
            data.insert("_additional_phones".into(), json!(phones[1..]));
        }
    }

    // Extract all unique email addresses found on the page
    let emails = extract_emails(&html);
    if !emails.is_empty() {
        data.insert("email".into(), json!(emails[0]));
        if emails.len() > 1 {
            data.insert("_additional_emails".into(), json!(emails[1..]));
        }
    }

    // Try to extract social media links
    let social = extract_social_links(&html);
    if !social.is_empty() {
        data.insert("social_links".into(), json!(social));
    }

    // Try to extract address from structured data or common patterns
    if let Some(addr) = extract_address_pattern(&html) {
        data.insert("address".into(), json!(addr));
    }

    if data.is_empty() {
        return Err("Could not extract any data from the website. The page may be dynamically rendered or empty.".into());
    }

    Ok(serde_json::Value::Object(data))
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Shared HTTP client — avoids rebuilding a connection pool on every import request.
///
/// Initialized once on first use via `std::sync::OnceLock` (stable Rust).
/// Build failures propagate as `Err` on the first call; on success the client
/// is stored and reused for all subsequent requests.
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

/// Fetch a web page via reqwest. Returns the HTML body as a string.
async fn fetch_page(url: &str) -> Result<String, String> {
    let client = match HTTP_CLIENT.get() {
        Some(c) => c,
        None => {
            let built = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (compatible; LuperIQ-CMS/1.0; +https://luperiq.com)")
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
                .map_err(|e| format!("HTTP client error: {e}"))?;
            let _ = HTTP_CLIENT.set(built);
            HTTP_CLIENT.get().expect("client was just set")
        }
    };

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}: failed to fetch page", resp.status()));
    }

    resp.text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))
}

/// Extract text between two delimiters (case-insensitive search).
fn extract_between(html: &str, start: &str, end: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start_lower = start.to_lowercase();
    let end_lower = end.to_lowercase();

    let start_idx = lower.find(&start_lower)?;
    let content_start = start_idx + start.len();
    let end_idx = lower[content_start..].find(&end_lower)?;

    Some(
        html[content_start..content_start + end_idx]
            .trim()
            .to_string(),
    )
}

/// Extract the content attribute from a <meta name="description"> tag.
fn extract_meta_description(html: &str) -> Option<String> {
    // Try standard meta description
    extract_meta_content(html, "description")
}

/// Extract content from a meta tag by name attribute.
fn extract_meta_content(html: &str, name: &str) -> Option<String> {
    let lower = html.to_lowercase();
    // Look for <meta name="NAME" content="VALUE">
    let pattern = format!("name=\"{}\"", name);
    if let Some(pos) = lower.find(&pattern) {
        // Search nearby for content="..."
        let region = &html[pos.saturating_sub(100)..std::cmp::min(pos + 500, html.len())];
        if let Some(content) = extract_attribute(region, "content") {
            return Some(html_decode(&content));
        }
    }
    None
}

/// Extract content from an OpenGraph meta tag.
fn extract_og_meta(html: &str, property: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let pattern = format!("property=\"{}\"", property);
    if let Some(pos) = lower.find(&pattern) {
        let region = &html[pos.saturating_sub(100)..std::cmp::min(pos + 500, html.len())];
        if let Some(content) = extract_attribute(region, "content") {
            return Some(html_decode(&content));
        }
    }
    None
}

/// Extract an attribute value from a tag fragment.
fn extract_attribute(fragment: &str, attr: &str) -> Option<String> {
    let lower = fragment.to_lowercase();
    let pattern = format!("{}=\"", attr);
    let pos = lower.find(&pattern)?;
    let start = pos + pattern.len();
    let end = fragment[start..].find('"')?;
    Some(fragment[start..start + end].to_string())
}

/// Extract US phone numbers from HTML text.
/// Looks for patterns like (555) 555-5555, 555-555-5555, 555.555.5555, +1-555-555-5555.
fn extract_phone_numbers(html: &str) -> Vec<String> {
    // Strip HTML tags for cleaner text matching
    let text = strip_tags(html);

    let mut phones = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Simple state-machine approach to find phone-number-like sequences
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Look for sequences that could be phone numbers
        if chars[i] == '(' || chars[i] == '+' || chars[i].is_ascii_digit() {
            let start = i;
            let mut digits = String::new();
            let mut j = i;

            // Collect up to 20 chars that look phone-number-ish
            while j < len && j - start < 20 {
                if chars[j].is_ascii_digit() {
                    digits.push(chars[j]);
                } else if chars[j] == '('
                    || chars[j] == ')'
                    || chars[j] == '-'
                    || chars[j] == '.'
                    || chars[j] == ' '
                    || chars[j] == '+'
                {
                    // These are valid phone number separators
                } else {
                    break;
                }
                j += 1;
            }

            // A US phone number has 10 digits (or 11 with country code)
            if digits.len() == 10 || (digits.len() == 11 && digits.starts_with('1')) {
                let formatted = text[start..j].trim().to_string();
                if !formatted.is_empty() && seen.insert(digits.clone()) {
                    phones.push(formatted);
                }
            }

            i = j;
        } else {
            i += 1;
        }
    }

    phones
}

/// Extract email addresses from HTML text.
fn extract_emails(html: &str) -> Vec<String> {
    let text = strip_tags(html);
    let mut emails = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Split on whitespace and check for email patterns
    for word in
        text.split(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '<' || c == '>')
    {
        let word = word.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '@' && c != '.' && c != '-' && c != '_' && c != '+'
        });
        if word.contains('@') && word.contains('.') {
            // Basic email validation
            let parts: Vec<&str> = word.split('@').collect();
            if parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.') {
                let domain_parts: Vec<&str> = parts[1].split('.').collect();
                if domain_parts.len() >= 2
                    && domain_parts.last().map_or(false, |tld| tld.len() >= 2)
                {
                    let email = word.to_lowercase();
                    // Skip common false positives
                    if !email.ends_with(".png")
                        && !email.ends_with(".jpg")
                        && !email.ends_with(".gif")
                        && !email.ends_with(".css")
                        && !email.ends_with(".js")
                        && !email.contains("example.com")
                        && !email.contains("sentry.io")
                    {
                        if seen.insert(email.clone()) {
                            emails.push(email);
                        }
                    }
                }
            }
        }
    }

    emails
}

/// Extract social media profile URLs from page HTML.
fn extract_social_links(html: &str) -> serde_json::Map<String, serde_json::Value> {
    let mut links = serde_json::Map::new();

    let social_patterns = [
        ("facebook", "facebook.com/"),
        ("instagram", "instagram.com/"),
        ("twitter", "twitter.com/"),
        ("twitter", "x.com/"),
        ("youtube", "youtube.com/"),
        ("linkedin", "linkedin.com/"),
        ("yelp", "yelp.com/biz/"),
        ("google_business", "google.com/maps/place/"),
        ("nextdoor", "nextdoor.com/"),
    ];

    let lower = html.to_lowercase();

    for (key, pattern) in &social_patterns {
        if links.contains_key(*key) {
            continue;
        }
        if let Some(pos) = lower.find(pattern) {
            // Walk backwards to find the start of the URL
            let mut url_start = pos;
            while url_start > 0 {
                let c = html.as_bytes()[url_start - 1];
                if c == b'"' || c == b'\'' || c == b' ' || c == b'>' {
                    break;
                }
                url_start -= 1;
            }
            // Walk forward to find the end
            let mut url_end = pos + pattern.len();
            while url_end < html.len() {
                let c = html.as_bytes()[url_end];
                if c == b'"' || c == b'\'' || c == b' ' || c == b'<' || c == b')' {
                    break;
                }
                url_end += 1;
            }
            let url = html[url_start..url_end].trim().to_string();
            if url.starts_with("http") {
                links.insert(key.to_string(), json!(url));
            }
        }
    }

    links
}

/// Try to find a US street address pattern in the HTML.
/// Looks for patterns like "123 Main St, City, ST 12345".
fn extract_address_pattern(html: &str) -> Option<String> {
    let text = strip_tags(html);

    // Look for ZIP code patterns and try to extract surrounding address context
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for i in 0..len.saturating_sub(5) {
        // Look for 5-digit ZIP code that follows a 2-letter state abbreviation
        if chars[i].is_ascii_digit()
            && i + 4 < len
            && chars[i + 1].is_ascii_digit()
            && chars[i + 2].is_ascii_digit()
            && chars[i + 3].is_ascii_digit()
            && chars[i + 4].is_ascii_digit()
        {
            // Check for state abbreviation before ZIP
            if i >= 4 {
                let before: String = chars[i.saturating_sub(4)..i].iter().collect();
                let before = before.trim();
                // 2-letter state abbreviation
                if before.len() >= 2 {
                    let st: String = before
                        .chars()
                        .rev()
                        .take(2)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    if st.chars().all(|c| c.is_ascii_uppercase()) {
                        // Walk backwards to get the full address line (up to 100 chars)
                        let start = i.saturating_sub(100);
                        let end = std::cmp::min(i + 5, len);
                        let candidate: String = chars[start..end].iter().collect();
                        // Take the last line/sentence that contains the address
                        if let Some(line) = candidate.lines().last() {
                            let addr = line.trim().to_string();
                            if addr.len() > 10 {
                                return Some(addr);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extract a star rating from common patterns in HTML.
fn extract_rating(html: &str) -> Option<f64> {
    let text = strip_tags(html);

    // Pattern: "4.8 stars", "4.8/5", "Rating: 4.8"
    for window in text.as_bytes().windows(20) {
        let s = String::from_utf8_lossy(window);
        // Look for a decimal number followed by rating indicators
        for word in s.split_whitespace() {
            if let Ok(val) = word.trim_end_matches('/').parse::<f64>() {
                if (1.0..=5.0).contains(&val)
                    && (s.contains("star") || s.contains("/5") || s.contains("rating"))
                {
                    return Some(val);
                }
            }
        }
    }

    None
}

/// Extract review count from common patterns.
fn extract_review_count(html: &str) -> Option<u32> {
    let text = strip_tags(html);

    // Pattern: "123 reviews", "(123 reviews)", "123 Google reviews"
    let words: Vec<&str> = text.split_whitespace().collect();
    for i in 0..words.len().saturating_sub(1) {
        if words[i + 1].to_lowercase().contains("review") {
            let num_str = words[i].trim_matches(|c: char| !c.is_ascii_digit());
            if let Ok(count) = num_str.parse::<u32>() {
                if count > 0 && count < 100_000 {
                    return Some(count);
                }
            }
        }
    }

    None
}

/// Strip HTML tags, returning plain text.
///
/// Operates on raw bytes and reconstructs a valid UTF-8 string via
/// `String::from_utf8_lossy` so that multi-byte characters in business names,
/// addresses, and titles are preserved rather than replaced with garbage.
fn strip_tags(html: &str) -> String {
    let mut result: Vec<u8> = Vec::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let lower = html.to_lowercase();
    let bytes = html.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        if !in_tag && i + 7 < bytes.len() && &lower[i..i + 7] == "<script" {
            in_script = true;
            in_tag = true;
        } else if in_script && i + 9 <= bytes.len() && &lower[i..i + 9] == "</script>" {
            in_script = false;
            i += 9;
            continue;
        } else if !in_tag && i + 6 < bytes.len() && &lower[i..i + 6] == "<style" {
            in_style = true;
            in_tag = true;
        } else if in_style && i + 8 <= bytes.len() && &lower[i..i + 8] == "</style>" {
            in_style = false;
            i += 8;
            continue;
        } else if bytes[i] == b'<' {
            in_tag = true;
        } else if bytes[i] == b'>' {
            in_tag = false;
            if !in_script && !in_style {
                result.push(b' ');
            }
        } else if !in_tag && !in_script && !in_style {
            result.push(bytes[i]);
        }
        i += 1;
    }

    String::from_utf8_lossy(&result).into_owned()
}

/// Basic HTML entity decoding.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
}
