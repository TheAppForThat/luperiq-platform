//! Built-in default site type definitions.
//!
//! These seed the catalog on first run. After that, the WAL versions are
//! authoritative and can be edited from the admin UI.

use super::presets;
use super::types::*;

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Process-wide memoized catalog. The default site-type definitions are pure
/// compile-time data (palettes, presets, nav) that never change at runtime, but
/// rebuilding them runs generate_presets for ~50 industries (heavy serde_json
/// construction) -- multiple seconds of CPU. Hot paths call this per request
/// (e.g. company-hero rendering) under the shared apex journal lock, which
/// serialized every apex homepage behind a full catalog rebuild. Build once.
fn cached_defaults() -> &'static Vec<SiteTypeDefinition> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Vec<SiteTypeDefinition>> = OnceLock::new();
    CACHE.get_or_init(build_all_defaults)
}

/// Zero-allocation accessor for read-only callers that only look up a
/// definition by slug (.iter().find(...)). Returns the memoized slice
/// directly -- no per-call rebuild, no clone.
pub fn all_defaults_ref() -> &'static [SiteTypeDefinition] {
    cached_defaults().as_slice()
}

/// Owned copy of the default catalog. Backed by the memoized build, so callers
/// that need an owned Vec (seed/reseed bins, mutation, WAL fallbacks) pay only
/// a clone of pre-built structures -- never the full rebuild. Prefer
/// all_defaults_ref for read-only lookups.
pub fn all_defaults() -> Vec<SiteTypeDefinition> {
    cached_defaults().clone()
}

fn build_all_defaults() -> Vec<SiteTypeDefinition> {
    vec![
        // Free / Group types
        family(),
        band(),
        roommates(),
        classroom(),
        homeschool(),
        sports_team(),
        club(),
        book_club(),
        nonprofit(),
        neighborhood(),
        travel(),
        elder_care(),
        wedding(),
        pet_owners(),
        scouts(),
        fitness(),
        farm(),
        support_group(),
        maker_space(),
        church(),
        small_group(),
        mission_team(),
        homeschool_coop(),
        business_team(),
        reunion(),
        memorial(),
        // Free / Creator types
        creator(),
        blog(),
        // Business types
        pest_control(),
        hvac(),
        plumbing(),
        electrical(),
        landscaping(),
        mobile_field_service(),
        restaurant(),
        bakery(),
        coffee_shop(),
        salon(),
        artisan_market(),
        cell_phone_repair(),
        electronics_repair(),
        auto_repair(),
        medical(),
        attorney(),
        accountant(),
        insurance(),
        app_publisher(),
    ]
}

fn family() -> SiteTypeDefinition {
    let ts = now();
    SiteTypeDefinition {
        slug: "family".into(),
        name: "LuperIQ Family".into(),
        emoji: "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}\u{200D}\u{1F466}".into(),
        category: "free".into(),
        description: "Private, ad-free family website. Shared calendar, chore board, recipes, shopping, inventory, feed, games, and vault.".into(),
        default_tagline: "Our family. One place. Always connected.".into(),
        publicly_listed: true,
        display_order: 1,
        enabled_modules: {
            let mut mods = group_core_modules();
            // Safe coexistence slice: add the new inventory and vault engines
            // without removing the legacy pantry/vault systems yet.
            push_unique_modules(&mut mods, &["family-inventory", "vault"]);
            mods
        },
        theme_presets: presets::generate_presets(&presets::palette_for("family"), "family"),
        theme_profile: Some(serde_json::json!({
            "tokens": {
                "primary": "#16a34a",
                "accent": "#f59e0b",
                "link": "#16a34a",
                "button_text": "#ffffff",
                "header_bg": "#fefce8",
                "header_text": "#422006",
                "background": "#fffbeb",
                "surface": "#ffffff",
                "text": "#422006",
                "radius": 20,
                "container": 1000,
                "brand_size": 48,
                "nav_size": 15,
                "body_size": 16,
                "body_font": "Humanist",
                "full_bleed": true
            },
            "header": {
                "enabled": true,
                "sticky": false
            },
            "footer": {
                "enabled": false
            }
        })),
        default_nav_items: with_reward_bank_nav(vec![
            NavItem { item_id: "home".into(), title: "Home".into(), url: "/".into(), emoji: "\u{1F3E0}".into(), position: 0, children: vec![] },
            NavItem { item_id: "chore-board".into(), title: "Chore Board".into(), url: "/chore-board".into(), emoji: "\u{2705}".into(), position: 1, children: vec![] },
            NavItem { item_id: "calendar".into(), title: "Calendar".into(), url: "/calendar".into(), emoji: "\u{1F4C5}".into(), position: 2, children: vec![] },
            NavItem { item_id: "recipes".into(), title: "Recipes".into(), url: "/recipes".into(), emoji: "\u{1F373}".into(), position: 3, children: vec![] },
            NavItem { item_id: "feed".into(), title: "Feed".into(), url: "/feed".into(), emoji: "\u{1F4F0}".into(), position: 4, children: vec![] },
            NavItem { item_id: "games".into(), title: "Games".into(), url: "/games".into(), emoji: "\u{1F3AE}".into(), position: 5, children: vec![] },
            NavItem { item_id: "vault".into(), title: "Vault".into(), url: "/vault".into(), emoji: "\u{1F512}".into(), position: 6, children: vec![] },
            NavItem { item_id: "shopping".into(), title: "Shopping".into(), url: "/shopping".into(), emoji: "\u{1F6D2}".into(), position: 7, children: vec![] },
            NavItem { item_id: "inventory".into(), title: "Inventory".into(), url: "/inventory".into(), emoji: "\u{1F4E6}".into(), position: 8, children: vec![] },
            NavItem { item_id: "messages".into(), title: "Messages".into(), url: "/messages".into(), emoji: "\u{1F4AC}".into(), position: 9, children: vec![] },
        ]),
        default_pages: vec![
            DefaultPage { slug: "chore-board".into(), title: "Chore Board".into(), blocks: None, seo_title: "{family} Chore Board".into(), seo_description: "Track chores, earn points, and stay on top of household tasks.".into() },
            DefaultPage { slug: "calendar".into(), title: "Calendar".into(), blocks: None, seo_title: "{family} Calendar".into(), seo_description: "Shared family calendar for events, practices, and appointments.".into() },
            DefaultPage { slug: "recipes".into(), title: "Recipes".into(), blocks: None, seo_title: "{family} Recipes".into(), seo_description: "Our family recipe collection.".into() },
            DefaultPage { slug: "feed".into(), title: "Feed".into(), blocks: None, seo_title: "{family} Feed".into(), seo_description: "Family updates, photos, and shared moments.".into() },
            DefaultPage { slug: "games".into(), title: "Games".into(), blocks: None, seo_title: "{family} Games".into(), seo_description: "Family trivia, challenges, and fun.".into() },
            DefaultPage { slug: "vault".into(), title: "Vault".into(), blocks: None, seo_title: "{family} Vault".into(), seo_description: "Private family documents and notes.".into() },
            DefaultPage { slug: "shopping".into(), title: "Shopping".into(), blocks: None, seo_title: "{family} Shopping".into(), seo_description: "Shared shopping lists.".into() },
            DefaultPage { slug: "inventory".into(), title: "Inventory".into(), blocks: None, seo_title: "{family} Inventory".into(), seo_description: "Keep track of what your family already has on hand.".into() },
            DefaultPage { slug: "vacation-planner".into(), title: "Vacation Planner".into(), blocks: None, seo_title: "{family} Vacation Planner".into(), seo_description: "Plan family trips and vacations.".into() },
            DefaultPage { slug: "homeschool".into(), title: "Homeschool".into(), blocks: None, seo_title: "{family} Homeschool".into(), seo_description: "Track lessons, progress, and curriculum.".into() },
        ],
        homepage_blocks: Some(serde_json::json!([
            { "type": "company-hero", "data": { "headline": "{family_name}", "description": "{tagline}", "show_cta": false, "logo_url": "/static/img/luperiq-family-icon-150.png" } },
            { "type": "roster-grid", "data": { "columns": 3, "show_role": true, "show_bio": true, "link_to_profile": true, "title": "Our Family" } },
            { "type": "family-quick-access", "data": { "title": "Quick Access", "subtitle": "Jump to what you need" } },
            { "type": "family-welcome", "data": { "title": "Today's Snapshot" } }
        ])),
        default_tone: "friendly".into(),
        default_brand_colors: BrandColors { primary: "#16a34a".into(), secondary: "#f59e0b".into(), accent: "#22c55e".into() },
        default_tier: "founding".into(),
        always_free: true,
        price_override_cents: 0,
        discount_codes: vec![],
        limited_time_offer: Some(LimitedOffer {
            label: "Free during early release".into(),
            description: "7-day free trial, no card required.".into(),
            expires_at: 0,
            tier_override: "founding".into(),
        }),
        onboarding_steps: vec![
            OnboardingStep {
                step_id: "family-info".into(),
                label: "Family Info".into(),
                skippable: false,
                fields: vec![
                    OnboardingField { key: "family_name".into(), label: "Family Name".into(), field_type: "text".into(), placeholder: "The Smith Family".into(), required: true, options: vec![], help_text: String::new(), admin_notes: String::new() },
                    OnboardingField { key: "family_photo_url".into(), label: "Family Hero Photo".into(), field_type: "image".into(), placeholder: "Optional hero photo for the homepage".into(), required: false, options: vec![], help_text: "Upload or paste a photo URL if the family wants the homepage to feel personal right away.".into(), admin_notes: String::new() },
                    OnboardingField { key: "family_members".into(), label: "Family Members".into(), field_type: "family_members".into(), placeholder: "Add names, roles, birthdays, emails, photos, and optional login setup.".into(), required: false, options: vec![], help_text: "Create member profiles during onboarding. Passwords are optional; members can also receive setup emails.".into(), admin_notes: String::new() },
                    OnboardingField { key: "members".into(), label: "Family Members".into(), field_type: "textarea".into(), placeholder: "One name per line".into(), required: false, options: vec![], help_text: "Fallback simple list for older onboarding screens.".into(), admin_notes: String::new() },
                    OnboardingField { key: "send_member_invites".into(), label: "Email setup links to members with emails".into(), field_type: "toggle".into(), placeholder: String::new(), required: false, options: vec![], help_text: "Send password setup links to family members who have email addresses and no password entered above.".into(), admin_notes: String::new() },
                ],
            },
            OnboardingStep {
                step_id: "family-rhythm".into(),
                label: "Household Rhythm".into(),
                skippable: true,
                fields: vec![
                    field_select(
                        "life_stage",
                        "Current Family Season",
                        false,
                        &[
                            "Young Kids",
                            "School Age",
                            "Teens",
                            "College / Launching",
                            "Multigenerational",
                            "Empty Nest / Grandparents",
                            "Mixed Ages",
                        ],
                    ),
                    field_grid(
                        "family_focus",
                        "What should this site help with most?",
                        false,
                        &[
                            "Shared calendar",
                            "Meals & recipes",
                            "Chores & routines",
                            "School / homeschool",
                            "Trips & travel",
                            "Photos & memories",
                            "Private documents",
                            "Connected families",
                        ],
                    ),
                    field_select(
                        "planning_style",
                        "How do you usually plan life together?",
                        false,
                        &["Day by day", "Weekly reset", "Monthly calendar", "Seasonal / event based"],
                    ),
                ],
            },
            OnboardingStep {
                step_id: "interests".into(),
                label: "Interests".into(),
                skippable: true,
                fields: vec![
                    OnboardingField { key: "interests".into(), label: "Family Interests".into(), field_type: "checkbox_grid".into(), placeholder: "".into(), required: false, options: vec!["Cooking".into(), "Sports".into(), "Music".into(), "Gaming".into(), "Outdoors".into(), "Travel".into(), "Movies".into(), "Reading".into(), "Crafts".into(), "Pets".into()], help_text: String::new(), admin_notes: String::new() },
                ],
            },
            OnboardingStep {
                step_id: "family-sharing".into(),
                label: "Sharing & Privacy".into(),
                skippable: true,
                fields: vec![
                    field_toggle("share_with_extended_family", "Allow sharing with connected families"),
                    field_toggle("private_family_space", "Keep some areas household-only"),
                    field_toggle("kid_friendly_homepage", "Highlight kid-friendly shortcuts on the homepage"),
                ],
            },
        ],
        seo_title_template: "{family_name} — {tagline}".into(),
        seo_description_template: "{family_name} private family website. Shared calendar, chore board, recipes, and more.".into(),
        created_at: ts,
        updated_at: ts,
    }
}

