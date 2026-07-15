//! `IndustryBlueprint` — the single source of RECORD per industry/site-type.
//!
//! ## Why this exists (Specifier Phase 1, WS1.1)
//!
//! An industry's definition is scattered across sources that hand-sync and drift:
//!
//! 1. `luperiq-cms/src/industry_defs.rs` — `IndustryDef { display_name, .. }`.
//!    (cms-local; NOT reachable from this crate — see the cms-side parity test.)
//!    NOTE: the onboarding-WIZARD producers (`industry_onboarding_steps`,
//!    `terminology_onboarding_json`) were RETIRED in WS1.5 Phase 2 — onboarding
//!    is now produced solely by provisioning (#2 below) and read through
//!    `IndustryBlueprint::onboarding()`.
//! 2. `luperiq-mod-site-catalog/.../defaults.rs` — per-site-type `enabled_modules`
//!    (the engine BUNDLE) + the provisioning onboarding steps + pricing/tier. 48
//!    site-types. This is the engine spine.
//! 3. `luperiq-forge/src/terminology.rs` — `default_terminology(group_type)`, the
//!    vocabulary layer (Guide→Recipes, Inventory→Pantry, …). Keyed by group_type;
//!    the live app passes the industry/site-type slug DIRECTLY as the group_type
//!    (verified: site_pages/helpers.rs, base_templates.rs, theme_studio,
//!    site_blueprint/retype.rs, main.rs all call
//!    `default_terminology(industry_slug)`), so industry_slug == group_type key.
//! 4. `scripts/provision-site.sh template_family_for()` — industry → 1 of 10
//!    template families. Shell-only; its table is PORTED here (Rust cannot call
//!    a bash function, and sourcing the script runs provisioning at load).
//!
//! ## Design: a consolidating VIEW, not a copy
//!
//! Duplicating `enabled_modules` / vocabulary / onboarding into a second literal
//! table would just create a NEW thing to drift from the old ones. Instead the
//! Blueprint RESOLVES the engine-derived facets against their live single source
//! (`all_defaults()`, `default_terminology()`), and stores as DATA only the
//! facets that today live in non-Rust / out-of-band places (template_family,
//! tiers, aliases). This makes the Blueprint a faithful single source of record
//! that cannot silently drift from the engine spine — the parity test proves it.
//!
//! ## WS1.1 scope: PURE ADDITION
//!
//! Nothing here is wired to a caller yet (that is WS1.5). The old sources are
//! untouched. This module + its tests are the only additions.

use crate::site_catalog::defaults::all_defaults_ref;
use crate::site_catalog::types::{OnboardingStep, SiteTypeDefinition};
use luperiq_forge::terminology::default_terminology;
use luperiq_forge::GroupTerminology;

/// A named subset of an industry's module bundle (the Lite/Pro mechanism).
///
/// `modules` is a subset of the industry's full `enabled_modules`. Every tier
/// MUST be dependency-closed under the WS0.1 module dep-gate
/// (`build_registry` + `validate_dependencies_result`) — that is asserted in
/// the cms-side `dep_gate_tests`, the same harness that already validates every
/// site-type bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tier {
    /// Tier slug, e.g. "full" / "free".
    pub name: &'static str,
    /// Human label, e.g. "Full Suite" / "Free".
    pub label: &'static str,
    /// The module slugs included in this tier. A subset of `enabled_modules`.
    pub modules: Vec<String>,
    /// True when this tier is known NOT to be dependency-closed yet (the
    /// dep-gate is EXPECTED to flag it). Used so the dep-gate test can treat a
    /// documented decoupling blocker as a known-open finding rather than a
    /// suite-breaking regression. As of 2026-06 the pest "free" tier is
    /// dependency-CLOSED (this flag is `false` for it) — see `tiers_for`.
    pub dep_gate_known_open: bool,
}

