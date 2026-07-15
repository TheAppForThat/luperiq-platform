//! Template body generation and AI prompts for the SEO Page Generator.
//!
//! Template mode produces professional HTML with silo-linked content.
//! AI mode sends structured prompts that produce unique, keyword-rich pages.
//!
//! Industry-agnostic: uses `TemplateContext` to adapt terminology for any
//! industry (pest control, HVAC, plumbing, restaurant, etc.).

use super::{PageKind, PlannedPage, SeoPhoto};

/// Context struct that adapts template language for any industry.
pub(crate) struct TemplateContext {
    pub industry_name: String,
    pub item_singular: String,
    pub item_plural: String,
    pub service_verb: String,
    pub city_hub_prefix: String,
}

// ── Template mode body generation ───────────────────────────────────

/// Generate the body_json for a page in template mode.
pub(crate) fn template_body(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> String {
    serde_json::to_string(&template_blocks(page, brand, phone, state_abbr, ctx))
        .unwrap_or_else(|_| "[]".to_string())
}

fn template_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    match page.kind {
        PageKind::ItemHub => item_hub_blocks(page, brand, phone, ctx),
        PageKind::CityHub => city_hub_blocks(page, brand, phone, state_abbr, ctx),
        PageKind::ItemCity => item_city_blocks(page, brand, phone, state_abbr, ctx),
        PageKind::CategoryHub => category_hub_blocks(page, brand, phone, ctx),
        PageKind::CategoryCity => category_city_blocks(page, brand, phone, state_abbr, ctx),
    }
}

fn item_hub_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    let brand_display = display_brand(brand);
    let verb_cap = capitalize(&ctx.service_verb);
    let item_name = page
        .title
        .strip_suffix(&format!(" {}", verb_cap))
        .unwrap_or(&page.title)
        .to_string();
    let item_lower = item_name.to_lowercase();
    let mut blocks = vec![
        marketing_hero(
            &ctx.industry_name,
            &page.title,
            &format!(
                "{brand_display} helps property owners handle {item_lower} issues with a clear plan, practical communication, and follow-through that keeps problems from cycling back."
            ),
            vec![
                "Local guidance".to_string(),
                "Clear next steps".to_string(),
                format!("{verb_cap} plans built for the property"),
            ],
            vec![
                action("Request Service", "/contact", "primary"),
                action("View Service Areas", "#service-areas", "outline"),
            ],
        ),
        feature_grid(
            3,
            vec![
                feature_item(
                    "Inspection first",
                    &format!(
                        "We start by identifying the scope, pressure points, and repeat triggers behind {item_lower} complaints."
                    ),
                    "",
                ),
                feature_item(
                    "Targeted work",
                    &format!(
                        "Our team matches the response to the real issue instead of treating everything the same way."
                    ),
                    "",
                ),
                feature_item(
                    "Prevention built in",
                    &format!(
                        "Recommendations focus on reducing return visits, surprises, and emergency calls."
                    ),
                    "",
                ),
            ],
        ),
        heading(2, &format!("How to spot {} issues early", item_name)),
        paragraph(&format!(
            "{item_name} trouble often starts small. Catching the early signs usually means less disruption, lower repair risk, and a better plan for keeping the property in shape."
        )),
        list(
            false,
            vec![
                format!("Visible signs that point to active {item_lower} pressure"),
                "Areas where customers or staff keep noticing the same problem".to_string(),
                "Seasonal or weather-related flare-ups that signal a predictable pattern".to_string(),
                "Damage, contamination, or nuisance issues that suggest the source has spread".to_string(),
            ],
        ),
        heading(2, &format!("Why professional {} matters", page.focus_keyword)),
        paragraph(&format!(
            "DIY efforts often knock back the visible problem without addressing why it showed up in the first place. {brand_display} focuses on the source, the entry conditions, and the follow-up plan so the work holds."
        )),
        heading(2, &format!("Our {} process", page.focus_keyword)),
        list(
            true,
            vec![
                "Consultation and property review".to_string(),
                "A treatment or service plan matched to the issue".to_string(),
                "Work performed with clear expectations and timing".to_string(),
                "Prevention notes for the owner or staff".to_string(),
                "Recommended follow-up when the situation calls for it".to_string(),
            ],
        ),
        note_box(
            "info",
            "What makes these pages useful",
            "The goal is not just to describe the problem. It is to help visitors understand the warning signs, the process, and what a trustworthy service experience looks like.",
        ),
        heading(2, &format!("Common questions about {}", item_name)),
        accordion(vec![
            faq(
                &format!("When should we call about {}?", item_lower),
                &format!(
                    "Call when the issue is repeating, spreading, or starting to affect comfort, cleanliness, safety, or operations. Early service usually gives you more options."
                ),
            ),
            faq(
                &format!("Do you only handle one-time {} jobs?", item_lower),
                "No. Some customers need a one-time response, while others need recurring protection, monitoring, or seasonal follow-up.",
            ),
            faq(
                &format!("Will you explain the plan before starting {}?", item_lower),
                "Yes. A good service call should explain what was found, what will be done, and what the next steps are without making the customer guess.",
            ),
        ]),
    ];

    if !page.related_slugs.is_empty() {
        blocks.push(heading(2, &format!("{} service areas", item_name)));
        blocks.push(feature_grid(
            3,
            page.related_slugs
                .iter()
                .take(6)
                .map(|slug| {
                    let city_label = humanize_slug(slug)
                        .replace(&format!(" {}", capitalize(&ctx.service_verb)), "")
                        .trim()
                        .to_string();
                    feature_item(
                        &city_label,
                        &format!(
                            "See how {brand_display} approaches {item_lower} work in this service area."
                        ),
                        &format!("/{slug}"),
                    )
                })
                .collect(),
        ));
    }

    blocks.push(contact_cta(phone));
    blocks
}