fn business_base(
    slug: &str,
    name: &str,
    emoji: &str,
    desc: &str,
    order: u32,
) -> SiteTypeDefinition {
    let ts = now();
    let base_modules = vec![
        "smtp",
        "forms",
        "messaging",
        "theme-studio",
        "seo",
        "booking",
        "site-blueprint",
        "technicians",
        // state-license-lookup is enabled by provisioning for the
        // field-trade group (scripts/provision-site.sh:239:
        // pest-control|hvac|plumbing|electrical|landscaping|*-repair), ALWAYS
        // alongside technicians, on which it hard-depends
        // (luperiq-mod-state-license-lookup dependencies()=["technicians"]).
        // It lives in business_base so the field trades inherit it, and the
        // per-industry `retain` blocks below DROP it together with technicians
        // for non-field-service industries (restaurant, attorney, accountant,
        // …) — keeping state-license-lookup co-occurrent with technicians so
        // the model stays dependency-closed everywhere. The pest free-tier
        // drop (blueprint.rs) drops it alongside technicians as well.
        "state-license-lookup",
        "inspections",
        "customer-portal",
        "invoicing",
        "dashboard",
        "dashboard-themes",
        "jobs",
        "notifications",
        "industry-profile",
        "location-profile",
        "company-profile",
        "availability",
        "field-ops",
        "tech-portal",
        "site-pages",
        "financing",
        "content-pipeline",
        "page-generator",
        "email-marketing",
        "customer-journey",
        "ab-testing",
        "help-feedback",
        "onboarding",
        "media-manager",
        "module-manager",
        "commerce",
        "service-catalog",
        "fleet",
        "contracts",
        "tech-utilization",
        "office-portal",
        "property-data",
    ];
    SiteTypeDefinition {
        slug: slug.into(),
        name: name.into(),
        emoji: emoji.into(),
        category: "business".into(),
        description: desc.into(),
        default_tagline: format!(
            "Clear {} help, easy booking, and follow-up in one place.",
            name.to_lowercase()
        ),
        publicly_listed: true,
        display_order: order,
        enabled_modules: base_modules.into_iter().map(String::from).collect(),
        theme_presets: presets::generate_presets(&presets::palette_for(slug), slug),
        theme_profile: None,
        default_nav_items: vec![
            NavItem {
                item_id: "home".into(),
                title: "Home".into(),
                url: "/".into(),
                emoji: "\u{1F3E0}".into(),
                position: 0,
                children: vec![],
            },
            NavItem {
                item_id: "services".into(),
                title: "Services".into(),
                url: "/services".into(),
                emoji: "\u{1F527}".into(),
                position: 1,
                children: vec![],
            },
            NavItem {
                item_id: "about".into(),
                title: "About".into(),
                url: "/about".into(),
                emoji: "\u{1F464}".into(),
                position: 2,
                children: vec![],
            },
            NavItem {
                item_id: "contact".into(),
                title: "Contact".into(),
                url: "/contact".into(),
                emoji: "\u{1F4DE}".into(),
                position: 3,
                children: vec![],
            },
            NavItem {
                item_id: "book".into(),
                title: "Book Now".into(),
                url: "/book".into(),
                emoji: "\u{1F4C5}".into(),
                position: 4,
                children: vec![],
            },
        ],
        default_pages: vec![
            DefaultPage {
                slug: "home".into(),
                title: "{business_name}".into(),
                blocks: None,
                seo_title: "{business_name} — {tagline}".into(),
                seo_description: "{description}".into(),
            },
            DefaultPage {
                slug: "services".into(),
                title: "Our Services".into(),
                blocks: None,
                seo_title: "{business_name} Services".into(),
                seo_description: "Explore services from {business_name}, compare options, and choose the right next step for booking, estimates, or follow-up.".into(),
            },
            DefaultPage {
                slug: "about".into(),
                title: "About Us".into(),
                blocks: None,
                seo_title: "About {business_name}".into(),
                seo_description: "Learn how {business_name} works, what customers can expect, and how the team handles service requests.".into(),
            },
            DefaultPage {
                slug: "contact".into(),
                title: "Contact Us".into(),
                blocks: None,
                seo_title: "Contact {business_name}".into(),
                seo_description: "Contact {business_name} to ask a question, request service, or confirm whether the team can help in your area.".into(),
            },
        ],
        homepage_blocks: Some(serde_json::json!([
            { "type": "company-hero", "data": { "headline": "{business_name}", "description": "{tagline}", "show_cta": true, "cta_text": "Book Now", "cta_url": "/book" } },
            { "type": "service-grid", "data": { "title": "Our Services" } },
            { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
            { "type": "cta-section", "data": { "title": "Ready to Get Started?", "cta_text": "Book Now", "cta_url": "/book" } }
        ])),
        default_tone: "professional".into(),
        default_brand_colors: BrandColors {
            primary: "#2563eb".into(),
            secondary: "#1e40af".into(),
            accent: "#3b82f6".into(),
        },
        default_tier: "pro-monthly".into(),
        always_free: false,
        price_override_cents: 0,
        discount_codes: vec![
            DiscountCode {
                code: "EARLYBIRD".into(),
                discount_type: "percent".into(),
                value: 20,
                expires_at: 0,
                max_uses: 500,
                uses: 0,
            },
            DiscountCode {
                code: "LAUNCH50".into(),
                discount_type: "percent".into(),
                value: 50,
                expires_at: 0,
                max_uses: 100,
                uses: 0,
            },
        ],
        limited_time_offer: Some(LimitedOffer {
            label: "Early Release Pricing".into(),
            description: "Lock in today's price forever. This price will never go higher for you."
                .into(),
            expires_at: 0,
            tier_override: "pro-lifetime".into(),
        }),
        onboarding_steps: common_biz_steps(),
        seo_title_template: "{business_name} — {tagline}".into(),
        seo_description_template:
            "{business_name}: professional {industry} services. Book online today.".into(),
        created_at: ts,
        updated_at: ts,
    }
}

fn theme(
    primary: &str,
    accent: &str,
    hdr_bg: &str,
    hdr_txt: &str,
    bg: &str,
    txt: &str,
    font: &str,
    radius: u32,
) -> serde_json::Value {
    serde_json::json!({
        "tokens": {
            "primary": primary, "accent": accent, "link": primary,
            "button_text": "#ffffff", "header_bg": hdr_bg, "header_text": hdr_txt,
            "background": bg, "surface": "#ffffff", "text": txt,
            "radius": radius, "container": 1100, "brand_size": 36, "nav_size": 15,
            "body_size": 16, "body_font": font, "full_bleed": true
        },
        "header": { "enabled": true, "sticky": true },
        "footer": { "enabled": true }
    })
}

/// Common onboarding steps shared by every business-type site.
fn common_biz_steps() -> Vec<OnboardingStep> {
    vec![
        OnboardingStep {
            step_id: "welcome".into(),
            label: "Welcome".into(),
            skippable: false,
            fields: vec![
                OnboardingField {
                    key: "business_name".into(),
                    label: "Business Name".into(),
                    field_type: "text".into(),
                    placeholder: "Your business name".into(),
                    required: true,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
                OnboardingField {
                    key: "tagline".into(),
                    label: "Tagline".into(),
                    field_type: "text".into(),
                    placeholder: "A short tagline for your site".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
            ],
        },
        OnboardingStep {
            step_id: "contact".into(),
            label: "Contact Info".into(),
            skippable: false,
            fields: vec![
                OnboardingField {
                    key: "address".into(),
                    label: "Business Address".into(),
                    field_type: "text".into(),
                    placeholder: "Street address".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
                OnboardingField {
                    key: "phone".into(),
                    label: "Phone".into(),
                    field_type: "text".into(),
                    placeholder: "(555) 123-4567".into(),
                    required: true,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
                OnboardingField {
                    key: "email".into(),
                    label: "Email".into(),
                    field_type: "text".into(),
                    placeholder: "info@yourbusiness.com".into(),
                    required: true,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
            ],
        },
        OnboardingStep {
            step_id: "service_area".into(),
            label: "Service Area".into(),
            skippable: true,
            fields: vec![
                OnboardingField {
                    key: "cities".into(),
                    label: "Cities Served".into(),
                    field_type: "text".into(),
                    placeholder: "Dallas, Fort Worth, Arlington...".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
                OnboardingField {
                    key: "radius".into(),
                    label: "Service Radius (miles)".into(),
                    field_type: "text".into(),
                    placeholder: "25".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
            ],
        },
        OnboardingStep {
            step_id: "team".into(),
            label: "Your Team".into(),
            skippable: true,
            fields: vec![
                OnboardingField {
                    key: "team_member_name".into(),
                    label: "Name".into(),
                    field_type: "text".into(),
                    placeholder: "Team member name".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
                OnboardingField {
                    key: "team_member_title".into(),
                    label: "Title".into(),
                    field_type: "text".into(),
                    placeholder: "Job title".into(),
                    required: false,
                    options: vec![],
                    help_text: String::new(),
                    admin_notes: String::new(),
                },
            ],
        },
    ]
}

fn tune_location_staff_steps(
    steps: &mut [OnboardingStep],
    location_label: &str,
    cities_label: &str,
    cities_placeholder: &str,
    radius_label: &str,
    staff_label: &str,
    staff_name_placeholder: &str,
    staff_role_placeholder: &str,
) {
    for step in steps {
        match step.step_id.as_str() {
            "service_area" => {
                step.label = location_label.into();
                for field in &mut step.fields {
                    match field.key.as_str() {
                        "cities" => {
                            field.label = cities_label.into();
                            field.placeholder = cities_placeholder.into();
                        }
                        "radius" => {
                            field.label = radius_label.into();
                            field.placeholder = "5".into();
                        }
                        _ => {}
                    }
                }
            }
            "team" => {
                step.label = staff_label.into();
                for field in &mut step.fields {
                    match field.key.as_str() {
                        "team_member_name" => {
                            field.label = "Staff Member Name".into();
                            field.placeholder = staff_name_placeholder.into();
                        }
                        "team_member_title" => {
                            field.label = "Role".into();
                            field.placeholder = staff_role_placeholder.into();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn step(id: &str, label: &str, skippable: bool, fields: Vec<OnboardingField>) -> OnboardingStep {
    OnboardingStep {
        step_id: id.into(),
        label: label.into(),
        skippable,
        fields,
    }
}

fn field_text(key: &str, label: &str, required: bool, placeholder: &str) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "text".into(),
        placeholder: placeholder.into(),
        required,
        options: vec![],
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn field_textarea(key: &str, label: &str, required: bool, placeholder: &str) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "textarea".into(),
        placeholder: placeholder.into(),
        required,
        options: vec![],
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn field_grid(key: &str, label: &str, required: bool, options: &[&str]) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "checkbox_grid".into(),
        placeholder: String::new(),
        required,
        options: options.iter().map(|s| (*s).into()).collect(),
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn field_grid_values(
    key: &str,
    label: &str,
    required: bool,
    options: Vec<String>,
) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "checkbox_grid".into(),
        placeholder: String::new(),
        required,
        options,
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn field_toggle(key: &str, label: &str) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "toggle".into(),
        placeholder: "Yes".into(),
        required: false,
        options: vec![],
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn field_select(key: &str, label: &str, required: bool, options: &[&str]) -> OnboardingField {
    OnboardingField {
        key: key.into(),
        label: label.into(),
        field_type: "select".into(),
        placeholder: String::new(),
        required,
        options: options.iter().map(|s| (*s).into()).collect(),
        help_text: String::new(),
        admin_notes: String::new(),
    }
}

fn pest_control() -> SiteTypeDefinition {
    let mut d = business_base(
        "pest-control",
        "Pest Control",
        "\u{1F41B}",
        "Full pest control suite with EPA chemical library, compliance tracking, and treatment rotation.",
        10,
    );
    d.enabled_modules.push("pest-control".into());
    d.default_brand_colors = BrandColors {
        primary: "#16a34a".into(),
        secondary: "#15803d".into(),
        accent: "#22c55e".into(),
    };
    d.theme_profile = Some(theme(
        "#16a34a",
        "#22c55e",
        "#14532d",
        "#ffffff",
        "#f0fdf4",
        "#14532d",
        "Geometric Humanist",
        8,
    ));
    d.default_tagline = "Protecting your home from unwanted pests.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Local pest experts protecting your home and business. Licensed, insured, and trusted by your neighbors.",
            "show_cta": true,
            "cta_text": "Book a Pest Inspection",
            "cta_url": "/book-a-pest-control-appointment"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Get Started?",
            "cta_text": "Book a Pest Inspection",
            "cta_url": "/book-a-pest-control-appointment"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "Services Offered".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "General Pest Control".into(),
                "Termite Treatment".into(),
                "Rodent Control".into(),
                "Mosquito Treatment".into(),
                "Bed Bug Treatment".into(),
                "Wildlife Removal".into(),
                "Lawn & Ornamental".into(),
                "Commercial Pest Control".into(),
            ],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "compliance".into(),
        label: "Licensing & Compliance".into(),
        skippable: true,
        fields: vec![
            OnboardingField {
                key: "license_number".into(),
                label: "License Number".into(),
                field_type: "text".into(),
                placeholder: "State license #".into(),
                required: false,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "chemical_tracking".into(),
                label: "Enable Chemical Tracking".into(),
                field_type: "checkbox_grid".into(),
                placeholder: "".into(),
                required: false,
                options: vec!["Yes".into(), "No".into()],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    });
    d.onboarding_steps = steps;
    d.default_pages.push(page_with_blocks(
        "pests",
        "Common Pests",
        "Common Pests | {business_name}",
        "Learn about common pest issues, treatment timing, and when to request pest control from {business_name}.",
        serde_json::json!([
            { "type": "marketing-hero", "data": {
                "kicker": "Pest Control",
                "title": "Common Pests",
                "text": "Learn the warning signs for common pest problems and find out when it's time to schedule a professional inspection.",
                "theme": "site",
                "chips": ["Identification help", "Practical next steps", "Treatment guidance"],
                "actions": [
                    { "label": "Book an Inspection", "url": "/book-a-pest-control-appointment", "style": "primary" },
                    { "label": "See Services", "url": "/pest-control-services", "style": "outline" }
                ]
            }},
            { "type": "feature-grid", "data": {
                "columns": 3,
                "items": [
                    { "title": "Ants", "text": "Watch for visible trails, mound activity, and recurring indoor sightings before the problem spreads.", "url": "", "eyebrow": "" },
                    { "title": "Cockroaches", "text": "Night activity, droppings, and hidden moisture often point to a larger problem nearby.", "url": "", "eyebrow": "" },
                    { "title": "Spiders", "text": "Frequent sightings and web patterns can mean a larger prey problem around the property.", "url": "", "eyebrow": "" },
                    { "title": "Termites", "text": "Mud tubes, damaged wood, and swarm activity are worth checking early.", "url": "", "eyebrow": "" },
                    { "title": "Rodents", "text": "Noise, droppings, gnaw marks, and contamination concerns usually need a full plan.", "url": "", "eyebrow": "" },
                    { "title": "Mosquitoes", "text": "Standing water and shaded outdoor areas can turn a yard into a repeat problem fast.", "url": "", "eyebrow": "" }
                ]
            }},
            { "type": "heading", "data": { "level": 2, "text": "When to call" } },
            { "type": "accordion", "data": { "items": [
                { "title": "Should we try DIY first?", "content": "Small visible issues may look simple, but repeated activity or hidden spread usually deserves a professional inspection." },
                { "title": "What makes an infestation urgent?", "content": "Escalating activity, health concerns, contamination, or property damage are the signs to move quickly." },
                { "title": "What should happen next?", "content": "We'll review our findings with you, explain your treatment options, and schedule follow-up visits until the problem is fully resolved." }
            ]}},
            { "type": "button", "data": { "alignment": "center", "style": "primary", "text": "Schedule an Inspection", "url": "/book-a-pest-control-appointment" } }
        ]),
    ));
    d.default_pages.push(page_with_blocks(
        "reviews",
        "Reviews",
        "Reviews for {business_name}",
        "See what customers notice about {business_name}, and learn how to share real pest-control feedback after service.",
        serde_json::json!([
            { "type": "marketing-hero", "data": {
                "kicker": "Pest Control",
                "title": "Reviews",
                "text": "Read what our customers say about response time, communication, care around your property, and follow-up after treatment.",
                "theme": "site",
                "chips": ["Response time", "Clear communication", "Follow-up"],
                "actions": [
                    { "label": "Book a Pest Inspection", "url": "/book-a-pest-control-appointment", "style": "primary" },
                    { "label": "Contact Us", "url": "/contact-pest-control", "style": "outline" }
                ]
            }},
            { "type": "heading", "data": { "level": 2, "text": "What to look for in reviews" } },
            { "type": "feature-grid", "data": {
                "columns": 3,
                "items": [
                    { "title": "Did the technician explain the plan?", "text": "Good pest-control reviews often mention whether the visit, chemicals, safety notes, and follow-up plan were easy to understand.", "url": "", "eyebrow": "" },
                    { "title": "Was the property treated carefully?", "text": "Customers should know whether the team respected children, pets, landscaping, food areas, and entry points around the property.", "url": "", "eyebrow": "" },
                    { "title": "Did the issue improve?", "text": "The most useful feedback describes the pest problem, what changed after service, and whether repeat activity was handled clearly.", "url": "", "eyebrow": "" }
                ]
            }},
            { "type": "paragraph", "data": { "text": "Not sure what to expect? These reviews give you an honest look at how we work — from the initial inspection and treatment to follow-up service and results." } },
            { "type": "button", "data": { "alignment": "center", "style": "primary", "text": "Book a Pest Inspection", "url": "/book-a-pest-control-appointment" } }
        ]),
    ));
    // Remap generic page slugs to industry-specific ones for SEO.
    for page in &mut d.default_pages {
        match page.slug.as_str() {
            "services" => {
                page.slug = "pest-control-services".into();
                page.seo_title = "{business_name} Pest Control Services".into();
            }
            "about" => {
                page.slug = "about-pest-control".into();
                page.seo_title = "About {business_name} | Pest Control".into();
            }
            "contact" => {
                page.slug = "contact-pest-control".into();
                page.seo_title = "Contact {business_name} | Pest Control".into();
            }
            _ => {}
        }
    }
    // Add a pest-control blog page.
    d.default_pages.push(DefaultPage {
        slug: "pest-control-blog".into(),
        title: "Pest Control Tips & Guides".into(),
        blocks: None,
        seo_title: "Pest Control Tips & Guides | {business_name}".into(),
        seo_description: "Expert pest control advice from {business_name}. Learn about seasonal pests, treatment methods, and how to keep your home protected year-round.".into(),
    });
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/pest-control-services".into(),
            emoji: "\u{1F6E1}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "pests".into(),
            title: "Pests".into(),
            url: "/pests".into(),
            emoji: "\u{1F41C}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Service Areas".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about-pest-control".into(),
            emoji: "\u{1F41B}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact-pest-control".into(),
            emoji: "\u{1F4DE}".into(),
            position: 6,
            children: vec![],
        },
        NavItem {
            item_id: "blog".into(),
            title: "Blog".into(),
            url: "/pest-control-blog".into(),
            emoji: "\u{1F4DD}".into(),
            position: 7,
            children: vec![],
        },
    ];
    d
}

fn hvac() -> SiteTypeDefinition {
    let mut d = business_base(
        "hvac",
        "HVAC",
        "\u{2744}\u{FE0F}",
        "Heating, ventilation, and air conditioning with equipment tracking and maintenance plans.",
        11,
    );
    d.enabled_modules.push("hvac".into());
    d.default_brand_colors = BrandColors {
        primary: "#0284c7".into(),
        secondary: "#0369a1".into(),
        accent: "#38bdf8".into(),
    };
    d.theme_profile = Some(theme(
        "#0284c7",
        "#38bdf8",
        "#0c4a6e",
        "#ffffff",
        "#f0f9ff",
        "#0c4a6e",
        "Transitional",
        10,
    ));
    d.default_tagline = "Keeping your home comfortable year-round.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Keep your home comfortable year-round. Installation, repair, and maintenance plans.",
            "show_cta": true,
            "cta_text": "Schedule Service",
            "cta_url": "/book"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Get Started?",
            "cta_text": "Schedule Service",
            "cta_url": "/book"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "HVAC Services".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "AC Installation".into(),
                "AC Repair".into(),
                "Heating Installation".into(),
                "Heating Repair".into(),
                "Duct Work".into(),
                "Indoor Air Quality".into(),
                "Commercial HVAC".into(),
                "Maintenance Plans".into(),
                "Emergency Service".into(),
            ],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "maintenance".into(),
        label: "Maintenance Plans".into(),
        skippable: true,
        fields: vec![
            OnboardingField {
                key: "maintenance_plans".into(),
                label: "Offer Maintenance Plans".into(),
                field_type: "checkbox_grid".into(),
                placeholder: "".into(),
                required: false,
                options: vec!["Yes".into(), "No".into()],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "emergency_service".into(),
                label: "24/7 Emergency Service".into(),
                field_type: "checkbox_grid".into(),
                placeholder: "".into(),
                required: false,
                options: vec!["Yes".into(), "No".into()],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    });
    d.onboarding_steps = steps;
    d.default_pages.push(page(
        "reviews",
        "Reviews",
        "Reviews for {business_name}",
        "Customer reviews and testimonials for {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{2744}\u{FE0F}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Service Areas".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F321}\u{FE0F}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn plumbing() -> SiteTypeDefinition {
    let mut d = business_base(
        "plumbing",
        "Plumbing",
        "\u{1F527}",
        "Plumbing services with parts inventory, service call management, and smart invoicing.",
        12,
    );
    d.enabled_modules.push("plumbing".into());
    d.default_brand_colors = BrandColors {
        primary: "#2563eb".into(),
        secondary: "#1d4ed8".into(),
        accent: "#60a5fa".into(),
    };
    d.theme_profile = Some(theme(
        "#2563eb",
        "#60a5fa",
        "#1e3a5f",
        "#ffffff",
        "#eff6ff",
        "#1e3a5f",
        "Geometric Humanist",
        8,
    ));
    d.default_tagline = "Reliable plumbing you can count on.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Fast, reliable plumbing for your home and business. Available for emergencies.",
            "show_cta": true,
            "cta_text": "Call Now",
            "cta_url": "/contact"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Get Started?",
            "cta_text": "Call Now",
            "cta_url": "/contact"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "Plumbing Services".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "Drain Cleaning".into(),
                "Water Heater".into(),
                "Pipe Repair".into(),
                "Sewer Line".into(),
                "Fixture Installation".into(),
                "Repiping".into(),
                "Gas Line".into(),
                "Commercial Plumbing".into(),
                "Emergency Service".into(),
            ],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "emergency".into(),
        label: "Emergency & Licensing".into(),
        skippable: true,
        fields: vec![
            OnboardingField {
                key: "emergency_service".into(),
                label: "24/7 Emergency Service".into(),
                field_type: "checkbox_grid".into(),
                placeholder: "".into(),
                required: false,
                options: vec!["Yes".into(), "No".into()],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "license_number".into(),
                label: "License Number".into(),
                field_type: "text".into(),
                placeholder: "State license #".into(),
                required: false,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    });
    d.onboarding_steps = steps;
    d.default_pages.push(page(
        "reviews",
        "Reviews",
        "Reviews for {business_name}",
        "Customer reviews and testimonials for {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1F527}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Service Areas".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F4A7}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn electrical() -> SiteTypeDefinition {
    let mut d = business_base(
        "electrical",
        "Electrical",
        "\u{26A1}",
        "Electrical services with code compliance tracking and inspection management.",
        13,
    );
    d.enabled_modules.push("electrical".into());
    d.default_brand_colors = BrandColors {
        primary: "#eab308".into(),
        secondary: "#ca8a04".into(),
        accent: "#facc15".into(),
    };
    d.theme_profile = Some(theme(
        "#ca8a04",
        "#facc15",
        "#1c1917",
        "#fef3c7",
        "#fffbeb",
        "#1c1917",
        "Industrial",
        6,
    ));
    d.default_tagline = "Licensed electricians you can trust.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Licensed electricians for residential and commercial projects. Safety first.",
            "show_cta": true,
            "cta_text": "Request Estimate",
            "cta_url": "/contact"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Get Started?",
            "cta_text": "Request Estimate",
            "cta_url": "/contact"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "Electrical Services".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "Residential Wiring".into(),
                "Commercial Electrical".into(),
                "Panel Upgrades".into(),
                "EV Charger Installation".into(),
                "Lighting".into(),
                "Generator Installation".into(),
                "Surge Protection".into(),
                "Code Compliance".into(),
                "Emergency Service".into(),
            ],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "licensing".into(),
        label: "Licensing & Specialties".into(),
        skippable: true,
        fields: vec![
            OnboardingField {
                key: "license_type".into(),
                label: "License Type".into(),
                field_type: "select".into(),
                placeholder: "".into(),
                required: false,
                options: vec![
                    "Journeyman".into(),
                    "Master Electrician".into(),
                    "Electrical Contractor".into(),
                ],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "license_number".into(),
                label: "License Number".into(),
                field_type: "text".into(),
                placeholder: "License #".into(),
                required: false,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    });
    d.onboarding_steps = steps;
    d.default_pages.push(page(
        "reviews",
        "Reviews",
        "Reviews for {business_name}",
        "Customer reviews and testimonials for {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{26A1}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Service Areas".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F50C}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn landscaping() -> SiteTypeDefinition {
    let mut d = business_base(
        "landscaping",
        "Landscaping",
        "\u{1F33F}",
        "Landscaping with seasonal planning, property management, and service area maps.",
        14,
    );
    d.enabled_modules.push("landscaping".into());
    d.default_brand_colors = BrandColors {
        primary: "#16a34a".into(),
        secondary: "#15803d".into(),
        accent: "#4ade80".into(),
    };
    d.theme_profile = Some(theme(
        "#15803d",
        "#4ade80",
        "#052e16",
        "#dcfce7",
        "#f0fdf4",
        "#052e16",
        "Neo-Grotesque",
        12,
    ));
    d.default_tagline = "Beautiful landscapes, expertly maintained.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Transform your outdoor space. Design, installation, and seasonal maintenance.",
            "show_cta": true,
            "cta_text": "Get Started",
            "cta_url": "/contact"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Get Started?",
            "cta_text": "Get Started",
            "cta_url": "/contact"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "Landscaping Services".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "Lawn Mowing".into(),
                "Landscape Design".into(),
                "Hardscaping".into(),
                "Irrigation".into(),
                "Tree Service".into(),
                "Mulching".into(),
                "Seasonal Cleanup".into(),
                "Snow Removal".into(),
                "Commercial Grounds".into(),
            ],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "seasonal".into(),
        label: "Seasonal Services".into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: "seasonal_services".into(),
            label: "Seasonal Availability".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: false,
            options: vec!["Yes".into(), "No".into()],
            help_text: String::new(),
            admin_notes: String::new(),
        }],
    });
    d.onboarding_steps = steps;
    d.default_pages.extend([
        page(
            "gallery",
            "Gallery",
            "{business_name} Gallery",
            "Browse recent work, project photos, and outdoor transformations from {business_name}.",
        ),
        page(
            "reviews",
            "Reviews",
            "Reviews for {business_name}",
            "Customer reviews and testimonials for {business_name}.",
        ),
    ]);
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1F333}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "gallery".into(),
            title: "Gallery".into(),
            url: "/gallery".into(),
            emoji: "\u{1F4F7}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F33F}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

/// Mobile / on-site field-service base — covers cleaning crews, locksmiths,
/// movers, handymen, mobile detailing/grooming, couriers, dog walkers, and
/// any other mobile service that doesn't fit a named industry. Inherits the
/// full Operations stack (Trucks, Day View, Reorders, Truck Map, customer
/// tracking) automatically via `business_base()`.
fn mobile_field_service() -> SiteTypeDefinition {
    let mut d = business_base(
        "mobile-field-service",
        "Mobile Service",
        "\u{1F69B}",
        "Mobile field-service business — cleaning, locksmith, moving, handyman, mobile detailing, courier, and any other on-site or delivery service.",
        15,
    );
    d.default_brand_colors = BrandColors {
        primary: "#0d9488".into(),
        secondary: "#0f766e".into(),
        accent: "#5eead4".into(),
    };
    d.theme_profile = Some(theme(
        "#0f766e",
        "#5eead4",
        "#042f2e",
        "#ccfbf1",
        "#f0fdfa",
        "#042f2e",
        "Geometric Humanist",
        10,
    ));
    d.default_tagline = "On-site service when and where you need it.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "We come to you. Call or book online — we'll be there on time.",
            "show_cta": true,
            "cta_text": "Book Online",
            "cta_url": "/booking"
        }},
        { "type": "service-grid", "data": { "title": "What We Do" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Schedule?",
            "cta_text": "Book Now",
            "cta_url": "/booking"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(OnboardingStep {
        step_id: "services".into(),
        label: "What you offer".into(),
        skippable: false,
        fields: vec![OnboardingField {
            key: "services".into(),
            label: "Services".into(),
            field_type: "checkbox_grid".into(),
            placeholder: "".into(),
            required: true,
            options: vec![
                "House Cleaning".into(),
                "Office Cleaning".into(),
                "Carpet Cleaning".into(),
                "Window Cleaning".into(),
                "Pressure Washing".into(),
                "Locksmith".into(),
                "Moving".into(),
                "Junk Removal".into(),
                "Handyman".into(),
                "Mobile Detailing".into(),
                "Mobile Grooming".into(),
                "Dog Walking".into(),
                "Courier / Delivery".into(),
                "Furniture Assembly".into(),
                "Yard Cleanup".into(),
                "Gutter Cleaning".into(),
                "Power Washing".into(),
                "Other".into(),
            ],
            help_text: "Pick everything you offer. You can edit this later.".into(),
            admin_notes: String::new(),
        }],
    });
    steps.push(OnboardingStep {
        step_id: "fleet".into(),
        label: "Vehicles & Crew".into(),
        skippable: true,
        fields: vec![
            OnboardingField {
                key: "vehicle_count".into(),
                label: "How many service vehicles?".into(),
                field_type: "text".into(),
                placeholder: "e.g. 3".into(),
                required: false,
                options: vec![],
                help_text: "Each vehicle becomes a Truck on the Trucks page so you can track inventory and location.".into(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "crew_size".into(),
                label: "How many techs/cleaners/drivers?".into(),
                field_type: "text".into(),
                placeholder: "e.g. 5".into(),
                required: false,
                options: vec![],
                help_text: "Each person becomes a Technician you can assign to jobs.".into(),
                admin_notes: String::new(),
            },
        ],
    });
    d.onboarding_steps = steps;
    d.default_pages.extend([
        page(
            "service-areas",
            "Service Areas",
            "{business_name} — Service Areas",
            "Cities and neighborhoods served by {business_name}.",
        ),
        page(
            "reviews",
            "Reviews",
            "Reviews for {business_name}",
            "Customer reviews and testimonials for {business_name}.",
        ),
    ]);
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1F69B}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "areas".into(),
            title: "Service Areas".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F5FA}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "booking".into(),
            title: "Book Now".into(),
            url: "/booking".into(),
            emoji: "\u{1F4C5}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn restaurant() -> SiteTypeDefinition {
    let mut d = business_base(
        "restaurant",
        "Restaurant & Food Service",
        "\u{1F37D}\u{FE0F}",
        "Food-business website with menus, ordering, reservations, table service, delivery, merch, staff scheduling, and kitchen operations.",
        15,
    );
    d.enabled_modules.retain(|module| {
        // Keep service-catalog because site-pages depends on it at
        // registration time. Drop invoicing because restaurants bill via
        // cart/checkout (and invoicing also depends on service-catalog).
        // (AUD-019)
        !matches!(
            module.as_str(),
            "technicians"
                | "state-license-lookup"
                | "tech-portal"
                | "field-ops"
                | "inspections"
                | "financing"
                | "invoicing"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["restaurant", "cart", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#dc2626".into(),
        secondary: "#b91c1c".into(),
        accent: "#f87171".into(),
    };
    d.theme_profile = Some(theme(
        "#b91c1c", "#f87171", "#1a0a0a", "#fecaca", "#fef2f2", "#1a0a0a", "Didone", 4,
    ));
    d.default_tagline = "Serve more guests with less chaos.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Menus, ordering, reservations, table service, delivery, merch, and guest follow-up in one place.",
            "show_cta": true,
            "cta_text": "View Our Menu",
            "cta_url": "/menu"
        }},
        { "type": "service-grid", "data": { "title": "Our Menu" } },
        { "type": "testimonials", "data": { "title": "What Our Guests Say" } },
        { "type": "cta-section", "data": {
            "title": "Join Us for a Meal",
            "cta_text": "View Our Menu",
            "cta_url": "/menu"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Location, Ordering & Service",
        "Locations / Delivery Areas",
        "Stockyards, downtown, food truck route, pickup window, delivery zone...",
        "Delivery Radius (miles)",
        "Staff & Roles",
        "Owner, chef, server, host, manager...",
        "Owner, chef, server, host, manager...",
    );
    steps.push(step(
        "food-business-model",
        "Food Business Model",
        false,
        vec![
            field_select(
                "business_kind",
                "What kind of food business is this?",
                true,
                &[
                    "Full-service restaurant",
                    "Fast casual / counter service",
                    "Food truck or trailer",
                    "Snow cone or seasonal stand",
                    "Coffee shop or cafe",
                    "Bakery or cupcake stand",
                    "Buffet",
                    "Catering or pop-up",
                    "Delivery-only kitchen",
                    "Hybrid food business",
                ],
            ),
            field_select(
                "operating_model",
                "How does it serve customers most days?",
                true,
                &[
                    "Fixed location",
                    "Mobile route",
                    "Events and pop-ups",
                    "Delivery only",
                    "Counter service",
                    "Table service",
                    "Hybrid",
                ],
            ),
            field_text(
                "location_label",
                "What should the site call the place people order from?",
                false,
                "restaurant, truck, stand, counter, cafe, kitchen...",
            ),
            field_textarea(
                "service_model_notes",
                "Anything unusual about how customers order, pickup, dine in, or find you?",
                false,
                "Example: parked at events Friday nights, pickup window only, buffet by day and catering by night...",
            ),
        ],
    ));
    steps.push(step(
        "menu",
        "Menu Setup",
        false,
        vec![
            field_grid(
                "menu_categories",
                "Menu Categories",
                true,
                &[
                    "Appetizers",
                    "Entrees",
                    "Salads",
                    "Soups",
                    "Sandwiches",
                    "Pizza",
                    "Coffee",
                    "Breakfast",
                    "Snow Cones",
                    "Cupcakes",
                    "Buffet",
                    "Seafood",
                    "Desserts",
                    "Drinks",
                    "Kids Menu",
                    "Specials",
                ],
            ),
            field_text(
                "cuisine_type",
                "Cuisine Type",
                false,
                "Italian, Mexican, American...",
            ),
        ],
    ));
    steps.push(step(
        "service-paths",
        "Ordering and Visit Paths",
        true,
        vec![
            field_toggle("reservations", "Accept Reservations"),
            field_toggle("delivery", "Offer Delivery"),
            field_toggle("takeout", "Offer Takeout"),
            field_toggle("table_service", "Allow table ordering or table requests"),
            field_toggle("counter_service", "Allow counter or window pickup"),
            field_toggle("merch", "Sell merch, sauces, gift items, or packaged goods"),
            field_toggle(
                "staff_scheduling",
                "Use staff schedule, time clock, tips, and payroll summaries",
            ),
            field_toggle(
                "kitchen_inventory",
                "Use recipes, inventory, food cost, and supplier reorder tools",
            ),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages = vec![
        page(
            "home",
            "{business_name}",
            "{business_name} — {tagline}",
            "{description}",
        ),
        page(
            "about",
            "About Us",
            "About {business_name}",
            "Meet {business_name}, a food business built around welcoming hospitality, ordering, reservations or visit planning, catering, merch, and guest-friendly follow-through.",
        ),
        page(
            "contact",
            "Contact Us",
            "Contact {business_name}",
            "Contact {business_name} for ordering help, reservations, mobile-location questions, catering, private events, merch, or guest support.",
        ),
        page(
            "catering",
            "Catering & Private Dining",
            "{business_name} Catering and Private Dining",
            "Plan catering, private dining, food-truck stops, pop-ups, large orders, and event meals with {business_name}.",
        ),
        page(
            "merch",
            "Merch",
            "{business_name} Merch",
            "Shop shirts, hats, sauces, gift items, and restaurant merchandise from {business_name}.",
        ),
        page(
            "social",
            "Social & Updates",
            "{business_name} Social Updates",
            "Follow specials, photos, events, announcements, and social updates from {business_name}.",
        ),
    ];
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "menu".into(),
            title: "Order Online".into(),
            url: "/menu".into(),
            emoji: "\u{1F37D}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "reservations".into(),
            title: "Reservations".into(),
            url: "/reservations".into(),
            emoji: "\u{1F4C5}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "catering".into(),
            title: "Catering".into(),
            url: "/catering".into(),
            emoji: "\u{1F370}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "merch".into(),
            title: "Merch".into(),
            url: "/merch".into(),
            emoji: "\u{1F9E2}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "blog".into(),
            title: "Blog".into(),
            url: "/blog".into(),
            emoji: "\u{1F4DD}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F468}\u{200D}\u{1F373}".into(),
            position: 6,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 7,
            children: vec![],
        },
    ];
    d
}

fn bakery() -> SiteTypeDefinition {
    let mut d = business_base(
        "bakery",
        "Bakery",
        "\u{1F9C1}",
        "Bakery with product showcase, daily specials, and online ordering.",
        16,
    );
    d.enabled_modules.retain(|module| {
        // Food businesses have menus, not "services" — drop the field-service
        // modules. Keep service-catalog because site-pages depends on it at
        // registration time (the dependency check in module-api panics if
        // service-catalog isn't present whenever site-pages is). Drop
        // invoicing because bakeries bill via cart/checkout, and invoicing
        // also depends on service-catalog. (AUD-019)
        !matches!(
            module.as_str(),
            "technicians"
                | "state-license-lookup"
                | "tech-portal"
                | "field-ops"
                | "inspections"
                | "availability"
                | "financing"
                | "jobs"
                | "invoicing"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["bakery", "cart", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#d97706".into(),
        secondary: "#b45309".into(),
        accent: "#fbbf24".into(),
    };
    d.theme_profile = Some(theme(
        "#b45309", "#fbbf24", "#451a03", "#fef3c7", "#fffbeb", "#451a03", "Humanist", 16,
    ));
    d.default_tagline = "Freshly baked, made with love.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Fresh-baked daily. Custom cakes, pastries, and bread made from scratch.",
            "show_cta": true,
            "cta_text": "Order Now",
            "cta_url": "/bakery/order"
        }},
        { "type": "service-grid", "data": { "title": "Our Baked Goods" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Order?",
            "cta_text": "Order Now",
            "cta_url": "/bakery/order"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Pickup, Delivery & Visits",
        "Pickup / Delivery Area",
        "Fort Worth, downtown, farmers markets, nearby neighborhoods...",
        "Delivery Radius (miles)",
        "Staff & Roles",
        "Owner, baker, counter staff, manager...",
        "Owner, head baker, decorator, manager...",
    );
    steps.push(step(
        "products",
        "Bakery Products",
        false,
        vec![
            field_grid(
                "product_categories",
                "Product Categories",
                true,
                &[
                    "Bread",
                    "Pastries",
                    "Cakes",
                    "Cookies",
                    "Pies",
                    "Cupcakes",
                    "Donuts",
                    "Gluten-Free",
                    "Custom Orders",
                ],
            ),
            field_toggle("custom_orders", "Accept Custom Orders"),
        ],
    ));
    steps.push(step(
        "allergens",
        "Allergen Info",
        true,
        vec![field_grid(
            "allergens",
            "Common Allergens",
            false,
            &["Nuts", "Dairy", "Gluten", "Eggs", "Soy"],
        )],
    ));
    d.onboarding_steps = steps;
    d.default_pages
        .retain(|page| !matches!(page.slug.as_str(), "services" | "about" | "contact"));
    d.default_pages.push(page(
        "about",
        "Our Story",
        "About {business_name}",
        "Meet {business_name}, learn what the bakery makes, how ordering works, and what customers can expect before visiting or placing an order.",
    ));
    d.default_pages.push(page(
        "contact",
        "Contact",
        "Contact {business_name}",
        "Contact {business_name} for bakery orders, custom cakes, catering, pickup questions, delivery details, or event dessert planning.",
    ));
    d.default_pages.push(page(
        "catering",
        "Catering",
        "{business_name} Catering",
        "Custom cake, dessert table, and catering details from {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "menu".into(),
            title: "Our Baked Goods".into(),
            url: "/bakery/menu".into(),
            emoji: "\u{1F370}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "order".into(),
            title: "Order Online".into(),
            url: "/bakery/order".into(),
            emoji: "\u{1F4E6}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "Our Story".into(),
            url: "/about".into(),
            emoji: "\u{1F9C1}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "catering".into(),
            title: "Catering".into(),
            url: "/catering".into(),
            emoji: "\u{1F382}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn coffee_shop() -> SiteTypeDefinition {
    let mut d = business_base(
        "coffee-shop",
        "Coffee Shop",
        "\u{2615}",
        "Coffee shop with menu, loyalty program, and online ordering.",
        17,
    );
    d.enabled_modules.retain(|module| {
        // Keep service-catalog because site-pages depends on it at
        // registration time. Drop invoicing because coffee shops bill via
        // cart/checkout (and invoicing also depends on service-catalog).
        // (AUD-019)
        !matches!(
            module.as_str(),
            "technicians"
                | "state-license-lookup"
                | "tech-portal"
                | "field-ops"
                | "inspections"
                | "availability"
                | "financing"
                | "jobs"
                | "invoicing"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["coffee", "cart", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#78350f".into(),
        secondary: "#92400e".into(),
        accent: "#d97706".into(),
    };
    d.theme_profile = Some(theme(
        "#78350f",
        "#d97706",
        "#1c0f05",
        "#fde68a",
        "#fefce8",
        "#1c0f05",
        "Slab Serif",
        10,
    ));
    d.default_tagline = "Your daily cup, crafted with care.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Your daily cup, crafted with care. Ethically sourced, locally roasted.",
            "show_cta": true,
            "cta_text": "See Our Menu",
            "cta_url": "/coffee/menu"
        }},
        { "type": "service-grid", "data": { "title": "Our Menu" } },
        { "type": "loyalty-check", "data": {
            "headline": "Earn a Free Drink",
            "subhead": "Every cup earns a punch. Punch your card with the email or phone you give at the counter."
        }},
        { "type": "testimonials", "data": { "title": "What Our Regulars Say" } },
        { "type": "cta-section", "data": {
            "title": "Come In for a Cup",
            "cta_text": "See Our Menu",
            "cta_url": "/coffee/menu"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Location, Pickup & Delivery",
        "Neighborhoods / Pickup Area",
        "Stockyards, downtown, campus, nearby neighborhoods...",
        "Delivery Radius (miles)",
        "Staff & Roles",
        "Owner, barista, roaster, manager...",
        "Owner, lead barista, roaster, manager...",
    );
    steps.push(step(
        "menu",
        "Coffee Menu",
        false,
        vec![field_grid(
            "menu_sections",
            "Menu Sections",
            true,
            &[
                "Hot Coffee",
                "Iced Coffee",
                "Espresso",
                "Tea",
                "Smoothies",
                "Pastries",
                "Breakfast",
                "Lunch",
            ],
        )],
    ));
    steps.push(step(
        "loyalty",
        "Loyalty & Specials",
        true,
        vec![
            field_toggle("loyalty_program", "Loyalty Program"),
            field_toggle("daily_specials", "Daily Specials"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages
        .retain(|page| !matches!(page.slug.as_str(), "services" | "about" | "contact"));
    d.default_pages.push(page(
        "about",
        "Our Story",
        "About {business_name}",
        "Meet {business_name}, learn what the shop serves, what the regulars love, and what guests can expect before they stop in or order ahead.",
    ));
    d.default_pages.push(page(
        "contact",
        "Visit Us",
        "Visit {business_name}",
        "Contact {business_name} for hours, menu questions, group orders, pickup details, loyalty help, or coffee shop events.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{2615}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "menu".into(),
            title: "Menu".into(),
            url: "/coffee/menu".into(),
            emoji: "\u{1F375}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "Our Story".into(),
            url: "/about".into(),
            emoji: "\u{1F331}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "loyalty".into(),
            title: "Loyalty".into(),
            url: "/coffee/loyalty".into(),
            emoji: "\u{2B50}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Visit Us".into(),
            url: "/contact".into(),
            emoji: "\u{1F4CD}".into(),
            position: 4,
            children: vec![],
        },
    ];
    d
}

fn salon() -> SiteTypeDefinition {
    let mut d = business_base(
        "salon",
        "Salon",
        "\u{1F487}",
        "Salon with online booking, client profiles, and service menu.",
        18,
    );
    d.enabled_modules
        .retain(|module| !matches!(module.as_str(), "tech-portal" | "field-ops" | "inspections"));
    d.enabled_modules.push("salon".into());
    d.default_brand_colors = BrandColors {
        primary: "#db2777".into(),
        secondary: "#be185d".into(),
        accent: "#f472b6".into(),
    };
    d.theme_profile = Some(theme(
        "#be185d", "#f472b6", "#500724", "#fce7f3", "#fdf2f8", "#500724", "Didone", 20,
    ));
    d.default_tagline = "Look your best, feel amazing.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Expert stylists dedicated to making you look and feel your best.",
            "show_cta": true,
            "cta_text": "Book Appointment",
            "cta_url": "/salon/book"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Clients Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Book?",
            "cta_text": "Book Appointment",
            "cta_url": "/salon/book"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Location & Appointment Area",
        "Studio / Mobile Appointment Areas",
        "In-studio, bridal locations, nearby neighborhoods...",
        "Mobile Travel Radius (miles)",
        "Providers & Staff",
        "Owner, stylist, esthetician, nail tech...",
        "Owner, stylist, colorist, esthetician, manager...",
    );
    steps.push(step(
        "services",
        "Salon Services",
        false,
        vec![field_grid(
            "services",
            "Services",
            true,
            &[
                "Haircuts",
                "Coloring",
                "Highlights",
                "Styling",
                "Extensions",
                "Nails",
                "Facials",
                "Waxing",
                "Massage",
                "Makeup",
            ],
        )],
    ));
    steps.push(step(
        "booking",
        "Booking Preferences",
        true,
        vec![
            field_toggle("online_booking", "Enable Online Booking"),
            field_toggle("walk_ins", "Accept Walk-Ins"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages.push(page(
        "gallery",
        "Gallery",
        "{business_name} Gallery",
        "Browse portfolio highlights, before-and-after looks, and style inspiration from {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/salon/services".into(),
            emoji: "\u{2702}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "book".into(),
            title: "Book Appointment".into(),
            url: "/salon/book".into(),
            emoji: "\u{1F4C5}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "gallery".into(),
            title: "Gallery".into(),
            url: "/salon/portfolio".into(),
            emoji: "\u{1F4F7}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "Our Team".into(),
            url: "/salon/team".into(),
            emoji: "\u{1F487}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn cell_phone_repair() -> SiteTypeDefinition {
    let mut d = business_base(
        "cell-phone-repair",
        "Cell Phone Repair",
        "\u{1F4F1}",
        "Repair shop with device catalog, work orders, POS, and protection plans.",
        19,
    );
    d.enabled_modules.extend(
        [
            "device-catalog",
            "device-registry",
            "work-orders",
            "counter-pos",
            "serialized-inventory",
            "protection-plans",
            "repair-intelligence",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    push_unique_modules(&mut d.enabled_modules, &["cart"]);
    d.default_brand_colors = BrandColors {
        primary: "#7c3aed".into(),
        secondary: "#6d28d9".into(),
        accent: "#a78bfa".into(),
    };
    d.theme_profile = Some(theme(
        "#6d28d9",
        "#a78bfa",
        "#1e1b4b",
        "#e0e7ff",
        "#eef2ff",
        "#1e1b4b",
        "Neo-Grotesque",
        12,
    ));
    d.default_tagline = "Fast, reliable device repairs.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Fast, affordable device repair. Most fixes done same day.",
            "show_cta": true,
            "cta_text": "Get a Quote",
            "cta_url": "#liq-repair-quote"
        }},
        { "type": "repair-quote-form", "data": {
            "headline": "Get a Free Quote",
            "subhead": "Tell us what's wrong and we'll get back to you with a written estimate."
        }},
        { "type": "service-grid", "data": { "title": "Our Repair Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Fix Your Device?",
            "cta_text": "Request a Quote",
            "cta_url": "#liq-repair-quote"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Locations & Drop-Off",
        "Locations / Pickup Areas",
        "Shop location, pickup/drop-off areas, nearby neighborhoods...",
        "Pickup Radius (miles)",
        "Repair Staff & Bench Roles",
        "Owner, repair tech, counter staff, manager...",
        "Owner, device specialist, bench tech, counter lead...",
    );
    steps.push(step(
        "devices",
        "Devices Serviced",
        false,
        vec![field_grid(
            "devices",
            "Devices",
            true,
            &[
                "iPhone",
                "Samsung",
                "Google Pixel",
                "iPad/Tablet",
                "Apple Watch",
                "Laptop",
                "Game Console",
            ],
        )],
    ));
    steps.push(step(
        "warranty",
        "Warranty & Parts",
        true,
        vec![
            field_text("warranty_days", "Warranty Period (days)", false, "90"),
            field_toggle("parts_tracking", "Enable Parts Tracking"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages = vec![
        page_with_blocks(
            "repairs",
            "Repairs",
            "{business_name} Repairs",
            "Screen repairs, battery replacements, diagnostics, and device fixes from {business_name}.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "{business_name} handles cracked screens, batteries, charging issues, diagnostics, camera problems, and other common repair needs. Start with a quick diagnosis and we will explain the repair path before work begins."
                }},
                { "type": "service-grid", "data": {
                    "title": "Common Repairs",
                    "limit": 8,
                    "show_prices": true
                }},
                { "type": "cta-bar", "data": {
                    "heading": "Need Help With a Damaged Device?",
                    "subheading": "Tell us what device you have and what is going wrong. We will help you with the next step.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page_with_blocks(
            "repair-pricing",
            "Repair Pricing",
            "{business_name} Repair Pricing",
            "Repair pricing and common service costs from {business_name}.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "Pricing depends on the model, the parts required, and whether the issue is simple damage or a deeper board-level problem. Use these common repair categories as a starting point, then contact us for an exact quote."
                }},
                { "type": "service-grid", "data": {
                    "title": "Popular Repairs and Starting Prices",
                    "limit": 8,
                    "show_prices": true
                }},
                { "type": "contact-info", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Want an Exact Quote?",
                    "subheading": "Share the device model, symptoms, and damage details so we can point you to the right repair.",
                    "cta_text": "Contact Us",
                    "cta_url": "/contact"
                }}
            ]),
        ),
        page_with_blocks(
            "about",
            "About Us",
            "About {business_name}",
            "Learn about {business_name} and our repair team.",
            serde_json::json!([
                { "type": "about-section", "data": { "max_chars": 900 } },
                { "type": "trust-badges", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Need a Trusted Repair Shop?",
                    "subheading": "Bring in your phone, tablet, or device and let us help you figure out the fastest repair path.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page_with_blocks(
            "contact",
            "Contact Us",
            "Contact {business_name}",
            "Get in touch with {business_name} for repair help.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "Tell us what device you have, what symptoms you are seeing, and whether the issue is urgent. We will help you figure out the next best step."
                }},
                { "type": "contact-info", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Ready to Start?",
                    "subheading": "Reach out for a quote, a repair estimate, or a quick device diagnosis.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page(
            "service-areas",
            "Locations & Pickup",
            "{business_name} Locations & Pickup",
            "Shop locations, walk-in hours, pickup options, and neighborhoods served by {business_name}.",
        ),
        page(
            "reviews",
            "Reviews",
            "Reviews for {business_name}",
            "Customer reviews and testimonials for {business_name}.",
        ),
    ];
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "repairs".into(),
            title: "Repairs".into(),
            url: "/repairs".into(),
            emoji: "\u{1F4F1}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "pricing".into(),
            title: "Pricing".into(),
            url: "/repair-pricing".into(),
            emoji: "\u{1F4B2}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F527}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Locations".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 6,
            children: vec![],
        },
    ];
    d
}

fn medical() -> SiteTypeDefinition {
    let mut d = business_base(
        "medical-office",
        "Medical Office",
        "\u{1FA7A}",
        "Medical practice website with appointment requests, patient forms, provider pages, and careful privacy-aware next steps.",
        20,
    );
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians"
                | "state-license-lookup"
                | "tech-portal"
                | "field-ops"
                | "inspections"
                | "availability"
                | "invoicing"
                | "financing"
                | "jobs"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["booking", "patient-intake"]);
    d.default_brand_colors = BrandColors {
        primary: "#0891b2".into(),
        secondary: "#0e7490".into(),
        accent: "#22d3ee".into(),
    };
    d.theme_profile = Some(theme(
        "#0e7490",
        "#22d3ee",
        "#083344",
        "#ecfeff",
        "#f0fdfa",
        "#083344",
        "Transitional",
        8,
    ));
    d.default_tagline = "Compassionate care, modern medicine.".into();
    d.seo_description_template =
        "{business_name}: medical office website with appointment requests, provider pages, patient forms, services, and careful next steps."
            .into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Accepting new patients. Comprehensive care for the whole family.",
            "show_cta": true,
            "cta_text": "Request Appointment",
            "cta_url": "#liq-patient-intake"
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "patient-intake-form", "data": {
            "headline": "Request an Appointment",
            "subhead": "Fill in your details and we'll be in touch to confirm a time. Your information is stored securely on this site only."
        }},
        { "type": "testimonials", "data": { "title": "What Our Patients Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Schedule?",
            "cta_text": "Request Appointment",
            "cta_url": "#liq-patient-intake"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Practice Location & Appointment Area",
        "Patient Communities / Visit Area",
        "Clinic location, nearby communities, telehealth availability...",
        "Appointment Area Radius (miles)",
        "Providers & Care Team",
        "Owner, provider, nurse, front desk...",
        "Physician, provider, nurse, care coordinator, front desk...",
    );
    steps.push(step(
        "appointment_types",
        "Appointment Types",
        false,
        vec![field_grid(
            "services",
            "Appointment Types",
            true,
            &[
                "New Patient Visit",
                "Annual Wellness Visit",
                "Same-Day Sick Visit",
                "Follow-up Visit",
                "Chronic Care Check-in",
                "Telehealth Visit",
            ],
        )],
    ));
    steps.push(step(
        "specialties",
        "Specialties",
        false,
        vec![field_grid(
            "specialties",
            "Specialties",
            true,
            &[
                "Family Medicine",
                "Internal Medicine",
                "Pediatrics",
                "Dermatology",
                "Orthopedics",
                "Cardiology",
                "Dental",
                "Optometry",
                "Mental Health",
                "Urgent Care",
                "Physical Therapy",
                "Chiropractic",
            ],
        )],
    ));
    steps.push(step(
        "patient_info",
        "Patient Experience",
        true,
        vec![
            field_toggle("online_scheduling", "Enable Online Scheduling"),
            field_toggle("patient_portal", "Enable Patient Portal"),
            field_text(
                "insurance_networks",
                "Insurance Networks",
                false,
                "Aetna, BlueCross, Cigna...",
            ),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages
        .retain(|page| !matches!(page.slug.as_str(), "services" | "about" | "contact"));
    d.default_pages.extend([
        page_with_blocks(
            "services",
            "Care Services",
            "{business_name} Care Services",
            "Compare appointment types, patient resources, and scheduling paths from {business_name}.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"Care services","title":"Care services that are easy to understand","text":"Compare appointment types, patient resources, and next steps before requesting care.","chips":["New patients","Wellness visits","Follow-up care"],"actions":[{"label":"Request Appointment","url":"/book","style":"primary"},{"label":"Meet Providers","url":"/providers","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"New patient visits","text":"Help new patients understand what to bring and how the first appointment is confirmed."},
                    {"title":"Wellness and follow-up care","text":"Keep routine visits, follow-ups, and chronic-care check-ins clear before the patient arrives."},
                    {"title":"Patient preparation","text":"Connect services to forms, portal access, medication lists, and records requests."}
                ]}},
                {"type":"button","data":{"text":"Request Appointment","url":"/book","style":"primary","alignment":"center"}}
            ]),
        ),
        page_with_blocks(
            "about",
            "About Us",
            "About {business_name}",
            "Learn about {business_name}, the care team, and how patients can prepare for a visit.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"About the practice","title":"Care should feel organized before the visit starts","text":"A clear medical office website helps patients understand who they will see, what to bring, and where to go next.","chips":["Patient-friendly","Local trust","Clear next steps"],"actions":[{"label":"Request Appointment","url":"/book","style":"primary"},{"label":"Patient Forms","url":"/patient-forms","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"Clear next steps","text":"Move patients from services to providers, forms, and appointment requests without confusion."},
                    {"title":"Local trust","text":"Give the community a practical view of the practice, not a generic healthcare brochure."},
                    {"title":"Helpful preparation","text":"Keep forms, insurance notes, medication-list reminders, and visit expectations close to booking."}
                ]}}
            ]),
        ),
        page_with_blocks(
            "contact",
            "Contact Us",
            "Contact {business_name}",
            "Contact {business_name} to ask a question, request an appointment, or confirm what to bring before a visit.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"Contact","title":"One clear place to reach the practice","text":"Use contact for questions, appointment requests for visits, and patient forms when paperwork is needed before care.","chips":["Phone and email","Location details","Patient prep"],"actions":[{"label":"Request Appointment","url":"/book","style":"primary"},{"label":"Patient Forms","url":"/patient-forms","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"Call or email","text":"Show the fastest way to reach the care team for questions and follow-up."},
                    {"title":"Location","text":"Keep address, hours, and visit preparation easy to find on mobile."},
                    {"title":"Before you arrive","text":"Point patients to forms, insurance notes, records requests, and medication-list reminders."}
                ]}}
            ]),
        ),
        page_with_blocks(
            "providers",
            "Our Providers",
            "Providers at {business_name}",
            "Meet the doctors, clinicians, and care team at {business_name}.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"Care team","title":"Meet the people behind the visit","text":"Introduce providers, focus areas, and the support team patients may hear from before or after care.","chips":["Provider trust","Care coordination","Patient support"],"actions":[{"label":"Request Appointment","url":"/book","style":"primary"},{"label":"Patient Forms","url":"/patient-forms","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"Primary care","text":"Routine visits, wellness checks, follow-ups, and everyday health questions."},
                    {"title":"Care coordination","text":"Help with records, referrals, visit preparation, and next steps after an appointment."},
                    {"title":"Patient support","text":"Clear phone, portal, and form paths so patients know how to reach the right person."}
                ]}}
            ]),
        ),
        page_with_blocks(
            "patient-forms",
            "Patient Forms",
            "{business_name} Patient Forms",
            "Patient forms, visit paperwork, and appointment preparation from {business_name}.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"Patient forms","title":"Know what to bring before the visit","text":"Keep intake forms, insurance notes, consent forms, medication-list reminders, and records-request instructions in one place.","chips":["New patient intake","Insurance and ID","Records requests"],"actions":[{"label":"Request Appointment","url":"/book","style":"primary"},{"label":"Meet Providers","url":"/providers","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"New patient intake","text":"Basic contact information, health history, current medications, allergies, and reason for visit."},
                    {"title":"Insurance and ID","text":"Bring your insurance card, photo ID, and any referral or authorization your plan requires."},
                    {"title":"Records and privacy","text":"Use records-release and privacy forms when the practice needs permission to request or share medical records."}
                ]}}
            ]),
        ),
        page_with_blocks(
            "privacy",
            "Notice of Privacy Practices",
            "Notice of Privacy Practices — {business_name}",
            "How {business_name} uses and protects patient health information under HIPAA. Required notice.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"blue","kicker":"HIPAA","title":"Notice of Privacy Practices","text":"This notice describes how medical information about you may be used and disclosed and how you can get access to this information. Please review it carefully.","chips":["HIPAA § 164.520","Your rights","Our duties"]}},
                {"type":"feature-grid","data":{"columns":2,"items":[
                    {"title":"Your rights","text":"You have the right to request and inspect copies of your medical records, request amendments, request an accounting of disclosures, request restrictions on how your information is shared, request confidential communications, and receive notification of any breach affecting your protected health information."},
                    {"title":"Our duties","text":"We are required by law to maintain the privacy of your protected health information, provide you with this notice of our legal duties and privacy practices, follow the terms of the notice currently in effect, and notify you in the event of a breach."},
                    {"title":"How we use your information","text":"We may use and disclose your health information for treatment, payment, and health care operations without your written authorization. We may also use it for appointment reminders, to inform you about treatment alternatives or health-related benefits, and as required by law."},
                    {"title":"Disclosures requiring your authorization","text":"Most uses and disclosures of psychotherapy notes, uses and disclosures for marketing, and disclosures that constitute a sale of protected health information require your written authorization. You may revoke authorization at any time in writing."}
                ]}},
                {"type":"feature-grid","data":{"columns":1,"items":[
                    {"title":"Contact and complaints","text":"To exercise any of your rights, to ask questions about this notice, or to file a complaint, contact the practice using the information on the Contact page. You may also file a complaint with the U.S. Department of Health and Human Services Office for Civil Rights. We will not retaliate against you for filing a complaint."},
                    {"title":"Changes to this notice","text":"We reserve the right to change this notice and to make the revised notice effective for all protected health information we maintain. Current copies are always available on this page and in the practice waiting area."}
                ]}}
            ]),
        ),
    ]);
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1FA7A}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "providers".into(),
            title: "Our Providers".into(),
            url: "/providers".into(),
            emoji: "\u{1F469}\u{200D}\u{2695}\u{FE0F}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "patient-forms".into(),
            title: "Patient Forms".into(),
            url: "/patient-forms".into(),
            emoji: "\u{1F4CB}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F3E5}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn church() -> SiteTypeDefinition {
    let ts = now();
    let mut d = SiteTypeDefinition {
        slug: "church".into(),
        name: "Church".into(),
        emoji: "\u{26EA}".into(),
        category: "free".into(),
        description: "Church website with event calendar, sermon archive, and small groups. Free tier includes 1 GB storage and standard traffic limits.".into(),
        default_tagline: String::new(),
        publicly_listed: true,
        display_order: 21,
        enabled_modules: {
            // Start with the community module set (calendar, feed, recipes, vault, etc.)
            let mut mods = group_modules("church");
            // Add content/marketing modules that churches also need for their public site
            push_unique_modules(&mut mods, &[
                "service-catalog",
                "booking",
                "customer-portal",
                "site-blueprint",
                "site-pages",
                "content-pipeline",
                "page-generator",
                "email-marketing",
                "notifications",
                "location-profile",
                "module-manager",
            ]);
            mods
        },
        theme_presets: presets::generate_presets(&presets::palette_for("church"), "church"),
        default_brand_colors: BrandColors::default(),
        theme_profile: None,
        homepage_blocks: None,
        default_nav_items: vec![],
        onboarding_steps: vec![
            step(
                "blog_profile",
                "Your Blog",
                false,
                vec![
                    field_text("blog_name", "Blog Name", true, "My Awesome Blog"),
                    field_select(
                        "blog_topic",
                        "Main Topic",
                        false,
                        &[
                            "Personal / Journal",
                            "Tech & Programming",
                            "Business & Marketing",
                            "Lifestyle",
                            "Food & Recipes",
                            "Travel",
                            "Parenting",
                            "Finance",
                            "Health & Wellness",
                            "Creative Writing",
                            "Other",
                        ],
                    ),
                    field_textarea("bio", "Short Bio", false, "A sentence or two about you"),
                ],
            ),
            step(
                "editorial_plan",
                "Editorial Plan",
                true,
                vec![
                    field_select(
                        "publishing_cadence",
                        "Publishing Cadence",
                        false,
                        &["Daily", "A few times a week", "Weekly", "Twice a month", "Monthly"],
                    ),
                    field_select(
                        "reader_promise",
                        "What should readers expect most?",
                        false,
                        &[
                            "How-to guides",
                            "Personal stories",
                            "Thought leadership",
                            "Research & resources",
                            "Reviews & recommendations",
                            "News & commentary",
                        ],
                    ),
                    field_toggle("newsletter_focus", "Build an email newsletter from day one"),
                ],
            ),
            step(
                "blog_features",
                "Features",
                true,
                vec![field_grid(
                    "features",
                    "Enable Features",
                    false,
                    &[
                        "Blog with Categories",
                        "Email Subscribers",
                        "SEO Tools",
                        "Analytics",
                        "Contact Form",
                        "Portfolio / Pages",
                    ],
                )],
            ),
        ],
        always_free: true,
        default_tier: "founding".into(),
        price_override_cents: 0,
        discount_codes: vec![],
        limited_time_offer: None,
        default_tone: String::new(),
        default_pages: vec![],
        seo_title_template: "{business_name} — {tagline}".into(),
        seo_description_template: "{business_name} is a welcoming church community in {city}, {state}.".into(),
        created_at: ts,
        updated_at: ts,
    };
    d.default_brand_colors = BrandColors {
        primary: "#1e3a5f".into(),
        secondary: "#7c2d12".into(),
        accent: "#b45309".into(),
    };
    d.theme_profile = Some(theme(
        "#1e3a5f",
        "#b45309",
        "#0f1d30",
        "#fef3c7",
        "#fef3c7",
        "#1e3a5f",
        "Transitional",
        8,
    ));
    d.default_nav_items = vec![
        nav("home", "Home", "/", "\u{1F3E0}", 0),
        nav("about", "About", "/about", "\u{26EA}", 1),
        nav("sermons", "Sermons", "/sermons", "\u{1F4D6}", 2),
        nav("calendar", "Events", "/calendar/", "\u{1F4C5}", 3),
        nav("ministries", "Ministries", "/ministries", "\u{1F64F}", 4),
        nav("feed", "Prayer Wall", "/feed", "\u{1F64F}", 5),
        nav("vault", "Church Files", "/vault", "\u{1F512}", 6),
        nav("contact", "Contact", "/contact", "\u{1F4DE}", 7),
        nav("giving", "Giving", "/giving", "\u{1F49D}", 8),
    ];
    d.default_tagline = "A place to belong, believe, and grow.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "A welcoming community of faith. Everyone is invited.",
            "show_cta": true,
            "cta_text": "Visit This Sunday",
            "cta_url": "/about"
        }},
        { "type": "family-quick-access", "data": {
            "title": "Church Life",
            "subtitle": "Events, sermons, ministries, and next steps",
            "cards": [
                { "icon": "\u{2139}\u{FE0F}", "label": "About", "description": "What we believe and what to expect when you visit.", "url": "/about" },
                { "icon": "\u{1F4D6}", "label": "Sermons", "description": "Recent messages and teaching from the church.", "url": "/sermons" },
                { "icon": "\u{1F4C5}", "label": "Events", "description": "Services, Bible studies, and upcoming church gatherings.", "url": "/calendar/" },
                { "icon": "\u{1F64F}", "label": "Ministries", "description": "Ways to serve, grow, and get connected.", "url": "/ministries" },
                { "icon": "\u{1F4DE}", "label": "Contact", "description": "Questions, service times, and how to reach us.", "url": "/contact" },
                { "icon": "\u{1F511}", "label": "Member Login", "description": "Private church files and member resources.", "url": "/family-login" },
                { "icon": "\u{1F49D}", "label": "Giving", "description": "Support the mission of the church with a one-time or recurring gift.", "url": "/giving" }
            ]
        } },
        { "type": "service-grid", "data": { "title": "Our Ministries" } },
        { "type": "testimonials", "data": { "title": "Hear from Our Members" } },
        { "type": "cta-section", "data": {
            "title": "Come Join Us",
            "cta_text": "Visit This Sunday",
            "cta_url": "/about"
        }}
    ]));
    let mut steps = onboard(
        "church-info",
        "Church Info",
        "Church Name",
        "Grace Community Church",
        "Leaders / Volunteers",
    );
    if let Some(details) = group_specific_details_step("church") {
        steps.push(details);
    }
    steps.push(step(
        "church-contact",
        "Contact & Location",
        true,
        vec![
            field_text("address", "Street Address", false, "123 Main Street"),
            field_text("city", "City", false, "Austin"),
            field_text("state", "State", false, "Texas"),
            field_text("phone", "Phone", false, "(555) 123-4567"),
            field_text("email", "Email", false, "hello@example.org"),
        ],
    ));
    steps.push(step(
        "services_schedule",
        "Service Schedule",
        true,
        vec![
            field_text("sunday_service", "Sunday Service Time", false, "10:30 AM"),
            field_text(
                "wednesday_service",
                "Wednesday Service Time",
                false,
                "7:00 PM",
            ),
            field_toggle("livestream", "Offer Livestream"),
        ],
    ));
    steps.push(group_features_step("church"));
    d.onboarding_steps = steps;
    d.default_pages = vec![
        DefaultPage {
            slug: "home".into(),
            title: "{business_name}".into(),
            blocks: None,
            seo_title: "{business_name} — {tagline}".into(),
            seo_description: "{description}".into(),
        },
        DefaultPage {
            slug: "about".into(),
            title: "About Our Church".into(),
            blocks: Some(simple_public_page_blocks(
                "About",
                "A clearer first visit for {business_name}",
                "Find everything you need before your first visit — what to expect on Sunday, service times, parking, childcare, and how to connect with our church family.",
                &[
                    "Service times, parking, childcare, and what to expect on Sunday.",
                    "Pastoral staff, church story, doctrine, and membership path.",
                    "Simple links to sermons, events, ministries, and contact details.",
                ],
                "Plan a Visit",
                "/contact",
            )),
            seo_title: "About {business_name}".into(),
            seo_description: "Learn about {business_name}, our history, beliefs, and pastoral staff.".into(),
        },
        DefaultPage {
            slug: "sermons".into(),
            title: "Sermons".into(),
            blocks: Some(simple_public_page_blocks(
                "Sermons",
                "Recent teaching in one place",
                "Browse recent messages, series notes, and scripture references. Share a sermon with someone who needs encouragement this week.",
                &[
                    "Post recent sermons, livestream replays, notes, and scripture references.",
                    "Organize teaching by series, speaker, date, or topic.",
                    "Point listeners toward the next service, small group, or ministry contact.",
                ],
                "See Upcoming Events",
                "/calendar/",
            )),
            seo_title: "Sermons — {business_name}".into(),
            seo_description: "Listen to recent sermons and Bible teaching from {business_name}.".into(),
        },
        DefaultPage {
            slug: "calendar".into(),
            title: "Events".into(),
            blocks: Some(simple_public_page_blocks(
                "Events",
                "Services, studies, and church gatherings",
                "Find upcoming services, Bible studies, youth nights, fellowship meals, volunteer opportunities, and special church events.",
                &[
                    "Every service and event listed in one place — no hunting through group chats.",
                    "RSVP for potlucks, volunteer sign-ups, or special events directly from the calendar.",
                    "Member-only details and private planning notes stay behind login where they belong.",
                ],
                "Contact the Church",
                "/contact",
            )),
            seo_title: "Events — {business_name}".into(),
            seo_description: "Upcoming services, Bible studies, potlucks, and community events at {business_name}.".into(),
        },
        DefaultPage {
            slug: "ministries".into(),
            title: "Our Ministries".into(),
            blocks: Some(simple_public_page_blocks(
                "Ministries",
                "Find where to worship, serve, and grow",
                "Discover the many ways to serve and connect — from youth programs and missions to music ministry, Bible study, and community outreach.",
                &[
                    "Share youth, missions, music, Bible study, outreach, nursery, and volunteer opportunities.",
                    "Learn who each ministry serves and how to take your first step toward getting involved.",
                    "Contact us to ask about joining a ministry or finding the right fit for your gifts.",
                ],
                "Ask About Ministries",
                "/contact",
            )),
            seo_title: "Ministries — {business_name}".into(),
            seo_description: "Explore the ministries of {business_name} — youth, missions, music, Bible study, and more.".into(),
        },
        DefaultPage {
            slug: "feed".into(),
            title: "Prayer Wall".into(),
            blocks: Some(simple_public_page_blocks(
                "Prayer Wall",
                "Pray together. Stay connected. Share your heart.",
                "A place for your church family to share prayer requests, testimonies, announcements, and community updates.",
                &[
                    "Public announcements can be visible to visitors when the church chooses.",
                    "Member prayer requests, replies, photos, and sensitive updates stay behind login.",
                    "Sign in to post a prayer request or share an update with the church family.",
                ],
                "Member Sign In",
                "/family-login?next=/feed",
            )),
            seo_title: "Community — {business_name}".into(),
            seo_description: "Updates, prayer requests, and announcements from {business_name}.".into(),
        },
        DefaultPage {
            slug: "vault".into(),
            title: "Church Files".into(),
            blocks: None,
            seo_title: "Resources — {business_name}".into(),
            seo_description: "Bulletins, newsletters, and resources from {business_name}.".into(),
        },
        DefaultPage {
            slug: "contact".into(),
            title: "Contact Us".into(),
            blocks: Some(simple_public_page_blocks(
                "Contact",
                "We'd love to hear from you",
                "Reach out about service times, directions, ministry questions, prayer requests, or how to get involved. We're here to help.",
                &[
                    "Make visiting easy with address, directions, service times, and contact options.",
                    "Let visitors ask about ministries, membership, prayer, or special needs.",
                    "Keep member-only files, private requests, and internal follow-up behind login.",
                ],
                "Plan a Visit",
                "/about",
            )),
            seo_title: "Contact {business_name}".into(),
            seo_description: "Get in touch with {business_name}. Service times, directions, and contact information.".into(),
        },
        DefaultPage {
            slug: "giving".into(),
            title: "Give".into(),
            blocks: Some(simple_public_page_blocks(
                "Give",
                "Support the mission of the church",
                "Your generosity makes our ministry possible. Give online, by check, or in person — one-time or recurring.",
                &[
                    "Explain how to give online, by check, or in person, and where funds go.",
                    "Share giving campaigns, building funds, missions support, or benevolence funds.",
                    "Allow members to set up recurring giving through the member portal.",
                ],
                "Give Now",
                "/book",
            )),
            seo_title: "Give — {business_name}".into(),
            seo_description: "Support the mission of {business_name} with a one-time or recurring gift.".into(),
        },
    ];
    d
}

