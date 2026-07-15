use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Aggregate type constant for Nexus client records.
pub const AGG_NEX_CLIENT: &str = "NexClient";

/// A license tier definition (static, hardcoded).
#[derive(Debug, Clone, Serialize)]
pub struct TierDef {
    pub slug: &'static str,
    pub name: &'static str,
    pub price: f64,
    pub max_sites: i32,
    pub monthly_credits: u64,
    pub modules_included: ModuleSet,
    pub features: &'static [&'static str],
    pub priority_support: bool,
}

/// Describes the set of modules included in a tier.
#[derive(Debug, Clone, Serialize)]
pub enum ModuleSet {
    /// Explicit list of module slugs.
    Explicit(&'static [&'static str]),
    /// Named sets: "standard", "standard_plus", "all".
    Named(&'static str),
}

/// Hardcoded global tier definitions (matches WordPress SubscriptionManager).
pub static TIERS: &[TierDef] = &[
    TierDef {
        slug: "free",
        name: "Free",
        price: 0.0,
        max_sites: 1,
        monthly_credits: 0,
        modules_included: ModuleSet::Explicit(&["dashboard"]),
        features: &["basic_analytics"],
        priority_support: false,
    },
    TierDef {
        slug: "starter",
        name: "Starter",
        price: 29.0,
        max_sites: 1,
        monthly_credits: 150,
        modules_included: ModuleSet::Named("standard"),
        features: &["basic_analytics", "ai_seo_basic", "email_reports"],
        priority_support: false,
    },
    TierDef {
        slug: "professional",
        name: "Professional",
        price: 99.0,
        max_sites: 3,
        monthly_credits: 500,
        modules_included: ModuleSet::Named("standard_plus"),
        features: &[
            "advanced_analytics",
            "ai_seo_full",
            "email_reports",
            "api_access",
            "white_label",
        ],
        priority_support: false,
    },
    TierDef {
        slug: "enterprise",
        name: "Enterprise",
        price: 299.0,
        max_sites: -1,
        monthly_credits: 1500,
        modules_included: ModuleSet::Named("all"),
        features: &[
            "advanced_analytics",
            "ai_seo_full",
            "email_reports",
            "api_access",
            "white_label",
            "custom_branding",
            "dedicated_support",
        ],
        priority_support: true,
    },
];

/// Field-service tier definition for truck/vehicle-based industries.
#[derive(Debug, Clone, Serialize)]
pub struct FieldServiceTierDef {
    pub slug: &'static str,
    pub name: &'static str,
    pub price_monthly: f64,
    pub trial_days: u32,
    pub trucks_included: u32,
    pub truck_addon_monthly: f64,
    pub monthly_credits: u64,
    pub modules_included: ModuleSet,
    pub features: &'static [&'static str],
    pub priority_support: bool,
    pub lifetime_truck_price: f64,
    pub setup_addon_price: f64,
}

/// Hardcoded field-service tier definitions.
pub static FIELD_SERVICE_TIERS: &[FieldServiceTierDef] = &[
    FieldServiceTierDef {
        slug: "free",
        name: "Free Trial",
        price_monthly: 0.0,
        trial_days: 7,
        trucks_included: 1,
        truck_addon_monthly: 0.0,
        monthly_credits: 50,
        modules_included: ModuleSet::Named("standard_plus"),
        features: &["full_features_trial"],
        priority_support: false,
        lifetime_truck_price: 0.0,
        setup_addon_price: 0.0,
    },
    FieldServiceTierDef {
        slug: "starter",
        name: "Starter",
        price_monthly: 49.0,
        trial_days: 0,
        trucks_included: 1,
        truck_addon_monthly: 49.0,
        monthly_credits: 0,
        modules_included: ModuleSet::Named("standard_plus"),
        features: &["basic_analytics", "ai_seo_basic", "email_reports"],
        priority_support: false,
        lifetime_truck_price: 0.0,
        setup_addon_price: 0.0,
    },
    FieldServiceTierDef {
        slug: "professional",
        name: "Professional",
        price_monthly: 147.0,
        trial_days: 0,
        trucks_included: 3,
        truck_addon_monthly: 49.0,
        monthly_credits: 0,
        modules_included: ModuleSet::Named("standard_plus"),
        features: &[
            "advanced_analytics",
            "ai_seo_full",
            "email_reports",
            "api_access",
        ],
        priority_support: false,
        lifetime_truck_price: 0.0,
        setup_addon_price: 499.0,
    },
];