fn city_hub_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    let brand_display = display_brand(brand);
    let city = page
        .title
        .strip_prefix(&format!("{} in ", ctx.industry_name))
        .unwrap_or(&page.title)
        .trim_end_matches(&format!(", {}", state_abbr))
        .to_string();
    let industry_lower = ctx.industry_name.to_lowercase();
    let mut blocks = vec![
        marketing_hero(
            &ctx.industry_name,
            &page.title,
            &format!(
                "{brand_display} serves {city} with practical {industry_lower} support designed around local schedules, property types, and the kind of issues residents actually run into."
            ),
            vec![
                format!("{city} coverage"),
                "Residential and commercial".to_string(),
                "Clear service process".to_string(),
            ],
            vec![
                action("Request Service", "/contact", "primary"),
                action("Explore Services", "#city-services", "outline"),
            ],
        ),
        feature_grid(
            3,
            vec![
                feature_item(
                    "Local knowledge",
                    &format!(
                        "Neighborhood patterns, weather swings, and property mix all influence the work we recommend in {city}."
                    ),
                    "",
                ),
                feature_item(
                    "Fast communication",
                    "Customers should know what to expect, when to expect it, and what happens after the first visit.",
                    "",
                ),
                feature_item(
                    "Built for repeat business",
                    "The best local pages show how ongoing maintenance, follow-up, and trust signals work together.",
                    "",
                ),
            ],
        ),
        heading(2, &format!("Why {} properties need a local plan", city)),
        paragraph(&format!(
            "A good city page should feel specific to the area instead of sounding like a copy pasted service blurb. {brand_display} uses the page to explain what matters locally, how service is delivered, and what customers can expect next."
        )),
        heading(2, "How service typically works"),
        list(
            true,
            vec![
                "Initial review of the issue and property context".to_string(),
                "A recommendation that fits the property and the level of urgency".to_string(),
                "Clear explanation of timing, follow-up, and what happens after the visit".to_string(),
            ],
        ),
        heading(2, &format!("Questions we hear from customers in {}", city)),
        accordion(vec![
            faq(
                &format!("Do you serve all of {}?", city),
                "If the property is in the normal coverage area, the page should make that clear and give customers an easy next step if they are unsure.",
            ),
            faq(
                &format!("Can you help both homes and businesses in {}?", city),
                "Yes. The page should explain the difference between residential and commercial needs without making the visitor dig for it.",
            ),
            faq(
                &format!("What makes your {} service different here?", industry_lower),
                "Consistency, clear expectations, and a service plan that matches the property are usually more persuasive than hype.",
            ),
        ]),
    ];

    if !page.related_slugs.is_empty() {
        blocks.push(heading(
            2,
            &format!("{} services in {}", ctx.industry_name, city),
        ));
        blocks.push(feature_grid(
            3,
            page.related_slugs
                .iter()
                .take(6)
                .map(|slug| {
                    let label = humanize_slug(slug)
                        .trim_end_matches(&format!(
                            " {}",
                            humanize_slug(&city.to_lowercase().replace(' ', "-"))
                        ))
                        .to_string();
                    feature_item(
                        &label,
                        &format!("See the city-specific service page for {city}."),
                        &format!("/{slug}"),
                    )
                })
                .collect(),
        ));
    }

    // Phase 7 / 2026-05-27 — approved field photos for this city.
    append_seo_photo_blocks(&mut blocks, page, brand);

    blocks.push(contact_cta(phone));
    blocks
}

