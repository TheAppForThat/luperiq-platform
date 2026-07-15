//! `claim.rs` — Phase 4 (directory hardening): owner claim flow + dashboard.
//!
//! Lets a real business owner CLAIM their directory listing with a free,
//! email-verified account, unlocking the Owner viewer tier (full, unmasked
//! data) and an engagement dashboard ("14 calls this month").
//!
//! ## Ownership authority
//! The ForgeJournal `user_id` — supplied by `main.rs` via the injected
//! [`DirViewer`] extension (Approach B; this crate never touches forge/quiz2
//! auth) — is the sole ownership authority. A visitor with no `liq_session`
//! (`DirViewer.user_id == None`) is bounced to the real login/register flow
//! with a `?next=` round-trip back to the claim URL.
//!
//! ## Verification (no auto-verify — security)
//! Submitting the form writes a *pending* (`verified=0`) claim with a one-time
//! token + 24h expiry and emails the member a verification link. Clicking it
//! flips `verified=1`, clears the token (single-use), and stamps
//! `companies.claimed_by`. Only THEN does `resolve_tier` see a verified owner
//! and unmask. If the email cannot be sent (sender absent or errored) the claim
//! stays pending and the user is told to retry/contact support — the row is
//! NEVER auto-verified.
//!
//! ## Email bridge (isolation preserved)
//! This crate does NOT depend on `luperiq-mod-smtp`. Instead [`EmailSender`] is
//! a boxed closure stored on `DirRouteState`; `main.rs` constructs it from the
//! existing smtp orchestrator (`send_email_internal`) and passes it into
//! [`crate::directory_routes`]. When `None`, sends are treated as a failure and
//! the claim stays pending (with a clear "couldn't send" message).
//!
//! ## CSRF posture
//! The CMS CSRF helpers live in the `luperiq-cms` auth module; importing them
//! would break this crate's dependency isolation. Instead the write is bound to
//! the authenticated `DirViewer.user_id`: a forged cross-site POST can only
//! create a *pending, email-unverified* claim for the attacker's OWN logged-in
//! session — it cannot verify (needs the token mailed to the member) and cannot
//! unmask anyone's data. Low impact; documented in the Phase 4 report.

use crate::store::DirectoryMiniSitePage;
use crate::{viewer::DirViewer, DirRouteState};
use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::{Form, Json};
use serde::Deserialize;
use std::sync::Arc;

/// Boxed email sender: `(to, subject, html_body) -> Result<(), String>`.
/// Constructed in `main.rs` from the smtp orchestrator; `None` ⇒ no mail.
pub type EmailSender = Arc<dyn Fn(&str, &str, &str) -> Result<(), String> + Send + Sync>;

/// 24 hours, in seconds — the verification-link lifetime.
const TOKEN_TTL_SECS: i64 = 24 * 60 * 60;
const SITE_BASE: &str = "https://pestcontroller.org";

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Build the login round-trip URL for an anonymous visitor. `/login` honors a
/// sanitized `?next=` and links onward to `/register` for new accounts, so a
/// member who signs up lands back on the claim page. (`/join` is the
/// membership funnel but does not round-trip a return path; `/login?next=` is
/// the real path that does.)
fn login_redirect(claim_path: &str) -> String {
    format!("/login?next={}", urlencode(claim_path))
}

/// Minimal application/x-www-form-urlencoded-style percent encoding for the
/// `next=` value (path + query). Avoids adding a urlencoding dep to this
/// dependency-isolated crate. Encodes everything outside the RFC-3986
/// unreserved set plus `/`.
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ── GET /directory/claim?company={id} ────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ClaimQuery {
    pub company: String,
}