/// The consolidated single source of record for one industry / site-type.
///
/// Facets fall in two classes:
/// - DERIVED (resolved live from the engine spine): `enabled_modules`,
///   `onboarding`, `vocabulary`. These cannot drift from the engine because
///   they ARE the engine's output.
/// - RECORDED (data owned here, consolidated from out-of-band sources):
///   `display_name` (ABSORBED WS1.5), `template_family`, `tiers`, `aliases`.
#[derive(Debug, Clone)]
pub struct IndustryBlueprint {
    /// Canonical industry/site-type slug (== the site-catalog SiteTypeDefinition
    /// slug, and == the group_type key passed to `default_terminology`).
    pub industry_slug: &'static str,
    /// RECORDED human label for this industry/site-type. ABSORBED in WS1.5: a
    /// real stored literal per slug (NOT derived from the site-catalog `name`).
    /// For the 43 agreeing slugs it equals site-catalog `name`; for the 4
    /// Dave-decided divergent slugs it is the canonical label Dave picked
    /// (restaurant/creator/blog/mobile-field-service). The two parity tests
    /// below enumerate exactly which callers see a changed label.
    pub display_name: &'static str,
    /// Template family (1 of 10, or `None` when the slug routes to base
    /// templates). Ported from `scripts/provision-site.sh template_family_for`.
    pub template_family: Option<&'static str>,
    /// Named module subsets (tiers). Always contains at least `"full"`.
    pub tiers: Vec<Tier>,
    /// Alternate slugs that normalize to this canonical slug (from
    /// `provision-site.sh canonical_industry`). Friendly/legacy inputs.
    pub aliases: &'static [&'static str],
}

impl IndustryBlueprint {
    /// The site-catalog definition that backs this blueprint (the engine spine).
    /// `None` only if the slug is absent from the catalog (should not happen for
    /// canonical slugs — every blueprint is seeded FROM the catalog).
    pub fn site_type_def(&self) -> Option<&'static SiteTypeDefinition> {
        all_defaults_ref().iter().find(|d| d.slug == self.industry_slug)
    }

    /// RECORDED: human display label. ABSORBED in WS1.5 — returns the stored
    /// literal (`self.display_name`), no longer derived from
    /// `SiteTypeDefinition.name`. Returns `Some` for parity with the prior
    /// `Option` signature so existing callers compile unchanged; it is always
    /// `Some` for a canonical blueprint.
    pub fn display_name(&self) -> Option<&'static str> {
        Some(self.display_name)
    }

    /// DERIVED: the full engine bundle. Single source = SiteTypeDefinition.enabled_modules.
    pub fn enabled_modules(&self) -> &'static [String] {
        self.site_type_def()
            .map(|d| d.enabled_modules.as_slice())
            .unwrap_or(&[])
    }

    /// DERIVED: provisioning onboarding. Single source = SiteTypeDefinition.onboarding_steps.
    /// (NOTE: distinct from the cms `industry_defs` onboarding WIZARD — see the
    /// module docs and the WS1.1 report; those two are a real divergence.)
    pub fn onboarding(&self) -> &'static [OnboardingStep] {
        self.site_type_def()
            .map(|d| d.onboarding_steps.as_slice())
            .unwrap_or(&[])
    }

    /// DERIVED: vocabulary. Single source = `default_terminology(group_type)`,
    /// where group_type == this industry_slug (the live-app key mapping).
    pub fn vocabulary(&self) -> GroupTerminology {
        default_terminology(self.industry_slug)
    }

    /// Look up a tier by slug.
    pub fn tier(&self, name: &str) -> Option<&Tier> {
        self.tiers.iter().find(|t| t.name == name)
    }
}