fn item_city_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    let brand_display = display_brand(brand);
    let verb_cap = capitalize(&ctx.service_verb);
    let parts: Vec<&str> = page
        .title
        .splitn(2, &format!(" {} in ", verb_cap))
        .collect();
    let item_name = parts.first().copied().unwrap_or("Service").to_string();
    let city = parts
        .get(1)
        .copied()
        .unwrap_or("")
        .trim_end_matches(&format!(", {}", state_abbr))
        .to_string();
    let item_lower = item_name.to_lowercase();

    let mut blocks = vec![
        marketing_hero(
            &ctx.industry_name,
            &page.title,
            &format!(
                "{brand_display} handles {item_lower} work in {city} with a clear local page, useful warning signs, and an easy path into service."
            ),
            vec![
                format!("{city} focus"),
                format!("{} expertise", item_name),
                "Built for conversion".to_string(),
            ],
            vec![
                action("Request Service", "/contact", "primary"),
                action("See Local Coverage", &format!("/{}", page.related_slugs.first().cloned().unwrap_or_default()), "outline"),
            ],
        ),
        heading(2, &format!("What to watch for in {}", city)),
        paragraph(&format!(
            "City-specific pages work best when they explain the issue in plain language, connect it to local conditions, and show visitors why a timely response matters."
        )),
        list(
            false,
            vec![
                format!("Visible signs of {item_lower} pressure"),
                "Repeat complaints in the same area of the property".to_string(),
                "Operational or comfort problems caused by the issue".to_string(),
                "Concerns about safety, cleanliness, or customer experience".to_string(),
            ],
        ),
        heading(2, &format!("Our {} process in {}", item_name, city)),
        list(
            true,
            vec![
                "Assess the issue and the property context".to_string(),
                "Match the service plan to the real conditions on site".to_string(),
                "Complete the work and explain next steps".to_string(),
                "Recommend any follow-up needed to keep the issue from cycling back".to_string(),
            ],
        ),
        feature_grid(
            3,
            vec![
                feature_item(
                    "Residential work",
                    "Explain what homeowners usually care about most: timing, clarity, disruption, and prevention.",
                    "",
                ),
                feature_item(
                    "Commercial work",
                    "Show that service can be planned around customers, staff, and operating hours.",
                    "",
                ),
                feature_item(
                    "Long-term prevention",
                    "Build trust by describing what happens after the first visit, not just the initial response.",
                    "",
                ),
            ],
        ),
        heading(2, &format!("Common questions about {} in {}", item_name, city)),
        accordion(vec![
            faq(
                &format!("How quickly should we deal with {} in {}?", item_lower, city),
                "As soon as it starts affecting comfort, operations, or risk. Local pages should make urgency clear without sounding exaggerated.",
            ),
            faq(
                &format!("Will you explain what is causing the {} issue?", item_lower),
                "Yes. Customers should leave the visit understanding what was found, what changed, and what to watch next.",
            ),
            faq(
                &format!("Do you offer follow-up after {} service?", item_lower),
                "When follow-up makes sense, the page should explain how it works and what customers can expect.",
            ),
        ]),
    ];

    let mut related_cards = Vec::new();
    if let Some(parent) = &page.parent_slug {
        related_cards.push(feature_item(
            &format!("All {}", item_name),
            &format!("View the broader {} page.", item_name),
            &format!("/{parent}"),
        ));
    }
    for slug in &page.related_slugs {
        related_cards.push(feature_item(
            &humanize_slug(slug),
            "Explore a closely related local page in the same silo.",
            &format!("/{slug}"),
        ));
    }
    if !related_cards.is_empty() {
        blocks.push(heading(2, "Keep exploring"));
        blocks.push(feature_grid(3, related_cards.into_iter().take(6).collect()));
    }

    // Phase 7 / 2026-05-27 — drop in approved field photos from the SEO
    // Photo Library. Skipped silently when the library is empty.
    append_seo_photo_blocks(&mut blocks, page, brand);

    blocks.push(contact_cta(phone));
    blocks
}