pub(crate) async fn claim_form_handler(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Query(q): Query<ClaimQuery>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    let claim_path = format!("/directory/claim?company={}", urlencode(&q.company));

    // Not logged in → bounce to login/register with a return round-trip.
    let Some(user_id) = viewer.user_id.clone() else {
        return Redirect::to(&login_redirect(&claim_path)).into_response();
    };

    let store = &s.store;
    let Some(co) = store.company_by_id(&q.company) else {
        return crate::not_found_public();
    };

    // Already a verified owner → straight to the (now unmasked) company page.
    if store.is_verified_owner(&co.id, &user_id) {
        return Redirect::to(&company_url(store, &co.id)).into_response();
    }

    let company_display = co.dba.clone().unwrap_or_else(|| co.entity_name.clone());
    let mut ctx = tera::Context::new();
    ctx.insert("company_id", &co.id);
    ctx.insert("company_display_name", &company_display);
    ctx.insert("state_abbr", &co.state_abbr);
    ctx.insert("page_title", &format!("Claim {company_display} — pestcontroller.org"));
    ctx.insert("error", &Option::<String>::None);
    crate::render_page(&s.tera, "pages/directory-claim.html", ctx)
}

// ── POST /directory/claim/submit ─────────────────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct ClaimSubmit {
    pub company_id: String,
    /// Bundled legal attestation checkbox. HTML checkboxes only POST when
    /// checked, so presence == checked. REQUIRED.
    #[serde(default)]
    pub attest_legal: Option<String>,
    /// Optional newsletter opt-in checkbox.
    #[serde(default)]
    pub newsletter: Option<String>,
}

pub(crate) async fn claim_submit_handler(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    headers: axum::http::HeaderMap,
    Form(form): Form<ClaimSubmit>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    let claim_path = format!("/directory/claim?company={}", urlencode(&form.company_id));

    // Auth gate (also the CSRF anchor — the write is bound to this user_id).
    let Some(user_id) = viewer.user_id.clone() else {
        return Redirect::to(&login_redirect(&claim_path)).into_response();
    };

    let store = &s.store;
    let Some(co) = store.company_by_id(&form.company_id) else {
        return crate::not_found_public();
    };
    let company_display = co.dba.clone().unwrap_or_else(|| co.entity_name.clone());

    // Already verified → nothing to do; send them to the company page.
    if store.is_verified_owner(&co.id, &user_id) {
        return Redirect::to(&company_url(store, &co.id)).into_response();
    }

    // REQUIRED: legal attestation must be checked.
    let attest = form.attest_legal.is_some();
    if !attest {
        let mut ctx = tera::Context::new();
        ctx.insert("company_id", &co.id);
        ctx.insert("company_display_name", &company_display);
        ctx.insert("state_abbr", &co.state_abbr);
        ctx.insert("page_title", &format!("Claim {company_display} — pestcontroller.org"));
        ctx.insert(
            "error",
            &Some("Please confirm the ownership attestation to continue.".to_string()),
        );
        return (
            StatusCode::BAD_REQUEST,
            crate::render_page(&s.tera, "pages/directory-claim.html", ctx),
        )
            .into_response();
    }
    let newsletter = form.newsletter.is_some();

    // Verification destination authority: the logged-in account's OWN email
    // (from the validated session, injected on DirViewer by main.rs) is the
    // correct, deliverable address for the member who is claiming. Fall back to
    // the scraped company contact email only when the account has no email on
    // file. If BOTH are absent we send nothing and the claim stays pending
    // (handled below) — never auto-verified.
    let to_email = viewer
        .email
        .clone()
        .filter(|e| e.contains('@'))
        .or_else(|| co.email.clone().filter(|e| e.contains('@')))
        .unwrap_or_default();

    // Generate single-use token + 24h expiry; store PENDING (verified=0).
    let token = store.new_claim_token();
    let expiry = now_secs() + TOKEN_TTL_SECS;
    let ip = crate::ip_hash_public(&crate::extract_ip_public(&headers));

    if let Err(e) = store.insert_or_update_claim(
        &co.id,
        &user_id,
        &to_email,
        &token,
        expiry,
        attest,
        newsletter,
        &ip,
    ) {
        tracing::error!("[directory] claim upsert failed: {e}");
        return claim_error_page(
            &s,
            &company_display,
            "We couldn't save your claim just now. Please try again shortly.",
        );
    }

    // Build verification link.
    let verify_url = format!("{SITE_BASE}/directory/claim/verify?token={token}");
    let subject = format!("Verify your claim of {company_display}");
    let body = verification_email_html(&company_display, &verify_url);

    // Attempt to send. NO sender, no address, or a send error → keep the claim
    // pending and tell the user. Never auto-verify.
    let send_result: Result<(), String> = match (&s.email_sender, to_email.is_empty()) {
        (_, true) => Err("no destination email on file".to_string()),
        (Some(sender), false) => sender(&to_email, &subject, &body),
        (None, false) => Err("email sender not configured".to_string()),
    };

    match send_result {
        Ok(()) => {
            let mut ctx = tera::Context::new();
            ctx.insert("company_display_name", &company_display);
            ctx.insert("email_to", &mask_email(&to_email));
            ctx.insert("page_title", "Check your email — pestcontroller.org");
            crate::render_page(&s.tera, "pages/directory-claim-sent.html", ctx)
        }
        Err(e) => {
            tracing::warn!("[directory] claim verification email not sent ({e}); claim left pending for {}", co.id);
            claim_error_page(
                &s,
                &company_display,
                "Your claim is saved, but we couldn't send the verification email. \
                 Please try again in a few minutes, or contact support so we can verify you manually.",
            )
        }
    }
}

