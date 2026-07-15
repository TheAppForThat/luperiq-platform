//! SEO Photo Library — Phase 7 (2026-05-27).
//!
//! Field technicians can tag photos taken during a service call as
//! "Good for SEO". When the parent `BookingRequest` carries
//! `seo_use_consent == true`, those photos are auto-submitted here as
//! `PhotoLibraryEntry` rows in `Pending` status. An operator (or office
//! reviewer with the `tenant.seo.review` capability) then approves or
//! rejects each photo. Approved photos are surfaced by the city × pest
//! page generator (see `luperiq_mod_page_generator`) and tagged with
//! geographic + pest metadata so the AI prompts and JSON-LD `ImageObject`
//! schema can use them.
//!
//! Storage: one event per state transition, keyed by `photo_id`.
//! Aggregate type: `SEO:PhotoLibraryEntry`.
//!
//! Tier-2 readiness: every entry carries `licensee_id` so a multi-tenant
//! licensee (ISP / reseller) can scope queries when that layer ships.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Json, Response};
use axum_extra::extract::cookie::CookieJar;
use luperiq_forge::{ApexEvent, ForgeJournal};
use serde::{Deserialize, Serialize};

use super::{SeoState, TOMBSTONE};

// ── Constants ────────────────────────────────────────────────────────

/// WAL aggregate type for photo-library entries.
pub const AGG_PHOTO_LIBRARY: &str = "SEO:PhotoLibraryEntry";

/// Default licensee_id stamped on entries when the platform layer hasn't
/// resolved a Tier-2 owner yet. Mirrors `luperiq_mod_tenant_email`.
pub const DEFAULT_LICENSEE_ID: &str = "platform";

/// Capability required to review (approve / reject) photos.
pub const CAP_SEO_REVIEW: &str = "tenant.seo.review";

// ── Types ────────────────────────────────────────────────────────────

/// Review state of a photo in the SEO library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotoReviewStatus {
    Pending,
    Approved,
    Rejected,
}

impl PhotoReviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }
    pub fn from_query(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }
}

/// A photo submitted by the tech portal that the office must review before
/// the AI page generator may use it on a marketing page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoLibraryEntry {
    /// Unique id (ULID). Doubles as the WAL aggregate key.
    pub photo_id: String,
    pub tenant_id: String,
    /// Tier-2 licensee scope. Defaults to `"platform"`.
    #[serde(default)]
    pub licensee_id: Option<String>,

    // Provenance — every approved photo is traceable back to the booking it came from.
    pub source_work_log_id: String,
    pub source_assignment_id: String,
    pub source_booking_id: String,

    pub image_url: String,
    /// Reserved for a future thumbnail pipeline. Empty until that ships.
    #[serde(default)]
    pub thumbnail_url: Option<String>,

    /// Pest the photo shows (free-form OR a slug from the pest catalog).
    #[serde(default)]
    pub pest_type: Option<String>,
    /// ZIP code where the photo was taken (5-digit, auto-filled from the
    /// parent BookingRequest when the tech didn't override).
    #[serde(default)]
    pub location_zip: Option<String>,
    /// Tech's SEO context note.
    #[serde(default)]
    pub notes: Option<String>,
    /// Office-edited caption (only set after approval).
    #[serde(default)]
    pub caption: Option<String>,

    pub status: PhotoReviewStatus,

    pub submitted_at: i64,
    /// `Technician.id` that submitted the photo.
    pub submitted_by: String,

    #[serde(default)]
    pub reviewed_at: Option<i64>,
    #[serde(default)]
    pub reviewed_by: Option<String>,
    #[serde(default)]
    pub reject_reason: Option<String>,

    /// Mirrors `BookingRequest.seo_use_consent` at submit time. Required to be
    /// true for approval — the office cannot approve a photo whose customer
    /// declined consent.
    pub customer_consent_verified: bool,
}

// ── Submit helper (called from the tech portal) ──────────────────────