fn artisan_market() -> SiteTypeDefinition {
    let mut d = business_base(
        "artisan-market",
        "Artisan Market",
        "\u{1F6CD}\u{FE0F}",
        "Handcrafted goods marketplace with vendor profiles, collections, and events.",
        19,
    );
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians" | "state-license-lookup" | "tech-portal" | "field-ops" | "inspections"
        )
    });
    push_unique_modules(
        &mut d.enabled_modules,
        &["cart", "checkout-pipeline", "brooke-grace", "blog"],
    );
    d.default_brand_colors = BrandColors {
        primary: "#14b8a6".into(),
        secondary: "#0d9488".into(),
        accent: "#2dd4bf".into(),
    };
    d.theme_profile = Some(theme(
        "#0d9488", "#2dd4bf", "#042f2e", "#ccfbf1", "#f0fdfa", "#042f2e", "Humanist", 14,
    ));
    d.default_tagline = "Handcrafted with heart.".into();
    d.seo_description_template =
        "{business_name}: artisan market website with collections, maker stories, cart, and checkout-ready ordering."
            .into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Handcrafted goods from local makers. Every piece tells a story.",
            "show_cta": true,
            "cta_text": "Shop Now",
            "cta_url": "/market/shop"
        }},
        { "type": "service-grid", "data": { "title": "Browse Our Collection" } },
        { "type": "testimonials", "data": { "title": "What Our Shoppers Say" } },
        { "type": "cta-section", "data": {
            "title": "Find Something Special",
            "cta_text": "Shop Now",
            "cta_url": "/market/shop"
        }}
    ]));
    d.onboarding_steps = vec![
        step(
            "welcome",
            "Market Identity",
            false,
            vec![
                field_text(
                    "business_name",
                    "Market / Shop Name",
                    true,
                    "Your market or shop name",
                ),
                field_text(
                    "tagline",
                    "Tagline",
                    false,
                    "Handmade goods, local makers, seasonal drops...",
                ),
            ],
        ),
        step(
            "contact",
            "Contact & Pickup",
            false,
            vec![
                field_text("email", "Customer Email", true, "hello@yourmarket.com"),
                field_text("phone", "Phone", false, "(555) 123-4567"),
                field_text(
                    "pickup_location",
                    "Pickup / Market Location",
                    false,
                    "Booth, studio, shop, or local pickup area",
                ),
            ],
        ),
        step(
            "products",
            "Product Categories",
            false,
            vec![
                field_grid(
                    "categories",
                    "What should shoppers be able to browse first?",
                    true,
                    &[
                        "Jewelry", "Pottery", "Woodwork", "Textiles", "Art", "Candles", "Soaps",
                        "Food", "Leather", "Glass",
                    ],
                ),
                field_grid(
                    "fulfillment",
                    "How do orders usually get handled?",
                    false,
                    &[
                        "Shipping",
                        "Local pickup",
                        "Market booth pickup",
                        "Custom orders",
                    ],
                ),
            ],
        ),
        step(
            "makers",
            "Makers & Stories",
            true,
            vec![field_textarea(
                "makers",
                "Featured Makers",
                false,
                "Add maker names, specialties, or notes for the first artisan pages",
            )],
        ),
        step(
            "events",
            "Events & Markets",
            true,
            vec![field_toggle(
                "recurring_market",
                "Recurring Market Schedule",
            )],
        ),
    ];
    d.default_pages = vec![
        page(
            "home",
            "{business_name}",
            "{business_name} — {tagline}",
            "{description}",
        ),
        page(
            "shop",
            "Shop",
            "{business_name} Shop",
            "Browse featured products, vendor collections, and highlighted finds from {business_name}.",
        ),
        page(
            "artisans",
            "Artisans",
            "Artisans at {business_name}",
            "Meet the makers and featured artisans behind {business_name}.",
        ),
        page(
            "about",
            "About",
            "About {business_name}",
            "Learn the story behind {business_name}, the makers, and the products shoppers can find here.",
        ),
        page(
            "contact",
            "Contact",
            "Contact {business_name}",
            "Ask about products, pickup, shipping, market dates, or custom orders from {business_name}.",
        ),
    ];
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/market".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "shop".into(),
            title: "Shop".into(),
            url: "/market/shop".into(),
            emoji: "\u{1F6CD}\u{FE0F}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "artisans".into(),
            title: "Creators".into(),
            url: "/market/creators".into(),
            emoji: "\u{1F3A8}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/market/about".into(),
            emoji: "\u{1F9F5}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/market/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 4,
            children: vec![],
        },
    ];
    d
}

fn electronics_repair() -> SiteTypeDefinition {
    let mut d = business_base(
        "electronics-repair",
        "Electronics Repair",
        "\u{1F50C}",
        "Diagnostics, bench work, and repair status tracking for all electronics.",
        20,
    );
    push_unique_modules(&mut d.enabled_modules, &["cart"]);
    d.default_brand_colors = BrandColors {
        primary: "#10b981".into(),
        secondary: "#059669".into(),
        accent: "#34d399".into(),
    };
    d.theme_profile = Some(theme(
        "#059669",
        "#34d399",
        "#022c22",
        "#d1fae5",
        "#ecfdf5",
        "#022c22",
        "Neo-Grotesque",
        8,
    ));
    d.default_tagline = "We fix what others can't.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Expert diagnostics and repair for all electronics. We fix what others can't.",
            "show_cta": true,
            "cta_text": "Get a Diagnosis",
            "cta_url": "#liq-repair-quote"
        }},
        { "type": "repair-quote-form", "data": {
            "headline": "Tell us what's broken",
            "subhead": "Send us a quick description and we'll quote it back — usually within a day."
        }},
        { "type": "service-grid", "data": { "title": "Our Repair Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Bring In Your Device",
            "cta_text": "Request a Diagnosis",
            "cta_url": "#liq-repair-quote"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Locations & Drop-Off",
        "Locations / Pickup Areas",
        "Shop location, pickup/drop-off areas, nearby neighborhoods...",
        "Pickup Radius (miles)",
        "Repair Staff & Bench Roles",
        "Owner, bench tech, counter staff, manager...",
        "Owner, electronics specialist, bench tech, counter lead...",
    );
    steps.push(step(
        "devices",
        "Device Categories",
        false,
        vec![field_grid(
            "devices",
            "Devices",
            true,
            &[
                "Computers",
                "Tablets",
                "Game Consoles",
                "Audio Equipment",
                "TVs",
                "Drones",
                "Cameras",
                "Printers",
            ],
        )],
    ));
    steps.push(step(
        "diagnostics",
        "Diagnostics & SLAs",
        true,
        vec![
            field_text("diagnostic_fee", "Diagnostic Fee", false, "$49.99"),
            field_text("turnaround", "Standard Turnaround", false, "3-5 days"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages = vec![
        page_with_blocks(
            "repairs",
            "Repairs",
            "{business_name} Repairs",
            "Diagnostics and repair services from {business_name}.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "{business_name} handles diagnostics, component replacement, troubleshooting, and repair work for electronics that need a real bench test instead of a guess. We explain the issue clearly before moving into the repair."
                }},
                { "type": "service-grid", "data": {
                    "title": "Repair Categories",
                    "limit": 8,
                    "show_prices": true
                }},
                { "type": "cta-bar", "data": {
                    "heading": "Need an Expert Diagnosis?",
                    "subheading": "Bring in the device, explain the symptoms, and let us point you toward the smartest fix.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page_with_blocks(
            "repair-pricing",
            "Repair Pricing",
            "{business_name} Repair Pricing",
            "Repair pricing and service options from {business_name}.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "Every repair starts with the specific device, the failure symptoms, and the parts required. Use these common services as a guide, then contact us for a precise quote."
                }},
                { "type": "service-grid", "data": {
                    "title": "Common Repairs and Starting Prices",
                    "limit": 8,
                    "show_prices": true
                }},
                { "type": "contact-info", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Need a Quote for a Specific Device?",
                    "subheading": "Share the model and symptoms and we will help you with the right repair path.",
                    "cta_text": "Contact Us",
                    "cta_url": "/contact"
                }}
            ]),
        ),
        page_with_blocks(
            "about",
            "About Us",
            "About {business_name}",
            "Learn about {business_name} and our repair process.",
            serde_json::json!([
                { "type": "about-section", "data": { "max_chars": 900 } },
                { "type": "trust-badges", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Looking for a Reliable Electronics Repair Partner?",
                    "subheading": "We help customers understand the problem first, then move into the repair with a clear plan.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page_with_blocks(
            "contact",
            "Contact Us",
            "Contact {business_name}",
            "Contact {business_name} for repair support.",
            serde_json::json!([
                { "type": "paragraph", "data": {
                    "text": "Tell us what device you have, what is failing, and whether the issue is intermittent or constant. That gives us a much better starting point for diagnostics."
                }},
                { "type": "contact-info", "data": {} },
                { "type": "cta-bar", "data": {
                    "heading": "Ready to Talk Through the Repair?",
                    "subheading": "Contact us for a diagnosis, a repair quote, or the next best step for your device.",
                    "cta_text": "Book a Repair",
                    "cta_url": "/book"
                }}
            ]),
        ),
        page(
            "service-areas",
            "Locations & Drop-Off",
            "{business_name} Locations & Drop-Off",
            "Shop locations, drop-off instructions, pickup options, and areas served by {business_name}.",
        ),
        page(
            "reviews",
            "Reviews",
            "Reviews for {business_name}",
            "Customer reviews and testimonials for {business_name}.",
        ),
    ];
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "repairs".into(),
            title: "Repairs".into(),
            url: "/repairs".into(),
            emoji: "\u{1F4BB}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "pricing".into(),
            title: "Pricing".into(),
            url: "/repair-pricing".into(),
            emoji: "\u{1F4B2}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F529}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "service-areas".into(),
            title: "Locations".into(),
            url: "/service-areas".into(),
            emoji: "\u{1F4CD}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 6,
            children: vec![],
        },
    ];
    d
}

fn auto_repair() -> SiteTypeDefinition {
    let mut d = business_base(
        "auto-repair",
        "Auto Repair",
        "\u{1F697}",
        "Service bays, estimates, maintenance plans, and appointment scheduling.",
        21,
    );
    d.default_brand_colors = BrandColors {
        primary: "#6366f1".into(),
        secondary: "#4f46e5".into(),
        accent: "#818cf8".into(),
    };
    d.theme_profile = Some(theme(
        "#4f46e5",
        "#818cf8",
        "#1e1b4b",
        "#e0e7ff",
        "#eef2ff",
        "#1e1b4b",
        "Industrial",
        6,
    ));
    d.default_tagline = "Honest auto care you can trust.".into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "company-hero", "data": {
            "headline": "{business_name}",
            "description": "Honest auto care from certified mechanics. Free estimates on most services.",
            "show_cta": true,
            "cta_text": "Get a Free Estimate",
            "cta_url": "#liq-repair-quote"
        }},
        { "type": "repair-quote-form", "data": {
            "headline": "Get a Free Estimate",
            "subhead": "Tell us about your vehicle and what's going on. We'll get back to you with an estimate."
        }},
        { "type": "service-grid", "data": { "title": "Our Services" } },
        { "type": "testimonials", "data": { "title": "What Our Customers Say" } },
        { "type": "cta-section", "data": {
            "title": "Ready to Bring In Your Vehicle?",
            "cta_text": "Get a Free Estimate",
            "cta_url": "#liq-repair-quote"
        }}
    ]));
    let mut steps = common_biz_steps();
    steps.push(step(
        "services",
        "Auto Services",
        false,
        vec![field_grid(
            "services",
            "Services",
            true,
            &[
                "Oil Change",
                "Brakes",
                "Transmission",
                "Engine Repair",
                "Diagnostics",
                "Tires",
                "AC Repair",
                "Electrical",
                "Body Work",
                "Inspection",
            ],
        )],
    ));
    steps.push(step(
        "estimates",
        "Estimates & Warranty",
        true,
        vec![
            field_toggle("free_estimates", "Offer Free Estimates"),
            field_text("warranty_months", "Parts Warranty (months)", false, "12"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages.push(page(
        "reviews",
        "Reviews",
        "Reviews for {business_name}",
        "Customer reviews and testimonials for {business_name}.",
    ));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1F697}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "schedule".into(),
            title: "Schedule Service".into(),
            url: "/book".into(),
            emoji: "\u{1F4C5}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "about".into(),
            title: "About".into(),
            url: "/about".into(),
            emoji: "\u{1F527}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    d
}

fn attorney() -> SiteTypeDefinition {
    let mut d = business_base(
        "attorney",
        "Attorney / Law Firm",
        "\u{2696}",
        "Law firm website with named-attorney roster (JD + bar #), practice areas, case results, free consultation, e-signed engagement letters, secure client portal, and online payment.",
        21,
    );
    // Drop modules that are field-service-specific.
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians" | "state-license-lookup" | "tech-portal" | "field-ops" | "inspections" | "jobs"
        )
    });
    // Add the contracts/e-signature module — engagement letters, retainer
    // agreements, and consents are core to a law-firm workflow.
    push_unique_modules(&mut d.enabled_modules, &["contracts", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#1e3a8a".into(), // deep navy — classic legal
        secondary: "#1e40af".into(),
        accent: "#b45309".into(), // antique gold accent
    };
    d.theme_profile = Some(theme(
        "#1e3a8a",
        "#b45309",
        "#0f172a",
        "#f8fafc",
        "#fffbeb",
        "#0f172a",
        "Transitional",
        4,
    ));
    d.default_tagline = "Local representation, plainly explained.".into();
    d.seo_description_template =
        "{business_name}: law firm with named attorneys, clear practice areas, free consultation, sanitized case results, and a secure client portal."
            .into();
    // Real-law-firm homepage composition. The default `company-hero`
    // looked like a generic small-business gradient. Real legal sites
    // open with: name + "Attorneys at Law" + free-consultation primary
    // CTA + trust badges (state bar, Avvo, Super Lawyers) + practice-
    // areas grid + sanitized case-results stats + reviews + final CTA.
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "marketing-hero", "data": {
            "theme": "site",
            "kicker": "Attorneys at Law",
            "title": "{business_name}",
            "text": "Free initial consultation. Real attorneys, plain answers, sanitized case results — local court experience.",
            "chips": [
                "Free consultation",
                "State bar verified",
                "Sanitized case results",
                "Local court experience"
            ],
            "actions": [
                { "label": "Request Free Consultation", "url": "/free-consultation", "style": "primary" },
                { "label": "Meet Our Attorneys", "url": "/attorneys", "style": "outline" }
            ]
        }},
        { "type": "trust-badges", "data": {
            "title": "Verified by",
            "badges": [
                { "label": "State Bar Verified" },
                { "label": "Avvo Rated" },
                { "label": "Martindale-Hubbell" },
                { "label": "Super Lawyers" },
                { "label": "BBB A+" }
            ]
        }},
        { "type": "feature-grid", "data": {
            "columns": 3,
            "items": [
                { "eyebrow": "Personal injury", "title": "Injury & accident claims", "text": "Auto, motorcycle, premises, product liability — handled with named-attorney attention." },
                { "eyebrow": "Family law", "title": "Divorce, custody, support", "text": "Plain explanations of process, timelines, and likely outcomes — before retainer." },
                { "eyebrow": "DUI / criminal defense", "title": "DUI, drug, assault charges", "text": "Same-day callbacks for arrests. Clear plea-vs-trial analysis." },
                { "eyebrow": "Estate planning", "title": "Wills, trusts, probate", "text": "Documents that hold up in your county's probate court." },
                { "eyebrow": "Business law", "title": "Formation, contracts, disputes", "text": "LLC, partnership, contract review, and shareholder dispute representation." },
                { "eyebrow": "Real estate", "title": "Closings & title disputes", "text": "Title review, easement disputes, and closing representation." }
            ]
        }},
        { "type": "stat-band", "data": {
            "stats": [
                { "value": "X", "label": "Cases handled" },
                { "value": "Y", "label": "Years combined experience" },
                { "value": "Z", "label": "Client reviews" },
                { "value": "Free", "label": "Initial consultation" }
            ]
        }},
        { "type": "about-section", "data": {
            "title": "Why families and businesses pick {business_name}",
            "body": "We are a family-owned law firm focused on local representation. We do not promise outcomes the bar would not let us promise. We do tell you, plainly, what happens next, what it costs, what could go wrong, and what we have seen in this courthouse before."
        }},
        { "type": "testimonials", "data": { "title": "What clients say" } },
        { "type": "cta-bar", "data": {
            "title": "Ready to talk to a real attorney?",
            "subtitle": "Free, confidential consultation. No obligation. Same-day callbacks when we can.",
            "cta_text": "Request Free Consultation",
            "cta_url": "/free-consultation",
            "secondary_cta_text": "Call Now",
            "secondary_cta_url": "tel:{phone}"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Office Location & Service Area",
        "Counties / Cities Served",
        "Office locations, counties served, and any virtual / telehealth-style remote consultation availability...",
        "Service Area Radius (miles)",
        "Attorneys & Staff",
        "Owner, partner, associate, paralegal, receptionist...",
        "Each attorney's name + JD + state bar # + practice areas + years admitted",
    );
    steps.push(step(
        "practice_areas",
        "Practice Areas",
        false,
        vec![field_grid(
            "services",
            "Practice Areas",
            true,
            &[
                "Personal injury",
                "Family law / divorce",
                "DUI / criminal defense",
                "Estate planning",
                "Business / corporate law",
                "Real estate law",
                "Employment law",
                "Immigration",
                "Bankruptcy",
                "Workers' compensation",
                "Wrongful death",
                "Medical malpractice",
            ],
        )],
    ));
    steps.push(step(
        "credentials",
        "Bar / Trust Signals",
        true,
        vec![
            field_text(
                "state_bar",
                "State Bar Association(s)",
                false,
                "Texas State Bar, ABA...",
            ),
            field_text(
                "accreditations",
                "Awards & Memberships",
                false,
                "Super Lawyers, Best Lawyers, Martindale-Hubbell AV, BBB...",
            ),
            field_toggle("offer_free_consultation", "Offer free initial consultation"),
            field_toggle("client_portal", "Enable secure client portal"),
            field_toggle("e_signatures", "Enable e-signed engagement letters"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_pages
        .retain(|page| !matches!(page.slug.as_str(), "services" | "about" | "contact"));
    d.default_pages.extend([
        page_with_blocks(
            "services",
            "Practice Areas",
            "{business_name} Practice Areas",
            "Compare practice areas at {business_name}: what we handle, how the process works, and what to bring to a free consultation.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"navy","kicker":"Practice areas","title":"Plain answers. Local representation.","text":"What we handle, how the process works, and what to bring to a free consultation.","chips":["Free consultation","Sanitized case results","Local court experience"],"actions":[{"label":"Request Free Consultation","url":"/free-consultation","style":"primary"},{"label":"Meet the Attorneys","url":"/attorneys","style":"outline"}]}},
                {"type":"feature-grid","data":{"columns":3,"items":[
                    {"title":"Real outcomes, sanitized","text":"Read case summaries by area without revealing protected client information."},
                    {"title":"Bar-verified attorneys","text":"Each attorney is named with JD school, state bar number, and years admitted."},
                    {"title":"Free initial consultation","text":"Bring documents, ask questions — no obligation. Same-day appointments available."}
                ]}},
                {"type":"button","data":{"text":"Request Free Consultation","url":"/free-consultation","style":"primary","alignment":"center"}}
            ]),
        ),
        page_with_blocks(
            "about",
            "About the Firm",
            "About {business_name}",
            "Learn about {business_name}, the attorneys, and how we communicate with clients from intake through resolution.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"navy","kicker":"About the firm","title":"Local court experience. Plain communication.","text":"Who we are, what we don't do, and how we keep clients informed.","chips":["Family-owned","Local courts","Plain language"],"actions":[{"label":"Meet the Attorneys","url":"/attorneys","style":"primary"},{"label":"Read Reviews","url":"/reviews","style":"outline"}]}}
            ]),
        ),
        page_with_blocks(
            "contact",
            "Contact",
            "Contact {business_name}",
            "Reach {business_name} for free consultation requests, after-hours messages, and office address. ABA Rule 7.2-compliant.",
            serde_json::json!([
                {"type":"marketing-hero","data":{"theme":"navy","kicker":"Contact","title":"Free consultation, real attorney.","text":"Request a callback or schedule a consultation. No obligation. Confidential.","actions":[{"label":"Request Free Consultation","url":"/free-consultation","style":"primary"}]}}
            ]),
        ),
    ]);
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3DB}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "attorneys".into(),
            title: "Attorneys".into(),
            url: "/attorneys".into(),
            emoji: "\u{1F468}\u{200D}\u{2696}\u{FE0F}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "practice-areas".into(),
            title: "Practice Areas".into(),
            url: "/services".into(),
            emoji: "\u{1F4DA}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "results".into(),
            title: "Case Results".into(),
            url: "/results".into(),
            emoji: "\u{1F3C6}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "consultation".into(),
            title: "Free Consultation".into(),
            url: "/free-consultation".into(),
            emoji: "\u{1F4C5}".into(),
            position: 6,
            children: vec![],
        },
    ];
    // Add the dedicated /attorneys roster page so the nav link doesn't 404.
    d.default_pages.push(page_with_blocks(
        "attorneys",
        "Our Attorneys",
        "Our Attorneys — {business_name}",
        "Meet the attorneys at {business_name}: each named with JD school, state bar number, practice areas, and years admitted to practice.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Our team","title":"Named, bar-verified attorneys.","text":"Each attorney is listed with JD school, state bar number, practice areas, and years admitted. No anonymous \"our team of experienced attorneys\" pitches.","actions":[{"label":"Request Free Consultation","url":"/free-consultation","style":"primary"}]}},
            {"type":"about-section","data":{
                "title":"How we vet who lists here",
                "body":"State bar number is shown for every attorney. Discipline history is checked annually. Bios list practice areas the attorney actually handles, not the firm-wide list. We do not pad the roster with paralegals or office staff."
            }},
            {"type":"feature-grid","data":{"columns":1,"items":[
                {"eyebrow":"Founding Partner","title":"James E. Harmon, JD","text":"Admitted 1998 · State Bar No. 6278844 · Loyola University Chicago School of Law · Civil litigation, contract disputes, and business law."}
            ]}},
            {"type":"cta-bar","data":{"title":"Want to schedule a call?","cta_text":"Request Free Consultation","cta_url":"/free-consultation"}}
        ]),
    ));
    // /reviews and /results placeholder pages so the nav doesn't 404.
    d.default_pages.push(page_with_blocks(
        "reviews",
        "Reviews",
        "Reviews — {business_name}",
        "Verified client reviews from Avvo, Google, and Martindale-Hubbell for {business_name}.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Reviews","title":"Verified reviews from real platforms.","text":"We pull reviews from third-party legal-review platforms — Avvo, Google, Martindale-Hubbell — instead of writing testimonials ourselves.","actions":[{"label":"Read on Avvo","url":"#avvo","style":"primary"}]}},
            {"type":"testimonials","data":{"title":"Recent client reviews"}}
        ]),
    ));
    d.default_pages.push(page_with_blocks(
        "results",
        "Case Results",
        "Case Results — {business_name}",
        "Sanitized case results by practice area at {business_name}. Confidential client details removed; outcomes shown.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Case results","title":"Outcomes, sanitized for client confidentiality.","text":"Past results do not guarantee future outcomes (ABA Rule 7.2). Names, locations, and specifics are removed. Practice area, type of matter, and outcome are shown.","actions":[{"label":"Talk About Your Case","url":"/free-consultation","style":"primary"}]}},
            {"type":"feature-grid","data":{"columns":2,"items":[
                {"eyebrow":"Personal injury","title":"Add a sanitized case here","text":"e.g. \"Auto accident · driver injured by uninsured motorist · settled for full policy limits + UM claim.\""},
                {"eyebrow":"Family law","title":"Add a sanitized case here","text":"e.g. \"Contested custody · same-day-care parent retained primary custody after 8-month proceeding.\""}
            ]}}
        ]),
    ));
    d.default_pages.push(page_with_blocks(
        "free-consultation",
        "Free Consultation",
        "Free Consultation — {business_name}",
        "Request a free initial consultation with an attorney at {business_name}. Confidential. No obligation.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Free consultation","title":"Talk to a real attorney before you sign anything.","text":"Free, confidential, no obligation. Bring your documents. Same-day callbacks when our schedule allows.","chips":["Confidential","No obligation","Same-day callbacks"],"actions":[{"label":"Schedule Now","url":"#liq-booking-form","style":"primary"},{"label":"Call Now","url":"tel:","style":"outline"}]}},
            {"type":"booking-form","data":{"title":"Schedule your free consultation","fields":["name","email","phone","matter_type","preferred_time","summary"]}}
        ]),
    ));
    d
}