fn claim_error_page(s: &DirRouteState, company_display: &str, msg: &str) -> Response {
    let mut ctx = tera::Context::new();
    ctx.insert("company_display_name", company_display);
    ctx.insert("message", msg);
    ctx.insert("ok", &false);
    ctx.insert("page_title", "Claim — pestcontroller.org");
    crate::render_page(&s.tera, "pages/directory-claim-verified.html", ctx)
}

// ── GET /directory/claim/verify?token={token} ────────────────────────────────

#[derive(Deserialize)]
pub(crate) struct VerifyQuery {
    pub token: String,
}

pub(crate) async fn claim_verify_handler(
    State(s): State<DirRouteState>,
    Query(q): Query<VerifyQuery>,
) -> Response {
    let store = &s.store;
    match store.verify_claim_by_token(&q.token) {
        Some((company_id, _user_id)) => {
            // Bust the 5s owner-tier cache so the very next render unmasks.
            crate::viewer::invalidate_owner_cache();
            let url = format!("{}?claimed=1", company_url(store, &company_id));
            Redirect::to(&url).into_response()
        }
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("company_display_name", &"");
            ctx.insert(
                "message",
                "This verification link is invalid or has expired. Please re-claim your listing to get a fresh link.",
            );
            ctx.insert("ok", &false);
            ctx.insert("page_title", "Link expired — pestcontroller.org");
            (
                StatusCode::BAD_REQUEST,
                crate::render_page(&s.tera, "pages/directory-claim-verified.html", ctx),
            )
                .into_response()
        }
    }
}

// ── GET /directory/my-listings ───────────────────────────────────────────────

