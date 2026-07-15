//! Trial WAL operations — SiteTrial aggregate read/write.

use luperiq_forge::{ApexEvent, ForgeJournal};
use serde::{Deserialize, Serialize};

/// WAL aggregate type for site trials.
pub const AGG_SITE_TRIAL: &str = "SalesPipeline:SiteTrial";

/// WAL aggregate type for granted lifetime entitlements (recorded by
/// the `/lifetime/thank-you` handler after a verified Stripe payment).
/// Aggregate id is the lowercased customer email.
pub const AGG_LIFETIME_ENTITLEMENT: &str = "LifetimeEntitlement:Granted";

/// One granted lifetime entitlement. Looked up by email when a new
/// signup arrives so the resulting SiteTrial gets created already in
/// the paid stage instead of the free trial stage.
#[derive(Debug, Clone, Deserialize)]
pub struct LifetimeEntitlement {
    pub email: String,
    pub tier: String,
    #[serde(default)]
    pub stripe_session_id: String,
    #[serde(default)]
    pub amount_cents: i64,
    #[serde(default)]
    pub site_type: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub granted_at: u64,
    /// Per-truck quantity purchased (field-service pricing). Defaults to 1
    /// for entitlements granted before this field existed.
    #[serde(default = "default_one_truck")]
    pub trucks: u32,
}

fn default_one_truck() -> u32 {
    1
}

/// Look up the latest LifetimeEntitlement for an email. Returns
/// `Some` only if an entitlement exists and hasn't been
/// tombstoned. Used by start_trial to skip the 7-day free
/// expiration when the visitor already paid lifetime.
pub fn find_lifetime_entitlement(
    journal: &ForgeJournal,
    email: &str,
) -> Option<LifetimeEntitlement> {
    let key = email.to_lowercase();
    let evt = journal.get_latest(AGG_LIFETIME_ENTITLEMENT, &key)?;
    if evt.payload == b"__DELETED__" {
        return None;
    }
    serde_json::from_slice::<LifetimeEntitlement>(&evt.payload).ok()
}

/// A site trial record persisted in the WAL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteTrial {
    pub trial_id: String,
    pub email: String,
    pub industry_slug: String,
    pub stage: String, // free | paid | expired | converted | deactivated
    pub free_started_at: u64,
    pub free_expires_at: u64,
    pub paid_started_at: Option<u64>,
    pub paid_expires_at: Option<u64>,
    pub stripe_session_id: Option<String>,
    pub converted_to: Option<String>,
    pub converted_at: Option<u64>,
    pub created_at: u64,
    /// Domain assigned to this trial site (e.g. "mysite.luperiq.com")
    #[serde(default)]
    pub domain: Option<String>,
    /// Random token for cancel-by-email link verification
    #[serde(default)]
    pub cancel_token: Option<String>,
    /// When the site was deactivated (if stage == "deactivated")
    #[serde(default)]
    pub deactivated_at: Option<u64>,
    /// Referral code used at signup, if any. Usually the referrer's
    /// subdomain (e.g. "acme-pest-control") so admins can see who
    /// sent whom.
    #[serde(default)]
    pub referred_by: Option<String>,
    /// Whether the visitor opted into the platform-operator "done-for-you"
    /// onboarding add-on at signup. None = older trial recorded before the
    /// field existed; treat as not requested.
    #[serde(default)]
    pub setup_addon_requested: Option<bool>,
    /// Snapshot of the add-on price (in cents) the visitor saw at signup.
    /// Locks in the agreed amount even if the operator later changes the
    /// field-service tier table. None = not requested / older trial.
    #[serde(default)]
    pub setup_addon_price_cents: Option<u32>,
    /// Promo code entered at signup, captured for billing integration.
    #[serde(default)]
    pub promo_code: Option<String>,
    /// Unix epoch SECONDS at which an active "flash 10% off" window expires.
    /// SET (= now + 4h) at the moment the day-1 `flash10` conversion email is
    /// sent by the apex reminder worker; this is the SINGLE source of truth the
    /// lifetime checkout reads to decide whether to apply the extra 10% off.
    /// `None` (serde default) = no active flash window — existing pre-feature
    /// records deserialize cleanly as None and get full price.
    #[serde(default)]
    pub flash_offer_expires_at: Option<u64>,
}

/// Get the latest trial for a given email address.
///
/// Scans all SiteTrial aggregates and returns the most recent one matching the
/// provided email (by highest `created_at`).
pub fn get_trial(journal: &ForgeJournal, email: &str) -> Option<SiteTrial> {
    let all = journal.latest_by_aggregate_type(AGG_SITE_TRIAL);
    let lower = email.to_lowercase();
    all.into_iter()
        .filter_map(|e| serde_json::from_slice::<SiteTrial>(&e.payload).ok())
        .filter(|t| t.email.to_lowercase() == lower)
        .max_by_key(|t| t.created_at)
}