fn accountant() -> SiteTypeDefinition {
    let mut d = business_base(
        "accountant",
        "Accountant / CPA Firm",
        "\u{1F4CA}",
        "CPA firm website with named CPAs (license # + state), tax + bookkeeping services, secure client document portal, e-signed engagement letters, and online invoice payment.",
        21,
    );
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians" | "state-license-lookup" | "tech-portal" | "field-ops" | "inspections" | "jobs"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["contracts", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#0f766e".into(), // teal — financial-trust palette
        secondary: "#0d9488".into(),
        accent: "#15803d".into(),
    };
    d.theme_profile = Some(theme(
        "#0f766e", "#15803d", "#022c22", "#f0fdf4", "#f0fdfa", "#022c22", "Modern", 6,
    ));
    d.default_tagline = "Tax season without the surprises.".into();
    d.seo_description_template =
        "{business_name}: CPA firm with named license-holders, tax + bookkeeping services, e-signed engagement letters, secure document portal, and a clear tax calendar."
            .into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "marketing-hero", "data": {
            "theme": "site",
            "kicker": "Certified Public Accountants",
            "title": "{business_name}",
            "text": "Personal and business tax, bookkeeping, payroll, audit support — by named CPAs with state license numbers, not anonymous bots.",
            "chips": ["Free initial consultation","Licensed CPAs","Year-round availability","Secure document portal"],
            "actions": [
                { "label": "Request a Free Consultation", "url": "/free-consultation", "style": "primary" },
                { "label": "Meet Our CPAs", "url": "/cpas", "style": "outline" }
            ]
        }},
        { "type": "trust-badges", "data": {
            "title": "Licensed and certified",
            "badges": [
                { "label": "AICPA member" },
                { "label": "State Board CPA" },
                { "label": "QuickBooks ProAdvisor" },
                { "label": "EA / IRS Enrolled Agent" },
                { "label": "BBB A+" }
            ]
        }},
        { "type": "feature-grid", "data": {
            "columns": 3,
            "items": [
                { "eyebrow": "Tax preparation", "title": "Individual & business returns", "text": "Federal + state, multi-state, K-1s, Schedule C, business entity returns." },
                { "eyebrow": "Bookkeeping", "title": "Monthly books that pass an audit", "text": "QuickBooks Online setup, monthly reconciliation, financial statements." },
                { "eyebrow": "Audit support", "title": "IRS letters & representation", "text": "We respond to IRS notices, represent you at audits, and resolve back-tax issues." },
                { "eyebrow": "Payroll", "title": "Payroll + quarterly filings", "text": "Direct deposit, W-2s/1099s, multi-state withholding, quarterly 941s." },
                { "eyebrow": "Estate & trust", "title": "Trust accounting & 1041 returns", "text": "Trustee accounting, estate income tax, beneficiary K-1s." },
                { "eyebrow": "Advisory", "title": "Tax planning & strategy", "text": "Year-end planning sessions, entity selection, retirement contribution strategy." }
            ]
        }},
        { "type": "stat-band", "data": {
            "stats": [
                { "value": "X", "label": "Returns filed last year" },
                { "value": "Y", "label": "Years in practice" },
                { "value": "Z", "label": "Avg. client review" },
                { "value": "Free", "label": "Initial consultation" }
            ]
        }},
        { "type": "about-section", "data": {
            "title": "Why pick {business_name}",
            "body": "We are a family-run CPA firm. We answer the phone year-round, not just January through April. We tell you, plainly, what your filing actually owes, what changed this year, and which strategy reduces it for next year — before the deadline."
        }},
        { "type": "testimonials", "data": { "title": "What clients say" } },
        { "type": "cta-bar", "data": {
            "title": "Tax season around the corner?",
            "subtitle": "Free initial consultation. Bring last year's return. We will tell you what we can do.",
            "cta_text": "Schedule Now",
            "cta_url": "/free-consultation"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Office Location & Service Area",
        "Counties / Cities Served",
        "Office, virtual / remote service availability...",
        "Service Area Radius (miles)",
        "CPAs & Staff",
        "Partner, CPA, EA, bookkeeper, admin...",
        "Each CPA's name + license # + state + specializations + years",
    );
    steps.push(step(
        "service_lines",
        "Services Offered",
        false,
        vec![field_grid(
            "services",
            "Services",
            true,
            &[
                "Individual tax preparation",
                "Business tax preparation",
                "Bookkeeping",
                "Payroll",
                "Audit support / IRS representation",
                "Estate & trust tax",
                "QuickBooks setup",
                "Financial planning",
                "Forensic accounting",
                "Sales / use tax",
            ],
        )],
    ));
    steps.push(step(
        "credentials",
        "Licenses & Trust Signals",
        true,
        vec![
            field_text(
                "state_licenses",
                "State CPA License(s)",
                false,
                "Texas CPA, AICPA...",
            ),
            field_text(
                "certifications",
                "Certifications",
                false,
                "EA, CMA, CFP, QuickBooks ProAdvisor...",
            ),
            field_toggle("offer_free_consultation", "Offer free initial consultation"),
            field_toggle("client_portal", "Enable secure document portal"),
            field_toggle("e_signatures", "Enable e-signed engagement letters"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E2}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "cpas".into(),
            title: "Our CPAs".into(),
            url: "/cpas".into(),
            emoji: "\u{1F464}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "services".into(),
            title: "Services".into(),
            url: "/services".into(),
            emoji: "\u{1F4CA}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "tax-calendar".into(),
            title: "Tax Calendar".into(),
            url: "/tax-calendar".into(),
            emoji: "\u{1F4C5}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "reviews".into(),
            title: "Reviews".into(),
            url: "/reviews".into(),
            emoji: "\u{2B50}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    // /cpas roster page so the nav link does not 404.
    d.default_pages.push(page_with_blocks(
        "cpas",
        "Our CPAs",
        "Our CPAs — {business_name}",
        "Meet the licensed CPAs at {business_name}: each listed with CPA license number, state, and the tax and accounting work they actually handle.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Our team","title":"Named, license-verified CPAs.","text":"Every CPA is listed with their state CPA license number, practice areas, and years of experience.","actions":[{"label":"Schedule a Consultation","url":"/contact","style":"primary"}]}},
            {"type":"about-section","data":{"title":"How we vet who lists here","body":"CPA license number is shown for every accountant. License status is checked annually against the state board. Bios list the types of returns and clients each CPA actually handles."}},
            {"type":"feature-grid","data":{"columns":1,"items":[{"eyebrow":"Founding CPA","title":"Sandra K. Willis, CPA","text":"Licensed in Texas since 2001 · CPA License No. TX-079832 · Specializes in small business tax preparation, bookkeeping, payroll, and advisory for LLCs and S-corps."}]}},
            {"type":"cta-bar","data":{"title":"Ready to get started?","cta_text":"Schedule a Consultation","cta_url":"/contact"}}
        ]),
    ));
    // /tax-calendar page so the nav link does not 404.
    d.default_pages.push(page_with_blocks(
        "tax-calendar",
        "Tax Calendar",
        "Tax Calendar — {business_name}",
        "Key tax deadlines and important dates for individuals and businesses, curated by {business_name}.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Important dates","title":"Key tax deadlines, curated by your CPA.","text":"Federal and state deadlines change year to year. Your CPA firm maintains this calendar — bookmark it and check back each quarter.","chips":["Federal deadlines","State filings","Quarterly estimates"],"actions":[{"label":"Talk to Your CPA","url":"/contact","style":"primary"}]}},
            {"type":"about-section","data":{"title":"A note on this calendar","body":"Tax deadlines are subject to IRS and state announcements, including disaster-area extensions. Dates here are for general awareness only. Your CPA will confirm deadlines for your specific situation. When in doubt, file early."}},
            {"type":"feature-grid","data":{"columns":2,"items":[
                {"eyebrow":"Q1 – Jan–Mar","title":"Replace with your current-year deadlines","text":"e.g. W-2 and 1099 distribution, IRA contribution windows, Q4 estimated payment due dates."},
                {"eyebrow":"Q2 – Apr–Jun","title":"Replace with your current-year deadlines","text":"e.g. April 15 individual returns, Q1 estimated payments, extension filing deadlines."},
                {"eyebrow":"Q3 – Jul–Sep","title":"Replace with your current-year deadlines","text":"e.g. Q2 estimated payments, trust and estate deadlines, September extension deadlines."},
                {"eyebrow":"Q4 – Oct–Dec","title":"Replace with your current-year deadlines","text":"e.g. Q3 estimated payments, extended return deadlines, year-end tax planning reminders."}
            ]}},
            {"type":"cta-bar","data":{"title":"Need deadline-specific guidance?","cta_text":"Talk to a CPA","cta_url":"/contact"}}
        ]),
    ));
    // /resources hub page — tax guides, calculators, checklists.
    d.default_pages.push(page_with_blocks(
        "resources",
        "Resources",
        "Accounting Resources — {business_name}",
        "Tax guides, financial calculators, checklists, and tools from {business_name} to help individuals and businesses stay organized year-round.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Helpful tools","title":"Guides, checklists, and resources.","text":"Free resources from our CPAs: tax prep checklists, deadlines, bookkeeping guides, and calculators to help you stay ahead.","chips":["Tax prep checklist","Bookkeeping guides","Deadline calendar"],"actions":[{"label":"Talk to a CPA","url":"/contact","style":"primary"},{"label":"See Tax Deadlines","url":"/tax-calendar","style":"outline"}]}},
            {"type":"feature-grid","data":{"columns":3,"items":[
                {"eyebrow":"Checklist","title":"Tax Prep Checklist","text":"W-2s, 1099s, mortgage interest, charitable donations, business expenses, and home office documentation — gathered before the meeting saves everyone time."},
                {"eyebrow":"Guide","title":"Bookkeeping Basics","text":"Separate accounts, monthly reconciliation, receipt tracking, and a chart of accounts that matches how your business actually runs."},
                {"eyebrow":"Calculator","title":"Quarterly Estimate Helper","text":"Self-employed individuals and business owners often owe Q1–Q4 estimates. Ask your CPA to confirm your safe-harbor amount before each due date."},
                {"eyebrow":"Guide","title":"Entity Comparison","text":"Sole prop, LLC, S-corp, or C-corp — each has different tax treatment. We walk through the trade-offs so you choose what fits your situation."},
                {"eyebrow":"Checklist","title":"New Business Startup","text":"EIN, business bank account, state registration, payroll setup, and the accounting software decision — step-by-step for a clean start."},
                {"eyebrow":"Reference","title":"Common Deductions","text":"Home office, vehicle, health insurance premiums, retirement contributions, and education expenses — tracked correctly throughout the year, not scrambled at filing."}
            ]}},
            {"type":"cta-bar","data":{"title":"Questions about any of these topics?","cta_text":"Schedule a Consultation","cta_url":"/contact"}}
        ]),
    ));
    d
}

fn insurance() -> SiteTypeDefinition {
    let mut d = business_base(
        "insurance",
        "Insurance Agency",
        "\u{1F6E1}",
        "Insurance agency website with named licensed agents (license # + states), coverage lines, online quote requests, claims help, and policy-document portal.",
        21,
    );
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians" | "state-license-lookup" | "tech-portal" | "field-ops" | "inspections" | "jobs"
        )
    });
    push_unique_modules(&mut d.enabled_modules, &["contracts", "blog"]);
    d.default_brand_colors = BrandColors {
        primary: "#1d4ed8".into(), // dependable blue
        secondary: "#1e40af".into(),
        accent: "#dc2626".into(), // claims/urgency red
    };
    d.theme_profile = Some(theme(
        "#1d4ed8", "#dc2626", "#0f172a", "#eff6ff", "#fef2f2", "#0f172a", "Modern", 6,
    ));
    d.default_tagline = "Coverage you can read in plain English.".into();
    d.seo_description_template =
        "{business_name}: insurance agency with named licensed agents, coverage lines (home, auto, life, business), free quote requests, and claims help."
            .into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "marketing-hero", "data": {
            "theme": "site",
            "kicker": "Insurance Agency",
            "title": "{business_name}",
            "text": "Home, auto, life, and business — by named licensed agents who answer the phone. Plain coverage explanations, claims help, no captive-only sales pressure.",
            "chips": ["Free quote","Licensed agents","Claims help","Multiple carriers"],
            "actions": [
                { "label": "Get a Free Quote", "url": "/quote", "style": "primary" },
                { "label": "Talk to an Agent", "url": "/contact", "style": "outline" }
            ]
        }},
        { "type": "trust-badges", "data": {
            "title": "Carriers represented",
            "badges": [
                { "label": "Allstate" },
                { "label": "State Farm" },
                { "label": "Progressive" },
                { "label": "Travelers" },
                { "label": "Liberty Mutual" }
            ]
        }},
        { "type": "feature-grid", "data": {
            "columns": 3,
            "items": [
                { "eyebrow": "Auto", "title": "Auto insurance", "text": "Liability, collision, comprehensive, uninsured-motorist. SR-22 filings handled." },
                { "eyebrow": "Home", "title": "Homeowners & renters", "text": "Home, renters, condo, flood, earthquake. Replacement-cost vs actual-cash-value explained." },
                { "eyebrow": "Life", "title": "Life insurance", "text": "Term, whole life, indexed universal. Plain comparison, not pressure." },
                { "eyebrow": "Business", "title": "Commercial / business", "text": "General liability, BOP, commercial auto, workers' comp." },
                { "eyebrow": "Health", "title": "Medicare supplement", "text": "Supplement plans, Part D, dental & vision riders." },
                { "eyebrow": "Umbrella", "title": "Umbrella / excess", "text": "Personal and business umbrella over $1M. We compare costs from multiple carriers." }
            ]
        }},
        { "type": "stat-band", "data": {
            "stats": [
                { "value": "X", "label": "Carriers represented" },
                { "value": "Y", "label": "Policies in force" },
                { "value": "Z", "label": "Years licensed" },
                { "value": "Free", "label": "Quote" }
            ]
        }},
        { "type": "about-section", "data": {
            "title": "Why pick {business_name}",
            "body": "We are independent — that means we shop multiple carriers when your policy comes up for renewal, instead of pushing whatever a single company happens to sell. We help with claims. We answer the phone after a wreck or after a tree falls."
        }},
        { "type": "testimonials", "data": { "title": "What clients say" } },
        { "type": "cta-bar", "data": {
            "title": "Need a quote or have a claim?",
            "subtitle": "Get a free quote in minutes, or talk to a real agent about a policy you already have.",
            "cta_text": "Get a Free Quote",
            "cta_url": "/quote",
            "secondary_cta_text": "Talk to an Agent",
            "secondary_cta_url": "/contact"
        }}
    ]));
    let mut steps = common_biz_steps();
    tune_location_staff_steps(
        &mut steps,
        "Office Location & Service Area",
        "States / Cities Served",
        "Where you sell — states licensed, agency offices, captive vs. independent...",
        "Service Area Radius (miles)",
        "Agents & Staff",
        "Owner, agent, customer-service rep, account manager...",
        "Each agent's name + license # + states + specializations",
    );
    steps.push(step(
        "coverage_lines",
        "Coverage Lines",
        false,
        vec![field_grid(
            "services",
            "Coverage",
            true,
            &[
                "Home insurance",
                "Auto insurance",
                "Life insurance",
                "Business / commercial",
                "Health / Medicare supplement",
                "Renters",
                "Umbrella",
                "Boat / RV",
                "Workers' compensation",
                "Disability",
            ],
        )],
    ));
    steps.push(step(
        "credentials",
        "Licenses & Trust Signals",
        true,
        vec![
            field_text(
                "state_licenses",
                "Licensed States",
                false,
                "Texas, Oklahoma, Louisiana...",
            ),
            field_text(
                "carriers",
                "Carriers Represented",
                false,
                "Allstate, State Farm, Progressive, Travelers...",
            ),
            field_toggle("captive", "Captive agency (single carrier)"),
            field_toggle("independent", "Independent agency (multiple carriers)"),
            field_toggle("client_portal", "Enable policy-document portal"),
        ],
    ));
    d.onboarding_steps = steps;
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "agents".into(),
            title: "Our Agents".into(),
            url: "/agents".into(),
            emoji: "\u{1F464}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "coverage".into(),
            title: "Coverage".into(),
            url: "/services".into(),
            emoji: "\u{1F6E1}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "quote".into(),
            title: "Get a Quote".into(),
            url: "/quote".into(),
            emoji: "\u{1F4DD}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "claims".into(),
            title: "Claims".into(),
            url: "/contact?topic=claim".into(),
            emoji: "\u{1F198}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "contact".into(),
            title: "Contact".into(),
            url: "/contact".into(),
            emoji: "\u{1F4DE}".into(),
            position: 5,
            children: vec![],
        },
    ];
    // /agents roster page so the nav link does not 404.
    d.default_pages.push(page_with_blocks(
        "agents",
        "Our Licensed Agents",
        "Our Licensed Agents — {business_name}",
        "Meet the licensed insurance agents at {business_name}: each listed with their license number, state, and the coverage lines they handle.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Our team","title":"Named, license-verified agents.","text":"Every agent is listed with their state insurance license number and the lines of coverage they are authorized to sell.","actions":[{"label":"Get a Quote","url":"/book","style":"primary"}]}},
            {"type":"about-section","data":{"title":"How we vet who lists here","body":"Insurance license number is shown for every agent. License status is checked annually against the state Department of Insurance. Bios list the actual coverage lines each agent handles."}},
            {"type":"feature-grid","data":{"columns":1,"items":[{"eyebrow":"Principal Agent","title":"Add your agency's principal agent here","text":"After signup, replace this with the principal agent's name, photo, insurance license #, state, lines of authority, and a short bio focused on the clients and coverage types they actually specialize in."}]}},
            {"type":"cta-bar","data":{"title":"Ready to compare rates?","cta_text":"Get a Quote","cta_url":"/book"}}
        ]),
    ));

    // /products page: coverage types as products for the insurance demo.
    d.default_pages.push(page_with_blocks(
        "products",
        "Coverage Products",
        "Insurance Coverage Products — {business_name}",
        "Compare home, auto, life, business, and umbrella coverage options from {business_name}. Get a free quote from a named, licensed agent.",
        serde_json::json!([
            {"type":"marketing-hero","data":{"theme":"site","kicker":"Coverage options","title":"Plain-English Coverage Comparison","text":"Home, auto, life, business, renters, umbrella — compare options and get a free quote. Every product listed by a named, licensed agent, not a call center.","actions":[{"label":"Get a Free Quote","url":"/book","style":"primary"},{"label":"Talk to an Agent","url":"/contact","style":"outline"}]}},
            {"type":"feature-grid","data":{"columns":3,"items":[
                {"eyebrow":"Home","title":"Homeowners Insurance","text":"Dwelling, personal property, liability, loss of use. Replacement-cost vs. actual-cash-value explained. Flood and earthquake riders available."},
                {"eyebrow":"Auto","title":"Auto Insurance","text":"Liability, collision, comprehensive, uninsured motorist, rental reimbursement. SR-22 filings handled. Multi-car discounts compared."},
                {"eyebrow":"Life","title":"Life Insurance","text":"Term, whole life, and indexed universal life. We compare costs from multiple carriers and explain the difference without pressure."},
                {"eyebrow":"Business","title":"Commercial Coverage","text":"General liability, business owner's policy (BOP), commercial auto, workers' compensation. Tailored to your business type and size."},
                {"eyebrow":"Health","title":"Medicare Supplement","text":"Supplement plans, Part D prescription drug coverage, dental and vision riders. Enrollment guidance for Medicare-eligible clients."},
                {"eyebrow":"Umbrella","title":"Umbrella / Excess Liability","text":"Personal and commercial umbrella over $1M. Covers gaps left by auto, home, and business policies. Compared across multiple carriers."}
            ]}},
            {"type":"about-section","data":{"title":"How we find the right fit","body":"We are independent — we shop multiple carriers when your policy renews instead of pushing whatever a single company happens to sell. We also help with claims, not just sales."}},
            {"type":"cta-bar","data":{"title":"Ready to compare rates?","subtitle":"Get a free quote in minutes from a licensed agent, or ask about a policy you already have.","cta_text":"Get a Free Quote","cta_url":"/book","secondary_cta_text":"Contact an Agent","secondary_cta_url":"/contact"}}
        ]),
    ));

    d
}

fn app_publisher() -> SiteTypeDefinition {
    let mut d = business_base(
        "app-publisher",
        "App Publisher",
        "\u{1F4F1}",
        "App showcase with store links, press kit, changelogs, and documentation.",
        22,
    );
    d.enabled_modules.retain(|module| {
        !matches!(
            module.as_str(),
            "technicians"
                | "state-license-lookup"
                | "tech-portal"
                | "field-ops"
                | "inspections"
                | "availability"
                | "invoicing"
                | "financing"
                | "jobs"
        )
    });
    // `site-pages` has hard runtime dependencies on booking, customer-portal,
    // and service-catalog today. Keep them loaded for startup, while the public
    // app-publisher nav/pages redirect away from generic service-business flows.
    push_unique_modules(
        &mut d.enabled_modules,
        &["blog", "email-marketing", "content-pipeline"],
    );
    d.default_brand_colors = BrandColors {
        primary: "#6366f1".into(),
        secondary: "#4338ca".into(),
        accent: "#a5b4fc".into(),
    };
    d.theme_profile = Some(theme(
        "#4338ca",
        "#a5b4fc",
        "#0f0d2e",
        "#e0e7ff",
        "#eef2ff",
        "#0f0d2e",
        "Neo-Grotesque",
        12,
    ));
    d.default_tagline = "Ship apps. Grow users.".into();
    d.seo_title_template = "{business_name} App Publisher Site".into();
    d.seo_description_template =
        "{business_name}: app publisher website with store links, docs, changelog, support paths, launch notes, and product updates."
            .into();
    d.homepage_blocks = Some(serde_json::json!([
        { "type": "marketing-hero", "data": {
            "kicker": "App publisher website",
            "title": "{business_name}",
            "text": "A launch home for your apps, store links, screenshots, docs, changelog, support path, and product updates.",
            "theme": "site",
            "chips": ["App showcase", "Docs and support", "Release notes"],
            "actions": [
                { "label": "View Apps", "url": "/apps", "style": "primary" },
                { "label": "Read Docs", "url": "/docs", "style": "outline" }
            ]
        }},
        { "type": "feature-grid", "data": {
            "columns": 3,
            "items": [
                { "title": "Show the app clearly", "text": "Use app cards, screenshots, platform badges, pricing notes, and feature highlights so visitors know what to install.", "url": "/apps", "eyebrow": "Showcase" },
                { "title": "Support users faster", "text": "Put docs, release notes, bug-report steps, and contact paths where users can find them before opening a ticket.", "url": "/support", "eyebrow": "Support" },
                { "title": "Publish product updates", "text": "Use the blog and changelog for launch notes, fixes, roadmap notes, and useful stories about how the app is improving.", "url": "/changelog", "eyebrow": "Updates" }
            ]
        }},
        { "type": "button", "data": { "alignment": "center", "style": "primary", "text": "Explore Apps", "url": "/apps" } }
    ]));
    d.default_nav_items = vec![
        NavItem {
            item_id: "home".into(),
            title: "Home".into(),
            url: "/".into(),
            emoji: "\u{1F3E0}".into(),
            position: 0,
            children: vec![],
        },
        NavItem {
            item_id: "apps".into(),
            title: "Apps".into(),
            url: "/apps".into(),
            emoji: "\u{1F4F1}".into(),
            position: 1,
            children: vec![],
        },
        NavItem {
            item_id: "docs".into(),
            title: "Docs".into(),
            url: "/docs".into(),
            emoji: "\u{1F4DA}".into(),
            position: 2,
            children: vec![],
        },
        NavItem {
            item_id: "blog".into(),
            title: "Blog".into(),
            url: "/blog".into(),
            emoji: "\u{270D}\u{FE0F}".into(),
            position: 3,
            children: vec![],
        },
        NavItem {
            item_id: "download".into(),
            title: "Download".into(),
            url: "/download".into(),
            emoji: "\u{2B07}\u{FE0F}".into(),
            position: 4,
            children: vec![],
        },
        NavItem {
            item_id: "support".into(),
            title: "Support".into(),
            url: "/support".into(),
            emoji: "\u{1F6DF}".into(),
            position: 5,
            children: vec![],
        },
        NavItem {
            item_id: "changelog".into(),
            title: "Changelog".into(),
            url: "/changelog".into(),
            emoji: "\u{1F4DD}".into(),
            position: 6,
            children: vec![],
        },
    ];
    d.default_pages = vec![
        page_with_blocks(
            "apps",
            "Our Apps",
            "{business_name} Apps",
            "Explore apps, store links, screenshots, feature highlights, and download paths from {business_name}.",
            serde_json::json!([
                { "type": "marketing-hero", "data": {
                    "kicker": "Apps",
                    "title": "Explore {business_name} apps",
                    "text": "Give visitors a useful first look at every app: what it does, who it helps, where to install it, and what to read next.",
                    "theme": "site",
                    "chips": ["Store links", "Feature cards", "Screenshots"],
                    "actions": [
                        { "label": "Download", "url": "/download", "style": "primary" },
                        { "label": "Read the docs", "url": "/docs", "style": "outline" }
                    ]
                }},
                { "type": "feature-grid", "data": {
                    "columns": 3,
                    "items": [
                        { "title": "Flagship app", "text": "Find store links, platform options, and everything you need to install the app in minutes.", "url": "/download", "eyebrow": "Primary" },
                        { "title": "Screenshots and proof", "text": "Show what the app looks like, what changed recently, and where users can learn more before installing.", "url": "/changelog", "eyebrow": "Preview" },
                        { "title": "Help before tickets", "text": "Point users to docs, known issues, device notes, and a clean support request path.", "url": "/support", "eyebrow": "Support" }
                    ]
                }}
            ]),
        ),
        page_with_blocks(
            "docs",
            "Documentation",
            "{business_name} Docs",
            "Read setup notes, platform instructions, FAQ answers, and support guidance for {business_name} apps.",
            serde_json::json!([
                { "type": "marketing-hero", "data": {
                    "kicker": "Documentation",
                    "title": "Docs that help users keep moving",
                    "text": "Organize setup notes, account help, platform-specific instructions, FAQs, and troubleshooting steps in one clear place.",
                    "theme": "site",
                    "chips": ["Setup notes", "FAQ", "Troubleshooting"],
                    "actions": [
                        { "label": "Get Support", "url": "/support", "style": "primary" },
                        { "label": "See Changelog", "url": "/changelog", "style": "outline" }
                    ]
                }},
                { "type": "accordion", "data": { "items": [
                    { "title": "Getting started", "content": "Explain install steps, account setup, supported devices, and the fastest way to get value from the app." },
                    { "title": "Common questions", "content": "Answer pricing, privacy, compatibility, billing, login, notification, and data questions before users need to contact support." },
                    { "title": "When something breaks", "content": "Tell users what app version, device, operating system, screenshots, or logs help the team respond faster." }
                ]}}
            ]),
        ),
        page_with_blocks(
            "download",
            "Download",
            "Download {business_name}",
            "Find app store links, platform options, and installation notes for {business_name}.",
            serde_json::json!([
                { "type": "marketing-hero", "data": {
                    "kicker": "Download",
                    "title": "Choose the right version",
                    "text": "Put App Store, Google Play, desktop, web app, beta, and waitlist links here so users know exactly where to go.",
                    "theme": "site",
                    "chips": ["iOS", "Android", "Web"],
                    "actions": [
                        { "label": "Contact Support", "url": "/support", "style": "primary" },
                        { "label": "View Apps", "url": "/apps", "style": "outline" }
                    ]
                }},
                { "type": "feature-grid", "data": {
                    "columns": 3,
                    "items": [
                        { "title": "Apple App Store", "text": "Add the live iPhone, iPad, or macOS store link when the app is approved.", "url": "", "eyebrow": "iOS" },
                        { "title": "Google Play", "text": "Add Android listing, beta track, testing note, or compatibility guidance.", "url": "", "eyebrow": "Android" },
                        { "title": "Web or desktop", "text": "Add PWA, Windows, Mac, Linux, Steam, or direct download guidance when it applies.", "url": "", "eyebrow": "Other platforms" }
                    ]
                }}
            ]),
        ),
        page_with_blocks(
            "blog",
            "Blog",
            "{business_name} Blog",
            "News, product updates, launch notes, and technical articles from {business_name}.",
            serde_json::json!([
                { "type": "marketing-hero", "data": {
                    "kicker": "Product Updates",
                    "title": "{business_name} Blog",
                    "text": "News, launch notes, technical articles, feature explainers, and product updates from {business_name}.",
                    "theme": "site",
                    "chips": ["Launch notes", "Feature updates", "Technical articles"],
                    "actions": [
                        { "label": "Read Docs", "url": "/docs", "style": "primary" },
                        { "label": "Get Support", "url": "/support", "style": "outline" }
                    ]
                }},
                { "type": "feature-grid", "data": {
                    "columns": 3,
                    "items": [
                        { "title": "Launch notes", "text": "Explain new releases, store approvals, beta changes, and what users should try next.", "url": "/changelog", "eyebrow": "Releases" },
                        { "title": "Feature explainers", "text": "Turn product details into useful articles that help people understand the app before and after download.", "url": "/apps", "eyebrow": "Education" },
                        { "title": "Support context", "text": "Link common questions back to docs and support so users can solve issues without hunting.", "url": "/support", "eyebrow": "Help" }
                    ]
                }}
            ]),
        ),
        page_with_blocks(
            "changelog",
            "Changelog",
            "{business_name} Changelog",
            "Release notes, fixes, roadmap notes, and product updates from {business_name}.",
            serde_json::json!([
                { "type": "marketing-hero", "data": {
                    "kicker": "Changelog",
                    "title": "What changed in {business_name}",
                    "text": "Follow what has been released, what has been fixed, and what is coming next.",
                    "theme": "site",
                    "chips": ["Latest release", "Fixes", "Roadmap"],
                    "actions": [
                        { "label": "View Apps", "url": "/apps", "style": "primary" },
                        { "label": "Get Support", "url": "/support", "style": "outline" }
                    ]
                }},
                { "type": "feature-grid", "data": {
                    "columns": 3,
                    "items": [
                        { "title": "Latest release", "text": "Summarize the newest version, platform status, and the most important user-facing change.", "url": "", "eyebrow": "Now" },
                        { "title": "Fixes and polish", "text": "Record bug fixes, accessibility work, performance improvements, and compatibility updates.", "url": "", "eyebrow": "Improved" },
                        { "title": "What is next", "text": "Point users toward beta invites, planned features, feedback paths, or a support article.", "url": "/support", "eyebrow": "Next" }
                    ]
                }}
            ]),
        ),
    ];
    let mut steps = vec![
        step(
            "app_identity",
            "App Identity",
            false,
            vec![
                field_text(
                    "business_name",
                    "Publisher or App Name",
                    true,
                    "Pixel Studios",
                ),
                field_text("tagline", "Tagline", false, "Ship apps. Grow users."),
                field_textarea(
                    "mission",
                    "Short App Story",
                    false,
                    "What does the app help people do, and why should they trust it?",
                ),
            ],
        ),
        step(
            "contact_support",
            "Contact & Support",
            false,
            vec![
                field_text("email", "Support Email", true, "support@example.com"),
                field_text("phone", "Support Phone", false, "(555) 123-4567"),
                field_text(
                    "support_url",
                    "Support URL",
                    false,
                    "https://example.com/support",
                ),
                field_text("docs_url", "Docs URL", false, "https://example.com/docs"),
            ],
        ),
    ];
    steps.push(step(
        "apps",
        "Your Apps",
        false,
        vec![
            field_select(
                "app_count",
                "How Many Apps",
                true,
                &["1", "2-5", "6-20", "20+"],
            ),
            field_grid(
                "categories",
                "App Categories",
                true,
                &[
                    "Productivity",
                    "Social",
                    "Games",
                    "Education",
                    "Health",
                    "Finance",
                    "Entertainment",
                    "Utilities",
                    "Business",
                    "Photography",
                    "Music",
                    "News",
                    "Shopping",
                    "Travel",
                    "Food",
                    "Sports",
                    "Weather",
                    "Developer Tools",
                ],
            ),
        ],
    ));
    steps.push(step(
        "platforms",
        "Platforms",
        false,
        vec![
            field_grid(
                "stores",
                "App Stores",
                true,
                &[
                    "Apple App Store",
                    "Google Play",
                    "PWA",
                    "macOS App Store",
                    "Microsoft Store",
                    "Steam",
                    "Epic Games",
                    "Amazon Appstore",
                ],
            ),
            field_text(
                "primary_store_url",
                "Primary Store URL",
                true,
                "https://apps.apple.com/...",
            ),
        ],
    ));
    steps.push(step(
        "site_features",
        "Site Features",
        true,
        vec![field_grid(
            "features",
            "Features",
            false,
            &[
                "App Showcase",
                "Screenshot Gallery",
                "Feature Highlights",
                "Pricing Table",
                "FAQ",
                "Changelog",
                "Blog",
                "Press Kit",
                "Support/Docs",
                "Privacy Policy",
                "Terms of Service",
                "Contact Form",
            ],
        )],
    ));
    d.onboarding_steps = steps;
    d
}