/// Template family for a canonical slug — a faithful PORT of
/// `scripts/provision-site.sh template_family_for()`. Returns `None` for the
/// shell `*) echo "" ;;` fallback (routes to base templates).
///
/// Only the CANONICAL slugs are included (the bash function is called AFTER
/// `canonical_industry()`); friendly aliases like "coffee"/"artisan" are folded
/// into `aliases` and resolved to their canonical slug before lookup.
fn template_family_for(slug: &str) -> Option<&'static str> {
    match slug {
        "restaurant" | "bakery" | "coffee-shop" => Some("food"),
        "pest-control" | "hvac" | "plumbing" | "electrical" | "landscaping"
        | "mobile-field-service" | "cell-phone-repair" | "electronics-repair"
        | "auto-repair" => Some("service"),
        "artisan-market" | "maker-space" => Some("artisan"),
        "medical-office" | "attorney" | "accountant" | "insurance" | "salon" => {
            Some("professional")
        }
        "creator" | "blog" | "app-publisher" => Some("creator"),
        "family" | "roommates" | "pet-owners" | "elder-care" | "support-group"
        | "farm" => Some("family"),
        "sports-team" | "club" | "book-club" | "neighborhood" | "band" | "scouts"
        | "fitness" | "business-team" | "nonprofit" => Some("community"),
        "church" | "small-group" | "mission-team" => Some("faith"),
        "classroom" | "homeschool" | "homeschool-coop" => Some("learning"),
        "wedding" | "reunion" | "memorial" | "travel" => Some("events"),
        _ => None,
    }
}

/// Aliases ported from `provision-site.sh canonical_industry()`. Maps a
/// canonical slug to its known friendly/legacy input forms. Empty for slugs
/// with no aliases.
fn aliases_for(slug: &str) -> &'static [&'static str] {
    match slug {
        "business-team" => &["business"],
        "coffee-shop" => &["coffee"],
        "artisan-market" => &["artisan"],
        "cell-phone-repair" => &["device-repair"],
        "salon" => &["salon-barbershop"],
        "medical-office" => &[
            "medical",
            "healthcare",
            "clinic",
            "doctor",
            "dentist",
            "dental",
            "med-spa",
            "medspa",
            "medical-spa",
            "plastic-surgeon",
            "plastic-surgery",
            "chiropractic",
            "mental-health",
        ],
        _ => &[],
    }
}

/// Build the `tiers` for a site-type. Today the ONLY structured tier that exists
/// across the engine is the implicit "everything" bundle, so every blueprint
/// gets a `"full"` tier == its `enabled_modules` (dependency-closed by
/// construction — it equals the bundle the WS0.1 dep-gate already validates).
///
/// The stripped "free" tier is modeled as a MECHANISM here but populated ONLY
/// where it is a real, decided product (pest-control today). It is the POST-TRIAL
/// FREE tier: a fresh-signup pest site runs FULL for a 7-day trial, then on expiry
/// the "meaty" business cluster goes dark while content/SEO/lead-capture stay live.
///
/// CLOSURE (2026-06): the free tier drops the WHOLE business cluster INCLUDING its
/// dependents — {booking, invoicing, commerce, technicians, customer-portal,
/// inspections, financing, availability, field-ops, tech-portal}. Dropping the
/// cluster wholesale (leaders + dependents) is the clean dependency-CLOSED cut, so
/// `dep_gate_known_open` is `false` and the dep-gate ASSERTS it resolves with zero
/// violations. The old "site-pages hard-deps booking/customer-portal" rationale is
/// STALE: `SitePagesModule::dependencies()` was decoupled to `["company-profile"]`
/// (which is in the keep-set), so site-pages no longer pulls the business modules.
fn tiers_for(def: &SiteTypeDefinition) -> Vec<Tier> {
    let full = Tier {
        name: "full",
        label: "Full Suite",
        modules: def.enabled_modules.clone(),
        dep_gate_known_open: false,
    };

    let mut tiers = vec![full];

    if def.slug == "pest-control" {
        // FREE (post-trial) = full pest site MINUS the entire business-management
        // cluster (11 modules): the 10-module business cluster PLUS
        // state-license-lookup, which hard-depends on technicians. We drop the
        // cluster WHOLESALE — and because state-license-lookup is enabled
        // service-wide by provisioning, dropping technicians WITHOUT also
        // dropping state-license-lookup orphans its dependency and crashloops
        // the tenant at boot. Dropping all 11 is the dependency-CLOSED cut
        // (proven: NO module outside this set depends on any module inside it,
        // so the stripped bundle boots with zero dep-gate violations).
        // Content/SEO/lead-capture (forms, messaging, seo, site-pages,
        // content-pipeline, page-generator, email-marketing, company-profile, …)
        // all stay live.
        let drop: &[&str] = &[
            "booking",
            "invoicing",
            "commerce",
            "technicians",
            "customer-portal",
            "inspections",
            "financing",
            "availability",
            "field-ops",
            "tech-portal",
            // Depends on technicians (dropped above) — must drop too or the
            // tenant registry panics on next boot. See defaults.rs note.
            "state-license-lookup",
        ];
        let free_modules: Vec<String> = def
            .enabled_modules
            .iter()
            .filter(|m| !drop.contains(&m.as_str()))
            .cloned()
            .collect();
        tiers.push(Tier {
            name: "free",
            label: "Free",
            modules: free_modules,
            dep_gate_known_open: false,
        });
    }

    tiers
}