/// Payload for a Nexus client record — the core tenant/license entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NexClientPayload {
    pub site_url: String,
    pub site_domain: String,
    pub site_name: String,
    pub license_key: String,
    pub license_tier: String,
    pub license_status: String,
    #[serde(default)]
    pub license_expires: Option<String>,
    pub credits_remaining: i64,
    #[serde(default)]
    pub bundle_credits_remaining: i64,
    #[serde(default)]
    pub bundle_credits_expires_at: Option<String>,
    pub credits_total: i64,
    #[serde(default)]
    pub credits_reset_date: Option<String>,
    pub max_domains: u32,
    #[serde(default)]
    pub admin_email: Option<String>,
    #[serde(default)]
    pub contact_name: Option<String>,
    #[serde(default)]
    pub site_type_slug: Option<String>,
    #[serde(default)]
    pub enabled_modules: Option<Vec<String>>,
    #[serde(default)]
    pub plugin_version: Option<String>,
    #[serde(default)]
    pub wp_version: Option<String>,
    #[serde(default)]
    pub user_id: Option<u64>,
    #[serde(default)]
    pub order_id: Option<u64>,
    #[serde(default)]
    pub product_id: Option<u64>,
    #[serde(default)]
    pub features_enabled: Option<String>,
    #[serde(default)]
    pub modules_entitled: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub cortex_shared_secret: Option<String>,
    #[serde(default)]
    pub tier_grant_active: Option<String>,
    #[serde(default)]
    pub tier_grant_expires_at: Option<String>,
    #[serde(default)]
    pub tier_grant_spending_counter: Option<u64>,
    #[serde(default)]
    pub trial_started_at: Option<String>,
    #[serde(default)]
    pub trial_paused_at: Option<String>,
    #[serde(default)]
    pub trial_days_used: Option<f64>,
    #[serde(default)]
    pub trial_pause_count: Option<u32>,
    #[serde(default)]
    pub trial_status: Option<String>,
    #[serde(default)]
    pub last_heartbeat_at: Option<String>,
    #[serde(default)]
    pub heartbeat_version: Option<String>,
    #[serde(default)]
    pub heartbeat_ip: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub created_by_email: Option<String>,
    #[serde(default)]
    pub created_from_ip: Option<String>,
    #[serde(default)]
    pub signup_user_agent: Option<String>,
    #[serde(default)]
    pub trial_expires_at: Option<String>,
    #[serde(default)]
    pub backend_port: Option<u16>,
    #[serde(default)]
    pub tenant_dir: Option<String>,
    #[serde(default)]
    pub last_login_at: Option<String>,
    #[serde(default)]
    pub last_login_ip: Option<String>,
    #[serde(default)]
    pub login_count: Option<u32>,
    #[serde(default)]
    pub magic_links_sent: Option<u32>,
    #[serde(default)]
    pub distinct_admin_emails: Option<u32>,
    #[serde(default)]
    pub last_admin_action_at: Option<String>,
    #[serde(default)]
    pub disk_usage_bytes: Option<u64>,
    #[serde(default)]
    pub wal_size_bytes: Option<u64>,
    #[serde(default)]
    pub disk_scanned_at: Option<String>,
    #[serde(default)]
    pub page_count: Option<u32>,
    #[serde(default)]
    pub post_count: Option<u32>,
    #[serde(default)]
    pub media_count: Option<u32>,
    #[serde(default)]
    pub form_submissions_30d: Option<u32>,
    #[serde(default)]
    pub ai_interactions_30d: Option<u32>,
    #[serde(default)]
    pub pageviews_30d: Option<u32>,
    #[serde(default)]
    pub human_pageviews_30d: Option<u32>,
    #[serde(default)]
    pub bot_pageviews_30d: Option<u32>,
    #[serde(default)]
    pub unique_visitors_30d: Option<u32>,
    #[serde(default)]
    pub last_published_at: Option<String>,
    #[serde(default)]
    pub metrics_collected_at: Option<String>,
    #[serde(default)]
    pub credits_used_lifetime: Option<u64>,
    #[serde(default)]
    pub credits_used_30d: Option<u64>,
    #[serde(default)]
    pub total_spent_usd_cents: Option<u64>,
    #[serde(default)]
    pub last_payment_at: Option<String>,
    #[serde(default = "default_trucks")]
    pub trucks: u32,
    #[serde(default)]
    pub billing_cadence: Option<String>,
}