// ── Small Group — Intimate Bible Study / Life Group ──────────────────────
// Warm leather-journal feel. Living room, not a church lobby.
// The #1 question every week: "When and where do we meet?"
// Prayer requests that don't scroll away in a group text.

fn small_group() -> SiteTypeDefinition {
    group_base(
        "small-group",
        "Small Group",
        "\u{1F56F}", // 🕯
        "Intimate small group website — prayer wall, study resources, discussion threads, and meeting schedule. A living room for your group.",
        "Together in prayer. Together in purpose.",
        50,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("this-week", "This Week", "/calendar", "\u{1F4C5}", 1),
            nav("prayer", "Prayer Wall", "/feed", "\u{1F64F}", 2),
            nav("study", "Study Resources", "/vault", "\u{1F4D6}", 3),
            nav("discuss", "Discussion", "/recipes", "\u{1F4AC}", 4),
            nav("members", "Our Group", "/shopping", "\u{1F465}", 5),
        ],
        vec![
            page(
                "calendar",
                "This Week",
                "{group} — This Week",
                "Next meeting, what to prepare, and who's coming.",
            ),
            page(
                "feed",
                "Prayer Wall",
                "{group} Prayer Wall",
                "Share prayer requests, mark answered prayers, and lift each other up.",
            ),
            page(
                "vault",
                "Study Resources",
                "{group} Resources",
                "Shared study guides, Bible passages, and discussion notes.",
            ),
            page(
                "recipes",
                "Discussion",
                "{group} Discussion",
                "Threaded conversations that don't get lost in a text thread.",
            ),
            page(
                "shopping",
                "Our Group",
                "{group} Members",
                "Names, faces, and how to reach each other.",
            ),
        ],
        group_onboarding(
            "small-group",
            "group-info",
            "Group Info",
            "Group Name",
            "Thursday Night Life Group",
            "Group Members",
        ),
        "{group} — {tagline}",
        "{group} small group website: prayer wall, study resources, meeting schedule, and discussion.",
    )
}

// ── Mission Team — Outreach / Mission Trips ─────────────────────────────
// Bold, purposeful, action-oriented. Fundraising thermometer meets
// packing checklist. "We're doing something that matters."

fn mission_team() -> SiteTypeDefinition {
    group_base(
        "mission-team",
        "Mission Team",
        "\u{1F30D}", // 🌍
        "Mission trip website — fundraising tracker, preparation checklist, team roster, itinerary, and field journal. Go with purpose.",
        "Called to serve. Ready to go.",
        51,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("fundraising", "Fundraising", "/chore-board", "\u{1F4B0}", 1),
            nav("prep", "Preparation", "/recipes", "\u{2705}", 2),
            nav("itinerary", "Itinerary", "/calendar", "\u{1F5FA}", 3),
            nav("journal", "Field Journal", "/feed", "\u{1F4F8}", 4),
            nav("team", "Our Team", "/shopping", "\u{1F91D}", 5),
            nav("docs", "Documents", "/vault", "\u{1F4C2}", 6),
        ],
        vec![
            page(
                "chore-board",
                "Fundraising",
                "{team} Fundraising",
                "Track support goals, share your personal fundraising page, and see team progress.",
            ),
            page(
                "recipes",
                "Preparation",
                "{team} Prep Checklist",
                "Passport, vaccines, training, packing — check off each item as you go.",
            ),
            page(
                "calendar",
                "Itinerary",
                "{team} Itinerary",
                "Day-by-day schedule, locations, contacts, and emergency info.",
            ),
            page(
                "feed",
                "Field Journal",
                "{team} Journal",
                "Photos, stories, and updates from the field — share with supporters back home.",
            ),
            page(
                "shopping",
                "Our Team",
                "{team} Roster",
                "Team members, roles, contact info, and fundraising progress.",
            ),
            page(
                "vault",
                "Documents",
                "{team} Documents",
                "Waivers, insurance, travel docs, and team agreements.",
            ),
        ],
        group_onboarding(
            "mission-team",
            "mission-info",
            "Mission Info",
            "Mission / Team Name",
            "Guatemala 2026 Mission Team",
            "Team Members",
        ),
        "{team} — {tagline}",
        "{team} mission website: fundraising, preparation checklist, itinerary, and field journal.",
    )
}

// ── Homeschool Co-op — Multi-Family Shared Teaching ─────────────────────
// Cheerful, collaborative, organized. Multiple families sharing the
// teaching load. "Who teaches what, when, and where?"

fn homeschool_coop() -> SiteTypeDefinition {
    group_base(
        "homeschool-coop",
        "Homeschool Co-op",
        "\u{1F3EB}", // 🏫
        "Co-op website — class schedule, teaching rotation, family directory, supply lists, attendance, and shared resources.",
        "Learning together. Teaching together.",
        52,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("classes", "Class Schedule", "/calendar", "\u{1F4DA}", 1),
            nav(
                "rotation",
                "Teaching Rotation",
                "/chore-board",
                "\u{1F501}",
                2,
            ),
            nav(
                "families",
                "Families",
                "/shopping",
                "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
                3,
            ),
            nav("supplies", "Supply Lists", "/recipes", "\u{270F}", 4),
            nav("board", "Co-op Board", "/feed", "\u{1F4E2}", 5),
            nav("resources", "Resource Library", "/vault", "\u{1F4DA}", 6),
        ],
        vec![
            page(
                "calendar",
                "Class Schedule",
                "{coop} Classes",
                "Day, time, subject, teacher, room, and age range for every class.",
            ),
            page(
                "chore-board",
                "Teaching Rotation",
                "{coop} Teaching Rotation",
                "Who's teaching what this semester. Request swaps and subs.",
            ),
            page(
                "shopping",
                "Families",
                "{coop} Families",
                "Every family, their kids, ages, and contact info.",
            ),
            page(
                "recipes",
                "Supply Lists",
                "{coop} Supplies",
                "What each class needs — checkable lists per semester.",
            ),
            page(
                "feed",
                "Co-op Board",
                "{coop} Announcements",
                "Co-op-wide news, field trip planning, and updates.",
            ),
            page(
                "vault",
                "Resource Library",
                "{coop} Resources",
                "Shared curriculum links, handouts, and recommended materials.",
            ),
        ],
        group_onboarding(
            "homeschool-coop",
            "coop-info",
            "Co-op Info",
            "Co-op Name",
            "Eastside Homeschool Co-op",
            "Participating Families",
        ),
        "{coop} — {tagline}",
        "{coop} homeschool co-op: class schedule, teaching rotation, families, and shared resources.",
    )
}

// ── Business Team — Small Work Team / Startup ───────────────────────────
// Clean, focused, professional but human. Basecamp's philosophy without
// the $299/mo price tag. "Let's get stuff done without the noise."

fn business_team() -> SiteTypeDefinition {
    group_base(
        "business-team",
        "Team Website",
        "\u{1F4BC}", // 💼
        "Small team website — task board, message threads, shared docs, weekly check-ins, and team calendar. Focus without the noise.",
        "Your team. One place. Always shipping.",
        53,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Tasks", "/chore-board", "\u{2705}", 1),
            nav("threads", "Threads", "/feed", "\u{1F4AC}", 2),
            nav("calendar", "Calendar", "/calendar", "\u{1F4C5}", 3),
            nav("docs", "Docs", "/vault", "\u{1F4C4}", 4),
            nav("team", "Team", "/shopping", "\u{1F465}", 5),
            nav("checkins", "Check-ins", "/recipes", "\u{1F4CB}", 6),
        ],
        vec![
            page(
                "chore-board",
                "Tasks",
                "{team} Tasks",
                "What needs doing, who's on it, and when it's due.",
            ),
            page(
                "feed",
                "Threads",
                "{team} Threads",
                "Async discussions by topic. Think before you post, reply when you're ready.",
            ),
            page(
                "calendar",
                "Calendar",
                "{team} Calendar",
                "Deadlines, meetings, milestones, and launches.",
            ),
            page(
                "vault",
                "Docs",
                "{team} Documents",
                "SOPs, meeting notes, specs, and shared files.",
            ),
            page(
                "shopping",
                "Team",
                "{team} Directory",
                "Who does what, how to reach them, and when they're available.",
            ),
            page(
                "recipes",
                "Check-ins",
                "{team} Check-ins",
                "Weekly pulse: what you shipped, what's next, any blockers.",
            ),
        ],
        group_onboarding(
            "business-team",
            "team-info",
            "Team Info",
            "Team / Company Name",
            "Acme Labs",
            "Team Members",
        ),
        "{team} — {tagline}",
        "{team} team website: tasks, threads, docs, check-ins, and calendar.",
    )
}

// ── Reunion — Family / Class / Military Reunion ─────────────────────────
// Nostalgic, celebratory, warm. A scrapbook that comes alive once a year
// and holds memories forever. "Come home to your people."

fn reunion() -> SiteTypeDefinition {
    group_base(
        "reunion",
        "Reunion Website",
        "\u{1F91D}", // 🤝
        "Reunion website — RSVP tracking, event schedule, potluck sign-ups, photo albums, memory wall, and cost splitting.",
        "Come home to your people.",
        54,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("event", "Event Details", "/calendar", "\u{1F4CD}", 1),
            nav("rsvp", "RSVP", "/chore-board", "\u{2709}", 2),
            nav("potluck", "Potluck Sign-up", "/recipes", "\u{1F372}", 3),
            nav("photos", "Photo Album", "/vault", "\u{1F4F7}", 4),
            nav("memories", "Memory Wall", "/feed", "\u{1F4AD}", 5),
            nav("directory", "Who's Coming", "/shopping", "\u{1F465}", 6),
            nav("games", "Reunion Games", "/games", "\u{1F389}", 7),
        ],
        vec![
            page(
                "calendar",
                "Event Details",
                "{reunion} Details",
                "When, where, lodging options, directions, and the full schedule.",
            ),
            page(
                "chore-board",
                "RSVP",
                "{reunion} RSVP",
                "Who's coming, plus-ones, dietary needs, and arrival dates.",
            ),
            page(
                "recipes",
                "Potluck Sign-up",
                "{reunion} Potluck",
                "Who's bringing what — sign up for a dish, see what's covered.",
            ),
            page(
                "vault",
                "Photo Album",
                "{reunion} Photos",
                "Upload, browse, and relive the moments together.",
            ),
            page(
                "feed",
                "Memory Wall",
                "{reunion} Memories",
                "\"Remember when...\" — stories, throwback photos, and shared history.",
            ),
            page(
                "shopping",
                "Who's Coming",
                "{reunion} Directory",
                "The full guest list with contact info.",
            ),
            page(
                "games",
                "Reunion Games",
                "{reunion} Games",
                "Trivia, bingo, and activities to bring everyone together.",
            ),
        ],
        group_onboarding(
            "reunion",
            "reunion-info",
            "Reunion Info",
            "Reunion Name",
            "Johnson Family Reunion 2026",
            "Organizers",
        ),
        "{reunion} — {tagline}",
        "{reunion} website: RSVP, event details, potluck, photo albums, and shared memories.",
    )
}

// ── Memorial — Remembering a Loved One ──────────────────────────────────
// Quiet dignity. Candlelight, not neon. A permanent, private space
// that families return to on birthdays and anniversaries.
// "They mattered, and this space proves it."

fn memorial() -> SiteTypeDefinition {
    group_base(
        "memorial",
        "Memorial Space",
        "\u{1F56F}", // 🕯
        "A gentle, permanent space to remember together — their story, shared memories, photos, favorite recipes, and important dates.",
        "Remembered. Cherished. Never forgotten.",
        55,
        vec![
            nav("home", "Home", "/", "\u{1F56F}", 0),
            nav("story", "Their Story", "/their-story", "\u{1F4D6}", 1),
            nav("memories", "Memories", "/memories", "\u{1F4AD}", 2),
            nav("photos", "Photos", "/photos", "\u{1F4F7}", 3),
            nav("dates", "Important Dates", "/calendar", "\u{1F4C5}", 4),
            nav("guestbook", "Guestbook", "/guestbook", "\u{270D}", 5),
        ],
        vec![
            page(
                "recipes",
                "Their Story",
                "Remembering {name}",
                "A life lived fully — their story, in the words of those who loved them.",
            ),
            page(
                "feed",
                "Memories",
                "{name} — Memories",
                "Stories, moments, and remembrances from family and friends.",
            ),
            page(
                "vault",
                "Photos",
                "{name} — Photos",
                "A lifetime in pictures — childhood, milestones, and everyday moments.",
            ),
            page(
                "calendar",
                "Important Dates",
                "{name} — Important Dates",
                "Birthday, anniversary, and other dates that bring us together to remember.",
            ),
            page(
                "shopping",
                "Guestbook",
                "{name} — Guestbook",
                "Leave a message, light a candle, share what they meant to you.",
            ),
        ],
        group_onboarding(
            "memorial",
            "memorial-info",
            "Memorial Info",
            "Name of Loved One",
            "John David Smith",
            "Family Members",
        ),
        "In Memory of {name} — {tagline}",
        "A private memorial space for {name}. Shared memories, photos, and stories from those who loved them.",
    )
}

fn creator() -> SiteTypeDefinition {
    let ts = now();
    SiteTypeDefinition {
        slug: "creator".into(),
        name: "Creator".into(),
        emoji: "\u{1F3AC}".into(),
        category: "free".into(),
        description: "Link-in-bio creator site with smart links, content calendar, audience space, and digital products.".into(),
        default_tagline: "Create. Share. Grow.".into(),
        publicly_listed: true,
        display_order: 2,
        enabled_modules: vec![
            "creator-profile", "smart-links", "content-calendar", "audience-hub",
            "digital-products", "brand-deals", "analytics-command", "creator-community",
            "media-hub", "collab-network", "theme-studio", "seo", "blog", "dashboard",
            "smtp", "forms", "help-feedback", "onboarding", "company-profile",
            "industry-profile", "media-manager", "email-marketing", "commerce",
            "messaging", "notifications", "site-pages",
            // site-pages requires service-catalog/booking/customer-portal at
            // registration time. They don't surface in the creator UI but
            // must be enabled or the daemon panics. Same shape as blog().
            // (AUD-019)
            "service-catalog", "booking", "customer-portal",
        ].into_iter().map(String::from).collect(),
        theme_presets: presets::generate_presets(&presets::palette_for("creator"), "creator"),
        theme_profile: None,
        default_nav_items: vec![
            NavItem { item_id: "home".into(), title: "Home".into(), url: "/".into(), emoji: "\u{1F3E0}".into(), position: 0, children: vec![] },
            NavItem { item_id: "links".into(), title: "Links".into(), url: "/links".into(), emoji: "\u{1F517}".into(), position: 1, children: vec![] },
            NavItem { item_id: "about".into(), title: "About".into(), url: "/about".into(), emoji: "\u{1F464}".into(), position: 2, children: vec![] },
            NavItem { item_id: "blog".into(), title: "Blog".into(), url: "/blog".into(), emoji: "\u{270D}\u{FE0F}".into(), position: 3, children: vec![] },
            NavItem { item_id: "products".into(), title: "Products".into(), url: "/products".into(), emoji: "\u{1F4E6}".into(), position: 4, children: vec![] },
            NavItem { item_id: "work-with-me".into(), title: "Work With Me".into(), url: "/work-with-me".into(), emoji: "\u{1F91D}".into(), position: 5, children: vec![] },
        ],
        default_pages: vec![
            DefaultPage { slug: "links".into(), title: "Links".into(), blocks: None, seo_title: "{creator_name} Links".into(), seo_description: "Find {creator_name}'s main links, channels, resources, products, and ways to follow or subscribe in one place.".into() },
            DefaultPage { slug: "about".into(), title: "About {creator_name}".into(), blocks: None, seo_title: "About {creator_name}".into(), seo_description: "Learn what {creator_name} creates, who the site is for, and where new visitors should start.".into() },
            DefaultPage { slug: "products".into(), title: "Products and Resources".into(), blocks: None, seo_title: "{creator_name} Products and Resources".into(), seo_description: "Browse downloads, offers, recommendations, memberships, or resources from {creator_name}.".into() },
            DefaultPage { slug: "work-with-me".into(), title: "Work With {creator_name}".into(), blocks: None, seo_title: "Work With {creator_name}".into(), seo_description: "See collaboration options, media-kit details, audience fit, and the best way to contact {creator_name}.".into() },
        ],
        homepage_blocks: Some(serde_json::json!([
            { "type": "company-hero", "data": {
                "headline": "{creator_name}",
                "description": "{tagline}",
                "show_cta": true,
                "cta_text": "See What's New",
                "cta_url": "/links"
            }},
            { "type": "family-quick-access", "data": {
                "title": "All Things {creator_name}",
                "subtitle": "Everything in one place — pick where you want to go next.",
                "cards": [
                    { "icon": "\u{1F517}", "label": "Links", "description": "Main channels, latest content, and the easiest places to follow.", "url": "/links" },
                    { "icon": "\u{270D}\u{FE0F}", "label": "Blog", "description": "Posts, essays, and notes from behind the scenes.", "url": "/blog" },
                    { "icon": "\u{1F4E6}", "label": "Products", "description": "Downloads, recommendations, memberships, and resources.", "url": "/products" },
                    { "icon": "\u{1F4E8}", "label": "Newsletter", "description": "Get new content in your inbox before it hits the feeds.", "url": "/newsletter" },
                    { "icon": "\u{1F464}", "label": "About", "description": "What I create, who it's for, and where new visitors should start.", "url": "/about" },
                    { "icon": "\u{1F91D}", "label": "Work With Me", "description": "Collaboration, sponsorships, and partnership opportunities.", "url": "/work-with-me" }
                ]
            }},
            { "type": "heading", "data": { "level": 2, "text": "Latest from the Blog" } },
            { "type": "recent-posts", "data": { "limit": 4, "columns": 2 } },
            { "type": "cta-section", "data": {
                "title": "Stay in Touch",
                "cta_text": "Subscribe",
                "cta_url": "/newsletter"
            }}
        ])),
        default_tone: "casual".into(),
        default_brand_colors: BrandColors { primary: "#8b5cf6".into(), secondary: "#7c3aed".into(), accent: "#a78bfa".into() },
        default_tier: "founding".into(),
        always_free: true,
        price_override_cents: 0,
        discount_codes: vec![],
        limited_time_offer: Some(LimitedOffer {
            label: "Free during early release".into(),
            description: "7-day free trial, no card required.".into(),
            expires_at: 0,
            tier_override: "founding".into(),
        }),
        onboarding_steps: vec![
            step(
                "creator_profile",
                "Your Profile",
                false,
                vec![
                    field_text("creator_name", "Creator / Brand Name", true, "Your name or brand"),
                    field_select(
                        "niche",
                        "Niche / Category",
                        false,
                        &[
                            "Lifestyle",
                            "Tech",
                            "Fashion & Beauty",
                            "Food & Cooking",
                            "Fitness & Health",
                            "Gaming",
                            "Education",
                            "Music",
                            "Art & Design",
                            "Business & Finance",
                            "Travel",
                            "Comedy & Entertainment",
                            "Other",
                        ],
                    ),
                    field_select(
                        "creator_style",
                        "Creator Style",
                        false,
                        &[
                            "Personal brand",
                            "Studio / brand account",
                            "Educator / expert",
                            "Performer / entertainer",
                            "Coach / consultant",
                            "Community-led brand",
                        ],
                    ),
                    field_textarea(
                        "creator_story",
                        "What do you want people to know about you?",
                        false,
                        "A short intro about your voice, what you make, and what followers come to you for",
                    ),
                ],
            ),
            step(
                "creator_audience",
                "Audience & Offers",
                true,
                vec![
                    field_textarea(
                        "primary_audience",
                        "Who is this for?",
                        false,
                        "Describe the people you want to reach and what they care about",
                    ),
                    field_grid(
                        "creator_goals",
                        "What should this site help you do?",
                        false,
                        &[
                            "Grow followers",
                            "Sell digital products",
                            "Book collaborations",
                            "Collect email subscribers",
                            "Promote events",
                            "Drive affiliate clicks",
                            "Share media kit / brand deals",
                            "Build community",
                        ],
                    ),
                    field_grid(
                        "offers",
                        "What do you offer today?",
                        false,
                        &[
                            "Free newsletter",
                            "Digital download",
                            "Course / workshop",
                            "Coaching / consulting",
                            "Affiliate recommendations",
                            "Community membership",
                            "Brand collaborations",
                            "Speaking / appearances",
                        ],
                    ),
                ],
            ),
            step(
                "creator_platforms",
                "Platforms & Content",
                true,
                vec![
                    field_select(
                        "primary_platform",
                        "Primary Platform",
                        false,
                        &[
                            "Instagram",
                            "TikTok",
                            "YouTube",
                            "Podcast",
                            "Newsletter",
                            "Blog",
                            "X / Twitter",
                            "LinkedIn",
                            "Pinterest",
                            "Other",
                        ],
                    ),
                    field_text("youtube", "YouTube URL", false, "https://youtube.com/@you"),
                    field_text("instagram", "Instagram", false, "@yourhandle"),
                    field_text("tiktok", "TikTok", false, "@yourhandle"),
                    field_grid(
                        "content_formats",
                        "Content Formats",
                        false,
                        &[
                            "Short-form video",
                            "Long-form video",
                            "Newsletter",
                            "Written articles",
                            "Podcast / audio",
                            "Photography",
                            "Livestreams",
                            "Downloads / resources",
                        ],
                    ),
                ],
            ),
            step(
                "creator_growth",
                "Growth Setup",
                true,
                vec![
                    field_select(
                        "posting_cadence",
                        "Publishing Rhythm",
                        false,
                        &["Daily", "Several times a week", "Weekly", "Biweekly", "Monthly", "Launch based"],
                    ),
                    field_toggle("newsletter_capture", "Collect email subscribers from day one"),
                    field_toggle("media_kit", "Show a media kit / collaboration page"),
                    field_toggle("shop_focus", "Feature products, downloads, or paid offers prominently"),
                ],
            ),
            step(
                "creator_features",
                "Features",
                true,
                vec![field_grid(
                    "features",
                    "Enable Features",
                    false,
                    &[
                        "Smart Links Page",
                        "Content Calendar",
                        "Digital Products",
                        "Audience Space",
                        "Brand Deals Tracker",
                        "Analytics",
                        "Email Newsletter",
                        "Blog",
                        "Media Kit / Partnerships",
                        "Event / Appearance Requests",
                    ],
                )],
            ),
        ],
        seo_title_template: "{creator_name} — {tagline}".into(),
        seo_description_template: "{creator_name}: creator, content maker, and digital entrepreneur.".into(),
        created_at: ts,
        updated_at: ts,
    }
}