/// RECORDED canonical human label per canonical slug (ABSORBED, WS1.5 Phase 1).
///
/// For the 43 agreeing slugs the value EQUALS the site-catalog
/// `SiteTypeDefinition.name`; for the 4 slugs where the two historical sources
/// (site-catalog `name` vs cms `industry_defs.display_name`) DIVERGED, this is
/// the canonical label Dave decided (FINAL):
/// - restaurant           -> "Restaurant & Food Service" (== site-catalog name)
/// - creator              -> "Creator / Influencer"      (== industry_defs)
/// - blog                 -> "Blog / Writer"             (== industry_defs)
/// - mobile-field-service -> "Mobile Field Service"      (== industry_defs)
///
/// The two parity tests below enumerate exactly which callers see a flip:
/// site-catalog readers flip on {creator, blog, mobile-field-service};
/// industry_defs readers flip on {restaurant}. A new/4th drift fails a test.
fn display_name_for(slug: &str) -> &'static str {
    match slug {
        // ── Dave-decided divergent labels (FINAL) ──
        "restaurant" => "Restaurant & Food Service",
        "creator" => "Creator / Influencer",
        "blog" => "Blog / Writer",
        "mobile-field-service" => "Mobile Field Service",
        // ── 43 agreeing slugs: recorded == site-catalog name ──
        "accountant" => "Accountant / CPA Firm",
        "app-publisher" => "App Publisher",
        "artisan-market" => "Artisan Market",
        "attorney" => "Attorney / Law Firm",
        "auto-repair" => "Auto Repair",
        "bakery" => "Bakery",
        "band" => "Creative Crew",
        "book-club" => "Book Club",
        "business-team" => "Team Website",
        "cell-phone-repair" => "Cell Phone Repair",
        "church" => "Church",
        "classroom" => "Classroom Website",
        "club" => "Club Website",
        "coffee-shop" => "Coffee Shop",
        "elder-care" => "Care Circle",
        "electrical" => "Electrical",
        "electronics-repair" => "Electronics Repair",
        "family" => "LuperIQ Family",
        "farm" => "Homestead Website",
        "fitness" => "Fitness Group",
        "homeschool-coop" => "Homeschool Co-op",
        "homeschool" => "Homeschool Academy",
        "hvac" => "HVAC",
        "insurance" => "Insurance Agency",
        "landscaping" => "Landscaping",
        "maker-space" => "Maker Space",
        "medical-office" => "Medical Office",
        "memorial" => "Memorial Space",
        "mission-team" => "Mission Team",
        "neighborhood" => "Neighborhood Website",
        "nonprofit" => "Nonprofit Website",
        "pest-control" => "Pest Control",
        "pet-owners" => "Pet Care Website",
        "plumbing" => "Plumbing",
        "reunion" => "Reunion Website",
        "roommates" => "Shared Household",
        "salon" => "Salon",
        "scouts" => "Troop Website",
        "small-group" => "Small Group",
        "sports-team" => "Sports Team",
        "support-group" => "Support Circle",
        "travel" => "Travel Group",
        "wedding" => "Wedding Planner",
        // Fallback: should never hit for a canonical slug (the parity test
        // proves every site-catalog slug is enumerated here). Returns "" so a
        // missing slug fails the parity test loudly rather than silently
        // resolving to the wrong label.
        _ => "",
    }
}

