use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Aggregate type for WAL storage.
pub const AGG_TERMINOLOGY: &str = "Group:Terminology";
/// Singleton aggregate ID.
pub const TERMINOLOGY_ID: &str = "global";

/// Root terminology config for a site. Every user-facing label comes from here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupTerminology {
    pub group_type: String,
    pub group_noun: String,
    pub group_noun_plural: String,
    pub member_noun: String,
    pub member_noun_plural: String,
    pub leader_noun: String,
    pub admin_section_label: String,
    pub login_greeting: String,
    pub guest_tagline: String,
    pub accent_color: String,
    pub roles: Vec<RoleDefinition>,
    pub modules: HashMap<String, ModuleLabels>,
    pub onboarding: OnboardingTerminology,
    pub permissions: PermissionLabels,
    #[serde(default)]
    pub route_slug_overrides: HashMap<String, String>,
}

/// Labels for a single module concept (task_board, calendar, feed, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleLabels {
    pub label: String,
    pub item_noun: String,
    pub item_noun_plural: String,
    pub emoji: String,
    pub subtitle: String,
    pub empty_state: String,
    pub add_button: String,
    pub field_label: String,
    pub placeholder: String,
    pub points_label: Option<String>,
}

/// A role definition within a group type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub slug: String,
    pub label: String,
    pub color: String,
    pub priority: u32,
}

/// Onboarding prompts and examples for a group type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingTerminology {
    pub member_prompt: String,
    pub name_field_label: String,
    pub name_placeholder: String,
    pub member_examples: Vec<String>,
    pub feature_labels: Vec<String>,
}

/// Permission labels for a group type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionLabels {
    pub can_post_feed: String,
    pub can_manage_tasks: String,
    pub can_manage_calendar: String,
    pub can_manage_resources: String,
    pub can_manage_vault: String,
    pub can_view_finances: String,
}

impl GroupTerminology {
    /// Safe lookup for a module's labels. Returns a generic fallback if the
    /// concept key is missing.
    pub fn module_labels(&self, concept: &str) -> &ModuleLabels {
        static FALLBACK: std::sync::LazyLock<ModuleLabels> =
            std::sync::LazyLock::new(|| ModuleLabels {
                label: "Module".into(),
                item_noun: "Item".into(),
                item_noun_plural: "Items".into(),
                emoji: "\u{1F4CB}".into(),
                subtitle: String::new(),
                empty_state: "Nothing here yet.".into(),
                add_button: "Add Item".into(),
                field_label: "Name".into(),
                placeholder: String::new(),
                points_label: None,
            });
        self.modules.get(concept).unwrap_or(&FALLBACK)
    }

    /// Given a chassis-shaped slug, return the industry overlay slug if configured.
    pub fn lookup_overlay_for_chassis(&self, chassis: &str) -> Option<&str> {
        self.route_slug_overrides.get(chassis).map(String::as_str)
    }

    /// Reverse lookup: given an overlay slug, find the chassis slug it maps to.
    pub fn lookup_chassis_for_overlay(&self, overlay: &str) -> Option<&str> {
        self.route_slug_overrides
            .iter()
            .find(|(_chassis, ov)| ov.as_str() == overlay)
            .map(|(chassis, _)| chassis.as_str())
    }
}

/// Returns the complete default terminology for a given industry slug.
/// Unknown slugs get generic "Group" terminology.
///
/// The full implementation lives in `luperiq-domain-core` on the hosted platform.
/// This stub returns a minimal generic terminology for compilation purposes.
pub fn default_terminology(slug: &str) -> GroupTerminology {
    GroupTerminology {
        group_type: slug.to_string(),
        group_noun: "Group".into(),
        group_noun_plural: "Groups".into(),
        member_noun: "Member".into(),
        member_noun_plural: "Members".into(),
        leader_noun: "Admin".into(),
        admin_section_label: "Group Website".into(),
        login_greeting: "Welcome! Who's checking in?".into(),
        guest_tagline: "Your group. One place. Always connected.".into(),
        accent_color: "#6366f1".into(),
        roles: vec![RoleDefinition {
            slug: "leader".into(),
            label: "Admin".into(),
            color: "#3b82f6".into(),
            priority: 1,
        }],
        modules: HashMap::new(),
        onboarding: OnboardingTerminology {
            member_prompt: "Who's in your group?".into(),
            name_field_label: "Group Name".into(),
            name_placeholder: "My Group".into(),
            member_examples: vec![],
            feature_labels: vec![],
        },
        permissions: PermissionLabels {
            can_post_feed: "Can post updates".into(),
            can_manage_tasks: "Can manage tasks".into(),
            can_manage_calendar: "Can manage calendar".into(),
            can_manage_resources: "Can manage resources".into(),
            can_manage_vault: "Can manage vault".into(),
            can_view_finances: "Can view finances".into(),
        },
        route_slug_overrides: HashMap::new(),
    }
}