fn blog() -> SiteTypeDefinition {
    let ts = now();
    SiteTypeDefinition {
        slug: "blog".into(),
        name: "Blog".into(),
        emoji: "\u{270D}\u{FE0F}".into(),
        category: "free".into(),
        description:
            "Blog and writing platform with SEO tools, email marketing, and content pipeline."
                .into(),
        default_tagline: "Write. Publish. Grow.".into(),
        publicly_listed: true,
        display_order: 3,
        enabled_modules: vec![
            "blog",
            "theme-studio",
            "seo",
            "dashboard",
            "smtp",
            "forms",
            "help-feedback",
            "onboarding",
            "company-profile",
            "industry-profile",
            "location-profile",
            "media-manager",
            "email-marketing",
            "content-pipeline",
            "site-pages",
            // site-pages depends on company-profile, service-catalog, booking,
            // customer-portal at registration time. The latter three aren't
            // surfaced in the blog UI but must be enabled or the daemon panics.
            // (AUD-019)
            "service-catalog",
            "booking",
            "customer-portal",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        theme_presets: presets::generate_presets(&presets::palette_for("blog"), "blog"),
        theme_profile: None,
        default_nav_items: vec![
            NavItem {
                item_id: "home".into(),
                title: "Home".into(),
                url: "/".into(),
                emoji: "\u{1F3E0}".into(),
                position: 0,
                children: vec![],
            },
            NavItem {
                item_id: "start-here".into(),
                title: "Start Here".into(),
                url: "/start-here".into(),
                emoji: "\u{1F4CD}".into(),
                position: 1,
                children: vec![],
            },
            NavItem {
                item_id: "blog".into(),
                title: "Blog".into(),
                url: "/blog".into(),
                emoji: "\u{270D}\u{FE0F}".into(),
                position: 2,
                children: vec![],
            },
            NavItem {
                item_id: "topics".into(),
                title: "Topics".into(),
                url: "/topics".into(),
                emoji: "\u{1F5C2}".into(),
                position: 3,
                children: vec![],
            },
            NavItem {
                item_id: "about".into(),
                title: "About".into(),
                url: "/about".into(),
                emoji: "\u{1F464}".into(),
                position: 4,
                children: vec![],
            },
            NavItem {
                item_id: "contact".into(),
                title: "Contact".into(),
                url: "/contact".into(),
                emoji: "\u{1F4E7}".into(),
                position: 5,
                children: vec![],
            },
            NavItem {
                item_id: "newsletter".into(),
                title: "Newsletter".into(),
                url: "/newsletter".into(),
                emoji: "\u{1F4E8}".into(),
                position: 6,
                children: vec![],
            },
        ],
        default_pages: vec![
            DefaultPage {
                slug: "start-here".into(),
                title: "Start Here".into(),
                blocks: None,
                seo_title: "Start Here — {blog_name}".into(),
                seo_description: "A first-stop guide to {blog_name}: what the blog covers, who it is for, and which topics to read first.".into(),
            },
            DefaultPage {
                slug: "topics".into(),
                title: "Topics".into(),
                blocks: None,
                seo_title: "{blog_name} Topics".into(),
                seo_description: "Explore the main themes, article types, and reader resources on {blog_name}.".into(),
            },
            DefaultPage {
                slug: "about".into(),
                title: "About".into(),
                blocks: None,
                seo_title: "About {blog_name}".into(),
                seo_description: "Learn who writes {blog_name}, who it helps, and what readers can expect from the publication.".into(),
            },
            DefaultPage {
                slug: "contact".into(),
                title: "Contact".into(),
                blocks: None,
                seo_title: "Contact {blog_name}".into(),
                seo_description: "Contact {blog_name} for reader questions, collaborations, tips, or publication feedback.".into(),
            },
            DefaultPage {
                slug: "newsletter".into(),
                title: "Newsletter".into(),
                blocks: None,
                seo_title: "{blog_name} Newsletter".into(),
                seo_description: "Subscribe or learn what the {blog_name} newsletter covers, how often it sends, and who it is written for.".into(),
            },
        ],
        homepage_blocks: Some(serde_json::json!([
            { "type": "company-hero", "data": {
                "headline": "{blog_name}",
                "description": "{tagline}",
                "show_cta": true,
                "cta_text": "Read Latest Posts",
                "cta_url": "/blog"
            }},
            { "type": "family-quick-access", "data": {
                "title": "Where to Start",
                "subtitle": "Pick a path that matches what you're looking for.",
                "cards": [
                    { "icon": "\u{1F4CD}", "label": "Start Here", "description": "New to the blog? A short guide to what's here and who it's for.", "url": "/start-here" },
                    { "icon": "\u{270D}\u{FE0F}", "label": "Latest Posts", "description": "The most recent articles, essays, and field notes.", "url": "/blog" },
                    { "icon": "\u{1F5C2}", "label": "Topics", "description": "Browse by theme — pick what you want to dig into.", "url": "/topics" },
                    { "icon": "\u{1F4E8}", "label": "Newsletter", "description": "Get new posts in your inbox. No spam, easy to unsubscribe.", "url": "/newsletter" },
                    { "icon": "\u{1F464}", "label": "About", "description": "Who writes this and what to expect from the publication.", "url": "/about" },
                    { "icon": "\u{1F4E7}", "label": "Contact", "description": "Questions, tips, or collaboration ideas — get in touch.", "url": "/contact" }
                ]
            }},
            { "type": "heading", "data": { "level": 2, "text": "Recent Posts" } },
            { "type": "recent-posts", "data": { "limit": 6, "columns": 3 } },
            { "type": "cta-section", "data": {
                "title": "Stay in the Loop",
                "cta_text": "Subscribe to the Newsletter",
                "cta_url": "/newsletter"
            }}
        ])),
        default_tone: "conversational".into(),
        default_brand_colors: BrandColors {
            primary: "#0f172a".into(),
            secondary: "#1e293b".into(),
            accent: "#3b82f6".into(),
        },
        default_tier: "founding".into(),
        always_free: true,
        price_override_cents: 0,
        discount_codes: vec![],
        limited_time_offer: Some(LimitedOffer {
            label: "Free during early release".into(),
            description: "7-day free trial, no card required.".into(),
            expires_at: 0,
            tier_override: "founding".into(),
        }),
        onboarding_steps: vec![
            step(
                "blog_profile",
                "Your Blog",
                false,
                vec![
                    field_text("blog_name", "Blog Name", true, "My Awesome Blog"),
                    field_select(
                        "blog_topic",
                        "Main Topic",
                        false,
                        &[
                            "Personal / Journal",
                            "Tech & Programming",
                            "Business & Marketing",
                            "Lifestyle",
                            "Food & Recipes",
                            "Travel",
                            "Parenting",
                            "Finance",
                            "Health & Wellness",
                            "Creative Writing",
                            "Other",
                        ],
                    ),
                    field_textarea("bio", "Short Bio", false, "A sentence or two about you"),
                ],
            ),
            step(
                "blog_audience",
                "Audience & Positioning",
                true,
                vec![
                    field_textarea(
                        "reader_audience",
                        "Who do you write for?",
                        false,
                        "Describe the readers you want to help, entertain, or attract",
                    ),
                    field_select(
                        "blog_goal",
                        "Main Goal",
                        false,
                        &[
                            "Share ideas / journal",
                            "Grow an audience",
                            "Build authority",
                            "Support a business",
                            "Promote services",
                            "Sell products",
                            "Earn affiliate income",
                            "Grow a newsletter",
                        ],
                    ),
                    field_grid(
                        "content_pillars",
                        "Core Content Pillars",
                        false,
                        &[
                            "How-to guides",
                            "Opinion / essays",
                            "News / commentary",
                            "Case studies",
                            "Interviews",
                            "Personal stories",
                            "Reviews / recommendations",
                            "Resource roundups",
                        ],
                    ),
                ],
            ),
            step(
                "blog_editorial",
                "Editorial Plan",
                true,
                vec![
                    field_select(
                        "publishing_cadence",
                        "Publishing Cadence",
                        false,
                        &["Several times a week", "Weekly", "Biweekly", "Monthly", "Seasonal / batch published"],
                    ),
                    field_toggle("newsletter", "Pair posts with an email newsletter"),
                    field_toggle("lead_magnet", "Offer a free guide, checklist, or download"),
                    field_textarea(
                        "editorial_notes",
                        "What topics or article types matter most?",
                        false,
                        "Examples: beginner guides, local commentary, recipes, product reviews, founder notes, thought leadership",
                    ),
                ],
            ),
            step(
                "blog_features",
                "Features",
                true,
                vec![field_grid(
                    "features",
                    "Enable Features",
                    false,
                    &[
                        "Blog with Categories",
                        "Email Subscribers",
                        "SEO Tools",
                        "Analytics",
                        "Contact Form",
                        "Portfolio / Pages",
                        "Lead Magnet / Download",
                        "Author Profile Page",
                    ],
                )],
            ),
        ],
        seo_title_template: "{blog_name}".into(),
        seo_description_template: "{blog_name}: thoughts, stories, and ideas.".into(),
        created_at: ts,
        updated_at: ts,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Group Types — free sites using the family module set with terminology remapping
// ═══════════════════════════════════════════════════════════════════════════

/// Shared module list for all group types.
///
/// AUD-023 fix · `vacation-planner`, `homeschool`, and `family-network`
/// were previously in this shared core list and showed up in the nav of
/// every group-type tenant — including pet-owners / church / mission /
/// business-team / etc. where they have no purpose. They now live in
/// `group_add_on_modules()` and ship only to the slugs where they fit
/// (family / travel / reunion / homeschool / homeschool-coop / memorial).
fn group_core_modules() -> Vec<String> {
    vec![
        "family-members",
        "family-dashboard",
        "chore-board",
        "booking",
        "service-catalog",
        "invoicing",
        "contracts",
        "work-orders",
        "family-feed",
        "family-calendar",
        "private-spaces",
        "family-recipes",
        "family-shopping",
        "family-games",
        "family-vault",
        "vault",
        "profile-selector",
        "orbit",
        "theme-studio",
        "seo",
        "dashboard",
        "dashboard-themes",
        "smtp",
        "forms",
        "messaging",
        "notifications",
        "help-feedback",
        "onboarding",
        "company-profile",
        "industry-profile",
        "media-manager",
        // site-blueprint exposes /api/modules/site-blueprint/setup/apply,
        // which the provisioning script and pre-create wizard call to
        // pre-populate CompanyProfile + industry aggregates. Without it,
        // every freshly provisioned community/family site starts empty.
        "site-blueprint",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn push_unique_modules(mods: &mut Vec<String>, extras: &[&str]) {
    for extra in extras {
        if !mods.iter().any(|existing| existing == extra) {
            mods.push((*extra).to_string());
        }
    }
}

/// Low-risk add-ons for the newer community platform recovery.
///
/// These are wired before the riskier owner swaps like pantry -> inventory.
/// `vault` now coexists with the legacy `family-vault` APIs and routes, so new
/// sites can light up the newer vault hub without losing access to old records.
fn group_add_on_modules(slug: &str) -> Vec<String> {
    // AUD-023 fix · `vacation-planner` / `homeschool` / `family-network`
    // moved out of group_core_modules so they're only enabled on the
    // slugs where they fit. Each per-slug arm below explicitly opts in.
    let extras = match slug {
        "family" => &[
            "family-inventory",
            "vault",
            "vacation-planner",  // family vacations
            "homeschool",        // family that homeschools
            "family-network",    // extended family / reunions
        ][..],
        "church" => &[
            "fundraising",
            "rsvp",
            "group-ai",
            "group-identity",
            "archive",
            "vault",
        ],
        "small-group" => &["reading-progress", "group-ai", "group-identity", "vault"],
        "mission-team" => &[
            "fundraising",
            "packing-list",
            "expense-split",
            "rsvp",
            "archive",
            "group-ai",
            "group-identity",
            "vault",
        ],
        "homeschool" => &[
            "reading-progress",
            "compliance-export",
            "group-ai",
            "group-identity",
            "vault",
            "homeschool",  // AUD-023: the homeschool module IS for homeschool tenants
        ],
        "homeschool-coop" => &[
            "compliance-export",
            "rsvp",
            "reading-progress",
            "group-ai",
            "group-identity",
            "vault",
            "homeschool",  // AUD-023: co-ops still want the homeschool module
        ],
        "classroom" => &["reading-progress", "group-ai", "group-identity", "vault"],
        "sports-team" => &[
            "family-inventory",
            "fundraising",
            "rsvp",
            "archive",
            "group-identity",
            "vault",
            // Commerce stack so the team-shop block has a real /products
            // backend (jerseys, hats, season passes, snack-shack vouchers).
            // The team can disable in admin if they don't want a store.
            "commerce",
            "universal-cart",
            "checkout-pipeline",
            // Media-hub powers a coach podcast / post-game show feed
            // (already public via /api/modules/media-hub/rss).
            "media-hub",
            // Event-social adds chat + photos pinned to game events.
            "event-social",
            // Phase 3: paid tickets with Stripe checkout + QR scan-in.
            // Useful for season passes, tournament gates, fundraisers.
            "tickets",
            // Phase 3: gift cards / concession vouchers — buyer pays
            // online, gets a code with a balance, redeems at the snack
            // shack or pro shop.
            "vouchers",
        ],
        "club" => &[
            "rsvp",
            "expense-split",
            "archive",
            "group-identity",
            "vault",
        ],
        "travel" => &[
            "packing-list",
            "expense-split",
            "rsvp",
            "archive",
            "group-identity",
            "vault",
            "vacation-planner",  // AUD-023: travel sites are exactly the right home for this
        ],
        "wedding" => &[
            "rsvp",
            "expense-split",
            "packing-list",
            "archive",
            "group-identity",
            "vault",
        ],
        "farm" => &[
            "family-inventory",
            "commerce",
            "cart",
            "checkout-pipeline",
            "rsvp",
            "archive",
            "group-ai",
            "group-identity",
            "vault",
        ],
        "reunion" => &[
            "rsvp",
            "expense-split",
            "archive",
            "group-identity",
            "vault",
            "family-network",   // AUD-023: family-of-origin tree fits a reunion
            "vacation-planner", // AUD-023: reunion-trip planning
        ],
        "memorial" => &[
            "archive",
            "group-ai",
            "group-identity",
            "vault",
            "family-network",  // AUD-023: family of the deceased uses the family tree
        ],
        "pet-owners" => &["family-inventory", "vault"],
        "nonprofit" => &[
            "commerce",
            "fundraising",
            "rsvp",
            "archive",
            "group-ai",
            "group-identity",
            "vault",
        ],
        "business-team" | "business" => &["archive", "group-identity", "vault"],
        "maker-space" => &["family-inventory", "archive", "group-ai", "vault"],
        "band" => &[
            "family-inventory",
            "archive",
            "group-ai",
            "group-identity",
            "vault",
        ],
        "book-club" => &["reading-progress", "archive", "group-ai", "vault"],
        _ => &[],
    };
    extras.iter().map(|extra| (*extra).to_string()).collect()
}

fn group_modules(slug: &str) -> Vec<String> {
    let mut mods = group_core_modules();
    let extras = group_add_on_modules(slug);
    for extra in extras {
        if !mods.contains(&extra) {
            mods.push(extra);
        }
    }
    mods
}

/// Nav item shorthand.
fn nav(id: &str, title: &str, url: &str, emoji: &str, pos: u32) -> NavItem {
    NavItem {
        item_id: id.into(),
        title: title.into(),
        url: url.into(),
        emoji: emoji.into(),
        position: pos,
        children: vec![],
    }
}

fn with_reward_bank_nav(mut items: Vec<NavItem>) -> Vec<NavItem> {
    if items.iter().any(|item| item.url == "/chore-board/bank") {
        return items;
    }
    let insert_at = items
        .iter()
        .position(|item| item.url == "/chore-board")
        .map(|idx| idx + 1)
        .unwrap_or(items.len());
    items.insert(
        insert_at,
        nav(
            "bank",
            "Bank",
            "/chore-board/bank",
            "\u{1F4B0}",
            insert_at as u32,
        ),
    );
    items.insert(
        insert_at + 1,
        nav(
            "reward-store",
            "Rewards",
            "/chore-board/store",
            "\u{1F381}",
            (insert_at + 1) as u32,
        ),
    );
    for (idx, item) in items.iter_mut().enumerate() {
        item.position = idx as u32;
    }
    items
}

/// Default page shorthand.
fn page(slug: &str, title: &str, seo_title: &str, seo_desc: &str) -> DefaultPage {
    DefaultPage {
        slug: slug.into(),
        title: title.into(),
        blocks: None,
        seo_title: seo_title.into(),
        seo_description: seo_desc.into(),
    }
}

fn page_with_blocks(
    slug: &str,
    title: &str,
    seo_title: &str,
    seo_desc: &str,
    blocks: serde_json::Value,
) -> DefaultPage {
    DefaultPage {
        slug: slug.into(),
        title: title.into(),
        blocks: Some(blocks),
        seo_title: seo_title.into(),
        seo_description: seo_desc.into(),
    }
}

fn simple_public_page_blocks(
    kicker: &str,
    heading: &str,
    body: &str,
    bullets: &[&str],
    cta_text: &str,
    cta_url: &str,
) -> serde_json::Value {
    let bullet_html = bullets
        .iter()
        .map(|item| format!("<li>{}</li>", item))
        .collect::<Vec<_>>()
        .join("");
    let cta_html = if cta_text.is_empty() || cta_url.is_empty() {
        String::new()
    } else {
        format!(
            "<p style=\"margin-top:24px;\"><a class=\"btn btn-primary\" href=\"{}\">{}</a></p>",
            cta_url, cta_text
        )
    };
    serde_json::json!([
        {"type":"custom-html","data":{"html":format!(
            "<section class=\"section\"><div class=\"container\" style=\"max-width:940px;\">\
                <p style=\"text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#1e3a5f);font-weight:800;margin:0 0 10px;\">{}</p>\
                <h1 style=\"font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;\">{}</h1>\
                <p style=\"font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);max-width:760px;\">{}</p>\
                <div style=\"margin-top:26px;border:1px solid #e2e8f0;border-radius:8px;padding:22px;background:var(--color-surface,#fff);\">\
                    <h2 style=\"margin:0 0 12px;font-size:1.2rem;\">What this page makes easy</h2>\
                    <ul style=\"margin:0;padding-left:20px;line-height:1.8;color:var(--color-text,#334155);\">{}</ul>\
                </div>{}\
            </div></section>",
            kicker, heading, body, bullet_html, cta_html
        )}}
    ])
}

/// Per-type extras injected between the roster grid and the Quick Access
/// tile grid in `group_base`'s default homepage. Each block listed here
/// is also part of build_grounded_homepage_blocks's community branch so
/// re-running setup/apply reaches a consistent state.
fn community_homepage_extras(slug: &str) -> Vec<serde_json::Value> {
    let mut extras = Vec::new();
    if matches!(slug, "sports-team" | "band" | "scouts") {
        extras.push(serde_json::json!({
            "type": "team-shop",
            "data": {
                "columns": 3,
                "limit": 6,
                "subtitle": "Gear, jerseys, tickets, and concession vouchers — checkout via secure cart."
            }
        }));
    }
    if matches!(
        slug,
        "church" | "mission-team" | "nonprofit" | "small-group" | "fitness"
    ) {
        extras.push(serde_json::json!({
            "type": "fundraising-thermometer",
            "data": {}
        }));
    }
    if matches!(slug, "wedding" | "reunion") {
        extras.push(serde_json::json!({
            "type": "rsvp-summary",
            "data": {}
        }));
    }
    extras
}

fn group_base(
    slug: &str,
    name: &str,
    emoji: &str,
    desc: &str,
    tagline: &str,
    order: u32,
    nav_items: Vec<NavItem>,
    default_pages: Vec<DefaultPage>,
    onboarding_steps: Vec<OnboardingStep>,
    seo_title_tpl: &str,
    seo_desc_tpl: &str,
) -> SiteTypeDefinition {
    let ts = now();
    let p = presets::palette_for(slug);
    SiteTypeDefinition {
        slug: slug.into(),
        name: name.into(),
        emoji: emoji.into(),
        category: "free".into(),
        description: desc.into(),
        default_tagline: tagline.into(),
        publicly_listed: true,
        display_order: order,
        enabled_modules: group_modules(slug),
        theme_presets: presets::generate_presets(&p, slug),
        theme_profile: Some(serde_json::json!({
            "tokens": {
                "primary": p.primary, "accent": p.accent, "link": p.primary,
                "button_text": "#ffffff", "header_bg": p.light_bg, "header_text": p.dark_text,
                "background": p.light_bg, "surface": "#ffffff", "text": p.dark_text,
                "radius": 20, "container": 1000, "brand_size": 48, "nav_size": 15,
                "body_size": 16, "body_font": "Humanist", "full_bleed": true
            },
            "header": { "enabled": true, "sticky": false },
            "footer": { "enabled": false }
        })),
        default_nav_items: nav_items,
        default_pages,
        // Default homepage layout for every group/community site.
        // Universal sequence: hero → roster → (per-type extras) →
        // Quick Access tile grid → Today's Snapshot.
        //
        // Per-type extras are injected by `community_homepage_extras`:
        //   - sports-team / band / scouts → `team-shop` (commerce-backed
        //     merch + tickets via universal-cart)
        //   - church / mission-team / nonprofit / small-group / fitness
        //     → `fundraising-thermometer` (Stripe-wired in Phase 3)
        //   - wedding / reunion / memorial → `rsvp-summary`
        //
        // Admins can soft-hide any of these via the editor's visibility
        // toggle; the same block code renders the right title per type
        // because smart_blocks::render_team_shop / render_fundraising_*
        // branch on `ctx.industry_slug` for the headline copy.
        homepage_blocks: Some({
            let mut blocks = vec![
                serde_json::json!({ "type": "company-hero", "data": { "headline": "{group_name}", "description": "{tagline}", "show_cta": false } }),
                serde_json::json!({ "type": "roster-grid", "data": { "columns": 3, "show_role": true, "show_bio": true, "link_to_profile": true } }),
            ];
            blocks.extend(community_homepage_extras(slug));
            blocks.push(serde_json::json!({ "type": "family-quick-access", "data": { "title": "Quick Access", "subtitle": "Jump to what you need" } }));
            blocks.push(serde_json::json!({ "type": "family-welcome", "data": { "title": "Today's Snapshot" } }));
            serde_json::Value::Array(blocks)
        }),
        default_tone: "friendly".into(),
        default_brand_colors: BrandColors {
            primary: p.primary.into(),
            secondary: p.dark.into(),
            accent: p.accent.into(),
        },
        default_tier: "founding".into(),
        always_free: true,
        price_override_cents: 0,
        discount_codes: vec![],
        limited_time_offer: Some(LimitedOffer {
            label: "Free during early release".into(),
            description: "7-day free trial, no card required.".into(),
            expires_at: 0,
            tier_override: "founding".into(),
        }),
        onboarding_steps,
        seo_title_template: seo_title_tpl.into(),
        seo_description_template: seo_desc_tpl.into(),
        created_at: ts,
        updated_at: ts,
    }
}

fn onboard(
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
    members_label: &str,
) -> Vec<OnboardingStep> {
    vec![OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "members".into(),
                label: members_label.into(),
                field_type: "textarea".into(),
                placeholder: "One name per line".into(),
                required: false,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    }]
}

fn group_specific_details_step(slug: &str) -> Option<OnboardingStep> {
    Some(match slug {
        "band" => step(
            "music-and-gigs",
            "Music & Gigs",
            true,
            vec![
                field_grid(
                    "genres",
                    "Genres / Style",
                    false,
                    &[
                        "Rock",
                        "Pop",
                        "Worship",
                        "Jazz",
                        "Country",
                        "Indie",
                        "Covers",
                        "Originals",
                    ],
                ),
                field_select(
                    "performance_type",
                    "Performance Type",
                    false,
                    &[
                        "Band",
                        "Creative Team",
                        "Solo Artist",
                        "Ensemble",
                        "Worship Team",
                    ],
                ),
                field_toggle("gear_inventory", "Track Gear & Shared Equipment"),
            ],
        ),
        "roommates" => step(
            "house-rules",
            "House Rules & Sharing",
            true,
            vec![
                field_grid(
                    "shared_expenses",
                    "Shared Expense Categories",
                    false,
                    &[
                        "Rent",
                        "Utilities",
                        "Internet",
                        "Groceries",
                        "Cleaning Supplies",
                        "Streaming",
                        "Parking",
                    ],
                ),
                field_select(
                    "split_style",
                    "How do you split costs?",
                    false,
                    &["Equal Split", "By Room", "Custom"],
                ),
                field_toggle("cleaning_rotation", "Use a Cleaning Rotation"),
            ],
        ),
        "classroom" => step(
            "class-setup",
            "Class Setup",
            true,
            vec![
                field_grid(
                    "grade_bands",
                    "Grade Bands",
                    false,
                    &["Pre-K", "K-2", "3-5", "6-8", "9-12"],
                ),
                field_select(
                    "subject_focus",
                    "Primary Focus",
                    false,
                    &[
                        "General Classroom",
                        "ELA",
                        "Math",
                        "Science",
                        "Social Studies",
                        "Arts",
                        "STEM Lab",
                    ],
                ),
                field_toggle("parent_updates", "Send Parent Updates"),
            ],
        ),
        "homeschool" => step(
            "learning-approach",
            "Learning Approach",
            true,
            vec![
                field_grid(
                    "grade_bands",
                    "Learner Levels",
                    false,
                    &["Pre-K", "Elementary", "Middle School", "High School"],
                ),
                field_select(
                    "approach",
                    "Primary Homeschool Style",
                    false,
                    &[
                        "Traditional",
                        "Charlotte Mason",
                        "Classical",
                        "Unit Study",
                        "Unschooling",
                        "Eclectic",
                    ],
                ),
                field_toggle("portfolio_tracking", "Track Portfolios & Progress"),
            ],
        ),
        "sports-team" => step(
            "team-season",
            "Season Setup",
            true,
            vec![
                field_select(
                    "sport",
                    "Sport",
                    false,
                    &[
                        "Baseball",
                        "Basketball",
                        "Cheer",
                        "Football",
                        "Soccer",
                        "Softball",
                        "Volleyball",
                        "Other",
                    ],
                ),
                field_select(
                    "age_group",
                    "Age Group",
                    false,
                    &["8U", "10U", "12U", "14U", "High School", "Adult", "Mixed"],
                ),
                field_toggle("equipment_tracking", "Track Equipment & Uniforms"),
            ],
        ),
        "club" => step(
            "club-focus",
            "Club Focus",
            true,
            vec![
                field_select(
                    "club_type",
                    "What kind of club is this?",
                    false,
                    &[
                        "Hobby Club",
                        "Community Club",
                        "Service Club",
                        "Alumni Group",
                        "Social Club",
                        "Car Club",
                        "Board Game Club",
                    ],
                ),
                field_toggle("event_calendar", "Run an Event Calendar"),
                field_toggle("member_directory", "Show a Member Directory"),
            ],
        ),
        "book-club" => step(
            "reading-style",
            "Reading Style",
            true,
            vec![
                field_grid(
                    "genres",
                    "Favorite Genres",
                    false,
                    &[
                        "Fiction",
                        "Mystery",
                        "Fantasy",
                        "Historical",
                        "Memoir",
                        "Business",
                        "Christian",
                        "Classics",
                    ],
                ),
                field_select(
                    "meeting_frequency",
                    "Meeting Frequency",
                    false,
                    &["Weekly", "Biweekly", "Monthly", "Quarterly"],
                ),
                field_toggle("reading_progress", "Track Reading Progress"),
            ],
        ),
        "nonprofit" => step(
            "mission-setup",
            "Mission Setup",
            true,
            vec![
                field_grid(
                    "mission_focus",
                    "Mission Focus",
                    false,
                    &[
                        "Community Aid",
                        "Youth",
                        "Education",
                        "Faith",
                        "Health",
                        "Animal Rescue",
                        "Arts",
                        "Food Pantry",
                    ],
                ),
                field_toggle("donations", "Accept Donations"),
                field_toggle("volunteer_signup", "Coordinate Volunteers"),
            ],
        ),
        "neighborhood" => step(
            "neighborhood-setup",
            "Neighborhood Setup",
            true,
            vec![
                field_toggle("directory", "Create a Neighbor Directory"),
                field_toggle("events", "Coordinate Neighborhood Events"),
                field_toggle("watch_updates", "Share Safety & Watch Updates"),
            ],
        ),
        "travel" => step(
            "trip-logistics",
            "Trip Logistics",
            true,
            vec![
                field_select(
                    "trip_type",
                    "Trip Type",
                    false,
                    &[
                        "Family Trip",
                        "Group Vacation",
                        "Retreat",
                        "Mission Trip",
                        "Study Trip",
                        "Road Trip",
                        "Tour Group",
                    ],
                ),
                field_toggle("packing_lists", "Use Shared Packing Lists"),
                field_toggle("expense_split", "Track Shared Expenses"),
            ],
        ),
        "elder-care" => step(
            "care-coordination",
            "Care Coordination",
            true,
            vec![
                field_grid(
                    "care_focus",
                    "Care Priorities",
                    false,
                    &[
                        "Medication",
                        "Meals",
                        "Mobility",
                        "Appointments",
                        "Companionship",
                        "Transportation",
                        "Memory Support",
                    ],
                ),
                field_toggle("visit_rotation", "Coordinate Visits"),
                field_toggle("medical_records", "Store Medical Records"),
            ],
        ),
        "wedding" => step(
            "celebration-plan",
            "Guest Site & Planning",
            true,
            vec![
                field_text("wedding_date", "Wedding Date", false, "October 10, 2026"),
                field_text("venue_location", "Venue / City", false, "Fort Worth, Texas"),
                field_grid(
                    "events",
                    "Wedding Events",
                    false,
                    &[
                        "Ceremony",
                        "Reception",
                        "Rehearsal Dinner",
                        "Welcome Party",
                        "Shower",
                        "Bachelor / Bachelorette",
                        "Brunch",
                        "Livestream",
                        "After Party",
                    ],
                ),
                field_toggle("guest_rsvp", "Manage Guest RSVPs"),
                field_toggle("meal_choices", "Collect meal choices or guest notes"),
                field_toggle("vendor_tracker", "Track Vendors & Contracts"),
            ],
        ),
        "pet-owners" => step(
            "pet-care",
            "Pet Care Setup",
            true,
            vec![
                field_grid(
                    "pet_types",
                    "Pets in the Pack",
                    false,
                    &[
                        "Dogs",
                        "Cats",
                        "Birds",
                        "Fish",
                        "Reptiles",
                        "Small Pets",
                        "Horses",
                    ],
                ),
                field_toggle("medication_tracking", "Track Medications"),
                field_toggle("vet_schedule", "Manage Vet Visits"),
            ],
        ),
        "scouts" => step(
            "troop-setup",
            "Troop Setup",
            true,
            vec![
                field_toggle("badge_tracking", "Track Badges & Advancement"),
                field_toggle("campouts", "Plan Campouts & Trips"),
                field_toggle("parent_roles", "Coordinate Parent Volunteers"),
            ],
        ),
        "fitness" => step(
            "fitness-focus",
            "Fitness Focus",
            true,
            vec![
                field_grid(
                    "activities",
                    "Primary Activities",
                    false,
                    &[
                        "Running", "Cycling", "Strength", "HIIT", "Yoga", "Walking", "CrossFit",
                        "Mobility",
                    ],
                ),
                field_toggle("challenge_tracking", "Run Challenges"),
                field_toggle("accountability", "Use Accountability Check-ins"),
            ],
        ),
        "farm" => step(
            "homestead-setup",
            "Homestead Setup",
            true,
            vec![
                field_text(
                    "farm_location",
                    "Farm / Market Area",
                    false,
                    "County road, town, or market area",
                ),
                field_grid(
                    "focus_areas",
                    "Focus Areas",
                    false,
                    &[
                        "Garden",
                        "Livestock",
                        "Poultry / Eggs",
                        "Orchard",
                        "Preserving",
                        "Farm Stand",
                        "Market Days",
                        "CSA / Produce Boxes",
                        "Workshops / Tours",
                        "Maintenance",
                        "Recipes",
                        "Inventory",
                    ],
                ),
                field_toggle("seasonal_plans", "Track Seasonal Plans"),
                field_toggle("supply_inventory", "Track Feed / Seed / Supplies"),
                field_toggle("public_farm_stand", "Show public farm stand updates"),
                field_toggle(
                    "customer_inquiries",
                    "Collect customer inquiries or pickup requests",
                ),
            ],
        ),
        "support-group" => step(
            "support-structure",
            "Support Structure",
            true,
            vec![
                field_select(
                    "group_style",
                    "Group Style",
                    false,
                    &[
                        "Peer Support",
                        "Recovery",
                        "Caregiver Support",
                        "Grief Support",
                        "Parent Support",
                        "Prayer & Encouragement",
                    ],
                ),
                field_toggle("private_requests", "Allow Private Requests"),
                field_toggle("resource_library", "Share Helpful Resources"),
            ],
        ),
        "maker-space" => step(
            "shop-setup",
            "Shop Setup",
            true,
            vec![
                field_grid(
                    "equipment",
                    "Equipment Available",
                    false,
                    &[
                        "3D Printers",
                        "Laser Cutter",
                        "CNC",
                        "Wood Shop",
                        "Metal Shop",
                        "Sewing",
                        "Electronics Bench",
                    ],
                ),
                field_toggle("reservations", "Reserve Equipment"),
                field_toggle("safety_docs", "Store Safety Docs & SOPs"),
            ],
        ),
        "church" => step(
            "ministry-setup",
            "Ministry Setup",
            true,
            vec![
                field_grid(
                    "ministries",
                    "Ministries",
                    false,
                    &[
                        "Sunday Services",
                        "Children's Ministry",
                        "Youth Group",
                        "Small Groups",
                        "Missions",
                        "Prayer Team",
                        "Worship Team",
                    ],
                ),
                field_toggle("sermon_archive", "Publish Sermons & Notes"),
                field_toggle("online_giving", "Support Giving & Fundraising"),
            ],
        ),
        "small-group" => step(
            "group-rhythm",
            "Group Rhythm",
            true,
            vec![
                field_select(
                    "group_focus",
                    "Group Focus",
                    false,
                    &[
                        "Bible Study",
                        "Prayer",
                        "Fellowship",
                        "Men's Group",
                        "Women's Group",
                        "Young Adults",
                        "Couples",
                    ],
                ),
                field_select(
                    "meeting_frequency",
                    "Meeting Frequency",
                    false,
                    &["Weekly", "Biweekly", "Monthly"],
                ),
                field_toggle("host_rotation", "Rotate Hosts / Leaders"),
            ],
        ),
        "mission-team" => step(
            "trip-prep",
            "Trip Prep",
            true,
            vec![
                field_text(
                    "destination",
                    "Destination",
                    false,
                    "Guatemala City, Guatemala",
                ),
                field_grid(
                    "mission_focus",
                    "Mission Focus",
                    false,
                    &[
                        "Evangelism",
                        "Construction",
                        "Medical",
                        "Children's Ministry",
                        "Relief",
                        "Discipleship",
                    ],
                ),
                field_toggle("fundraising_goal", "Track Fundraising Goals"),
            ],
        ),
        "homeschool-coop" => step(
            "coop-planning",
            "Co-op Planning",
            true,
            vec![
                field_grid(
                    "age_groups",
                    "Age Groups Served",
                    false,
                    &["Pre-K", "Elementary", "Middle School", "High School"],
                ),
                field_toggle("teacher_rotation", "Manage Teacher Rotation"),
                field_toggle("resource_exchange", "Share Curriculum & Materials"),
            ],
        ),
        "business-team" => step(
            "workflow-setup",
            "Workflow Setup",
            true,
            vec![
                field_grid(
                    "team_focus",
                    "Primary Team Functions",
                    false,
                    &[
                        "Operations",
                        "Sales",
                        "Marketing",
                        "Product",
                        "Support",
                        "Leadership",
                        "Admin",
                    ],
                ),
                field_toggle("weekly_checkins", "Use Weekly Check-ins"),
                field_toggle("docs_vault", "Keep SOPs & Docs in a Shared Vault"),
            ],
        ),
        "reunion" => step(
            "reunion-plan",
            "Reunion Plan",
            true,
            vec![
                field_select(
                    "reunion_type",
                    "Reunion Type",
                    false,
                    &[
                        "Family",
                        "Class",
                        "Military",
                        "Team",
                        "Church",
                        "Neighborhood",
                    ],
                ),
                field_toggle("potluck", "Coordinate Potluck Sign-ups"),
                field_toggle("memory_wall", "Share Photos & Memories"),
            ],
        ),
        "memorial" => step(
            "remembrance-setup",
            "Remembrance Setup",
            true,
            vec![
                field_grid(
                    "memory_features",
                    "Include These Spaces",
                    false,
                    &[
                        "Story Timeline",
                        "Photo Gallery",
                        "Guestbook",
                        "Service Details",
                        "Recipe Collection",
                        "Donation Link",
                    ],
                ),
                field_textarea(
                    "tribute_summary",
                    "A Few Words About Them",
                    false,
                    "How would you like this memorial space to describe and honor them?",
                ),
                field_text(
                    "service_date",
                    "Service / Gathering Date",
                    false,
                    "April 21, 2026",
                ),
                field_toggle("private_family_space", "Keep Some Areas Family-Only"),
            ],
        ),
        _ => return None,
    })
}

fn group_specific_operations_step(slug: &str) -> Option<OnboardingStep> {
    Some(match slug {
        "band" => step(
            "band-operations",
            "Crew Operations",
            true,
            vec![
                field_select(
                    "rehearsal_cadence",
                    "Rehearsal Cadence",
                    false,
                    &["Weekly", "Biweekly", "Monthly", "Tour / event based"],
                ),
                field_toggle("booking_requests", "Track show or booking requests"),
                field_toggle("member_portal", "Give each member their own login"),
            ],
        ),
        "roommates" => step(
            "household-ops",
            "Household Ops",
            true,
            vec![
                field_toggle(
                    "guest_tracker",
                    "Track guests, overnight stays, or visitor notes",
                ),
                field_toggle("shared_pantry", "Use shared shopping and pantry lists"),
                field_select(
                    "quiet_hours_style",
                    "Quiet Hours",
                    false,
                    &[
                        "No set quiet hours",
                        "Weeknights only",
                        "Every night",
                        "Custom / house vote",
                    ],
                ),
            ],
        ),
        "classroom" => step(
            "classroom-operations",
            "Student & Parent Flow",
            true,
            vec![
                field_toggle("assignment_tracker", "Track assignments and due dates"),
                field_toggle("classroom_photos", "Share classroom photos or highlights"),
                field_select(
                    "parent_touchpoint",
                    "Parent Communication Rhythm",
                    false,
                    &["As needed", "Weekly", "Biweekly", "Monthly"],
                ),
            ],
        ),
        "homeschool" => step(
            "homeschool-operations",
            "Learning Rhythm",
            true,
            vec![
                field_toggle("lesson_plans", "Track lesson plans and weekly rhythm"),
                field_toggle("attendance_logs", "Keep attendance and progress logs"),
                field_select(
                    "portfolio_style",
                    "Portfolio Style",
                    false,
                    &[
                        "Simple record",
                        "Samples + photos",
                        "Full academic archive",
                        "State reporting focused",
                    ],
                ),
            ],
        ),
        "sports-team" => step(
            "team-operations",
            "Team Operations",
            true,
            vec![
                field_toggle(
                    "snack_rotation",
                    "Coordinate snacks, rides, or parent volunteer jobs",
                ),
                field_toggle("stats_tracking", "Track player stats or game recaps"),
                field_select(
                    "coach_updates",
                    "Coach Update Rhythm",
                    false,
                    &[
                        "Game days only",
                        "Weekly",
                        "After every practice",
                        "As needed",
                    ],
                ),
            ],
        ),
        "club" => step(
            "club-operations",
            "Member Experience",
            true,
            vec![
                field_toggle("member_directory", "Publish a member directory"),
                field_toggle("dues_tracking", "Track dues or recurring contributions"),
                field_select(
                    "meeting_style",
                    "Meeting Style",
                    false,
                    &[
                        "Casual hangout",
                        "Structured agenda",
                        "Project / task focused",
                        "Speaker or workshop based",
                    ],
                ),
            ],
        ),
        "book-club" => step(
            "bookclub-operations",
            "Discussion Flow",
            true,
            vec![
                field_toggle("host_rotation", "Rotate hosts or discussion leaders"),
                field_toggle("book_voting", "Vote on the next book together"),
                field_select(
                    "discussion_style",
                    "Discussion Style",
                    false,
                    &[
                        "Casual conversation",
                        "Guided discussion questions",
                        "Chapter-by-chapter",
                        "Theme focused",
                    ],
                ),
            ],
        ),
        "nonprofit" => step(
            "nonprofit-operations",
            "Donors & Volunteers",
            true,
            vec![
                field_toggle("campaign_pages", "Launch campaigns or fundraiser pages"),
                field_toggle("impact_updates", "Publish regular impact updates"),
                field_select(
                    "volunteer_flow",
                    "Volunteer Coordination",
                    false,
                    &[
                        "Simple sign-up",
                        "Shift scheduling",
                        "Role-based teams",
                        "Event-based only",
                    ],
                ),
            ],
        ),
        "neighborhood" => step(
            "neighborhood-operations",
            "Neighborhood Coordination",
            true,
            vec![
                field_toggle("directory_opt_in", "Offer a neighbor directory"),
                field_toggle("yard_sale_board", "Run a yard sale / swap board"),
                field_select(
                    "news_style",
                    "Neighborhood Updates",
                    false,
                    &[
                        "As needed",
                        "Weekly digest",
                        "Monthly bulletin",
                        "Event-only",
                    ],
                ),
            ],
        ),
        "travel" => step(
            "travel-operations",
            "Trip Coordination",
            true,
            vec![
                field_toggle("room_assignments", "Track rooming or lodging assignments"),
                field_toggle(
                    "travel_docs_checklist",
                    "Use a passports / confirmations checklist",
                ),
                field_select(
                    "expense_style",
                    "Expense Coordination",
                    false,
                    &[
                        "No shared expenses",
                        "Split big trip costs",
                        "Track everything",
                        "One organizer pays",
                    ],
                ),
            ],
        ),
        "elder-care" => step(
            "care-operations",
            "Care Team Workflow",
            true,
            vec![
                field_toggle(
                    "emergency_contacts",
                    "Keep emergency contacts front and center",
                ),
                field_toggle("medication_reminders", "Track medication reminders"),
                field_select(
                    "update_style",
                    "Care Update Rhythm",
                    false,
                    &["As needed", "Daily", "Weekly summary", "After appointments"],
                ),
            ],
        ),
        "wedding" => step(
            "wedding-operations",
            "Guest & Vendor Flow",
            true,
            vec![
                field_toggle("guest_public_site", "Publish guest-facing wedding details"),
                field_toggle("private_planning", "Keep planning notes private"),
                field_toggle("rsvp_tracking", "Track RSVPs and meal choices"),
                field_toggle(
                    "vendor_contacts",
                    "Keep vendor contacts and contracts together",
                ),
                field_toggle("travel_details", "Share travel, hotel, and direction notes"),
                field_select(
                    "planning_window",
                    "Planning Window",
                    false,
                    &[
                        "Less than 3 months",
                        "3-6 months",
                        "6-12 months",
                        "More than 12 months",
                    ],
                ),
            ],
        ),
        "pet-owners" => step(
            "pet-owner-operations",
            "Pet Care Workflow",
            true,
            vec![
                field_toggle("feeding_schedule", "Track feeding or medication schedules"),
                field_toggle("pet_profiles", "Give each pet its own profile"),
                field_select(
                    "care_style",
                    "Care Coordination Style",
                    false,
                    &[
                        "Daily routine",
                        "Appointment focused",
                        "Travel / sitter focused",
                        "Health and meds focused",
                    ],
                ),
            ],
        ),
        "scouts" => step(
            "scout-operations",
            "Troop Coordination",
            true,
            vec![
                field_toggle("permission_forms", "Store permission slips and forms"),
                field_toggle("camp_checklists", "Use camp packing and prep checklists"),
                field_select(
                    "advancement_style",
                    "Advancement Tracking",
                    false,
                    &[
                        "Simple badge list",
                        "Detailed advancement records",
                        "Event based only",
                        "Leader managed",
                    ],
                ),
            ],
        ),
        "fitness" => step(
            "fitness-operations",
            "Challenge Setup",
            true,
            vec![
                field_toggle("leaderboards", "Show challenge leaderboards"),
                field_toggle("event_signups", "Run workout or meetup signups"),
                field_select(
                    "accountability_style",
                    "Accountability Style",
                    false,
                    &[
                        "Daily check-ins",
                        "Weekly progress",
                        "Challenge based",
                        "Casual community",
                    ],
                ),
            ],
        ),
        "farm" => step(
            "farm-operations",
            "Seasonal Workflow",
            true,
            vec![
                field_toggle("animal_records", "Track animal records or care routines"),
                field_toggle("harvest_logs", "Track harvests, yields, or preserves"),
                field_toggle("market_days", "Coordinate farmers market or sales days"),
                field_toggle("farm_stand_pickups", "Manage farm stand or pickup requests"),
                field_toggle(
                    "workshops_tours",
                    "Share workshops, tours, or volunteer days",
                ),
                field_select(
                    "farm_focus",
                    "Primary Focus",
                    false,
                    &[
                        "Homestead",
                        "Market garden",
                        "Livestock",
                        "Mixed farm",
                        "Family food supply",
                    ],
                ),
            ],
        ),
        "support-group" => step(
            "support-operations",
            "Care & Check-ins",
            true,
            vec![
                field_toggle(
                    "anonymous_requests",
                    "Allow anonymous prayer or support requests",
                ),
                field_toggle("resource_sharing", "Share resources and next steps"),
                field_select(
                    "checkin_style",
                    "Check-in Rhythm",
                    false,
                    &["Every meeting", "Weekly", "As needed", "Facilitator only"],
                ),
            ],
        ),
        "maker-space" => step(
            "makerspace-operations",
            "Membership & Safety",
            true,
            vec![
                field_toggle(
                    "waiver_tracking",
                    "Track waivers or member safety acknowledgements",
                ),
                field_toggle(
                    "machine_training",
                    "Track who is cleared to use each machine",
                ),
                field_select(
                    "access_style",
                    "Access Model",
                    false,
                    &[
                        "Open studio hours",
                        "Reservation based",
                        "Class / workshop driven",
                        "Membership only",
                    ],
                ),
            ],
        ),
        "small-group" => step(
            "smallgroup-operations",
            "Care & Rhythm",
            true,
            vec![
                field_toggle(
                    "prayer_requests",
                    "Share prayer requests and follow-up needs",
                ),
                field_toggle("discussion_guides", "Post discussion guides or homework"),
                field_select(
                    "group_rhythm",
                    "Group Rhythm",
                    false,
                    &[
                        "Discussion focused",
                        "Prayer focused",
                        "Meal + fellowship",
                        "Care / support circle",
                    ],
                ),
            ],
        ),
        "mission-team" => step(
            "mission-operations",
            "Trip Support",
            true,
            vec![
                field_toggle("support_letters", "Track support letters or donor updates"),
                field_toggle(
                    "packing_checklists",
                    "Use team packing and supply checklists",
                ),
                field_select(
                    "prep_stage",
                    "Current Trip Stage",
                    false,
                    &[
                        "Interest / planning",
                        "Fundraising",
                        "Training",
                        "Travel prep",
                        "On trip",
                        "Post-trip follow-up",
                    ],
                ),
            ],
        ),
        "homeschool-coop" => step(
            "coop-operations",
            "Co-op Coordination",
            true,
            vec![
                field_toggle("family_volunteer_roles", "Track family volunteer roles"),
                field_toggle("class_rosters", "Organize rosters by class or age group"),
                field_select(
                    "meeting_day",
                    "Primary Gathering Rhythm",
                    false,
                    &[
                        "One day a week",
                        "Two days a week",
                        "Monthly gatherings",
                        "Hybrid / flexible",
                    ],
                ),
            ],
        ),
        "business-team" => step(
            "team-operations",
            "Team Rhythm",
            true,
            vec![
                field_toggle(
                    "meeting_notes",
                    "Keep meeting notes and follow-ups together",
                ),
                field_toggle("approvals", "Track decisions or approvals inside the site"),
                field_select(
                    "planning_cadence",
                    "Planning Cadence",
                    false,
                    &[
                        "Daily standups",
                        "Weekly sprint",
                        "Monthly planning",
                        "Quarterly goals",
                    ],
                ),
            ],
        ),
        "reunion" => step(
            "reunion-operations",
            "Guest Coordination",
            true,
            vec![
                field_toggle("registration", "Collect registrations or RSVPs"),
                field_toggle("lodging_notes", "Coordinate lodging or local hosting"),
                field_select(
                    "memory_style",
                    "Memory Sharing",
                    false,
                    &[
                        "Photo wall",
                        "Story posts",
                        "Both photos and stories",
                        "Private archive only",
                    ],
                ),
            ],
        ),
        "memorial" => step(
            "memorial-operations",
            "Guestbook & Tribute Flow",
            true,
            vec![
                field_toggle(
                    "public_guestbook",
                    "Allow visitors to leave public tributes",
                ),
                field_toggle("livestream_link", "Share a livestream or service replay"),
                field_select(
                    "memorial_tone",
                    "Page Tone",
                    false,
                    &[
                        "Quiet remembrance",
                        "Story-rich celebration",
                        "Service details first",
                        "Family-only archive",
                    ],
                ),
            ],
        ),
        _ => return None,
    })
}

fn group_features_step(slug: &str) -> OnboardingStep {
    let terms = luperiq_forge::terminology::default_terminology(slug);
    step(
        "features",
        "Features",
        true,
        vec![field_grid_values(
            "features",
            "Enable Features",
            false,
            terms.onboarding.feature_labels,
        )],
    )
}

fn group_onboarding(
    site_slug: &str,
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
    members_label: &str,
) -> Vec<OnboardingStep> {
    // Sports teams need sport context (so the roster builder can show the
    // right positions) and a structured roster widget rather than a names
    // textarea. Replace the generic team-info step with a sport-first
    // sequence.
    if site_slug == "sports-team" {
        return sports_team_onboarding(step_id, label, name_label, name_placeholder);
    }
    if site_slug == "band" {
        return band_onboarding(step_id, label, name_label, name_placeholder);
    }
    if site_slug == "classroom" {
        return classroom_onboarding(step_id, label, name_label, name_placeholder);
    }
    if site_slug == "scouts" {
        return scouts_onboarding(step_id, label, name_label, name_placeholder);
    }
    // Generic two-section roster overrides that share the structured-roster
    // widget on the front-end. Each one provides its own field_type tag so
    // ROSTER_CONFIGS in start-trial.html can pick the right column set.
    if matches!(
        site_slug,
        "book-club" | "small-group" | "mission-team" | "fitness" | "club" | "nonprofit"
    ) {
        return generic_group_roster_onboarding(
            site_slug,
            step_id,
            label,
            name_label,
            name_placeholder,
        );
    }
    let mut steps = onboard(step_id, label, name_label, name_placeholder, members_label);
    if let Some(details) = group_specific_details_step(site_slug) {
        steps.push(details);
    }
    if let Some(operations) = group_specific_operations_step(site_slug) {
        steps.push(operations);
    }
    steps.push(group_features_step(site_slug));
    steps
}

fn sports_team_onboarding(
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
) -> Vec<OnboardingStep> {
    // Step 1 — team identity + sport (sport drives position lists in step 2)
    let identity = OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "sport".into(),
                label: "Sport".into(),
                field_type: "select".into(),
                placeholder: "Pick a sport".into(),
                required: true,
                options: vec![
                    "Baseball".into(),
                    "Basketball".into(),
                    "Cheer".into(),
                    "Football".into(),
                    "Soccer".into(),
                    "Softball".into(),
                    "Volleyball".into(),
                    "Hockey".into(),
                    "Lacrosse".into(),
                    "Track & Field".into(),
                    "Other".into(),
                ],
                help_text: "Picks position list + stat fields for the roster.".into(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "age_group".into(),
                label: "Age Group".into(),
                field_type: "select".into(),
                placeholder: "Pick a level".into(),
                required: false,
                options: vec![
                    "8U".into(),
                    "10U".into(),
                    "12U".into(),
                    "14U".into(),
                    "Middle School".into(),
                    "High School".into(),
                    "College".into(),
                    "Adult".into(),
                    "Mixed".into(),
                ],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    };

    // Step 2 — roster builder (players + coaches in one widget, position
    // dropdowns and stat fields driven by the sport selected in step 1).
    let roster = OnboardingStep {
        step_id: "team-roster".into(),
        label: "Roster".into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: "team_roster".into(),
            label: "Players & Coaches".into(),
            field_type: "team_roster".into(),
            placeholder: "Add a player or coach".into(),
            required: false,
            options: vec![],
            help_text: "Add as many or few as you like. Positions and stat fields adapt to the sport you picked.".into(),
            admin_notes: String::new(),
        }],
    };

    let mut steps = vec![identity, roster];
    if let Some(operations) = group_specific_operations_step("sports-team") {
        steps.push(operations);
    }
    steps.push(group_features_step("sports-team"));
    steps
}

fn band_onboarding(
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
) -> Vec<OnboardingStep> {
    let identity = OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "performance_type".into(),
                label: "Performance Style".into(),
                field_type: "select".into(),
                placeholder: "Pick a style".into(),
                required: false,
                options: vec![
                    "Band".into(),
                    "Solo Artist".into(),
                    "Worship Team".into(),
                    "Ensemble".into(),
                    "Choir".into(),
                    "Creative Team".into(),
                ],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    };
    let roster = OnboardingStep {
        step_id: "band-roster".into(),
        label: "Members & Crew".into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: "band_roster".into(),
            label: "Members & Production".into(),
            field_type: "band_roster".into(),
            placeholder: "Add a member or crew person".into(),
            required: false,
            options: vec![],
            help_text: "Members get instrument + role. Production is your manager, sound engineer, lighting, producer, etc.".into(),
            admin_notes: String::new(),
        }],
    };
    let mut steps = vec![identity, roster];
    if let Some(operations) = group_specific_operations_step("band") {
        steps.push(operations);
    }
    steps.push(group_features_step("band"));
    steps
}