/// The full registry: one `IndustryBlueprint` per site-catalog site-type.
///
/// Seeded FROM `all_defaults()` (the engine spine is the canonical key set — 48
/// site-types), so the Blueprint never invents or omits a slug. The derived
/// facets resolve live against the spine; the recorded facets (template_family,
/// tiers, aliases) are attached per slug.
pub fn all_blueprints() -> Vec<IndustryBlueprint> {
    all_defaults_ref()
        .iter()
        .map(|def| {
            // `all_defaults_ref()` is OnceLock-memoized and returns &'static, so
            // `def.slug.as_str()` is already a process-lifetime &'static str —
            // borrow it (NO leak). This keeps `all_blueprints()` / `blueprint_for`
            // allocation-light enough for a per-request WS1.5 caller.
            let slug: &'static str = def.slug.as_str();
            IndustryBlueprint {
                industry_slug: slug,
                display_name: display_name_for(slug),
                template_family: template_family_for(slug),
                tiers: tiers_for(def),
                aliases: aliases_for(slug),
            }
        })
        .collect()
}

/// Look up a blueprint by canonical slug OR any of its aliases.
pub fn blueprint_for(slug: &str) -> Option<IndustryBlueprint> {
    all_blueprints()
        .into_iter()
        .find(|b| b.industry_slug == slug || b.aliases.contains(&slug))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every site-catalog site-type yields exactly one blueprint, keyed identically.
    #[test]
    fn blueprint_covers_every_site_type() {
        let defs = all_defaults_ref();
        let bps = all_blueprints();
        assert_eq!(
            bps.len(),
            defs.len(),
            "blueprint count must equal site-type count"
        );
        for def in defs {
            let bp = blueprint_for(&def.slug)
                .unwrap_or_else(|| panic!("no blueprint for site-type '{}'", def.slug));
            assert_eq!(bp.industry_slug, def.slug);
        }
    }

    /// CONSISTENCY BY CONSTRUCTION (NOT independent parity): the Blueprint
    /// DELEGATES these facets to the live single source (`all_defaults_ref`), it
    /// does not independently re-derive them. So this asserts the delegation is
    /// wired correctly (the facade returns the spine's value for every slug) —
    /// it can surface a wiring/lookup bug, but by design it cannot surface a
    /// drift between two independent copies, because there is only one copy.
    /// Independent parity for these facets is a WS1.5 absorption step (copy the
    /// data INTO the Blueprint, then parity-test absorbed-vs-live). See report.
    #[test]
    fn consistency_by_construction_derived_facets() {
        for def in all_defaults_ref() {
            let bp = blueprint_for(&def.slug).unwrap();

            // NOTE: display_name was ABSORBED in WS1.5 (it is now a RECORDED
            // literal, no longer derived) — its parity is owned by
            // `display_name_recorded_matches_site_catalog_EXCEPT_known` below,
            // which allows the 3 intentional divergences this test could not.
            assert_eq!(
                bp.enabled_modules(),
                def.enabled_modules.as_slice(),
                "enabled_modules parity drift for '{}'",
                def.slug
            );
            // OnboardingStep does not derive PartialEq (and we keep the old
            // source untouched per WS1.1), so compare by serialized form — both
            // sides are the SAME type resolved from the SAME def, so this proves
            // the Blueprint's onboarding() faithfully returns the spine's steps.
            assert_eq!(
                serde_json::to_value(bp.onboarding()).unwrap(),
                serde_json::to_value(def.onboarding_steps.as_slice()).unwrap(),
                "onboarding parity drift for '{}'",
                def.slug
            );
        }
    }

    /// CONSISTENCY BY CONSTRUCTION (vocabulary): the Blueprint delegates
    /// vocabulary to `default_terminology(industry_slug)` — the SAME call the
    /// live app makes (verified in site_pages/helpers.rs, base_templates.rs,
    /// theme_studio, site_blueprint/retype.rs, main.rs). This locks the KEY
    /// MAPPING used (industry_slug passed directly as group_type) and that the
    /// delegation is faithful. It is NOT independent parity: both sides call the
    /// same function with the same key. The value is locking the live key
    /// mapping into the Blueprint contract, not detecting drift.
    #[test]
    fn consistency_vocabulary_uses_live_group_type_key() {
        for def in all_defaults_ref() {
            let bp = blueprint_for(&def.slug).unwrap();
            let via_bp = bp.vocabulary();
            let via_source = default_terminology(&def.slug);
            assert_eq!(
                via_bp.group_type, via_source.group_type,
                "vocabulary group_type drift for '{}'",
                def.slug
            );
            assert_eq!(
                via_bp.group_noun, via_source.group_noun,
                "vocabulary group_noun drift for '{}'",
                def.slug
            );
            // Module label maps must match key-for-key.
            assert_eq!(
                via_bp.modules.len(),
                via_source.modules.len(),
                "vocabulary module-label count drift for '{}'",
                def.slug
            );
        }
    }

    /// PARITY: template_family port matches the bash table for known slugs, and
    /// the fallback is None (= bash `*) echo "" ;;`). Golden assertions on the
    /// canonical service/family/food slugs that have live template trees.
    #[test]
    fn template_family_golden() {
        let cases: &[(&str, Option<&str>)] = &[
            ("pest-control", Some("service")),
            ("hvac", Some("service")),
            ("restaurant", Some("food")),
            ("bakery", Some("food")),
            ("coffee-shop", Some("food")),
            ("artisan-market", Some("artisan")),
            ("maker-space", Some("artisan")),
            ("medical-office", Some("professional")),
            ("salon", Some("professional")),
            ("creator", Some("creator")),
            ("blog", Some("creator")),
            ("family", Some("family")),
            ("church", Some("faith")),
            ("classroom", Some("learning")),
            ("wedding", Some("events")),
            ("memorial", Some("events")),
        ];
        for (slug, want) in cases {
            let bp = blueprint_for(slug)
                .unwrap_or_else(|| panic!("no blueprint for '{slug}'"));
            assert_eq!(
                bp.template_family, *want,
                "template_family drift for '{slug}'"
            );
        }
    }

    /// Aliases resolve to the canonical blueprint (canonical_industry port).
    #[test]
    fn aliases_resolve_to_canonical() {
        assert_eq!(
            blueprint_for("coffee").map(|b| b.industry_slug),
            Some("coffee-shop")
        );
        assert_eq!(
            blueprint_for("business").map(|b| b.industry_slug),
            Some("business-team")
        );
        assert_eq!(
            blueprint_for("medical").map(|b| b.industry_slug),
            Some("medical-office")
        );
    }

    /// Every blueprint has a "full" tier whose modules == its enabled_modules.
    /// Tiers are NET-NEW structured data (no Lite/Pro subsets existed before),
    /// so this is a structural invariant, NOT a parity-vs-old-source claim.
    #[test]
    fn full_tier_equals_bundle() {
        for def in all_defaults_ref() {
            let bp = blueprint_for(&def.slug).unwrap();
            let full = bp.tier("full").expect("every blueprint has a full tier");
            assert_eq!(
                full.modules,
                def.enabled_modules,
                "full tier must equal bundle for '{}'",
                def.slug
            );
            assert!(!full.dep_gate_known_open, "full tier is dep-closed");
        }
    }

    /// The pest "free" (post-trial) tier is dependency-CLOSED: it drops the
    /// whole business cluster (11 modules = the 10 business modules + the
    /// state-license-lookup orphan, which hard-deps technicians) and is a
    /// strict subset of the bundle.
    #[test]
    fn free_pest_tier_is_closed_subset_dropping_business_cluster() {
        let bp = blueprint_for("pest-control").unwrap();
        let free = bp.tier("free").expect("pest-control has a free tier");
        assert!(
            !free.dep_gate_known_open,
            "free-pest is dependency-closed (no known-open flag)"
        );
        let full: std::collections::HashSet<_> =
            bp.enabled_modules().iter().cloned().collect();
        for m in &free.modules {
            assert!(full.contains(m), "free module '{m}' must be in the bundle");
        }
        // The whole 11-module business cluster must be ABSENT (proves we dropped
        // what we meant — a misspelled drop slug would silently keep the module).
        // state-license-lookup is included because it hard-deps technicians and
        // must drop with it (its presence here also cross-checks that edit #1
        // landed it into the pest bundle via business_base).
        const DROPPED: &[&str] = &[
            "booking", "invoicing", "commerce", "technicians", "customer-portal",
            "inspections", "financing", "availability", "field-ops", "tech-portal",
            "state-license-lookup",
        ];
        for d in DROPPED {
            assert!(
                full.contains(*d),
                "sanity: '{d}' must exist in the pest bundle to be a real drop"
            );
            assert!(
                !free.modules.iter().any(|m| m == d),
                "free tier must NOT contain dropped business module '{d}'"
            );
        }
        assert_eq!(
            free.modules.len(),
            bp.enabled_modules().len() - DROPPED.len(),
            "free tier = bundle minus exactly the 11 dropped modules"
        );
    }

    /// PARITY + DIVERGENCE LOCK (site-catalog side). The RECORDED
    /// `display_name` ABSORBED in WS1.5 equals the live site-catalog
    /// `SiteTypeDefinition.name` for EVERY slug, EXCEPT the exact set
    /// {creator, blog, mobile-field-service} — the slugs where Dave's canonical
    /// label intentionally differs from the site-catalog name (his pick there
    /// equals the cms `industry_defs` value). This is half of the caller-visible
    /// label enumeration: callers that read site-catalog `name` as the human
    /// industry label FLIP on exactly these 3 slugs. A 4th drift (or a missing
    /// expected drift) fails this test. The other half — callers that read
    /// `industry_defs.display_name`, which flip on {restaurant} — is locked by
    /// the cms-side `display_name_recorded_matches_industry_defs_EXCEPT_known`.
    #[test]
    #[allow(non_snake_case)] // EXCEPT_known intentional per WS1.5 spec
    fn display_name_recorded_matches_site_catalog_EXCEPT_known() {
        use std::collections::BTreeSet;
        // The ONLY slugs where recorded display_name may differ from the
        // site-catalog name. Caller-visible: site-catalog `name` readers flip
        // here.
        let expected_divergent: BTreeSet<&str> =
            ["creator", "blog", "mobile-field-service"].into_iter().collect();

        let mut observed_divergent: BTreeSet<&str> = BTreeSet::new();
        for def in all_defaults_ref() {
            let bp = blueprint_for(&def.slug).unwrap();
            let recorded = bp.display_name().expect("canonical slug has a label");
            if recorded != def.name.as_str() {
                observed_divergent.insert(def.slug.as_str());
            }
        }

        assert_eq!(
            observed_divergent, expected_divergent,
            "display_name vs site-catalog divergence set changed: expected {:?}, observed {:?}. \
             A new drift means a caller reading site-catalog `name` flips label — decide deliberately.",
            expected_divergent, observed_divergent
        );

        // Spot-lock the exact recorded values on the divergent slugs so a value
        // typo (not just set membership) is caught.
        assert_eq!(
            blueprint_for("creator").unwrap().display_name(),
            Some("Creator / Influencer")
        );
        assert_eq!(
            blueprint_for("blog").unwrap().display_name(),
            Some("Blog / Writer")
        );
        assert_eq!(
            blueprint_for("mobile-field-service").unwrap().display_name(),
            Some("Mobile Field Service")
        );
        // And lock restaurant: recorded == site-catalog name (so it must NOT be
        // in the divergent set here, even though it DOES diverge from
        // industry_defs — that's the cms-side test's job).
        assert_eq!(
            blueprint_for("restaurant").unwrap().display_name(),
            Some("Restaurant & Food Service")
        );
    }
}