fn category_hub_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    let brand_display = display_brand(brand);
    let verb_cap = capitalize(&ctx.service_verb);
    let category = page
        .title
        .strip_suffix(&format!(" {}", verb_cap))
        .unwrap_or(&page.title)
        .to_string();
    let category_lower = category.to_lowercase();
    vec![
        marketing_hero(
            &ctx.industry_name,
            &page.title,
            &format!(
                "{brand_display} covers {category_lower} work with structured service plans, better education for the customer, and clear paths into the next step."
            ),
            vec![
                "Category hub".to_string(),
                "Useful support content".to_string(),
                "Built for internal linking".to_string(),
            ],
            vec![
                action("Request Service", "/contact", "primary"),
                action("Read Related Pages", "#related-pages", "outline"),
            ],
        ),
        heading(2, &format!("What falls under {}", category)),
        paragraph(&format!(
            "Category hubs help visitors understand the group of services before they choose a specific page. That makes them useful for both search and actual customers."
        )),
        list(
            false,
            vec![
                format!("Common {} problems or service requests", category_lower),
                "What property owners usually notice first".to_string(),
                "How technicians prioritize inspection and treatment".to_string(),
            ],
        ),
        note_box(
            "info",
            "Why this page exists",
            "Category pages keep the silo organized and create a stronger internal-linking path into the more specific local pages.",
        ),
        contact_cta(phone),
    ]
}

fn category_city_blocks(
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> Vec<serde_json::Value> {
    let brand_display = display_brand(brand);
    let verb_cap = capitalize(&ctx.service_verb);
    let parts: Vec<&str> = page
        .title
        .splitn(2, &format!(" {} in ", verb_cap))
        .collect();
    let category = parts.first().copied().unwrap_or("Service").to_string();
    let city = parts
        .get(1)
        .copied()
        .unwrap_or("")
        .trim_end_matches(&format!(", {}", state_abbr))
        .to_string();
    vec![
        marketing_hero(
            &ctx.industry_name,
            &page.title,
            &format!(
                "{brand_display} uses this page to explain how {category} work shows up in {city}, what customers should watch for, and how to move into service."
            ),
            vec![
                city.clone(),
                category.clone(),
                "Service-ready content".to_string(),
            ],
            vec![
                action("Request Service", "/contact", "primary"),
                action("Call Our Team", "/contact", "outline"),
            ],
        ),
        heading(2, &format!("Why {} matters in {}", category, city)),
        paragraph("These location-specific pages perform better when they answer practical buying questions, not just generic information queries."),
        heading(2, "What visitors should find here"),
        list(
            false,
            vec![
                "A clear explanation of the local service need".to_string(),
                "A simple process for starting service".to_string(),
                "Links to broader hubs and related pages".to_string(),
            ],
        ),
        contact_cta(phone),
    ]
}

pub(crate) fn normalize_ai_body(
    raw: &str,
    page: &PlannedPage,
    brand: &str,
    phone: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> String {
    let cleaned = strip_code_fences(raw);
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&cleaned) {
        if let Some(arr) = value.as_array() {
            if !arr.is_empty() && arr.iter().all(|block| block.get("type").is_some()) {
                return serde_json::to_string(arr)
                    .unwrap_or_else(|_| template_body(page, brand, phone, state_abbr, ctx));
            }
        } else if let Some(arr) = value.get("blocks").and_then(|v| v.as_array()) {
            if !arr.is_empty() && arr.iter().all(|block| block.get("type").is_some()) {
                return serde_json::to_string(arr)
                    .unwrap_or_else(|_| template_body(page, brand, phone, state_abbr, ctx));
            }
        }
    }
    template_body(page, brand, phone, state_abbr, ctx)
}

fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim().to_string();
    }
    trimmed.to_string()
}

fn marketing_hero(
    kicker: &str,
    title: &str,
    text: &str,
    chips: Vec<String>,
    actions: Vec<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({
        "type": "marketing-hero",
        "data": {
            "theme": "blue",
            "kicker": kicker,
            "title": title,
            "text": text,
            "chips": chips,
            "actions": actions,
        }
    })
}