/// Arguments accepted by `submit_for_review`. Grouped into a struct so the
/// call sites in `luperiq-mod-tech-portal` stay readable and a future
/// optional field doesn't break callers.
#[derive(Debug, Clone)]
pub struct SubmitArgs {
    pub tenant_id: String,
    pub licensee_id: Option<String>,
    pub source_work_log_id: String,
    pub source_assignment_id: String,
    pub source_booking_id: String,
    pub image_url: String,
    pub pest_type: Option<String>,
    pub location_zip: Option<String>,
    pub notes: Option<String>,
    pub submitted_by: String,
    pub customer_consent_verified: bool,
    pub now_unix: i64,
}

/// Auto-create a `Pending` `PhotoLibraryEntry`. Called by the tech portal
/// after it appends the underlying `FieldOps:WorkLog` event.
///
/// Returns the new `photo_id` on success; returns `Err` when consent is
/// missing (the tech portal already enforces this on the WAL, but we
/// re-check here as defense in depth).
pub fn submit_for_review(j: &mut ForgeJournal, args: SubmitArgs) -> Result<String, String> {
    if !args.customer_consent_verified {
        return Err("customer_consent_verified must be true to submit a photo".to_string());
    }

    let photo_id = ulid::Ulid::new().to_string();
    let entry = PhotoLibraryEntry {
        photo_id: photo_id.clone(),
        tenant_id: args.tenant_id,
        licensee_id: Some(args.licensee_id.unwrap_or_else(|| DEFAULT_LICENSEE_ID.to_string())),
        source_work_log_id: args.source_work_log_id,
        source_assignment_id: args.source_assignment_id,
        source_booking_id: args.source_booking_id,
        image_url: args.image_url,
        thumbnail_url: None,
        pest_type: args.pest_type,
        location_zip: args.location_zip,
        notes: args.notes,
        caption: None,
        status: PhotoReviewStatus::Pending,
        submitted_at: args.now_unix,
        submitted_by: args.submitted_by,
        reviewed_at: None,
        reviewed_by: None,
        reject_reason: None,
        customer_consent_verified: true,
    };

    save_entry(j, &entry)?;
    Ok(photo_id)
}

// ── WAL helpers ──────────────────────────────────────────────────────

/// Persist a `PhotoLibraryEntry` to the journal.
pub fn save_entry(j: &mut ForgeJournal, entry: &PhotoLibraryEntry) -> Result<(), String> {
    let payload = serde_json::to_vec(entry)
        .map_err(|e| format!("serialize PhotoLibraryEntry: {e}"))?;
    let event = ApexEvent::new(AGG_PHOTO_LIBRARY, &entry.photo_id, payload);
    j.append(event)
        .map_err(|e| format!("journal append PhotoLibraryEntry: {e}"))?;
    Ok(())
}

/// Load a single `PhotoLibraryEntry` by id. Returns `None` if the latest
/// event is a tombstone or the entry doesn't exist.
pub fn load_entry(j: &ForgeJournal, photo_id: &str) -> Option<PhotoLibraryEntry> {
    let event = j.get_latest(AGG_PHOTO_LIBRARY, photo_id)?;
    if event.payload == TOMBSTONE {
        return None;
    }
    serde_json::from_slice(&event.payload).ok()
}

/// Load every non-tombstoned entry in the journal. Callers filter further
/// by status / pest / zip.
pub fn load_all_entries(j: &ForgeJournal) -> Vec<PhotoLibraryEntry> {
    j.latest_by_aggregate_type(AGG_PHOTO_LIBRARY)
        .into_iter()
        .filter(|e| e.payload != TOMBSTONE)
        .filter_map(|e| serde_json::from_slice::<PhotoLibraryEntry>(&e.payload).ok())
        .collect()
}

/// Approve a photo and mark the reviewer + timestamp. Idempotent — repeated
/// calls re-stamp the reviewer fields without changing meaning.
pub fn approve(
    j: &mut ForgeJournal,
    photo_id: &str,
    reviewer_id: &str,
    now_unix: i64,
    caption: Option<String>,
) -> Result<(), String> {
    let mut entry =
        load_entry(j, photo_id).ok_or_else(|| format!("photo {photo_id} not found"))?;
    if !entry.customer_consent_verified {
        return Err("cannot approve a photo without customer consent".to_string());
    }
    entry.status = PhotoReviewStatus::Approved;
    entry.reviewed_at = Some(now_unix);
    entry.reviewed_by = Some(reviewer_id.to_string());
    entry.reject_reason = None;
    if caption.is_some() {
        entry.caption = caption;
    }
    save_entry(j, &entry)
}