fn default_trucks() -> u32 {
    1
}

impl Default for NexClientPayload {
    fn default() -> Self {
        Self {
            site_url: String::new(),
            site_domain: String::new(),
            site_name: String::new(),
            license_key: String::new(),
            license_tier: "free".into(),
            license_status: "active".into(),
            license_expires: None,
            credits_remaining: 0,
            bundle_credits_remaining: 0,
            bundle_credits_expires_at: None,
            credits_total: 0,
            credits_reset_date: None,
            max_domains: 1,
            admin_email: None,
            contact_name: None,
            site_type_slug: None,
            enabled_modules: None,
            plugin_version: None,
            wp_version: None,
            user_id: None,
            order_id: None,
            product_id: None,
            features_enabled: None,
            modules_entitled: None,
            notes: None,
            cortex_shared_secret: None,
            tier_grant_active: None,
            tier_grant_expires_at: None,
            tier_grant_spending_counter: None,
            trial_started_at: None,
            trial_paused_at: None,
            trial_days_used: None,
            trial_pause_count: None,
            trial_status: None,
            last_heartbeat_at: None,
            heartbeat_version: None,
            heartbeat_ip: None,
            created_at: None,
            created_by_email: None,
            created_from_ip: None,
            signup_user_agent: None,
            trial_expires_at: None,
            backend_port: None,
            tenant_dir: None,
            last_login_at: None,
            last_login_ip: None,
            login_count: None,
            magic_links_sent: None,
            distinct_admin_emails: None,
            last_admin_action_at: None,
            disk_usage_bytes: None,
            wal_size_bytes: None,
            disk_scanned_at: None,
            page_count: None,
            post_count: None,
            media_count: None,
            form_submissions_30d: None,
            ai_interactions_30d: None,
            pageviews_30d: None,
            human_pageviews_30d: None,
            bot_pageviews_30d: None,
            unique_visitors_30d: None,
            last_published_at: None,
            metrics_collected_at: None,
            credits_used_lifetime: None,
            credits_used_30d: None,
            total_spent_usd_cents: None,
            last_payment_at: None,
            trucks: 1,
            billing_cadence: None,
        }
    }
}

impl NexClientPayload {
    pub fn enabled_module_keys(&self) -> Vec<String> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();

        if let Some(modules) = &self.enabled_modules {
            for module in modules {
                let key = module.trim();
                if key.is_empty() {
                    continue;
                }
                let key = key.to_string();
                if seen.insert(key.clone()) {
                    out.push(key);
                }
            }
        }

        if let Some(csv) = &self.modules_entitled {
            for module in csv.split(',') {
                let key = module.trim();
                if key.is_empty() {
                    continue;
                }
                let key = key.to_string();
                if seen.insert(key.clone()) {
                    out.push(key);
                }
            }
        }

        out
    }

    pub fn has_enabled_module(&self, module_key: &str) -> bool {
        let target = module_key.trim();
        !target.is_empty()
            && self
                .enabled_module_keys()
                .iter()
                .any(|module| module == target)
    }
}