fn feature_grid(columns: u64, items: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "type": "feature-grid",
        "data": {
            "columns": columns,
            "items": items,
        }
    })
}

fn feature_item(title: &str, text: &str, url: &str) -> serde_json::Value {
    serde_json::json!({
        "eyebrow": "",
        "title": title,
        "text": text,
        "url": url,
    })
}

fn heading(level: u64, text: &str) -> serde_json::Value {
    serde_json::json!({"type": "heading", "data": {"level": level, "text": text}})
}

fn paragraph(text: &str) -> serde_json::Value {
    serde_json::json!({"type": "paragraph", "data": {"text": text}})
}

fn list(ordered: bool, items: Vec<String>) -> serde_json::Value {
    serde_json::json!({"type": "list", "data": {"ordered": ordered, "items": items}})
}

fn note_box(tone: &str, title: &str, text: &str) -> serde_json::Value {
    serde_json::json!({"type": "note-box", "data": {"tone": tone, "title": title, "text": text}})
}

fn accordion(items: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({"type": "accordion", "data": {"items": items}})
}

fn faq(title: &str, content: &str) -> serde_json::Value {
    serde_json::json!({"title": title, "content": content})
}

fn action(label: &str, url: &str, style: &str) -> serde_json::Value {
    serde_json::json!({"label": label, "url": url, "style": style})
}

fn button(text: &str, url: &str, style: &str) -> serde_json::Value {
    serde_json::json!({"type": "button", "data": {"text": text, "url": url, "style": style, "alignment": "center"}})
}

fn contact_cta(phone: &str) -> serde_json::Value {
    if phone.trim().is_empty() {
        button("Contact Us", "/contact", "primary")
    } else {
        button(&format!("Call {phone}"), "/contact", "primary")
    }
}

fn display_brand(brand: &str) -> &str {
    if brand.trim().is_empty() {
        "our team"
    } else {
        brand
    }
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|part| !part.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

// ── AI mode prompts ─────────────────────────────────────────────────

pub(crate) fn ai_system_prompt(ctx: &TemplateContext) -> &'static str {
    // The system prompt is industry-agnostic enough to use a static string.
    // The user prompt provides all the industry-specific context.
    let _ = ctx;
    r#"You are an expert SEO copywriter and structured content builder for local service businesses.

Requirements:
- Output ONLY a valid JSON array of LuperIQ page-editor blocks
- Do not output HTML, markdown, prose explanations, or code fences
- Allowed block types: marketing-hero, heading, paragraph, list, feature-grid, note-box, accordion, button
- The first block must be a marketing-hero block
- Include 4-6 heading blocks with level 2 for the major sections
- Write 800-1200 words of natural, helpful, detailed copy across the paragraph, list, note-box, and accordion content
- Include the focus keyword naturally 5-8 times throughout the content
- Include at least one clear CTA using a button block
- Use list blocks for processes, signs, benefits, and service features
- Include one accordion block with 3-4 practical FAQ items
- Add specific, factual details (service methods, seasonal patterns, safety considerations)
- Write as if you are the business, using "we" and "our"
- Make content locally relevant when a city is specified — mention local landmarks, climate, neighborhoods, or regional patterns
- Include trust signals: years of experience, licensing, guarantees, free consultations
- Do NOT use lorem ipsum or placeholder text
- Do NOT start multiple paragraphs the same way — vary your sentence structure
- Keep every block valid JSON with a top-level shape like {"type":"paragraph","data":{"text":"..."}}"#
}