/// Reject a photo with a reviewer-supplied reason.
pub fn reject(
    j: &mut ForgeJournal,
    photo_id: &str,
    reviewer_id: &str,
    reason: &str,
    now_unix: i64,
) -> Result<(), String> {
    let mut entry =
        load_entry(j, photo_id).ok_or_else(|| format!("photo {photo_id} not found"))?;
    entry.status = PhotoReviewStatus::Rejected;
    entry.reviewed_at = Some(now_unix);
    entry.reviewed_by = Some(reviewer_id.to_string());
    entry.reject_reason = Some(reason.to_string());
    save_entry(j, &entry)
}

/// Query approved photos for the page generator. Filters on pest_type (case
/// and slug insensitive) and optional ZIP membership. When `allow_unzoned`
/// is true, entries without `location_zip` are included as a fallback for
/// thin libraries.
pub fn query_approved_for_generator(
    j: &ForgeJournal,
    pest: Option<&str>,
    city_zips: &[String],
    allow_unzoned: bool,
    limit: usize,
) -> Vec<PhotoLibraryEntry> {
    let pest_norm = pest.map(normalize_pest);
    load_all_entries(j)
        .into_iter()
        .filter(|e| e.status == PhotoReviewStatus::Approved)
        .filter(|e| match (&pest_norm, &e.pest_type) {
            (Some(want), Some(got)) => normalize_pest(got) == *want,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .filter(|e| match &e.location_zip {
            Some(zip) if !zip.is_empty() => {
                city_zips.is_empty() || city_zips.iter().any(|z| z == zip)
            }
            _ => allow_unzoned || city_zips.is_empty(),
        })
        .take(limit)
        .collect()
}

/// Normalize a pest string for matching — lowercased, hyphenated.
fn normalize_pest(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ── Auth ────────────────────────────────────────────────────────────

/// Resolve the operator and check that they have the SEO review capability.
/// Returns `(user_id)` on success; the caller maps `None` to 401.
async fn resolve_seo_reviewer(state: &SeoState, jar: &CookieJar) -> Option<String> {
    let token = jar.get("liq_session")?.value().to_string();
    if token.is_empty() {
        return None;
    }
    let mut j = state.journal.lock().await;
    let auth_cfg = luperiq_forge::AuthConfig {
        jwt_secret: state.jwt_secret.clone(),
        session_ttl_secs: 86400,
        refresh_window_secs: 3600,
    };
    let auth = luperiq_forge::ForgeAuthManager::new(&mut j, auth_cfg).ok()?;
    let claims = auth.validate_session(&token).ok()?;
    // Either tenant.seo.review or platform_operator or admin is enough.
    let ok = luperiq_forge::user_has_capability(&j, &claims.sub, CAP_SEO_REVIEW)
        || luperiq_forge::user_has_capability(&j, &claims.sub, "platform_operator")
        || luperiq_forge::user_has_capability(&j, &claims.sub, "admin");
    if !ok {
        return None;
    }
    Some(claims.sub)
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ── HTTP response helpers ────────────────────────────────────────────

fn entry_to_json(e: &PhotoLibraryEntry) -> serde_json::Value {
    serde_json::json!({
        "photo_id": e.photo_id,
        "tenant_id": e.tenant_id,
        "licensee_id": e.licensee_id,
        "source_work_log_id": e.source_work_log_id,
        "source_assignment_id": e.source_assignment_id,
        "source_booking_id": e.source_booking_id,
        "image_url": e.image_url,
        "thumbnail_url": e.thumbnail_url,
        "pest_type": e.pest_type,
        "location_zip": e.location_zip,
        "notes": e.notes,
        "caption": e.caption,
        "status": e.status.as_str(),
        "submitted_at": e.submitted_at,
        "submitted_by": e.submitted_by,
        "reviewed_at": e.reviewed_at,
        "reviewed_by": e.reviewed_by,
        "reject_reason": e.reject_reason,
        "customer_consent_verified": e.customer_consent_verified,
    })
}

// ── Request types ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct ReviewListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    pest: Option<String>,
    #[serde(default)]
    zip: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApprovePayload {
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RejectPayload {
    pub reason: String,
}

// ── Handlers ────────────────────────────────────────────────────────

/// GET /api/modules/seo/photo-review?status=pending
pub(crate) async fn list_for_review(
    State(state): State<SeoState>,
    jar: CookieJar,
    Query(q): Query<ReviewListQuery>,
) -> Response {
    if resolve_seo_reviewer(&state, &jar).await.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let want = q
        .status
        .as_deref()
        .and_then(PhotoReviewStatus::from_query)
        .unwrap_or(PhotoReviewStatus::Pending);
    let j = state.journal.lock().await;
    let mut entries: Vec<PhotoLibraryEntry> = load_all_entries(&j)
        .into_iter()
        .filter(|e| e.status == want)
        .collect();
    entries.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
    let limit = q.limit.unwrap_or(200).min(500);
    entries.truncate(limit);

    let items: Vec<serde_json::Value> = entries.iter().map(entry_to_json).collect();
    Json(serde_json::json!({
        "ok": true,
        "count": items.len(),
        "status": want.as_str(),
        "items": items,
    }))
    .into_response()
}

/// GET /api/modules/seo/photo-library?pest=X&zip=Y
///
/// Lists approved photos. Used by the page generator AND by the admin
/// browser (so the office can see what's already in the library).
pub(crate) async fn list_library(
    State(state): State<SeoState>,
    jar: CookieJar,
    Query(q): Query<ReviewListQuery>,
) -> Response {
    if resolve_seo_reviewer(&state, &jar).await.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let j = state.journal.lock().await;
    let pest = q.pest.as_deref();
    let zip_filter: Vec<String> = q.zip.iter().cloned().filter(|s| !s.is_empty()).collect();
    let entries = query_approved_for_generator(
        &j,
        pest,
        &zip_filter,
        true,
        q.limit.unwrap_or(100).min(500),
    );
    let items: Vec<serde_json::Value> = entries.iter().map(entry_to_json).collect();
    Json(serde_json::json!({
        "ok": true,
        "count": items.len(),
        "items": items,
    }))
    .into_response()
}

/// POST /api/modules/seo/photo-review/{photo_id}/approve
pub(crate) async fn approve_handler(
    State(state): State<SeoState>,
    jar: CookieJar,
    Path(photo_id): Path<String>,
    payload: Option<Json<ApprovePayload>>,
) -> Response {
    let reviewer = match resolve_seo_reviewer(&state, &jar).await {
        Some(uid) => uid,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    let caption = payload.and_then(|Json(p)| p.caption);
    let mut j = state.journal.lock().await;
    match approve(&mut j, &photo_id, &reviewer, now_unix(), caption) {
        Ok(()) => Json(serde_json::json!({"ok": true, "photo_id": photo_id})).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": e})),
        )
            .into_response(),
    }
}

/// POST /api/modules/seo/photo-review/{photo_id}/reject
pub(crate) async fn reject_handler(
    State(state): State<SeoState>,
    jar: CookieJar,
    Path(photo_id): Path<String>,
    Json(payload): Json<RejectPayload>,
) -> Response {
    let reviewer = match resolve_seo_reviewer(&state, &jar).await {
        Some(uid) => uid,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if payload.reason.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": "reason required"})),
        )
            .into_response();
    }
    let mut j = state.journal.lock().await;
    match reject(&mut j, &photo_id, &reviewer, &payload.reason, now_unix()) {
        Ok(()) => Json(serde_json::json!({"ok": true, "photo_id": photo_id})).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"ok": false, "error": e})),
        )
            .into_response(),
    }
}