/// Get a trial by its trial_id (WAL aggregate ID).
pub fn get_trial_by_id(journal: &ForgeJournal, trial_id: &str) -> Option<SiteTrial> {
    journal
        .get_latest(AGG_SITE_TRIAL, trial_id)
        .and_then(|e| serde_json::from_slice::<SiteTrial>(&e.payload).ok())
}

/// Get all trials (latest version of each aggregate).
pub fn list_trials(journal: &ForgeJournal) -> Vec<SiteTrial> {
    journal
        .latest_by_aggregate_type(AGG_SITE_TRIAL)
        .into_iter()
        .filter_map(|e| serde_json::from_slice::<SiteTrial>(&e.payload).ok())
        .collect()
}

/// Append (or update) a trial event to the WAL.
pub fn write_trial(journal: &mut ForgeJournal, trial: &SiteTrial) -> Result<(), String> {
    let bytes = serde_json::to_vec(trial).map_err(|e| format!("Serialize error: {e}"))?;
    let event = ApexEvent::new(AGG_SITE_TRIAL, &trial.trial_id, bytes);
    journal
        .append(event)
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use luperiq_forge::DurabilityMode;
    use tempfile::tempdir;

    fn fresh_journal() -> (tempfile::TempDir, ForgeJournal) {
        let dir = tempdir().unwrap();
        let j = ForgeJournal::open(
            dir.path().join("w.bin"),
            dir.path().join("s.bin"),
            DurabilityMode::Sync,
        )
        .unwrap();
        (dir, j)
    }

    fn base_trial(id: &str) -> SiteTrial {
        SiteTrial {
            trial_id: id.into(),
            email: "x@example.com".into(),
            industry_slug: "pest-control".into(),
            stage: "free".into(),
            free_started_at: 1,
            free_expires_at: 2,
            paid_started_at: None,
            paid_expires_at: None,
            stripe_session_id: None,
            converted_to: None,
            converted_at: None,
            created_at: 1,
            domain: Some("d.example.com".into()),
            cancel_token: None,
            deactivated_at: None,
            referred_by: None,
            setup_addon_requested: None,
            setup_addon_price_cents: None,
            promo_code: None,
            flash_offer_expires_at: None,
        }
    }

    #[test]
    fn site_trial_roundtrips_with_addon_fields_populated() {
        let (_dir, mut j) = fresh_journal();
        let mut t = base_trial("t1");
        t.setup_addon_requested = Some(true);
        t.setup_addon_price_cents = Some(49900);
        write_trial(&mut j, &t).unwrap();
        let back = get_trial_by_id(&j, "t1").expect("trial roundtrips");
        assert_eq!(back.setup_addon_requested, Some(true));
        assert_eq!(back.setup_addon_price_cents, Some(49900));
    }

    #[test]
    fn site_trial_with_addon_fields_none_serializes_compactly() {
        // skip_serializing_if isn't set on these fields, but defaults to None
        // serialize as `null` and back-deserialize as None — verify the
        // round-trip preserves both nones.
        let (_dir, mut j) = fresh_journal();
        let t = base_trial("t2");
        write_trial(&mut j, &t).unwrap();
        let back = get_trial_by_id(&j, "t2").expect("trial roundtrips");
        assert_eq!(back.setup_addon_requested, None);
        assert_eq!(back.setup_addon_price_cents, None);
    }

    #[test]
    fn site_trial_older_payload_without_new_fields_still_deserializes() {
        // Simulates a trial recorded before the add-on fields existed.
        // JSON intentionally omits the two new fields — #[serde(default)]
        // on Option<T> should fill them with None.
        let json = serde_json::json!({
            "trial_id": "legacy",
            "email": "x@example.com",
            "industry_slug": "pest-control",
            "stage": "free",
            "free_started_at": 1,
            "free_expires_at": 2,
            "paid_started_at": null,
            "paid_expires_at": null,
            "stripe_session_id": null,
            "converted_to": null,
            "converted_at": null,
            "created_at": 1,
            "domain": "d.example.com",
            "cancel_token": null,
            "deactivated_at": null,
            "referred_by": null
            // setup_addon_requested + setup_addon_price_cents intentionally omitted
        });
        let bytes = serde_json::to_vec(&json).unwrap();
        let back: SiteTrial = serde_json::from_slice(&bytes).expect("legacy payload decodes");
        assert_eq!(back.trial_id, "legacy");
        assert_eq!(back.setup_addon_requested, None);
        assert_eq!(back.setup_addon_price_cents, None);
    }
}