fn classroom_onboarding(
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
) -> Vec<OnboardingStep> {
    let identity = OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "grade_band".into(),
                label: "Grade Band".into(),
                field_type: "select".into(),
                placeholder: "Pick a band".into(),
                required: false,
                options: vec![
                    "Pre-K".into(),
                    "K-2".into(),
                    "3-5".into(),
                    "6-8".into(),
                    "9-12".into(),
                    "Mixed".into(),
                ],
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    };
    let roster = OnboardingStep {
        step_id: "classroom-roster".into(),
        label: "Class Roster".into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: "classroom_roster".into(),
            label: "Students & Staff".into(),
            field_type: "classroom_roster".into(),
            placeholder: "Add a student or staff member".into(),
            required: false,
            options: vec![],
            help_text: "Students get a grade level and subject focus. Staff includes teachers, aides, and specialists.".into(),
            admin_notes: String::new(),
        }],
    };
    let mut steps = vec![identity, roster];
    if let Some(operations) = group_specific_operations_step("classroom") {
        steps.push(operations);
    }
    steps.push(group_features_step("classroom"));
    steps
}

fn scouts_onboarding(
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
) -> Vec<OnboardingStep> {
    let identity = OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: "scout_program".into(),
                label: "Program".into(),
                field_type: "select".into(),
                placeholder: "Pick a program".into(),
                required: true,
                options: vec![
                    "Cub Scouts".into(),
                    "Scouts BSA".into(),
                    "Girl Scouts".into(),
                    "Trail Life".into(),
                    "Other".into(),
                ],
                help_text: "Drives the rank list on the next step.".into(),
                admin_notes: String::new(),
            },
        ],
    };
    let roster = OnboardingStep {
        step_id: "scouts-roster".into(),
        label: "Scouts & Leaders".into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: "scouts_roster".into(),
            label: "Scouts & Leaders".into(),
            field_type: "scouts_roster".into(),
            placeholder: "Add a scout or leader".into(),
            required: false,
            options: vec![],
            help_text: "Scouts get a rank (filtered by your program), patrol/den, and age. Leaders get a role.".into(),
            admin_notes: String::new(),
        }],
    };
    let mut steps = vec![identity, roster];
    if let Some(operations) = group_specific_operations_step("scouts") {
        steps.push(operations);
    }
    steps.push(group_features_step("scouts"));
    steps
}

/// Generic two-section roster wizard for community group types that share
/// the same structure: pick a context value in step 1, then add two lists
/// (members + leaders, or staff + volunteers, etc.) in step 2.
fn generic_group_roster_onboarding(
    site_slug: &str,
    step_id: &str,
    label: &str,
    name_label: &str,
    name_placeholder: &str,
) -> Vec<OnboardingStep> {
    // (context_field_key, context_label, context_options, roster_field_type,
    //  roster_step_label)
    let (ctx_key, ctx_label, ctx_options, roster_type, roster_label) = match site_slug {
        "book-club" => (
            "meeting_frequency",
            "Meeting Frequency",
            vec!["Weekly", "Biweekly", "Monthly", "Quarterly"],
            "book_club_roster",
            "Members & Organizers",
        ),
        "small-group" => (
            "group_focus",
            "Group Focus",
            vec![
                "Bible Study",
                "Life Group",
                "Recovery",
                "Discipleship",
                "Marriage",
                "Men",
                "Women",
                "Mixed",
                "Other",
            ],
            "small_group_roster",
            "Members & Leaders",
        ),
        "mission-team" => (
            "trip_type",
            "Trip Type",
            vec![
                "Short-term",
                "Long-term",
                "Disaster Relief",
                "Medical",
                "Construction",
                "Evangelism",
                "Other",
            ],
            "mission_team_roster",
            "Goers & Senders",
        ),
        "fitness" => (
            "discipline",
            "Discipline",
            vec![
                "CrossFit",
                "Strength",
                "Cardio",
                "Yoga",
                "Pilates",
                "Martial Arts",
                "Cycling",
                "Running",
                "Mixed",
            ],
            "fitness_roster",
            "Members & Coaches",
        ),
        "club" => (
            "club_type",
            "Club Type",
            vec![
                "Hobby Club",
                "Social Club",
                "Service Club",
                "Alumni",
                "Car Club",
                "Tabletop / Games",
                "Other",
            ],
            "club_roster",
            "Members & Officers",
        ),
        "nonprofit" => (
            "mission_focus",
            "Mission Focus",
            vec![
                "Education",
                "Health",
                "Hunger",
                "Housing",
                "Animals",
                "Environment",
                "Arts",
                "Faith",
                "Community",
                "Other",
            ],
            "nonprofit_roster",
            "Staff & Volunteers",
        ),
        _ => return Vec::new(),
    };

    let identity = OnboardingStep {
        step_id: step_id.into(),
        label: label.into(),
        skippable: false,
        fields: vec![
            OnboardingField {
                key: "group_name".into(),
                label: name_label.into(),
                field_type: "text".into(),
                placeholder: name_placeholder.into(),
                required: true,
                options: vec![],
                help_text: String::new(),
                admin_notes: String::new(),
            },
            OnboardingField {
                key: ctx_key.into(),
                label: ctx_label.into(),
                field_type: "select".into(),
                placeholder: "Pick one".into(),
                required: false,
                options: ctx_options.iter().map(|s| s.to_string()).collect(),
                help_text: String::new(),
                admin_notes: String::new(),
            },
        ],
    };
    let roster = OnboardingStep {
        step_id: format!("{site_slug}-roster"),
        label: roster_label.into(),
        skippable: true,
        fields: vec![OnboardingField {
            key: roster_type.replace('-', "_").to_string(),
            label: roster_label.into(),
            field_type: roster_type.into(),
            placeholder: "Add a person".into(),
            required: false,
            options: vec![],
            help_text: "Lists are optional — you can add more from the dashboard later.".into(),
            admin_notes: String::new(),
        }],
    };
    let mut steps = vec![identity, roster];
    if let Some(operations) = group_specific_operations_step(site_slug) {
        steps.push(operations);
    }
    steps.push(group_features_step(site_slug));
    steps
}

// ── Band / Creative Crew ──────────────────────────────────────────────────

fn band() -> SiteTypeDefinition {
    group_base(
        "band",
        "Creative Crew",
        "\u{1F3B8}",
        "Your creative group — setlists, rehearsal schedule, gig recaps, gear exchange.",
        "Your crew. One stage. Always creating.",
        30,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Rehearsal Tasks", "/chore-board", "\u{1F3B5}", 1),
            nav("calendar", "Gig Calendar", "/calendar", "\u{1F4C5}", 2),
            nav("setlists", "Setlists", "/recipes", "\u{1F3B6}", 3),
            nav("feed", "Band Updates", "/feed", "\u{1F4E2}", 4),
            nav("vault", "Band Files", "/vault", "\u{1F512}", 5),
            nav("gear", "Gear List", "/shopping", "\u{1F3B8}", 6),
            nav("jams", "Jam Sessions", "/games", "\u{1F3B9}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Rehearsal Tasks",
                "{crew} Rehearsal Tasks",
                "Stay tight, stay ready. Track rehearsals and practice.",
            ),
            page(
                "calendar",
                "Gig Calendar",
                "{crew} Gig Calendar",
                "Shows, rehearsals, and studio sessions.",
            ),
            page(
                "recipes",
                "Setlists",
                "{crew} Setlists",
                "Song lists, tabs, and lyrics.",
            ),
            page(
                "feed",
                "Band Updates",
                "{crew} Updates",
                "Tour stories, new tracks, behind the scenes.",
            ),
            page(
                "vault",
                "Band Files",
                "{crew} Files",
                "Contracts, masters, and rider docs.",
            ),
            page(
                "shopping",
                "Gear List",
                "{crew} Gear List",
                "Equipment, merch supplies, strings.",
            ),
            page(
                "games",
                "Jam Sessions",
                "{crew} Jam Sessions",
                "Improv challenges and cover battles.",
            ),
        ],
        group_onboarding(
            "band",
            "band-info",
            "Band Info",
            "Band Name",
            "The Wavelengths",
            "Band Members",
        ),
        "{crew_name} — {tagline}",
        "{crew_name}: setlists, gig calendar, rehearsal tracking, and more.",
    )
}

// ── Roommates / Shared Household ──────────────────────────────────────────

fn roommates() -> SiteTypeDefinition {
    group_base(
        "roommates",
        "Shared Household",
        "\u{1F3E0}",
        "Shared household with cleaning rotation, bill splitting, house schedule, and shared groceries.",
        "Your house. One board. Always organized.",
        31,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Cleaning Rotation", "/chore-board", "\u{1F9F9}", 1),
            nav("calendar", "House Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("recipes", "House Recipes", "/recipes", "\u{1F373}", 3),
            nav("feed", "House Chat", "/feed", "\u{1F4AC}", 4),
            nav("vault", "House Docs", "/vault", "\u{1F512}", 5),
            nav("shopping", "Shared Groceries", "/shopping", "\u{1F6D2}", 6),
            nav("games", "Movie Night", "/games", "\u{1F3AC}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Cleaning Rotation",
                "{household} Cleaning Rotation",
                "Fair, tracked, no arguments.",
            ),
            page(
                "calendar",
                "House Schedule",
                "{household} Schedule",
                "Who's home, quiet hours, maintenance.",
            ),
            page(
                "recipes",
                "House Recipes",
                "{household} Recipes",
                "Meal prep and shared favorites.",
            ),
            page(
                "feed",
                "House Chat",
                "{household} Chat",
                "Announcements and heads-up.",
            ),
            page(
                "vault",
                "House Docs",
                "{household} Docs",
                "Lease, WiFi, landlord contact.",
            ),
            page(
                "shopping",
                "Shared Groceries",
                "{household} Groceries",
                "Groceries and household supplies.",
            ),
            page(
                "games",
                "Movie Night",
                "{household} Movie Night",
                "Pick what we watch, play, do.",
            ),
        ],
        group_onboarding(
            "roommates",
            "house-info",
            "House Info",
            "Household Name",
            "Oak Street House",
            "Roommates",
        ),
        "{household_name} — {tagline}",
        "{household_name}: shared cleaning rotation, house schedule, groceries, and more.",
    )
}

// ── Classroom ─────────────────────────────────────────────────────────────

fn classroom() -> SiteTypeDefinition {
    group_base(
        "classroom",
        "Classroom Website",
        "\u{1F3EB}",
        "Classroom website with jobs board, class calendar, lesson plans, and supply lists.",
        "Your classroom. One space. Always learning.",
        32,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Classroom Jobs", "/chore-board", "\u{2B50}", 1),
            nav("calendar", "Class Calendar", "/calendar", "\u{1F4C5}", 2),
            nav("lessons", "Lesson Plans", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Class News", "/feed", "\u{1F4E3}", 4),
            nav("vault", "Class Files", "/vault", "\u{1F512}", 5),
            nav("supplies", "Supply List", "/shopping", "\u{1F4DD}", 6),
            nav("games", "Educational Games", "/games", "\u{1F3AE}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Classroom Jobs",
                "{class} Jobs",
                "Responsibility builds character.",
            ),
            page(
                "calendar",
                "Class Calendar",
                "{class} Calendar",
                "Assignments, field trips, and events.",
            ),
            page(
                "recipes",
                "Lesson Plans",
                "{class} Lesson Plans",
                "Curriculum and learning resources.",
            ),
            page(
                "feed",
                "Class News",
                "{class} News",
                "Announcements and classroom updates.",
            ),
            page(
                "vault",
                "Class Files",
                "{class} Files",
                "Handouts, forms, and important docs.",
            ),
            page(
                "shopping",
                "Supply List",
                "{class} Supply List",
                "What the class needs.",
            ),
            page(
                "games",
                "Educational Games",
                "{class} Games",
                "Learning through play.",
            ),
        ],
        group_onboarding(
            "classroom",
            "class-info",
            "Class Info",
            "Class Name",
            "Mrs. Smith's 3rd Grade",
            "Students",
        ),
        "{class_name} — {tagline}",
        "{class_name}: classroom jobs, calendar, lesson plans, and more.",
    )
}

// ── Homeschool ────────────────────────────────────────────────────────────

fn homeschool() -> SiteTypeDefinition {
    group_base(
        "homeschool",
        "Homeschool Academy",
        "\u{1F4D6}",
        "Homeschool website with learning tasks, curriculum, academic records, typing practice, and learning games.",
        "Your academy. One place. Always growing.",
        33,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Learning Tasks", "/chore-board", "\u{2B50}", 1),
            nav("calendar", "Learning Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("curriculum", "Curriculum", "/recipes", "\u{1F4DA}", 3),
            nav("journal", "Learning Journal", "/feed", "\u{1F4D3}", 4),
            nav("vault", "Academic Records", "/vault", "\u{1F512}", 5),
            nav(
                "materials",
                "Curriculum Materials",
                "/shopping",
                "\u{1F4E6}",
                6,
            ),
            nav("games", "Learning Games", "/games", "\u{1F3AE}", 7),
            nav(
                "typing",
                "Typing Quest",
                "/games/typing-test",
                "\u{2328}\u{FE0F}",
                8,
            ),
        ],
        vec![
            page(
                "chore-board",
                "Learning Tasks",
                "{academy} Tasks",
                "Track progress on learning goals.",
            ),
            page(
                "calendar",
                "Learning Schedule",
                "{academy} Schedule",
                "Lessons, activities, and milestones.",
            ),
            page(
                "recipes",
                "Curriculum",
                "{academy} Curriculum",
                "Subjects, units, and lesson plans.",
            ),
            page(
                "feed",
                "Learning Journal",
                "{academy} Journal",
                "Progress, discoveries, and proud moments.",
            ),
            page(
                "vault",
                "Academic Records",
                "{academy} Records",
                "Transcripts, tests, and portfolios.",
            ),
            page(
                "shopping",
                "Curriculum Materials",
                "{academy} Materials",
                "Books, supplies, and resources.",
            ),
            page(
                "games",
                "Learning Games",
                "{academy} Games",
                "Educational challenges and activities.",
            ),
        ],
        group_onboarding(
            "homeschool",
            "school-info",
            "School Info",
            "Academy Name",
            "The Learning Loft",
            "Students",
        ),
        "{academy_name} — {tagline}",
        "{academy_name}: homeschool curriculum, learning schedule, academic records, and more.",
    )
}

// ── Sports Team ───────────────────────────────────────────────────────────

fn sports_team() -> SiteTypeDefinition {
    group_base(
        "sports-team",
        "Sports Team",
        "\u{1F3C6}",
        "Team website with practice schedule, playbook, equipment list, and skill challenges.",
        "Your team. One playbook. Always ready.",
        34,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Team Duties", "/chore-board", "\u{1F3C3}", 1),
            nav("calendar", "Team Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("playbook", "Playbook", "/recipes", "\u{1F4CB}", 3),
            nav("feed", "Team Updates", "/feed", "\u{1F4E2}", 4),
            nav("vault", "Team Records", "/vault", "\u{1F512}", 5),
            nav("gear", "Equipment List", "/shopping", "\u{26BD}", 6),
            nav("games", "Skill Challenges", "/games", "\u{1F3AF}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Team Duties",
                "{team} Duties",
                "Keep the team running smoothly.",
            ),
            page(
                "calendar",
                "Team Schedule",
                "{team} Schedule",
                "Practices, games, and tournaments.",
            ),
            page(
                "recipes",
                "Playbook",
                "{team} Playbook",
                "Plays, drills, and strategy.",
            ),
            page(
                "feed",
                "Team Updates",
                "{team} Updates",
                "Game recaps, highlights, and news.",
            ),
            page(
                "vault",
                "Team Records",
                "{team} Records",
                "Rosters, stats, and waivers.",
            ),
            page(
                "shopping",
                "Equipment List",
                "{team} Equipment",
                "Gear, uniforms, and supplies.",
            ),
            page(
                "games",
                "Skill Challenges",
                "{team} Challenges",
                "Drills, competitions, and training.",
            ),
        ],
        group_onboarding(
            "sports-team",
            "team-info",
            "Team Info",
            "Team Name",
            "Thunder FC",
            "Players",
        ),
        "{team_name} — {tagline}",
        "{team_name}: team schedule, playbook, roster, and more.",
    )
}

// ── Club ──────────────────────────────────────────────────────────────────

fn club() -> SiteTypeDefinition {
    group_base(
        "club",
        "Club Website",
        "\u{2B50}",
        "Club website with duties, schedule, resources, news, and competitions.",
        "Your club. One home. Always together.",
        35,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Club Duties", "/chore-board", "\u{1F4CB}", 1),
            nav("calendar", "Club Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("resources", "Club Resources", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Club News", "/feed", "\u{1F4E2}", 4),
            nav("vault", "Club Files", "/vault", "\u{1F512}", 5),
            nav("shopping", "Group Buys", "/shopping", "\u{1F6D2}", 6),
            nav("games", "Club Competitions", "/games", "\u{1F3AF}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Club Duties",
                "{club} Duties",
                "Keep the club running smoothly.",
            ),
            page(
                "calendar",
                "Club Schedule",
                "{club} Schedule",
                "Meetings, events, and activities.",
            ),
            page(
                "recipes",
                "Club Resources",
                "{club} Resources",
                "Guides, references, and shared knowledge.",
            ),
            page(
                "feed",
                "Club News",
                "{club} News",
                "Announcements and updates.",
            ),
            page(
                "vault",
                "Club Files",
                "{club} Files",
                "Bylaws, minutes, and important docs.",
            ),
            page(
                "shopping",
                "Group Buys",
                "{club} Group Buys",
                "Shared purchases and group orders.",
            ),
            page(
                "games",
                "Club Competitions",
                "{club} Competitions",
                "Friendly challenges and contests.",
            ),
        ],
        group_onboarding(
            "club",
            "club-info",
            "Club Info",
            "Club Name",
            "Sunset Running Club",
            "Members",
        ),
        "{club_name} — {tagline}",
        "{club_name}: club schedule, duties, resources, and more.",
    )
}

// ── Book Club ─────────────────────────────────────────────────────────────

fn book_club() -> SiteTypeDefinition {
    group_base(
        "book-club",
        "Book Club",
        "\u{1F4DA}",
        "Book club with reading lists, meeting schedule, book reviews, and literary trivia.",
        "Your book club. One shelf. Always reading.",
        36,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Reading Tasks", "/chore-board", "\u{1F4D6}", 1),
            nav("calendar", "Meeting Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("lists", "Reading Lists", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Book Reviews", "/feed", "\u{270D}\u{FE0F}", 4),
            nav("vault", "Club Archives", "/vault", "\u{1F512}", 5),
            nav("orders", "Book Orders", "/shopping", "\u{1F4E6}", 6),
            nav("games", "Literary Trivia", "/games", "\u{1F3AF}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Reading Tasks",
                "{club} Reading Tasks",
                "Stay on track with this month's read.",
            ),
            page(
                "calendar",
                "Meeting Schedule",
                "{club} Meetings",
                "When and where we meet.",
            ),
            page(
                "recipes",
                "Reading Lists",
                "{club} Reading Lists",
                "Past, current, and future reads.",
            ),
            page(
                "feed",
                "Book Reviews",
                "{club} Reviews",
                "Thoughts, ratings, and discussions.",
            ),
            page(
                "vault",
                "Club Archives",
                "{club} Archives",
                "Past reads and discussion notes.",
            ),
            page(
                "shopping",
                "Book Orders",
                "{club} Book Orders",
                "Group orders and wishlists.",
            ),
            page(
                "games",
                "Literary Trivia",
                "{club} Trivia",
                "Book trivia and reading challenges.",
            ),
        ],
        group_onboarding(
            "book-club",
            "bookclub-info",
            "Book Club Info",
            "Club Name",
            "Page Turners",
            "Members",
        ),
        "{club_name} — {tagline}",
        "{club_name}: reading lists, meeting schedule, book reviews, and more.",
    )
}

// ── Nonprofit / Mission Website ───────────────────────────────────────────

fn nonprofit() -> SiteTypeDefinition {
    group_base(
        "nonprofit",
        "Nonprofit Website",
        "\u{1F64C}",
        "Nonprofit website with volunteer shifts, event calendar, impact stories, and donation tracking.",
        "Your mission. One platform. Always serving.",
        37,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Volunteer Shifts", "/chore-board", "\u{1F91D}", 1),
            nav("calendar", "Event Calendar", "/calendar", "\u{1F4C5}", 2),
            nav("guides", "Program Guides", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Impact Stories", "/feed", "\u{2764}\u{FE0F}", 4),
            nav("vault", "Org Documents", "/vault", "\u{1F512}", 5),
            nav("needs", "Donation Needs", "/shopping", "\u{1F381}", 6),
            nav("events", "Fundraiser Activities", "/games", "\u{2728}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Volunteer Shifts",
                "{org} Volunteer Shifts",
                "Sign up to serve.",
            ),
            page(
                "calendar",
                "Event Calendar",
                "{org} Events",
                "Upcoming events and campaigns.",
            ),
            page(
                "recipes",
                "Program Guides",
                "{org} Program Guides",
                "How our programs work.",
            ),
            page(
                "feed",
                "Impact Stories",
                "{org} Impact Stories",
                "Stories of change and progress.",
            ),
            page(
                "vault",
                "Org Documents",
                "{org} Documents",
                "Bylaws, reports, and policies.",
            ),
            page(
                "shopping",
                "Donation Needs",
                "{org} Needs",
                "Current needs and wish lists.",
            ),
            page(
                "games",
                "Fundraiser Activities",
                "{org} Fundraisers",
                "Events that support the mission.",
            ),
        ],
        group_onboarding(
            "nonprofit",
            "org-info",
            "Organization Info",
            "Organization Name",
            "Helping Hands Foundation",
            "Volunteers",
        ),
        "{org_name} — {tagline}",
        "{org_name}: volunteer coordination, events, impact stories, and more.",
    )
}

// ── Neighborhood ──────────────────────────────────────────────────────────

fn neighborhood() -> SiteTypeDefinition {
    group_base(
        "neighborhood",
        "Neighborhood Website",
        "\u{1F3D8}\u{FE0F}",
        "Neighborhood website with block duties, events, local guides, news, and yard sales.",
        "Your neighborhood. One site. Always connected.",
        38,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Block Duties", "/chore-board", "\u{1F4CB}", 1),
            nav(
                "calendar",
                "Neighborhood Events",
                "/calendar",
                "\u{1F4C5}",
                2,
            ),
            nav("guides", "Local Guides", "/recipes", "\u{1F5FA}\u{FE0F}", 3),
            nav("feed", "Neighborhood News", "/feed", "\u{1F4E3}", 4),
            nav("vault", "Community Docs", "/vault", "\u{1F512}", 5),
            nav("shopping", "Group Buys", "/shopping", "\u{1F6D2}", 6),
            nav("games", "Block Party Games", "/games", "\u{1F389}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Block Duties",
                "{neighborhood} Duties",
                "Shared neighborhood responsibilities.",
            ),
            page(
                "calendar",
                "Neighborhood Events",
                "{neighborhood} Events",
                "Block parties, meetings, and clean-ups.",
            ),
            page(
                "recipes",
                "Local Guides",
                "{neighborhood} Guides",
                "Recommendations and local knowledge.",
            ),
            page(
                "feed",
                "Neighborhood News",
                "{neighborhood} News",
                "Announcements and updates.",
            ),
            page(
                "vault",
                "Community Docs",
                "{neighborhood} Docs",
                "HOA docs, contacts, and bylaws.",
            ),
            page(
                "shopping",
                "Group Buys",
                "{neighborhood} Group Buys",
                "Bulk orders and shared purchases.",
            ),
            page(
                "games",
                "Block Party Games",
                "{neighborhood} Games",
                "Games and activities for gatherings.",
            ),
        ],
        group_onboarding(
            "neighborhood",
            "hood-info",
            "Neighborhood Info",
            "Neighborhood Name",
            "Maple Street",
            "Neighbors",
        ),
        "{neighborhood_name} — {tagline}",
        "{neighborhood_name}: neighborhood events, block duties, local guides, and more.",
    )
}

// ── Travel Group ──────────────────────────────────────────────────────────