pub(crate) fn ai_user_prompt(
    page: &PlannedPage,
    brand: &str,
    state_abbr: &str,
    ctx: &TemplateContext,
) -> String {
    let brand_str = if brand.is_empty() {
        "the business"
    } else {
        brand
    };
    let state_str = if state_abbr.is_empty() {
        ""
    } else {
        state_abbr
    };
    let industry = &ctx.industry_name;
    let industry_lower = industry.to_lowercase();
    let item_singular = &ctx.item_singular;
    let verb = &ctx.service_verb;

    let context = match page.kind {
        PageKind::ItemHub => format!(
            "This is a {item_singular} hub page for {brand_str} ({industry}). Focus keyword: \"{kw}\".\n\
            Write a comprehensive guide about this specific {item_singular}. Include:\n\
            - How to identify this {item_singular} issue (signs, symptoms, common indicators)\n\
            - Common problems that property owners miss\n\
            - Risks and potential damage if left unaddressed\n\
            - Our step-by-step {verb} process (consultation, plan, service, follow-up)\n\
            - Prevention tips between professional visits\n\
            - A FAQ section with 3-4 questions property owners commonly ask about this {item_singular}\n\
            - Why DIY approaches fail and professional {verb} is worth the investment\n\
            - A strong call-to-action for a free consultation",
            brand_str = brand_str,
            industry = industry,
            item_singular = item_singular,
            verb = verb,
            kw = page.focus_keyword,
        ),
        PageKind::CityHub => format!(
            "This is a city landing page for {brand_str} ({industry}) in {state_str}. Focus keyword: \"{kw}\".\n\
            Write about {industry_lower} services in this specific area. Include:\n\
            - Why this city/area has unique {industry_lower} challenges (climate, geography, development patterns)\n\
            - The most common {industry_lower} needs residents face and seasonal patterns\n\
            - Our service coverage and response times in this area\n\
            - What makes us the trusted local choice (years serving the area, local knowledge)\n\
            - Our service process from initial call to ongoing protection\n\
            - Residential and commercial service differences\n\
            - A FAQ section about {industry_lower} in this area\n\
            - A strong call-to-action mentioning the specific city name",
            brand_str = brand_str,
            industry = industry,
            industry_lower = industry_lower,
            state_str = state_str,
            kw = page.focus_keyword,
        ),
        PageKind::ItemCity => format!(
            "This is a cross-product page (specific {item_singular} + specific city) for {brand_str} ({industry}). \
            Focus keyword: \"{kw}\".\n\
            Write a detailed, locally-relevant page about this specific {item_singular} {verb} in this specific location. Include:\n\
            - Why this {item_singular} is particularly important in this area (local climate, terrain, building styles)\n\
            - Local seasonal patterns for this {item_singular}\n\
            - Specific neighborhoods or property types most affected\n\
            - Our {verb} approach tailored to local conditions\n\
            - Service options with expected timelines and results\n\
            - Prevention strategies specific to this area\n\
            - A FAQ section with 3-4 location-specific questions\n\
            - A call-to-action with the city name and free consultation offer",
            brand_str = brand_str,
            industry = industry,
            item_singular = item_singular,
            verb = verb,
            kw = page.focus_keyword,
        ),
        PageKind::CategoryHub => format!(
            "This is a category hub page for {brand_str} ({industry}). Focus keyword: \"{kw}\".\n\
            Write about this entire category and our {verb} services. Include:\n\
            - Overview of {item_singular} types in this category and what they have in common\n\
            - Why this category requires professional {verb}\n\
            - Our comprehensive approach to this category\n\
            - Common signs that indicate issues in this category\n\
            - Risks associated with this category\n\
            - A FAQ section about this category\n\
            - A call-to-action for professional assessment",
            brand_str = brand_str,
            industry = industry,
            item_singular = item_singular,
            verb = verb,
            kw = page.focus_keyword,
        ),
        PageKind::CategoryCity => format!(
            "This is a category x city page for {brand_str} ({industry}). Focus keyword: \"{kw}\".\n\
            Write about this category of {industry_lower} services in this specific area. Include:\n\
            - Which {item_singular} types in this category are most common in this area and why\n\
            - Local environmental factors related to this category\n\
            - Our {verb} services for this category in this location\n\
            - Seasonal considerations for this area\n\
            - Prevention and ongoing protection plans\n\
            - A FAQ section with locally relevant questions\n\
            - A call-to-action mentioning both the category and city",
            brand_str = brand_str,
            industry = industry,
            industry_lower = industry_lower,
            item_singular = item_singular,
            verb = verb,
            kw = page.focus_keyword,
        ),
    };

    let mut silo_context = String::new();
    if let Some(parent) = &page.parent_slug {
        silo_context.push_str(&format!(
            "\nInclude an internal link to the parent hub: <a href=\"/{}\">parent hub page</a>.",
            parent
        ));
    }
    if !page.related_slugs.is_empty() {
        silo_context.push_str("\nInclude internal links to related pages:");
        for slug in &page.related_slugs {
            silo_context.push_str(&format!(" /{}", slug));
        }
    }

    // Include content sources so the AI uses real data, not guesses.
    // Supports three tiers: LuperIQ verified facts, customer-specific facts,
    // and raw reference material.
    let fact_section = if page.sources.is_empty() {
        String::new()
    } else {
        let mut parts = String::new();
        parts.push_str(&format!(
            "\n\nIMPORTANT — Use the following factual data about this {} in your content. \
            Do NOT make up details; incorporate these real facts naturally:",
            item_singular
        ));

        if !page.sources.luperiq_facts.is_empty() {
            parts.push_str("\n\n--- LuperIQ Verified Facts ---\n");
            parts.push_str(&page.sources.luperiq_facts);
        }

        if !page.sources.customer_facts.is_empty() {
            parts.push_str("\n\n--- Business-Specific Information ---\n");
            parts.push_str(&page.sources.customer_facts);
        }

        if !page.sources.raw_reference.is_empty() {
            parts.push_str("\n\n--- Additional Reference Material ---\n");
            parts.push_str(&page.sources.raw_reference);
        }

        // When both LuperIQ and customer facts exist, tell AI to prefer customer's
        if !page.sources.luperiq_facts.is_empty() && !page.sources.customer_facts.is_empty() {
            parts.push_str(
                "\n\nWhen facts conflict, prefer the Business-Specific Information \
                as the business owner has verified these details.",
            );
        }

        parts
    };

    let location_section = if page.location_context.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nIMPORTANT — Use the following location intelligence to write highly specific, \
            locally-relevant content. Weave in local search terms naturally, acknowledge the \
            local service landscape, and reflect authentic local patterns:\n\n{}",
            page.location_context
        )
    };

    format!(
        "Write the page content for:\n\
        Title: {title}\n\
        URL slug: /{slug}\n\
        Focus keyword: {kw}\n\
        Meta description: {meta}\n\n\
        {context}\n\
        {silo}{facts}{location}",
        title = page.title,
        slug = page.slug,
        kw = page.focus_keyword,
        meta = page.meta_description,
        context = context,
        silo = silo_context,
        facts = fact_section,
        location = location_section,
    )
}