pub(crate) async fn my_listings_handler(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    let Some(user_id) = viewer.user_id.clone() else {
        return Redirect::to(&login_redirect("/directory/my-listings")).into_response();
    };

    let store = &s.store;
    let since = now_secs() - 30 * 24 * 60 * 60;

    let mut listings: Vec<serde_json::Value> = Vec::new();
    for claim in store.claims_for_user(&user_id) {
        let Some(co) = store.company_by_id(&claim.company_id) else {
            continue;
        };
        let company_display = co.dba.clone().unwrap_or_else(|| co.entity_name.clone());
        // 30-day engagement counts by event_type.
        let counts = store.engagement_counts_for_company(&co.id, since);
        let mut phone = 0i64;
        let mut email = 0i64;
        let mut website = 0i64;
        let mut total = 0i64;
        for (etype, n) in &counts {
            total += n;
            match etype.as_str() {
                "phone_reveal" => phone += n,
                "email_reveal" => email += n,
                "website_reveal" => website += n,
                _ => {}
            }
        }
        // Company URL for management link.
        let url = company_url(store, &co.id);
        listings.push(serde_json::json!({
            "company_id": co.id,
            "name": company_display,
            "location": location_label(&co),
            "url": url,
            "edit_url": format!("/directory/my-listings/{}/edit", co.id),
            "phone_reveals": phone,
            "email_reveals": email,
            "website_reveals": website,
            "total_reveals": total,
        }));
    }

    let mut ctx = tera::Context::new();
    ctx.insert("listings", &listings);
    ctx.insert("has_listings", &!listings.is_empty());
    ctx.insert("page_title", "My Listings — pestcontroller.org");
    crate::render_page(&s.tera, "pages/directory-my-listings.html", ctx)
}

// ── Owner mini-site editor (verified-owner scoped) ─────────────────────────────

/// Resolve the caller to the owner of `company_id`, or `Err(404)`. Ownership authority is
/// the verified directory_claim — an owner can only touch a listing they've verified.
/// Returning 404 (not 403) avoids confirming the listing exists to a non-owner.
fn require_owner(viewer: &DirViewer, s: &DirRouteState, company_id: &str) -> Result<String, Response> {
    let not_found = || StatusCode::NOT_FOUND.into_response();
    let user_id = viewer.user_id.clone().ok_or_else(not_found)?;
    if s.store.is_verified_owner(company_id, &user_id) {
        Ok(user_id)
    } else {
        Err(not_found())
    }
}

fn owner_query_fmt(q: &std::collections::HashMap<String, String>) -> &'static str {
    match q.get("format").map(|s| s.as_str()) {
        Some("csv") => "csv",
        _ => "json",
    }
}

/// GET /directory/my-listings/{company_id}/edit — the self-contained editor page.
pub(crate) async fn mini_site_editor_handler(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if viewer.user_id.is_none() {
        return Redirect::to(&login_redirect("/directory/my-listings")).into_response();
    }
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    let name = s
        .store
        .company_by_id(&company_id)
        .map(|c| c.dba.unwrap_or(c.entity_name))
        .unwrap_or_else(|| "Your listing".to_string());
    let base = format!("/directory/my-listings/{company_id}");
    Html(crate::pages_io::editor_html(
        &base,
        &company_id,
        &name,
        "/directory/my-listings",
        "Owner",
    ))
    .into_response()
}

/// GET {base}/pages.json — current mini-site pages for the owned company.
pub(crate) async fn owner_pages_json(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    Json(serde_json::json!({ "pages": s.store.mini_site_pages_for(&company_id) })).into_response()
}

/// POST {base}/pages — upsert one tab.
pub(crate) async fn owner_pages_save(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
    Json(page): Json<DirectoryMiniSitePage>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    match s.store.upsert_mini_site_page(
        &company_id,
        &page.page_slug,
        page.page_title.as_deref(),
        &page.blocks,
    ) {
        Ok(()) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": e }))).into_response(),
    }
}

/// POST {base}/pages/{slug}/delete
pub(crate) async fn owner_page_delete(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path((company_id, slug)): Path<(String, String)>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    let n = s.store.delete_mini_site_page(&company_id, &slug);
    Json(serde_json::json!({ "ok": true, "deleted": n })).into_response()
}

/// GET {base}/export?format=json|csv
pub(crate) async fn owner_pages_export(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    let rows: Vec<(String, DirectoryMiniSitePage)> = s
        .store
        .mini_site_pages_for(&company_id)
        .into_iter()
        .map(|p| (company_id.clone(), p))
        .collect();
    crate::pages_io::export_response(&rows, owner_query_fmt(&q), &company_id)
}