fn travel() -> SiteTypeDefinition {
    group_base(
        "travel",
        "Travel Group",
        "\u{2708}\u{FE0F}",
        "Travel group with itinerary, packing lists, trip journal, travel guides, and expense tracking.",
        "Your adventure. One place. Always exploring.",
        39,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Trip Tasks", "/chore-board", "\u{1F4CB}", 1),
            nav("calendar", "Itinerary", "/calendar", "\u{1F4C5}", 2),
            nav(
                "guides",
                "Travel Guides",
                "/recipes",
                "\u{1F5FA}\u{FE0F}",
                3,
            ),
            nav("feed", "Trip Journal", "/feed", "\u{1F4D3}", 4),
            nav("vault", "Travel Docs", "/vault", "\u{1F512}", 5),
            nav("packing", "Packing List", "/shopping", "\u{1F9F3}", 6),
            nav("games", "Travel Games", "/games", "\u{1F3B2}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Trip Tasks",
                "{trip} Tasks",
                "Research, bookings, and prep.",
            ),
            page(
                "calendar",
                "Itinerary",
                "{trip} Itinerary",
                "Day-by-day plans and reservations.",
            ),
            page(
                "recipes",
                "Travel Guides",
                "{trip} Guides",
                "Destination tips and must-sees.",
            ),
            page(
                "feed",
                "Trip Journal",
                "{trip} Journal",
                "Photos, stories, and memories.",
            ),
            page(
                "vault",
                "Travel Docs",
                "{trip} Docs",
                "Passports, confirmations, and insurance.",
            ),
            page(
                "shopping",
                "Packing List",
                "{trip} Packing List",
                "What to bring.",
            ),
            page(
                "games",
                "Travel Games",
                "{trip} Games",
                "Road trip games and activities.",
            ),
        ],
        group_onboarding(
            "travel",
            "trip-info",
            "Trip Info",
            "Trip Name",
            "Europe 2026",
            "Travelers",
        ),
        "{trip_name} — {tagline}",
        "{trip_name}: itinerary, packing list, travel guides, and trip journal.",
    )
}

// ── Elder Care / Care Circle ──────────────────────────────────────────────

fn elder_care() -> SiteTypeDefinition {
    group_base(
        "elder-care",
        "Care Circle",
        "\u{1F91D}",
        "Care coordination with tasks, schedule, care plans, updates, and medical records.",
        "Your circle. One place. Always caring.",
        40,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Care Tasks", "/chore-board", "\u{1F4CB}", 1),
            nav("calendar", "Care Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("plans", "Care Plans", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Care Updates", "/feed", "\u{2764}\u{FE0F}", 4),
            nav("vault", "Medical Records", "/vault", "\u{1F512}", 5),
            nav("supplies", "Medical Supplies", "/shopping", "\u{1FA7A}", 6),
            nav("games", "Memory Activities", "/games", "\u{1F9E9}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Care Tasks",
                "{circle} Care Tasks",
                "Medication, appointments, daily needs.",
            ),
            page(
                "calendar",
                "Care Schedule",
                "{circle} Schedule",
                "Appointments, visits, and medication times.",
            ),
            page(
                "recipes",
                "Care Plans",
                "{circle} Care Plans",
                "Treatment plans and care routines.",
            ),
            page(
                "feed",
                "Care Updates",
                "{circle} Updates",
                "Health updates and daily notes.",
            ),
            page(
                "vault",
                "Medical Records",
                "{circle} Records",
                "Prescriptions, insurance, and directives.",
            ),
            page(
                "shopping",
                "Medical Supplies",
                "{circle} Supplies",
                "Medications, equipment, and essentials.",
            ),
            page(
                "games",
                "Memory Activities",
                "{circle} Activities",
                "Engaging activities and memory exercises.",
            ),
        ],
        group_onboarding(
            "elder-care",
            "care-info",
            "Care Circle Info",
            "Circle Name",
            "Mom's Care Team",
            "Care Team Members",
        ),
        "{circle_name} — {tagline}",
        "{circle_name}: care coordination, schedule, medical records, and more.",
    )
}

// ── Wedding ───────────────────────────────────────────────────────────────

fn wedding() -> SiteTypeDefinition {
    let mut d = group_base(
        "wedding",
        "Wedding Planner",
        "\u{1F48D}",
        "Public wedding website with RSVP, schedule, travel details, registry links, guest updates, and private planning tools.",
        "Your wedding. One guest page. One private plan.",
        41,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("rsvp", "RSVP", "/rsvp", "\u{1F4CB}", 1),
            nav("schedule", "Schedule", "/schedule", "\u{1F4C5}", 2),
            nav(
                "registry",
                "Registry & Travel",
                "/registry-travel",
                "\u{1F381}",
                3,
            ),
            nav("updates", "Guest Updates", "/guest-updates", "\u{1F48C}", 4),
            nav(
                "planning",
                "Private Planner",
                "/family-login?next=/chore-board",
                "\u{2728}",
                5,
            ),
            nav(
                "docs",
                "Private Docs",
                "/family-login?next=/vault",
                "\u{1F512}",
                6,
            ),
            nav("games", "Party Fun", "/games", "\u{1F389}", 7),
        ],
        vec![
            page_with_blocks(
                "rsvp",
                "RSVP",
                "{couple} Wedding RSVP",
                "RSVP for {couple}, share meal notes, plus-ones, and guest details.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":r###"<section class="section"><div class="container" style="max-width:920px;">
  <p style="text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#be185d);font-weight:800;margin:0 0 10px;">RSVP</p>
  <h1 style="font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;">Tell us who is coming.</h1>
  <p style="font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);">Guests can confirm attendance, meal notes, plus-ones, travel questions, and anything the couple needs before the day arrives. The couple sees the request privately and can follow up without exposing the working guest list.</p>
  <form id="weddingRsvpForm" class="card" style="margin-top:24px;display:grid;gap:14px;">
    <input type="text" name="fax_number" autocomplete="off" tabindex="-1" aria-hidden="true" style="position:absolute;left:-9999px;">
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:12px;">
      <label>Your name <input id="wedRsvpName" type="text" autocomplete="name" required style="width:100%;"></label>
      <label>Email <input id="wedRsvpEmail" type="email" autocomplete="email" required style="width:100%;"></label>
    </div>
    <div style="display:grid;grid-template-columns:repeat(auto-fit,minmax(160px,1fr));gap:12px;">
      <label>Will you attend? <select id="wedRsvpAttend" style="width:100%;"><option>Yes, I will be there</option><option>No, I cannot make it</option><option>Not sure yet</option></select></label>
      <label>Party size <input id="wedRsvpParty" type="number" min="1" max="20" value="1" style="width:100%;"></label>
    </div>
    <label>Meal notes, plus-ones, accessibility needs, or travel questions <textarea id="wedRsvpMessage" rows="5" placeholder="Tell the couple what they should know." style="width:100%;"></textarea></label>
    <button id="wedRsvpSubmit" type="submit" class="btn btn-primary">Send RSVP</button>
    <div id="wedRsvpStatus" role="status" style="min-height:22px;"></div>
  </form>
  <div class="card-grid" style="margin-top:24px;">
    <div class="card"><h2>Guest details</h2><p>Attendance, meal choices, plus-ones, accessibility notes, and follow-up questions stay organized.</p></div>
    <div class="card"><h2>Private planning</h2><p>Seating notes, vendor calls, helper tasks, and budget details stay private for the couple and trusted helpers.</p></div>
  </div>
  <p style="margin-top:22px;"><a class="btn btn-secondary" href="/schedule">See Schedule</a> <a class="btn btn-secondary" href="/registry-travel">Registry &amp; Travel</a></p>
</div></section>
<script>
(function(){
  var form=document.getElementById('weddingRsvpForm');
  if(!form)return;
  function val(id){var el=document.getElementById(id);return el?el.value.trim():'';}
  function csrf(){var m=document.cookie.match(/(?:^|;\s*)liq_csrf=([^;]+)/);return m?decodeURIComponent(m[1]):'';}
  form.addEventListener('submit',function(e){
    e.preventDefault();
    var btn=document.getElementById('wedRsvpSubmit');
    var status=document.getElementById('wedRsvpStatus');
    var honey=form.querySelector('[name="fax_number"]');
    var name=val('wedRsvpName');
    var email=val('wedRsvpEmail');
    var attending=val('wedRsvpAttend');
    var party=val('wedRsvpParty')||'1';
    var notes=val('wedRsvpMessage');
    if(!name||!email){
      if(status){status.textContent='Please add your name and email.';status.style.color='#b91c1c';}
      return;
    }
    var description=['RSVP: '+attending,'Party size: '+party,notes?'Notes: '+notes:''].filter(Boolean).join('\n');
    var headers={'Content-Type':'application/json'};
    var token=csrf();
    if(token)headers['X-CSRF-Token']=token;
    if(btn){btn.disabled=true;btn.textContent='Sending RSVP...';}
    fetch('/api/modules/forms/support/submit',{method:'POST',credentials:'same-origin',headers:headers,body:JSON.stringify({
      name:name,
      email:email,
      priority:'Medium',
      description:description,
      message:description,
      topic:'wedding',
      topic_label:'Wedding RSVP',
      contact_flow:'Wedding RSVP page',
      contact_kind:'wedding_rsvp',
      email_verification_required:'true',
      verification_policy:'verify-email-before-conversation',
      requested_recipient_name:'Wedding couple',
      fax_number:honey?honey.value:''
    })})
      .then(function(r){return r.json();})
      .then(function(res){
        if(!res||res.ok===false)throw new Error((res&&res.message)||'RSVP could not be sent.');
        form.reset();
        if(status){status.textContent='RSVP sent. The couple can review it privately.';status.style.color='#166534';}
      })
      .catch(function(err){if(status){status.textContent=err.message||'RSVP could not be sent.';status.style.color='#b91c1c';}})
      .finally(function(){if(btn){btn.disabled=false;btn.textContent='Send RSVP';}});
  });
}());
</script>"###}}
                ]),
            ),
            page_with_blocks(
                "schedule",
                "Wedding Schedule",
                "{couple} Wedding Schedule",
                "Ceremony, reception, rehearsal, travel, and guest schedule details.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":"<section class=\"section\"><div class=\"container\" style=\"max-width:900px;\"><p style=\"text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#be185d);font-weight:800;margin:0 0 10px;\">Schedule</p><h1 style=\"font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;\">One place for wedding day details.</h1><p style=\"font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);\">Share the public ceremony, reception, travel, and guest timing here. Rehearsal details, vendor arrivals, private family timing, and planning reminders can stay behind login.</p><div class=\"card-grid\" style=\"margin-top:24px;\"><div class=\"card\"><h2>Ceremony</h2><p>Add arrival time, location notes, dress guidance, parking, and directions.</p></div><div class=\"card\"><h2>Reception</h2><p>Keep meal timing, music, send-off notes, and after-party details easy to find.</p></div><div class=\"card\"><h2>Travel</h2><p>Hotel blocks, airport notes, shuttle timing, and local tips can live beside the schedule.</p></div></div></div></section>"}}
                ]),
            ),
            page_with_blocks(
                "registry-travel",
                "Registry & Travel",
                "{couple} Registry and Travel",
                "Registry links, hotel details, travel notes, and guest needs.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":"<section class=\"section\"><div class=\"container\" style=\"max-width:900px;\"><p style=\"text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#be185d);font-weight:800;margin:0 0 10px;\">Registry &amp; Travel</p><h1 style=\"font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;\">Help guests plan without another text thread.</h1><p style=\"font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);\">Link registries, hotel blocks, maps, airport notes, rideshare tips, dress guidance, and guest reminders in one friendly public place.</p><div class=\"card-grid\" style=\"margin-top:24px;\"><div class=\"card\"><h2>Registry links</h2><p>Place the couple's registry links here without mixing them into private planning notes.</p></div><div class=\"card\"><h2>Travel notes</h2><p>Hotels, directions, parking, local food, and accessibility notes can stay easy to find.</p></div><div class=\"card\"><h2>Guest questions</h2><p>Use the contact path for questions, dietary notes, and travel details the couple needs.</p></div></div></div></section>"}}
                ]),
            ),
            page_with_blocks(
                "contact",
                "Guest Questions",
                "Contact {couple}",
                "Guest questions, travel notes, and wedding follow-up for {couple}.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":r###"<section class="section"><div class="container" style="max-width:920px;">
  <p style="text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#be185d);font-weight:800;margin:0 0 10px;">Guest Questions</p>
  <h1 style="font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;">Ask without starting another group text.</h1>
  <p style="font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);">Ask about travel, meal preferences, accessibility needs, or RSVP changes — all in one place. The couple keeps private planning details behind login.</p>
  <div class="card-grid" style="margin-top:24px;">
    <a class="card" href="/rsvp" style="text-decoration:none;color:inherit;"><h2>Send or update an RSVP</h2><p>Attendance, party size, meal notes, and follow-up questions go through the RSVP page.</p></a>
    <a class="card" href="/schedule" style="text-decoration:none;color:inherit;"><h2>Check the schedule</h2><p>Ceremony, reception, travel, and guest timing stay easy to find.</p></a>
    <a class="card" href="/registry-travel" style="text-decoration:none;color:inherit;"><h2>Registry and travel</h2><p>Gift links, lodging notes, parking, maps, dress guidance, and guest reminders live together.</p></a>
  </div>
</div></section>"###}}
                ]),
            ),
            page(
                "chore-board",
                "Planning Board",
                "{couple} Planning Board",
                "Private helper tasks, vendor follow-ups, seating notes, and wedding party jobs.",
            ),
            page(
                "calendar",
                "Wedding Schedule",
                "{couple} Wedding Schedule",
                "Ceremony, reception, rehearsal, travel windows, vendor arrivals, and private planning dates.",
            ),
            page(
                "recipes",
                "Planning Board",
                "{couple} Planning Board",
                "Inspiration, shot lists, music ideas, decor notes, and planning references.",
            ),
            page_with_blocks(
                "guest-updates",
                "Guest Updates",
                "{couple} Guest Updates",
                "Travel notes, weather updates, guest reminders, and wedding party announcements.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":"<section class=\"section\"><div class=\"container\" style=\"max-width:900px;\"><p style=\"text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#be185d);font-weight:800;margin:0 0 10px;\">Guest Updates</p><h1 style=\"font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;\">The latest guest notes in one place.</h1><p style=\"font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);\">Use this public page for weather notes, parking changes, hotel reminders, ceremony timing, registry reminders, livestream details, and friendly updates guests need before the wedding day.</p><div class=\"card-grid\" style=\"margin-top:24px;\"><div class=\"card\"><h2>Before the day</h2><p>Post arrival guidance, attire notes, parking, hotel blocks, shuttle timing, and RSVP reminders.</p></div><div class=\"card\"><h2>Day-of notes</h2><p>Keep ceremony, reception, weather, travel, and accessibility updates easy to find on a phone.</p></div><div class=\"card\"><h2>Private planning stays private</h2><p>Budgets, vendor calls, helper jobs, seating details, and family-only notes stay behind login.</p></div></div><p style=\"margin-top:22px;\"><a class=\"btn btn-secondary\" href=\"/rsvp\">RSVP</a> <a class=\"btn btn-secondary\" href=\"/schedule\">See Schedule</a></p></div></section>"}}
                ]),
            ),
            page(
                "vault",
                "Private Wedding Docs",
                "{couple} Private Wedding Docs",
                "Contracts, budget notes, vows, licenses, vendor contacts, and family-only records.",
            ),
            page(
                "shopping",
                "Registry & Travel",
                "{couple} Registry & Travel",
                "Registry links, travel details, hotel notes, supplies, gifts, and guest needs.",
            ),
            page(
                "games",
                "Wedding Party Fun",
                "{couple} Wedding Party Fun",
                "Shower games, reception prompts, song ideas, and guest activities.",
            ),
        ],
        group_onboarding(
            "wedding",
            "wedding-info",
            "Wedding Info",
            "Wedding Name",
            "Smith-Jones Wedding",
            "Wedding Party Members",
        ),
        "{couple} Wedding — {tagline}",
        "{couple} wedding website: RSVP, schedule, travel details, registry links, guest updates, and private planning tools.",
    );
    d.homepage_blocks = Some(serde_json::json!([
        {
            "type": "company-hero",
            "data": {
                "headline": "{business_name}",
                "description": "A guest-friendly wedding page with RSVP, schedule, registry, travel details, updates, and private planning tools behind login.",
                "cta_text": "RSVP",
                "cta_url": "/rsvp",
                "show_cta": true
            }
        },
        {
            "type": "feature-grid",
            "data": {
                "title": "One public guest page. One private plan.",
                "items": [
                    {
                        "title": "RSVP",
                        "text": "Guests can confirm attendance, meal notes, plus-ones, and questions without a group text.",
                        "url": "/rsvp"
                    },
                    {
                        "title": "Schedule",
                        "text": "Ceremony, reception, travel notes, parking, hotel blocks, and timing stay easy to find.",
                        "url": "/schedule"
                    },
                    {
                        "title": "Registry & Travel",
                        "text": "Keep gift links, directions, lodging, dress notes, and guest details in one place.",
                        "url": "/registry-travel"
                    },
                    {
                        "title": "Private Planner",
                        "text": "The couple and helpers keep vendor tasks, docs, seating notes, and reminders behind login.",
                        "url": "/family-login?next=/chore-board"
                    }
                ]
            }
        },
        {
            "type": "custom-html",
            "data": {
                "html": "<section class=\"section\"><div class=\"container\" style=\"max-width:900px;\"><h2>Guests should not need instructions</h2><p>The public side answers the questions guests actually have. The private side gives the couple and wedding party a working planner without exposing contracts, budgets, helper assignments, or family notes.</p></div></section>"
            }
        }
    ]));
    d
}

// ── Pet Owners / Pet Care Website ─────────────────────────────────────────

fn pet_owners() -> SiteTypeDefinition {
    group_base(
        "pet-owners",
        "Pet Care Website",
        "\u{1F43E}",
        "Pet care website with care tasks, vet schedule, care guides, photo sharing, and pet records.",
        "Your pack. One place. Always loved.",
        42,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Pet Care Tasks", "/chore-board", "\u{1F43E}", 1),
            nav("calendar", "Vet Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("guides", "Care Guides", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Pet Photos", "/feed", "\u{1F4F7}", 4),
            nav("vault", "Pet Records", "/vault", "\u{1F512}", 5),
            nav("supplies", "Pet Supplies", "/shopping", "\u{1F6D2}", 6),
            nav("games", "Pet Tricks", "/games", "\u{1F3BE}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Pet Care Tasks",
                "{group} Care Tasks",
                "Feeding, walking, grooming, and meds.",
            ),
            page(
                "calendar",
                "Vet Schedule",
                "{group} Vet Schedule",
                "Vet visits, vaccinations, and check-ups.",
            ),
            page(
                "recipes",
                "Care Guides",
                "{group} Care Guides",
                "Nutrition, training, and health tips.",
            ),
            page(
                "feed",
                "Pet Photos",
                "{group} Photos",
                "Adorable moments and milestones.",
            ),
            page(
                "vault",
                "Pet Records",
                "{group} Records",
                "Vet records, microchip info, and insurance.",
            ),
            page(
                "shopping",
                "Pet Supplies",
                "{group} Supplies",
                "Food, toys, and essentials.",
            ),
            page(
                "games",
                "Pet Tricks",
                "{group} Tricks",
                "Training challenges and trick tracking.",
            ),
        ],
        group_onboarding(
            "pet-owners",
            "pet-info",
            "Pet Website Info",
            "Website Name",
            "The Barkers",
            "Family Members",
        ),
        "{group_name} — {tagline}",
        "{group_name}: pet care tasks, vet schedule, care guides, and photo sharing.",
    )
}

// ── Scouts / Troop Website ────────────────────────────────────────────────

fn scouts() -> SiteTypeDefinition {
    group_base(
        "scouts",
        "Troop Website",
        "\u{26FA}",
        "Scout troop website with duties, calendar, field guides, badge challenges, and gear exchange.",
        "Your troop. One campfire. Always ready.",
        43,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Troop Duties", "/chore-board", "\u{1F4CB}", 1),
            nav("calendar", "Troop Calendar", "/calendar", "\u{1F4C5}", 2),
            nav("guides", "Field Guides", "/recipes", "\u{1F33F}", 3),
            nav("feed", "Troop News", "/feed", "\u{1F4E2}", 4),
            nav("vault", "Troop Records", "/vault", "\u{1F512}", 5),
            nav(
                "gear",
                "Camping Supplies",
                "/shopping",
                "\u{1F3D5}\u{FE0F}",
                6,
            ),
            nav(
                "games",
                "Badge Challenges",
                "/games",
                "\u{1F396}\u{FE0F}",
                7,
            ),
        ],
        vec![
            page(
                "chore-board",
                "Troop Duties",
                "{troop} Duties",
                "Leadership roles and responsibilities.",
            ),
            page(
                "calendar",
                "Troop Calendar",
                "{troop} Calendar",
                "Meetings, campouts, and service projects.",
            ),
            page(
                "recipes",
                "Field Guides",
                "{troop} Guides",
                "Outdoor skills and reference material.",
            ),
            page(
                "feed",
                "Troop News",
                "{troop} News",
                "Updates, photos, and reports.",
            ),
            page(
                "vault",
                "Troop Records",
                "{troop} Records",
                "Rosters, forms, and advancement records.",
            ),
            page(
                "shopping",
                "Camping Supplies",
                "{troop} Supplies",
                "Gear, food, and equipment.",
            ),
            page(
                "games",
                "Badge Challenges",
                "{troop} Badges",
                "Merit badge work and skill challenges.",
            ),
        ],
        group_onboarding(
            "scouts",
            "troop-info",
            "Troop Info",
            "Troop Name",
            "Troop 42",
            "Scouts",
        ),
        "{troop_name} — {tagline}",
        "{troop_name}: troop calendar, field guides, badge challenges, and more.",
    )
}

// ── Fitness / Workout Group ───────────────────────────────────────────────

fn fitness() -> SiteTypeDefinition {
    group_base(
        "fitness",
        "Fitness Group",
        "\u{1F4AA}",
        "Fitness group with workout plans, schedule, exercise library, progress tracking, and challenges.",
        "Your goals. One tracker. Always moving.",
        44,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav(
                "tasks",
                "Workout Plan",
                "/chore-board",
                "\u{1F3CB}\u{FE0F}",
                1,
            ),
            nav("calendar", "Workout Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("library", "Exercise Library", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Progress Updates", "/feed", "\u{1F4C8}", 4),
            nav("vault", "Health Records", "/vault", "\u{1F512}", 5),
            nav("supps", "Supplement List", "/shopping", "\u{1F4A7}", 6),
            nav("games", "Fitness Challenges", "/games", "\u{1F525}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Workout Plan",
                "{group} Workout Plan",
                "Daily and weekly workout tracking.",
            ),
            page(
                "calendar",
                "Workout Schedule",
                "{group} Schedule",
                "Classes, runs, and gym sessions.",
            ),
            page(
                "recipes",
                "Exercise Library",
                "{group} Exercises",
                "Movements, routines, and form guides.",
            ),
            page(
                "feed",
                "Progress Updates",
                "{group} Progress",
                "PRs, progress photos, and wins.",
            ),
            page(
                "vault",
                "Health Records",
                "{group} Records",
                "Measurements, goals, and benchmarks.",
            ),
            page(
                "shopping",
                "Supplement List",
                "{group} Supplements",
                "Supplements, gear, and nutrition.",
            ),
            page(
                "games",
                "Fitness Challenges",
                "{group} Challenges",
                "Step challenges, lifting contests, and goals.",
            ),
        ],
        group_onboarding(
            "fitness",
            "fitness-info",
            "Group Info",
            "Group Name",
            "Morning Grind Crew",
            "Members",
        ),
        "{group_name} — {tagline}",
        "{group_name}: workout plans, schedule, progress tracking, and fitness challenges.",
    )
}

// ── Farm / Homestead ──────────────────────────────────────────────────────

fn farm() -> SiteTypeDefinition {
    let mut d = group_base(
        "farm",
        "Homestead Website",
        "\u{1F33E}",
        "Homestead website for chores, crop and animal records, harvest journal, supply inventory, market days, farm stand updates, and customer inquiries.",
        "Your land. Your harvest. One place to run it.",
        45,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("stand", "Farm Stand", "/farm-stand", "\u{1F9FA}", 1),
            nav("journal", "Farm Journal", "/farm-journal", "\u{1F4D3}", 2),
            nav(
                "work",
                "Farm Work",
                "/family-login?next=/chore-board",
                "\u{1F69C}",
                3,
            ),
            nav(
                "calendar",
                "Farm Calendar",
                "/family-login?next=/calendar",
                "\u{1F4C5}",
                4,
            ),
            nav(
                "guides",
                "Guides & Recipes",
                "/family-login?next=/recipes",
                "\u{1F331}",
                5,
            ),
            nav(
                "vault",
                "Farm Records",
                "/family-login?next=/vault",
                "\u{1F512}",
                6,
            ),
            nav(
                "inventory",
                "Inventory & Orders",
                "/family-login?next=/shopping",
                "\u{1F33F}",
                7,
            ),
            nav(
                "games",
                "Farm Fun",
                "/family-login?next=/games",
                "\u{1F383}",
                8,
            ),
        ],
        vec![
            page_with_blocks(
                "farm-stand",
                "Farm Stand",
                "{farm} Farm Stand",
                "Farm stand availability, pickup requests, seasonal goods, and customer inquiries.",
                serde_json::json!([
                    {"type":"custom-html","data":{"html":"<section class=\"section\"><div class=\"container\" style=\"max-width:980px;\"><p style=\"text-transform:uppercase;letter-spacing:.12em;color:var(--luperiq-primary,#65a30d);font-weight:800;margin:0 0 10px;\">Farm Stand</p><h1 style=\"font-size:clamp(34px,8vw,58px);line-height:1.02;margin:0 0 16px;\">Shop what is fresh, then choose pickup.</h1><p style=\"font-size:18px;line-height:1.75;color:var(--color-text-light,#64748b);\">Use this public page for eggs, produce boxes, preserves, cut flowers, honey, meat shares, workshops, tours, or seasonal farm goods. Visitors can add starter items to the order tray, then the farm can accept manual pickup requests or connect payment keys when online checkout is ready.</p><div class=\"card-grid\" style=\"margin-top:24px;\"><div class=\"card\"><h2>Pasture-raised eggs</h2><p>One dozen eggs from the weekly farm stand list.</p><p><strong>$6.00</strong></p><button class=\"btn btn-primary\" type=\"button\" data-add-to-cart data-source-module=\"farm-stand\" data-product-id=\"farm-eggs-dozen\" data-product-name=\"Pasture-Raised Eggs\" data-unit-price=\"600\">Add eggs</button></div><div class=\"card\"><h2>Seasonal produce box</h2><p>A pickup box for whatever is best this week: greens, herbs, squash, tomatoes, or roots.</p><p><strong>$28.00</strong></p><button class=\"btn btn-primary\" type=\"button\" data-add-to-cart data-source-module=\"farm-stand\" data-product-id=\"farm-produce-box\" data-product-name=\"Seasonal Produce Box\" data-unit-price=\"2800\">Add produce box</button></div><div class=\"card\"><h2>Honey jar</h2><p>Small-batch honey, perfect for pickup, markets, or gift baskets.</p><p><strong>$12.00</strong></p><button class=\"btn btn-primary\" type=\"button\" data-add-to-cart data-source-module=\"farm-stand\" data-product-id=\"farm-honey-jar\" data-product-name=\"Homestead Honey Jar\" data-unit-price=\"1200\">Add honey</button></div></div><div class=\"card-grid\" style=\"margin-top:24px;\"><div class=\"card\"><h2>Pickup notes</h2><p>Guests can ask about pickup windows, quantities, allergies, delivery limits, or farm stand timing.</p></div><div class=\"card\"><h2>Private operations</h2><p>Feed, seed, harvest, animal records, permits, receipts, and helper tasks stay behind login.</p></div></div><p style=\"margin-top:22px;\"><a class=\"btn btn-secondary\" href=\"/contact\">Ask About Pickup</a> <a class=\"btn btn-secondary\" href=\"/farm-journal\">Read Farm Journal</a></p></div></section>"}}
                ]),
            ),
            page(
                "chore-board",
                "Farm Work",
                "{farm} Farm Work",
                "Daily animal care, garden tasks, repairs, market prep, and seasonal jobs.",
            ),
            page(
                "calendar",
                "Farm Calendar",
                "{farm} Calendar",
                "Planting, harvest, vet visits, breeding, market days, tours, workshops, and maintenance windows.",
            ),
            page(
                "recipes",
                "Guides & Recipes",
                "{farm} Guides & Recipes",
                "Growing notes, preservation recipes, animal care routines, and farm procedures.",
            ),
            page(
                "farm-journal",
                "Farm Journal",
                "{farm} Journal",
                "Harvest notes, availability updates, weather notes, photos, and lessons learned.",
            ),
            page(
                "vault",
                "Farm Records",
                "{farm} Records",
                "Animal records, seed logs, receipts, equipment manuals, permits, and important contacts.",
            ),
            page(
                "shopping",
                "Inventory & Orders",
                "{farm} Inventory & Orders",
                "Seed, feed, and supply inventory plus CSA, farm stand, and pickup request lists.",
            ),
            page(
                "games",
                "Farm Fun",
                "{farm} Farm Fun",
                "Kid-friendly challenges, harvest games, and farm learning activities.",
            ),
        ],
        group_onboarding(
            "farm",
            "farm-info",
            "Farm Info",
            "Farm Name",
            "Sunny Acres",
            "Homesteaders",
        ),
        "{farm_name} — {tagline}",
        "{farm_name}: homestead chores, animal and crop records, harvest journal, supply inventory, market days, and farm stand updates.",
    );
    d.homepage_blocks = Some(serde_json::json!([
        {
            "type": "company-hero",
            "data": {
                "headline": "{business_name}",
                "description": "Farm stand updates, harvest journal, private records, inventory, and daily farm work in one place.",
                "cta_text": "Visit the farm stand",
                "cta_url": "/farm-stand",
                "show_cta": true
            }
        },
        {
            "type": "feature-grid",
            "data": {
                "title": "Public where it helps. Private where it matters.",
                "items": [
                    {
                        "title": "Farm Stand",
                        "text": "Share fresh goods, pickup windows, CSA notes, workshops, tours, and customer questions.",
                        "url": "/farm-stand"
                    },
                    {
                        "title": "Farm Journal",
                        "text": "Publish harvest notes, photos, weather updates, and seasonal stories customers can follow.",
                        "url": "/farm-journal"
                    },
                    {
                        "title": "Farm Work",
                        "text": "Keep chores, repairs, animal care, helper jobs, and market prep behind login.",
                        "url": "/family-login?next=/chore-board"
                    },
                    {
                        "title": "Inventory & Records",
                        "text": "Track seed, feed, supplies, records, permits, receipts, and pickup lists privately.",
                        "url": "/family-login?next=/shopping"
                    }
                ]
            }
        },
        {
            "type": "custom-html",
            "data": {
                "html": "<section class=\"section\"><div class=\"container\" style=\"max-width:900px;\"><h2>Built for real homesteads</h2><p>A homestead site needs to sell the public story without exposing the private operation. Visitors can read updates, ask about pickup, and understand what the farm offers. The people doing the work can sign in for tasks, schedules, recipes, records, inventory, and planning.</p></div></section>"
            }
        }
    ]));
    d
}

// ── Support Group ─────────────────────────────────────────────────────────

fn support_group() -> SiteTypeDefinition {
    group_base(
        "support-group",
        "Support Circle",
        "\u{1F49C}",
        "Safe support circle with self-care tasks, meetings, resource library, and mindfulness activities.",
        "Your circle. One space. Always safe.",
        46,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav("tasks", "Self-Care Tasks", "/chore-board", "\u{1F33F}", 1),
            nav("calendar", "Meeting Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("library", "Resource Library", "/recipes", "\u{1F4DA}", 3),
            nav("feed", "Share Wall", "/feed", "\u{1F49C}", 4),
            nav("vault", "Private Records", "/vault", "\u{1F512}", 5),
            nav("wellness", "Wellness Supplies", "/shopping", "\u{2728}", 6),
            nav("games", "Mindfulness Activities", "/games", "\u{1F9D8}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Self-Care Tasks",
                "{circle} Self-Care",
                "Small steps, big progress.",
            ),
            page(
                "calendar",
                "Meeting Schedule",
                "{circle} Meetings",
                "When and where we meet.",
            ),
            page(
                "recipes",
                "Resource Library",
                "{circle} Resources",
                "Guides, articles, and helpful links.",
            ),
            page(
                "feed",
                "Share Wall",
                "{circle} Share Wall",
                "A safe space to share.",
            ),
            page(
                "vault",
                "Private Records",
                "{circle} Records",
                "Personal notes and private documents.",
            ),
            page(
                "shopping",
                "Wellness Supplies",
                "{circle} Wellness",
                "Self-care tools and resources.",
            ),
            page(
                "games",
                "Mindfulness Activities",
                "{circle} Mindfulness",
                "Breathing, journaling, and grounding.",
            ),
        ],
        group_onboarding(
            "support-group",
            "group-info",
            "Circle Info",
            "Circle Name",
            "Healing Together",
            "Members",
        ),
        "{circle_name} — {tagline}",
        "{circle_name}: safe support space with meetings, resources, and mindfulness.",
    )
}

// ── Maker Space ───────────────────────────────────────────────────────────

fn maker_space() -> SiteTypeDefinition {
    group_base(
        "maker-space",
        "Maker Space",
        "\u{1F527}",
        "Maker space with build queue, workshop schedule, project plans, and material orders.",
        "Your workshop. One space. Always building.",
        47,
        vec![
            nav("home", "Home", "/", "\u{1F3E0}", 0),
            nav(
                "tasks",
                "Build Queue",
                "/chore-board",
                "\u{1F6E0}\u{FE0F}",
                1,
            ),
            nav("calendar", "Workshop Schedule", "/calendar", "\u{1F4C5}", 2),
            nav("plans", "Project Plans", "/recipes", "\u{1F4D0}", 3),
            nav("feed", "Build Log", "/feed", "\u{1F4DD}", 4),
            nav("vault", "Design Files", "/vault", "\u{1F512}", 5),
            nav("materials", "Material Orders", "/shopping", "\u{1F4E6}", 6),
            nav("games", "Build Challenges", "/games", "\u{1F3AF}", 7),
        ],
        vec![
            page(
                "chore-board",
                "Build Queue",
                "{space} Build Queue",
                "Projects in progress and up next.",
            ),
            page(
                "calendar",
                "Workshop Schedule",
                "{space} Schedule",
                "Open hours, classes, and events.",
            ),
            page(
                "recipes",
                "Project Plans",
                "{space} Project Plans",
                "Blueprints, tutorials, and guides.",
            ),
            page(
                "feed",
                "Build Log",
                "{space} Build Log",
                "Completed projects and in-progress updates.",
            ),
            page(
                "vault",
                "Design Files",
                "{space} Files",
                "CAD files, schematics, and templates.",
            ),
            page(
                "shopping",
                "Material Orders",
                "{space} Orders",
                "Wood, filament, electronics, and supplies.",
            ),
            page(
                "games",
                "Build Challenges",
                "{space} Challenges",
                "Timed builds, contests, and skill tests.",
            ),
        ],
        group_onboarding(
            "maker-space",
            "maker-info",
            "Maker Space Info",
            "Space Name",
            "The Workshop",
            "Makers",
        ),
        "{space_name} — {tagline}",
        "{space_name}: build queue, workshop schedule, project plans, and more.",
    )
}