// ── SEO Photo Library integration (Phase 7 / 2026-05-27) ────────────

/// Push an image gallery + JSON-LD `ImageObject` schema for the photos
/// stamped on `page.seo_photos`. Skipped silently when the list is empty.
///
/// Block layout (matches the existing block-editor schema):
///   1. heading: "Local photos from our team"
///   2. paragraph: brand context line tying the photos to the page topic
///   3. one `image` block per photo, with `alt` derived from caption/notes
///   4. custom-html block containing the JSON-LD `ImageObject` schema
fn append_seo_photo_blocks(
    blocks: &mut Vec<serde_json::Value>,
    page: &PlannedPage,
    brand: &str,
) {
    // SeoPhoto is intentionally referenced here so the type is treated as
    // used by the dead-code analyzer.
    let _phantom: Option<&SeoPhoto> = page.seo_photos.first();
    if page.seo_photos.is_empty() {
        return;
    }
    let brand_display = display_brand(brand);
    blocks.push(heading(2, "Local photos from our team"));
    blocks.push(paragraph(&format!(
        "{brand_display} captures these photos on real jobs in the field. Each one shows actual work or conditions our team has handled."
    )));

    let mut schema_objects: Vec<serde_json::Value> = Vec::with_capacity(page.seo_photos.len());
    for ph in &page.seo_photos {
        let alt = if ph.alt.is_empty() {
            page.title.clone()
        } else {
            ph.alt.clone()
        };
        blocks.push(serde_json::json!({
            "type": "image",
            "data": {
                "url": ph.image_url,
                "alt": alt,
            }
        }));

        let mut obj = serde_json::json!({
            "@type": "ImageObject",
            "contentUrl": ph.image_url,
            "description": alt,
        });
        if let Some(zip) = &ph.location_zip {
            obj["contentLocation"] = serde_json::json!({
                "@type": "Place",
                "address": {
                    "@type": "PostalAddress",
                    "postalCode": zip,
                }
            });
        }
        if let Some(pest) = &ph.pest_type {
            obj["keywords"] = serde_json::Value::String(pest.clone());
        }
        schema_objects.push(obj);
    }

    let schema = serde_json::json!({
        "@context": "https://schema.org",
        "@graph": schema_objects,
    });
    let schema_str = serde_json::to_string(&schema).unwrap_or_default();
    blocks.push(serde_json::json!({
        "type": "custom-html",
        "data": {
            "html": format!("<script type=\"application/ld+json\">{}</script>", schema_str),
        }
    }));
}