/// GET /admin/seo/photo-review — standalone HTML admin page. The richer
/// in-shell experience is in the SPA via the `seo-photo-review` AdminView;
/// this route exists so deep-links and the office portal Phase 4 can embed
/// a single-purpose page without loading the whole admin shell.
///
/// Returns:
///   * 302 → /login when no session cookie present
///   * 401 when the user is logged in but lacks `tenant.seo.review`
///   * HTML otherwise
pub(crate) async fn admin_page(
    State(state): State<SeoState>,
    jar: CookieJar,
) -> Response {
    if jar.get("liq_session").is_none() {
        return axum::response::Redirect::to("/login").into_response();
    }
    if resolve_seo_reviewer(&state, &jar).await.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let j = state.journal.lock().await;
    let mut pending: Vec<PhotoLibraryEntry> = load_all_entries(&j)
        .into_iter()
        .filter(|e| e.status == PhotoReviewStatus::Pending)
        .collect();
    pending.sort_by(|a, b| b.submitted_at.cmp(&a.submitted_at));
    drop(j);

    let mut cards = String::new();
    for entry in &pending {
        let pest = entry.pest_type.as_deref().unwrap_or("—");
        let zip = entry.location_zip.as_deref().unwrap_or("—");
        let notes = entry.notes.as_deref().unwrap_or("");
        cards.push_str(&format!(
            r#"<div class="card" style="border:1px solid #334155;border-radius:10px;padding:14px;background:#0f172a;margin-bottom:14px;display:flex;gap:14px;align-items:flex-start">
              <img src="{img}" alt="" style="width:180px;height:180px;object-fit:cover;border-radius:8px;background:#1e293b">
              <div style="flex:1;color:#e2e8f0;font-size:13px">
                <div style="margin-bottom:6px"><strong>Pest:</strong> {pest} &nbsp;·&nbsp; <strong>ZIP:</strong> {zip}</div>
                <div style="margin-bottom:6px;color:#94a3b8">{notes}</div>
                <div style="font-size:12px;color:#64748b;margin-bottom:10px">Submitted {ts} · booking {booking}</div>
                <div style="display:flex;gap:8px">
                  <button class="btn btn-primary btn-sm" data-action="approve" data-id="{id}">Approve</button>
                  <button class="btn btn-ghost btn-sm" data-action="reject" data-id="{id}">Reject</button>
                </div>
              </div>
            </div>"#,
            img = html_escape(&entry.image_url),
            pest = html_escape(pest),
            zip = html_escape(zip),
            notes = html_escape(notes),
            ts = entry.submitted_at,
            booking = html_escape(&entry.source_booking_id),
            id = html_escape(&entry.photo_id),
        ));
    }
    if pending.is_empty() {
        cards.push_str(
            r#"<div class="muted" style="padding:24px;text-align:center;color:#94a3b8">No photos awaiting review.</div>"#,
        );
    }

    let body = format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>SEO Photo Review</title>
<style>
body{{background:#0b1220;color:#e2e8f0;font-family:-apple-system,Segoe UI,Roboto,sans-serif;margin:0;padding:24px}}
.btn{{cursor:pointer;border:none;border-radius:6px;padding:6px 12px;font-size:13px}}
.btn-primary{{background:#22c55e;color:#022c14}}
.btn-ghost{{background:#1e293b;color:#e2e8f0}}
.btn-sm{{font-size:12px;padding:4px 10px}}
h1{{font-size:22px;margin:0 0 18px 0}}
.muted{{color:#94a3b8}}
</style></head><body>
<h1>SEO Photo Review · {n} pending</h1>
{cards}
<script>
document.body.addEventListener('click', async function(ev) {{
  var btn = ev.target.closest('button[data-action]');
  if (!btn) return;
  var action = btn.getAttribute('data-action');
  var id = btn.getAttribute('data-id');
  if (!id) return;
  if (action === 'approve') {{
    var caption = window.prompt('Optional caption for SEO use:', '');
    var r = await fetch('/api/modules/seo/photo-review/' + encodeURIComponent(id) + '/approve', {{
      method: 'POST', headers: {{'Content-Type':'application/json'}},
      body: JSON.stringify({{caption: caption || null}}), credentials:'include'
    }}).then(function(r) {{ return r.json(); }}).catch(function() {{ return {{ok:false}}; }});
    if (r.ok) {{ btn.closest('.card').style.opacity = '0.4'; }}
    else alert('Approve failed: ' + (r.error || ''));
  }} else if (action === 'reject') {{
    var reason = window.prompt('Reason for rejection (required):', '');
    if (!reason) return;
    var r = await fetch('/api/modules/seo/photo-review/' + encodeURIComponent(id) + '/reject', {{
      method: 'POST', headers: {{'Content-Type':'application/json'}},
      body: JSON.stringify({{reason: reason}}), credentials:'include'
    }}).then(function(r) {{ return r.json(); }}).catch(function() {{ return {{ok:false}}; }});
    if (r.ok) {{ btn.closest('.card').style.opacity = '0.4'; }}
    else alert('Reject failed: ' + (r.error || ''));
  }}
}});
</script>
</body></html>"#,
        n = pending.len(),
        cards = cards,
    );
    Html(body).into_response()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use luperiq_forge::DurabilityMode;
    use tempfile::TempDir;

    fn make_journal() -> (ForgeJournal, TempDir) {
        let dir = TempDir::new().expect("tempdir");
        let wal = dir.path().join("events.wal");
        let snap = dir.path().join("snapshot.bin");
        let journal = ForgeJournal::open(wal, snap, DurabilityMode::Sync).expect("journal open");
        (journal, dir)
    }

    fn make_args(consent: bool) -> SubmitArgs {
        SubmitArgs {
            tenant_id: "test-tenant".to_string(),
            licensee_id: None,
            source_work_log_id: "wl-1".to_string(),
            source_assignment_id: "asg-1".to_string(),
            source_booking_id: "bk-1".to_string(),
            image_url: "/uploads/p1.jpg".to_string(),
            pest_type: Some("Termites".to_string()),
            location_zip: Some("75201".to_string()),
            notes: Some("nest visible on baseboard".to_string()),
            submitted_by: "tech-1".to_string(),
            customer_consent_verified: consent,
            now_unix: 1_700_000_000,
        }
    }

    #[test]
    fn submit_creates_pending_entry_when_consent_ok() {
        let (mut j, _dir) = make_journal();
        let id = submit_for_review(&mut j, make_args(true)).expect("submit");
        let e = load_entry(&j, &id).expect("load");
        assert_eq!(e.status, PhotoReviewStatus::Pending);
        assert_eq!(e.tenant_id, "test-tenant");
        assert_eq!(e.licensee_id.as_deref(), Some("platform"));
        assert_eq!(e.pest_type.as_deref(), Some("Termites"));
        assert_eq!(e.location_zip.as_deref(), Some("75201"));
        assert!(e.customer_consent_verified);
    }

    #[test]
    fn submit_rejects_when_consent_missing() {
        let (mut j, _dir) = make_journal();
        let res = submit_for_review(&mut j, make_args(false));
        assert!(res.is_err());
        assert!(load_all_entries(&j).is_empty());
    }

    #[test]
    fn approve_changes_status_and_caption() {
        let (mut j, _dir) = make_journal();
        let id = submit_for_review(&mut j, make_args(true)).unwrap();
        approve(&mut j, &id, "reviewer-1", 1_700_000_100, Some("Termite nest in Dallas garage".into()))
            .unwrap();
        let e = load_entry(&j, &id).unwrap();
        assert_eq!(e.status, PhotoReviewStatus::Approved);
        assert_eq!(e.reviewed_by.as_deref(), Some("reviewer-1"));
        assert_eq!(e.caption.as_deref(), Some("Termite nest in Dallas garage"));
    }

    #[test]
    fn reject_records_reason() {
        let (mut j, _dir) = make_journal();
        let id = submit_for_review(&mut j, make_args(true)).unwrap();
        reject(&mut j, &id, "reviewer-1", "blurry", 1_700_000_100).unwrap();
        let e = load_entry(&j, &id).unwrap();
        assert_eq!(e.status, PhotoReviewStatus::Rejected);
        assert_eq!(e.reject_reason.as_deref(), Some("blurry"));
    }

    #[test]
    fn query_approved_filters_pest_and_zip() {
        let (mut j, _dir) = make_journal();
        let mut args1 = make_args(true);
        args1.pest_type = Some("Termites".into());
        args1.location_zip = Some("75201".into());
        let id1 = submit_for_review(&mut j, args1).unwrap();
        approve(&mut j, &id1, "rev", 0, None).unwrap();

        let mut args2 = make_args(true);
        args2.pest_type = Some("Roaches".into());
        args2.location_zip = Some("75201".into());
        let id2 = submit_for_review(&mut j, args2).unwrap();
        approve(&mut j, &id2, "rev", 0, None).unwrap();

        let mut args3 = make_args(true);
        args3.pest_type = Some("Termites".into());
        args3.location_zip = Some("77001".into());
        let _id3 = submit_for_review(&mut j, args3).unwrap();
        // intentionally not approved → should never appear

        let mut args4 = make_args(true);
        args4.pest_type = Some("Termites".into());
        args4.location_zip = None;
        let id4 = submit_for_review(&mut j, args4).unwrap();
        approve(&mut j, &id4, "rev", 0, None).unwrap();

        // Termites + ZIPs of Dallas (75201), unzoned allowed → id1 + id4
        let dallas = vec!["75201".to_string()];
        let approved = query_approved_for_generator(&j, Some("Termites"), &dallas, true, 10);
        let ids: Vec<&str> = approved.iter().map(|e| e.photo_id.as_str()).collect();
        assert!(ids.contains(&id1.as_str()), "expected id1, got {ids:?}");
        assert!(ids.contains(&id4.as_str()), "expected id4 (unzoned), got {ids:?}");
        assert!(!ids.iter().any(|x| *x == id2.as_str()), "id2 is Roaches");

        // Disallowing unzoned drops id4
        let strict = query_approved_for_generator(&j, Some("Termites"), &dallas, false, 10);
        let ids2: Vec<&str> = strict.iter().map(|e| e.photo_id.as_str()).collect();
        assert!(ids2.contains(&id1.as_str()));
        assert!(!ids2.contains(&id4.as_str()));

        // No-pest filter just returns everything approved
        let any = query_approved_for_generator(&j, None, &[], true, 10);
        assert_eq!(any.len(), 3);
    }

    #[test]
    fn cross_tenant_isolation_via_separate_journals() {
        // Each tenant has its own ForgeJournal in production. We model that
        // by opening two independent journals and verifying neither sees the
        // other's events.
        let (mut j_a, _dir_a) = make_journal();
        let (mut j_b, _dir_b) = make_journal();

        let mut a_args = make_args(true);
        a_args.tenant_id = "tenant-a".into();
        let id_a = submit_for_review(&mut j_a, a_args).unwrap();

        let mut b_args = make_args(true);
        b_args.tenant_id = "tenant-b".into();
        let id_b = submit_for_review(&mut j_b, b_args).unwrap();

        // A sees only A; B sees only B.
        let a_entries: Vec<String> = load_all_entries(&j_a)
            .into_iter()
            .map(|e| e.photo_id)
            .collect();
        let b_entries: Vec<String> = load_all_entries(&j_b)
            .into_iter()
            .map(|e| e.photo_id)
            .collect();

        assert_eq!(a_entries, vec![id_a.clone()]);
        assert_eq!(b_entries, vec![id_b.clone()]);
        assert!(load_entry(&j_a, &id_b).is_none());
        assert!(load_entry(&j_b, &id_a).is_none());
    }

    #[test]
    fn normalize_pest_handles_casing_and_punctuation() {
        assert_eq!(normalize_pest("Termites"), "termites");
        assert_eq!(normalize_pest("German Cockroaches"), "german-cockroaches");
        assert_eq!(normalize_pest("Termites!!"), "termites");
        assert_eq!(normalize_pest("  brown recluse spider  "), "brown-recluse-spider");
        assert_eq!(normalize_pest("ANT/SUGAR"), "ant-sugar");
    }
}