/// POST {base}/import?format=json|csv  (body = raw file text). Imported rows are forced to
/// THIS company_id, so an owner can never write to a listing they don't own.
pub(crate) async fn owner_pages_import(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
    Query(q): Query<std::collections::HashMap<String, String>>,
    body: String,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    crate::pages_io::import_apply(&s.store, &body, owner_query_fmt(&q), Some(&company_id))
}

/// GET {base}/upgrade-bundle — the carry-over bundle (business + converted pages) for the
/// "upgrade to full website" flow.
pub(crate) async fn owner_upgrade_bundle(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    if let Err(r) = require_owner(&viewer, &s, &company_id) {
        return r;
    }
    crate::pages_io::upgrade_bundle_response(&s.store, &company_id)
}

/// POST {base}/upgrade — record an upgrade request + return the carry-over bundle.
pub(crate) async fn owner_upgrade_request(
    State(s): State<DirRouteState>,
    ext: Option<Extension<DirViewer>>,
    Path(company_id): Path<String>,
) -> Response {
    let viewer = ext.map(|Extension(v)| v).unwrap_or_default();
    let user_id = match require_owner(&viewer, &s, &company_id) {
        Ok(u) => u,
        Err(r) => return r,
    };
    s.store.record_upgrade_request(&company_id, Some(&user_id), "owner", None);
    let bundle = crate::pages_io::upgrade_bundle(&s.store, &company_id);
    Json(serde_json::json!({ "ok": true, "requested": true, "bundle": bundle })).into_response()
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn location_label(co: &crate::store::DirectoryCompany) -> String {
    match &co.city {
        Some(c) if !c.is_empty() => format!("{c}, {}", co.state_abbr),
        _ => co.state_abbr.clone(),
    }
}

/// Canonical company URL (3-segment when a city slug exists, else 2-segment).
fn company_url(store: &crate::store::DirectoryStore, company_id: &str) -> String {
    match store.company_by_id(company_id) {
        Some(co) => {
            let st = co.state_abbr.to_lowercase();
            match co.city_slug.as_deref().filter(|s| !s.is_empty()) {
                Some(cs) => format!("{SITE_BASE}/directory/{st}/{cs}/{}", co.company_slug),
                None => format!("{SITE_BASE}/directory/{st}/{}", co.company_slug),
            }
        }
        None => format!("{SITE_BASE}/directory"),
    }
}

/// Lightly mask an email for the "check your email" page: `j***@example.com`.
fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let first = local.chars().next().unwrap_or('*');
            format!("{first}***@{domain}")
        }
        None => "your email".to_string(),
    }
}

fn verification_email_html(company_display: &str, verify_url: &str) -> String {
    let co = html_escape(company_display);
    let url = html_escape(verify_url);
    format!(
        r#"<!doctype html><html><body style="font-family:Arial,Helvetica,sans-serif;color:#1a2733;line-height:1.6">
<h2 style="color:#1a3c5e">Verify your directory claim</h2>
<p>You asked to claim the <strong>{co}</strong> listing on pestcontroller.org. Confirm it's you to unlock full management of your listing.</p>
<p style="margin:28px 0">
  <a href="{url}" style="background:#e8752a;color:#fff;padding:13px 26px;border-radius:8px;text-decoration:none;font-weight:700;display:inline-block">Verify &amp; claim my listing</a>
</p>
<p style="font-size:13px;color:#64748b">This link expires in 24 hours and can be used once. If you didn't request this, you can safely ignore this email — nothing changes until the link is clicked.</p>
<p style="font-size:13px;color:#64748b">If the button doesn't work, paste this into your browser:<br><span style="word-break:break-all">{url}</span></p>
</body></html>"#
    )
}

#[allow(dead_code)]
fn _assert_html_response_unused(_: Html<String>) {}
