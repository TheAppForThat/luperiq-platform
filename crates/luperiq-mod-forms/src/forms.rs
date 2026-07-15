//! LuperIQ Forms Module — form builder, submissions, and email notifications.
//!
//! Native Rust replacement for the WordPress `forms` module.
//! Stores form definitions and submissions in ForgeJournal via ForgeContent.
//! Sends notification emails via the SMTP module's `/api/modules/smtp/send`.
//!
//! Security notes:
//! - Honeypot field for bot detection
//! - Rate limiting per IP (in-memory, resets on restart)
//! - Admin UI uses DOM methods (no innerHTML) for XSS safety

use axum::extract::State;
use axum::response::{Html, Json};
use axum::routing::{get, post, put};
use axum::Router;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use luperiq_forge::{
    ForgeContent, ForgeContentManager, ForgeIpManager, ForgeMessageStore, ForgeThreadManager,
    Message, Thread, ThreadStatus, ThreadType,
};

use luperiq_module_api::{
    AdminView, AiFeatureConfig, AiFeatureRegistry, AppContext, CmsModule, SharedJournal,
};

// ── Aggregate type for form submissions in ForgeJournal ──────────────

const FORM_DEF_TYPE: &str = "form_definition";
const FORM_SUB_TYPE: &str = "form_submission";
const SUPPORT_EMAIL_VERIFICATION_TYPE: &str = "support_email_verification";
const SUPPORT_TRUST_SIGNAL_TYPE: &str = "support_trust_signal";
/// Aggregate type for support ticket status tracking.
/// Stored separately from submissions so status updates don't mutate original data.
const SUPPORT_STATUS_TYPE: &str = "support_ticket_status";
const SUPPORT_EMAIL_VERIFICATION_TTL_SECS: u64 = 60 * 60 * 24 * 3;

// ── Module definition ─────────────────────────────────────────────────

pub struct FormsModule;

impl CmsModule for FormsModule {
    fn slug(&self) -> &str {
        "forms"
    }
    fn name(&self) -> &str {
        "LuperIQ Forms"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn description(&self) -> &str {
        "Form builder with submissions, honeypot protection, and email notifications."
    }
    fn category(&self) -> &str {
        "Content"
    }
    fn dependencies(&self) -> &[&str] {
        &["smtp"]
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        Some(forms_router(ctx.journal.clone(), ctx.ai_features.clone()))
    }

    fn admin_views(&self) -> Vec<AdminView> {
        vec![
            AdminView {
                id: "forms".into(),
                label: "Forms".into(),
                section: "Communication".into(),
            },
            AdminView {
                id: "form-submissions".into(),
                label: "Submissions".into(),
                section: "Communication".into(),
            },
            AdminView {
                id: "support-inbox".into(),
                label: "Support Inbox".into(),
                section: "Communication".into(),
            },
        ]
    }

    fn admin_js(&self) -> Option<String> {
        Some(FORMS_ADMIN_JS.to_string())
    }
}

// ── Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormDefinition {
    pub slug: String,
    pub title: String,
    pub fields: Vec<FormField>,
    pub notify_email: Option<String>,
    pub success_message: String,
    pub honeypot_field: String,
    pub rate_limit_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub name: String,
    pub label: String,
    pub field_type: String, // text, email, textarea, select, checkbox, hidden
    pub required: bool,
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub options: Vec<String>, // for select fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FormSubmission {
    form_slug: String,
    data: HashMap<String, String>,
    ip_address: String,
    user_agent: String,
    submitted_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupportEmailVerification {
    token: String,
    submission_id: String,
    email: String,
    status: String,
    requested_at: u64,
    expires_at: u64,
    verified_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SupportTrustSignal {
    submission_id: String,
    email: String,
    ip_address: String,
    user_agent: String,
    review_first: bool,
    verification_required: bool,
    email_verified: bool,
    trust_account_policy: String,
    contact_kind: String,
    contact_flow: String,
    connection_flow: String,
    topic: String,
    risk_level: String,
    reason: String,
    created_at: u64,
}

#[derive(Serialize)]
struct ApiResult {
    ok: bool,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

fn support_priority_for_topic(topic: &str) -> &'static str {
    match topic {
        "technical" => "High",
        "correction" => "High",
        "emergency" => "High",
        "connection-request-access" => "Medium",
        "ordering" => "Medium",
        "reservation" | "catering" | "merch" => "Medium",
        "pricing" => "Medium",
        "collab"
        | "sponsorship"
        | "media"
        | "guest_post"
        | "booking"
        | "product_question"
        | "permission"
        | "connection-contact-admins"
        | "connection-event-question"
        | "connection-connect-groups" => "Medium",
        "question" => "Low",
        "reader_question" | "story_idea" | "connection-other" => "Low",
        _ => "Medium",
    }
}

fn support_topic_label(topic: &str) -> &'static str {
    match topic {
        "question" => "General Question",
        "pricing" => "Pricing & Plans",
        "ordering" => "Help Ordering",
        "reservation" => "Reservation",
        "catering" => "Catering or Private Dining",
        "menu" => "Menu Question",
        "feedback" => "Feedback",
        "merch" => "Merch Order",
        "technical" => "Technical Issue",
        "collab" => "Collaboration",
        "sponsorship" => "Sponsorship",
        "media" => "Media / Interview",
        "booking" => "Booking or Appearance",
        "product_question" => "Product Question",
        "story_idea" => "Story Idea",
        "correction" => "Correction",
        "reader_question" => "Reader Question",
        "guest_post" => "Guest Post",
        "permission" => "Permission / Licensing",
        "connection-request-access" => "Request Access",
        "connection-contact-admins" => "Contact Admins",
        "connection-event-question" => "Event or Calendar Question",
        "connection-connect-groups" => "Connect Groups",
        "connection-other" => "Other Connection Request",
        "other" => "Other",
        _ => "General Question",
    }
}

fn support_submission_is_review_first(sub: &FormSubmission) -> bool {
    let contact_flow = sub
        .data
        .get("contact_flow")
        .map(|value| value.trim())
        .unwrap_or_default();
    let connection_flow = sub
        .data
        .get("connection_flow")
        .map(|value| value.trim())
        .unwrap_or_default();

    matches!(contact_flow, "owner_review_first") || matches!(connection_flow, "admin_review_first")
}

fn support_submission_flow_label(sub: &FormSubmission) -> &'static str {
    if sub.data.get("connection_flow").map(|value| value.trim()) == Some("admin_review_first") {
        return "Connection Request";
    }

    match sub.data.get("contact_kind").map(|value| value.trim()) {
        Some("creator") => "Creator Contact",
        Some("blog") => "Writer Contact",
        _ if support_submission_is_review_first(sub) => "Owner Contact",
        _ => "Support Request",
    }
}

fn support_submission_topic_label(sub: &FormSubmission) -> String {
    sub.data
        .get("topic_label")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            sub.data
                .get("topic")
                .map(|value| support_topic_label(value.trim()).to_string())
        })
        .unwrap_or_else(|| "Support Request".to_string())
}

fn support_submission_email(sub: &FormSubmission) -> Option<String> {
    sub.data
        .get("email")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn support_submission_requires_email_verification(sub: &FormSubmission) -> bool {
    let required = sub
        .data
        .get("email_verification_required")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let policy = sub
        .data
        .get("verification_policy")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();

    matches!(required.as_str(), "true" | "1" | "yes" | "required")
        || matches!(policy.as_str(), "email_before_conversation")
}

fn support_verification_status_text(sub: &FormSubmission) -> &'static str {
    if support_submission_requires_email_verification(sub) {
        "Required, pending"
    } else {
        "Not required"
    }
}

fn support_submission_title(sub: &FormSubmission) -> String {
    let name = sub
        .data
        .get("name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("Customer");
    let topic = support_submission_topic_label(sub);
    format!("{}: {topic} — {name}", support_submission_flow_label(sub))
}

fn support_submission_message(sub: &FormSubmission) -> String {
    let name = sub
        .data
        .get("name")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("Unknown");
    let email = sub
        .data
        .get("email")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("No email provided");
    let topic = support_submission_topic_label(sub);
    let priority = sub
        .data
        .get("priority")
        .cloned()
        .unwrap_or_else(|| "Medium".to_string());
    let description = sub
        .data
        .get("description")
        .cloned()
        .or_else(|| sub.data.get("message").cloned())
        .unwrap_or_default();
    let flow_label = support_submission_flow_label(sub);
    let mut body = format!(
        "{flow_label} submitted from the public site.\n\nFrom: {name} <{email}>\nTopic: {topic}\nPriority: {priority}"
    );

    if let Some(policy) = sub
        .data
        .get("review_policy")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        body.push_str(&format!("\nReview policy: {policy}"));
    } else if support_submission_is_review_first(sub) {
        body.push_str("\nReview policy: Owner/admin review first before routing this into a participant-visible conversation.");
    }

    body.push_str(&format!(
        "\nEmail verification: {}",
        support_verification_status_text(sub)
    ));

    if !description.trim().is_empty() {
        body.push_str("\n\n");
        body.push_str(description.trim());
    }

    body
}

fn ensure_support_thread_for_submission(
    journal: &mut luperiq_forge::ForgeJournal,
    submission_id: &str,
) -> Result<String, String> {
    let submission = {
        let mgr = ForgeContentManager::new(journal);
        let content = mgr
            .get_content(submission_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Support submission not found: {submission_id}"))?;
        let sub: FormSubmission = serde_json::from_str(&content.body_json)
            .map_err(|e| format!("Invalid support submission payload: {e}"))?;
        if sub.form_slug != "support" {
            return Err("Submission is not a support ticket".to_string());
        }
        sub
    };

    if let Some((thread_id, _thread)) = {
        let tmgr = ForgeThreadManager::new(journal);
        tmgr.by_context("support_submission", submission_id)
            .map_err(|e| e.to_string())?
    } {
        return Ok(thread_id);
    }

    let now = now_secs();
    let review_first = support_submission_is_review_first(&submission);
    let author_id = if review_first {
        "public-contact".to_string()
    } else {
        support_submission_email(&submission).unwrap_or_else(|| "anonymous".to_string())
    };
    let participants = if review_first {
        Vec::new()
    } else {
        support_submission_email(&submission)
            .into_iter()
            .collect::<Vec<_>>()
    };

    let thread_id = {
        let mut tmgr = ForgeThreadManager::new(journal);
        tmgr.create(&Thread {
            thread_id: String::new(),
            created_by: author_id.clone(),
            title: support_submission_title(&submission),
            thread_type: ThreadType::Support,
            status: ThreadStatus::Open,
            participants,
            context_type: "support_submission".to_string(),
            context_id: submission_id.to_string(),
            created_at: now,
            updated_at: now,
        })
        .map_err(|e| e.to_string())?
    };

    let mut messages = ForgeMessageStore::new(journal);
    messages
        .post(&Message {
            message_id: String::new(),
            thread_id: thread_id.clone(),
            author_id,
            body: support_submission_message(&submission),
            created_at: now,
        })
        .map_err(|e| e.to_string())?;

    Ok(thread_id)
}

fn support_latest_verification_by_submission(
    mgr: &ForgeContentManager<'_>,
) -> HashMap<String, SupportEmailVerification> {
    let records = mgr
        .list_content(
            Some(SUPPORT_EMAIL_VERIFICATION_TYPE),
            None,
            None,
            2000,
            0,
            None,
            None,
        )
        .map(|(v, _total)| v)
        .unwrap_or_default();

    let mut by_submission = HashMap::new();
    for content in records {
        let Ok(record) = serde_json::from_str::<SupportEmailVerification>(&content.body_json)
        else {
            continue;
        };
        let replace = by_submission
            .get(&record.submission_id)
            .map(|existing: &SupportEmailVerification| record.requested_at >= existing.requested_at)
            .unwrap_or(true);
        if replace {
            by_submission.insert(record.submission_id.clone(), record);
        }
    }

    by_submission
}

fn support_email_verified_for_submission(
    verifications: &HashMap<String, SupportEmailVerification>,
    submission_id: &str,
) -> bool {
    verifications
        .get(submission_id)
        .map(|record| record.status == "verified")
        .unwrap_or(false)
}

fn create_support_email_verification(
    journal: &mut luperiq_forge::ForgeJournal,
    submission_id: &str,
    sub: &FormSubmission,
) -> Option<SupportEmailVerification> {
    if !support_submission_requires_email_verification(sub) {
        return None;
    }
    let email = support_submission_email(sub)?;
    let now = now_secs();
    let record = SupportEmailVerification {
        token: format!("{}{}", ulid::Ulid::new(), ulid::Ulid::new()).to_ascii_lowercase(),
        submission_id: submission_id.to_string(),
        email: email.clone(),
        status: "pending".to_string(),
        requested_at: now,
        expires_at: now + SUPPORT_EMAIL_VERIFICATION_TTL_SECS,
        verified_at: None,
    };
    let content = ForgeContent {
        content_id: String::new(),
        content_type: SUPPORT_EMAIL_VERIFICATION_TYPE.into(),
        title: format!("Support email verification for {email}"),
        slug: format!("support-email-verification-{}", record.token),
        // SAFETY: SupportEmailVerification only contains String/u64/Option<u64> — serialization
        // cannot fail. unwrap_or_default() would silently write an empty body_json (corrupt WAL)
        // if this ever changes; prefer an explicit expect so a regression is caught at test time.
        body_json: serde_json::to_string(&record).unwrap_or_default(),
        excerpt: None,
        author_id: "system".into(),
        status: "published".into(),
        created_at: now,
        updated_at: now,
        published_at: Some(now),
    };

    let mut mgr = ForgeContentManager::new(journal);
    mgr.create_content(&content).ok()?;
    Some(record)
}

fn record_support_trust_signal(
    journal: &mut luperiq_forge::ForgeJournal,
    submission_id: &str,
    sub: &FormSubmission,
    email_verified: bool,
) {
    if sub.form_slug != "support" {
        return;
    }

    let verification_required = support_submission_requires_email_verification(sub);
    let review_first = support_submission_is_review_first(sub);
    let signal = SupportTrustSignal {
        submission_id: submission_id.to_string(),
        email: support_submission_email(sub).unwrap_or_default(),
        ip_address: sub.ip_address.clone(),
        user_agent: sub.user_agent.clone(),
        review_first,
        verification_required,
        email_verified,
        trust_account_policy: sub
            .data
            .get("trust_account_policy")
            .cloned()
            .unwrap_or_default(),
        contact_kind: sub.data.get("contact_kind").cloned().unwrap_or_default(),
        contact_flow: sub.data.get("contact_flow").cloned().unwrap_or_default(),
        connection_flow: sub.data.get("connection_flow").cloned().unwrap_or_default(),
        topic: sub.data.get("topic").cloned().unwrap_or_default(),
        risk_level: if verification_required && !email_verified {
            "review"
        } else {
            "low"
        }
        .to_string(),
        reason: if verification_required && !email_verified {
            "Email verification required before participant-visible conversation."
        } else if review_first {
            "Review-first public contact flow."
        } else {
            "Standard support submission."
        }
        .to_string(),
        created_at: now_secs(),
    };
    let content = ForgeContent {
        content_id: String::new(),
        content_type: SUPPORT_TRUST_SIGNAL_TYPE.into(),
        title: format!("Support trust signal {submission_id}"),
        slug: format!("support-trust-signal-{}", ulid::Ulid::new()),
        // SAFETY: SupportTrustSignal only contains String/bool/u64 — serialization cannot fail.
        // unwrap_or_default() would silently write an empty body_json (corrupt WAL record).
        body_json: serde_json::to_string(&signal).unwrap_or_default(),
        excerpt: None,
        author_id: "system".into(),
        status: "published".into(),
        created_at: signal.created_at,
        updated_at: signal.created_at,
        published_at: Some(signal.created_at),
    };

    let mut mgr = ForgeContentManager::new(journal);
    let _ = mgr.create_content(&content);
}

fn support_ip_is_blocked(journal: &mut luperiq_forge::ForgeJournal, ip: &str) -> bool {
    if ip.trim().is_empty() || ip == "unknown" {
        return false;
    }
    ForgeIpManager::new(journal)
        .is_blocked(ip, now_secs())
        .unwrap_or(false)
}

fn header_first_value(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn support_request_origin(headers: &axum::http::HeaderMap) -> String {
    let scheme = header_first_value(headers, "x-forwarded-proto")
        .filter(|value| value.eq_ignore_ascii_case("http"))
        .unwrap_or_else(|| "https".to_string());
    let host = header_first_value(headers, "x-forwarded-host")
        .or_else(|| header_first_value(headers, "host"))
        .unwrap_or_else(|| "luperiq.com".to_string());
    let safe_host: String = host
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | ':'))
        .collect();

    format!(
        "{}://{}",
        scheme,
        if safe_host.is_empty() {
            "luperiq.com"
        } else {
            safe_host.as_str()
        }
    )
}

fn send_support_verification_email(
    to: String,
    flow_label: String,
    verification: SupportEmailVerification,
    origin: String,
) {
    tokio::spawn(async move {
        let verify_url = format!(
            "{}/api/modules/forms/support/verify/{}",
            origin.trim_end_matches('/'),
            verification.token
        );
        let escaped_flow = html_escape(&flow_label);
        let escaped_url = html_escape(&verify_url);
        let body = format!(
            "<h2>Verify your email</h2>\
             <p>Someone used this email address for a {escaped_flow} request. Please verify it before the request can become a conversation.</p>\
             <p><a href=\"{escaped_url}\" style=\"display:inline-block;background:#2563eb;color:#fff;padding:10px 14px;border-radius:6px;text-decoration:none;font-weight:700;\">Verify email</a></p>\
             <p>If you did not send this request, you can ignore this email.</p>\
             <p style=\"font-size:12px;color:#64748b;\">This link expires in 3 days.</p>"
        );
        let payload = serde_json::json!({
            "to": to,
            "subject": format!("Verify your email for {}", flow_label),
            "body": body,
            "is_html": true,
        });
        // TODO(review): Replace this HTTP loopback call with the platform EmailTransport trait
        // seam. Creating a new reqwest::Client per send incurs TLS handshake overhead for a
        // same-process call and fails silently if the smtp module hasn't started yet.
        // AppContext carries enough to inject the transport handle (audit item 1).
        let client = reqwest::Client::new();
        let _ = client
            .post("http://127.0.0.1:3000/api/modules/smtp/send")
            .json(&payload)
            .send()
            .await;
    });
}

fn normalize_submission_fields(slug: &str, fields: &mut HashMap<String, String>) {
    if slug != "support" {
        return;
    }

    if !fields.contains_key("fax_number") {
        if let Some(honeypot) = fields.get("liqSupportHoney").cloned() {
            fields.insert("fax_number".into(), honeypot);
        }
    }

    let topic = fields
        .get("topic")
        .map(|value| value.trim().to_string())
        .unwrap_or_default();

    if !fields.contains_key("priority") {
        fields.insert(
            "priority".into(),
            support_priority_for_topic(topic.as_str()).into(),
        );
    }

    if !fields.contains_key("description") {
        let message = fields
            .get("message")
            .map(|value| value.trim().to_string())
            .unwrap_or_default();
        if !message.is_empty() {
            let description = if topic.is_empty() {
                message
            } else {
                format!(
                    "Topic: {}\n\n{}",
                    support_topic_label(topic.as_str()),
                    message
                )
            };
            fields.insert("description".into(), description);
        }
    }
}

// ── Rate limiter ──────────────────────────────────────────────────────

struct RateLimiter {
    /// ip -> (count, window_start_secs)
    entries: HashMap<String, (u32, u64)>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn check(&mut self, ip: &str, max_per_minute: u32) -> bool {
        let now = now_secs();
        let entry = self.entries.entry(ip.to_string()).or_insert((0, now));
        if now - entry.1 > 60 {
            // New window
            *entry = (1, now);
            true
        } else if entry.0 < max_per_minute {
            entry.0 += 1;
            true
        } else {
            false
        }
    }
}

type SharedRateLimiter = Arc<Mutex<RateLimiter>>;

// ── Router state ──────────────────────────────────────────────────────

#[derive(Clone)]
struct FormsState {
    journal: SharedJournal,
    rate_limiter: SharedRateLimiter,
}

fn forms_router(journal: SharedJournal, ai_features: AiFeatureRegistry) -> Router {
    // Register AI features in the shared registry.
    // NOTE: This spawns a task to acquire the registry lock asynchronously. If the lock is
    // already held at startup, registration silently races. Consider replacing with a direct
    // blocking_lock() / try_lock() if startup ordering becomes a problem (audit item 7).
    {
        let features = ai_features.clone();
        tokio::task::spawn(async move {
            let mut reg = features.lock().await;
            reg.insert("forms_ai_response".into(), AiFeatureConfig {
                system_prompt: "You are a helpful customer service assistant. Draft a professional, friendly response to this form submission. Keep it concise (2-4 sentences). Address the person by name if available. Be warm but professional.".to_string(),
                max_input_len: 2000,
                credit_cost: 1,
                escalation_credit_cost: 1,
                result_parser: |s| Ok(serde_json::Value::String(s.trim().to_string())),
            });
            reg.insert("forms_ai_improve".into(), AiFeatureConfig {
                system_prompt: "You are a UX copywriter. Improve this form's field labels, descriptions, and placeholder text to be clearer and more user-friendly. Return as a JSON array of objects with keys: field_name, label, placeholder, description. Only include fields that need improvement.".to_string(),
                max_input_len: 4000,
                credit_cost: 2,
                escalation_credit_cost: 1,
                result_parser: |s| {
                    let trimmed = s.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
                    serde_json::from_str(trimmed).map_err(|e| format!("JSON parse error: {e}"))
                },
            });
        });
    }

    let state = FormsState {
        journal,
        rate_limiter: Arc::new(Mutex::new(RateLimiter::new())),
    };

    Router::new()
        // Admin API
        .route("/api/modules/forms", get(list_forms).post(save_form))
        // Support inbox — fixed route BEFORE parameterized {slug} routes
        .route("/api/modules/forms/support/inbox", get(get_support_inbox))
        .route(
            "/api/modules/forms/support/submissions/{id}/conversation",
            post(open_support_conversation),
        )
        .route(
            "/api/modules/forms/{slug}",
            get(get_form).delete(delete_form),
        )
        .route(
            "/api/modules/forms/{slug}/submissions",
            get(get_submissions),
        )
        .route(
            "/api/modules/forms/{slug}/submissions/{id}/status",
            put(update_submission_status),
        )
        // Public submit endpoint
        .route("/api/modules/forms/{slug}/submit", post(submit_form))
        // Public email verification for review-first contact flows
        .route(
            "/api/modules/forms/support/verify/{token}",
            get(verify_support_email),
        )
        // Public form render (standalone page)
        .route("/forms/{slug}", get(render_form_page))
        .with_state(state)
}

// ── Built-in form definitions ─────────────────────────────────────────

fn builtin_forms() -> Vec<FormDefinition> {
    vec![
        FormDefinition {
            slug: "contact".into(),
            title: "Contact Us".into(),
            fields: vec![
                FormField {
                    name: "name".into(),
                    label: "Your Name".into(),
                    field_type: "text".into(),
                    required: true,
                    placeholder: "Jane Smith".into(),
                    options: vec![],
                },
                FormField {
                    name: "email".into(),
                    label: "Email Address".into(),
                    field_type: "email".into(),
                    required: true,
                    placeholder: "jane@example.com".into(),
                    options: vec![],
                },
                FormField {
                    name: "subject".into(),
                    label: "Subject".into(),
                    field_type: "text".into(),
                    required: true,
                    placeholder: "How can we help?".into(),
                    options: vec![],
                },
                FormField {
                    name: "message".into(),
                    label: "Message".into(),
                    field_type: "textarea".into(),
                    required: true,
                    placeholder: "Tell us more...".into(),
                    options: vec![],
                },
            ],
            notify_email: None, // resolved at send time from WAL notification config
            success_message: "Thank you! We'll get back to you soon.".into(),
            honeypot_field: "website_url".into(),
            rate_limit_per_minute: 3,
        },
        FormDefinition {
            slug: "beta-signup".into(),
            title: "Beta Signup".into(),
            fields: vec![
                FormField {
                    name: "name".into(),
                    label: "Name".into(),
                    field_type: "text".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![],
                },
                FormField {
                    name: "email".into(),
                    label: "Email".into(),
                    field_type: "email".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![],
                },
                FormField {
                    name: "company".into(),
                    label: "Company / Website".into(),
                    field_type: "text".into(),
                    required: false,
                    placeholder: "".into(),
                    options: vec![],
                },
                FormField {
                    name: "interest".into(),
                    label: "What interests you?".into(),
                    field_type: "select".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![
                        "AI Certification".into(),
                        "CMS Migration".into(),
                        "Module Development".into(),
                        "Theme Studio".into(),
                        "Other".into(),
                    ],
                },
            ],
            notify_email: None, // resolved at send time from WAL notification config
            success_message: "Welcome to the beta! Check your email for next steps.".into(),
            honeypot_field: "company_url".into(),
            rate_limit_per_minute: 2,
        },
        FormDefinition {
            slug: "support".into(),
            title: "Support Request".into(),
            fields: vec![
                FormField {
                    name: "name".into(),
                    label: "Name".into(),
                    field_type: "text".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![],
                },
                FormField {
                    name: "email".into(),
                    label: "Email".into(),
                    field_type: "email".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![],
                },
                FormField {
                    name: "priority".into(),
                    label: "Priority".into(),
                    field_type: "select".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![
                        "Low".into(),
                        "Medium".into(),
                        "High".into(),
                        "Urgent".into(),
                    ],
                },
                FormField {
                    name: "description".into(),
                    label: "Describe the issue".into(),
                    field_type: "textarea".into(),
                    required: true,
                    placeholder: "".into(),
                    options: vec![],
                },
            ],
            notify_email: None, // resolved at send time from WAL notification config (support role)
            success_message: "Support request submitted. We'll respond within 24 hours.".into(),
            honeypot_field: "fax_number".into(),
            rate_limit_per_minute: 3,
        },
    ]
}

// ── Admin API handlers ────────────────────────────────────────────────

async fn list_forms(State(state): State<FormsState>) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);

    let stored = mgr
        .list_content(Some(FORM_DEF_TYPE), None, None, 100, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut forms: Vec<serde_json::Value> = stored
        .iter()
        .filter_map(|(_id, c)| {
            let def: FormDefinition = serde_json::from_str(&c.body_json).ok()?;
            Some(serde_json::json!({
                "slug": def.slug,
                "title": def.title,
                "fields": def.fields.len(),
                "notify_email": def.notify_email,
                "builtin": false,
            }))
        })
        .collect();

    // Add builtins that aren't overridden
    let stored_slugs: Vec<String> = forms
        .iter()
        .filter_map(|f| f.get("slug").and_then(|s| s.as_str()).map(String::from))
        .collect();
    for def in builtin_forms() {
        if !stored_slugs.contains(&def.slug) {
            forms.push(serde_json::json!({
                "slug": def.slug,
                "title": def.title,
                "fields": def.fields.len(),
                "notify_email": def.notify_email,
                "builtin": true,
            }));
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("{} forms", forms.len()),
        data: Some(serde_json::json!(forms)),
    })
}

async fn get_form(
    State(state): State<FormsState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let def = resolve_form(&state.journal, &slug).await;
    match def {
        Some(d) => Json(ApiResult {
            ok: true,
            message: "Form found".into(),
            data: Some(serde_json::to_value(&d).unwrap_or_default()),
        }),
        None => Json(ApiResult {
            ok: false,
            message: "Form not found".into(),
            data: None,
        }),
    }
}

async fn save_form(
    State(state): State<FormsState>,
    axum::extract::Json(def): axum::extract::Json<FormDefinition>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mut mgr = ForgeContentManager::new(&mut j);

    let body_json = serde_json::to_string(&def).unwrap_or_default();

    // Check if form already exists (update) or create new
    let existing = mgr
        .list_content(Some(FORM_DEF_TYPE), None, None, 100, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .find(|(_id, c)| c.slug == def.slug);

    if let Some((_, existing_content)) = existing {
        if let Err(e) = mgr.update_content(
            &existing_content.content_id,
            Some(&def.title),
            None,
            Some(&body_json),
            None,
        ) {
            return Json(ApiResult {
                ok: false,
                message: e.to_string(),
                data: None,
            });
        }
    } else {
        let content = ForgeContent {
            content_id: ulid::Ulid::new().to_string(),
            content_type: FORM_DEF_TYPE.into(),
            title: def.title.clone(),
            slug: def.slug.clone(),
            body_json,
            excerpt: None,
            author_id: "system".into(),
            status: "published".into(),
            created_at: now_secs(),
            updated_at: now_secs(),
            published_at: Some(now_secs()),
        };
        if let Err(e) = mgr.create_content(&content) {
            return Json(ApiResult {
                ok: false,
                message: e.to_string(),
                data: None,
            });
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("Form '{}' saved", def.slug),
        data: None,
    })
}

async fn delete_form(
    State(state): State<FormsState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mut mgr = ForgeContentManager::new(&mut j);

    let forms = mgr
        .list_content(Some(FORM_DEF_TYPE), None, None, 100, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let found = forms.into_iter().find(|(_id, c)| c.slug == slug);

    match found {
        Some((_id, content)) => {
            if let Err(e) = mgr.delete_content(&content.content_id) {
                return Json(ApiResult {
                    ok: false,
                    message: e.to_string(),
                    data: None,
                });
            }
            Json(ApiResult {
                ok: true,
                message: "Form deleted".into(),
                data: None,
            })
        }
        None => Json(ApiResult {
            ok: false,
            message: "Form not found (may be built-in)".into(),
            data: None,
        }),
    }
}

// ── Submission handlers ───────────────────────────────────────────────

async fn get_submissions(
    State(state): State<FormsState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let limit: usize = q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50);

    let subs = mgr
        .list_content(Some(FORM_SUB_TYPE), None, None, 500, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let filtered: Vec<serde_json::Value> = subs
        .iter()
        .filter_map(|(_id, c)| {
            let sub: FormSubmission = serde_json::from_str(&c.body_json).ok()?;
            if sub.form_slug != slug {
                return None;
            }
            Some(serde_json::json!({
                "id": c.content_id,
                "data": sub.data,
                "ip_address": sub.ip_address,
                "submitted_at": sub.submitted_at,
            }))
        })
        .take(limit)
        .collect();

    Json(ApiResult {
        ok: true,
        message: format!("{} submissions", filtered.len()),
        data: Some(serde_json::json!(filtered)),
    })
}

// ── Support inbox — dedicated view for support ticket management ─────

async fn get_support_inbox(
    State(state): State<FormsState>,
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let limit: usize = q.get("limit").and_then(|l| l.parse().ok()).unwrap_or(100);
    let status_filter = q.get("status").cloned();

    // Load all support form submissions
    let subs = mgr
        .list_content(Some(FORM_SUB_TYPE), None, None, 500, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Load all status overrides
    let statuses: HashMap<String, String> = mgr
        .list_content(Some(SUPPORT_STATUS_TYPE), None, None, 500, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .filter_map(|c| {
                    let val: serde_json::Value = serde_json::from_str(&c.body_json).ok()?;
                    let sub_id = val.get("submission_id")?.as_str()?.to_string();
                    let status = val.get("status")?.as_str()?.to_string();
                    Some((sub_id, status))
                })
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let verifications = support_latest_verification_by_submission(&mgr);

    let mut tickets: Vec<serde_json::Value> = subs
        .iter()
        .filter_map(|(_id, c)| {
            let sub: FormSubmission = serde_json::from_str(&c.body_json).ok()?;
            if sub.form_slug != "support" {
                return None;
            }
            let ticket_status = statuses
                .get(&c.content_id)
                .cloned()
                .unwrap_or_else(|| "new".to_string());
            // Apply status filter if specified
            if let Some(ref filter) = status_filter {
                if &ticket_status != filter {
                    return None;
                }
            }
            let topic = sub.data.get("topic").cloned().unwrap_or_default();
            let topic_label = support_submission_topic_label(&sub);
            let email_verification_required = support_submission_requires_email_verification(&sub);
            let email_verified =
                support_email_verified_for_submission(&verifications, &c.content_id);
            let email_verification_status = if email_verified {
                "verified"
            } else if email_verification_required {
                "pending"
            } else {
                "not_required"
            };
            Some(serde_json::json!({
                "id": c.content_id,
                "name": sub.data.get("name").cloned().unwrap_or_default(),
                "email": sub.data.get("email").cloned().unwrap_or_default(),
                "topic": topic,
                "topic_label": topic_label,
                "flow_label": support_submission_flow_label(&sub),
                "review_first": support_submission_is_review_first(&sub),
                "email_verification_required": email_verification_required,
                "email_verified": email_verified,
                "email_verification_status": email_verification_status,
                "priority": sub.data.get("priority").cloned().unwrap_or_else(|| "Medium".to_string()),
                "message": sub.data.get("message").cloned()
                    .or_else(|| sub.data.get("description").cloned())
                    .unwrap_or_default(),
                "description": sub.data.get("description").cloned().unwrap_or_default(),
                "status": ticket_status,
                "submitted_at": sub.submitted_at,
                "ip_address": sub.ip_address,
            }))
        })
        .collect();

    // Sort newest first
    tickets.sort_by(|a, b| {
        let at = b.get("submitted_at").and_then(|v| v.as_u64()).unwrap_or(0);
        let bt = a.get("submitted_at").and_then(|v| v.as_u64()).unwrap_or(0);
        at.cmp(&bt)
    });

    if tickets.len() > limit {
        tickets.truncate(limit);
    }

    // Count by status for summary badges
    let total = tickets.len();
    let new_count = tickets
        .iter()
        .filter(|t| t.get("status").and_then(|v| v.as_str()) == Some("new"))
        .count();
    let in_progress_count = tickets
        .iter()
        .filter(|t| t.get("status").and_then(|v| v.as_str()) == Some("in-progress"))
        .count();
    let resolved_count = tickets
        .iter()
        .filter(|t| t.get("status").and_then(|v| v.as_str()) == Some("resolved"))
        .count();

    Json(ApiResult {
        ok: true,
        message: format!("{} support tickets", total),
        data: Some(serde_json::json!({
            "tickets": tickets,
            "counts": {
                "total": total,
                "new": new_count,
                "in_progress": in_progress_count,
                "resolved": resolved_count,
            }
        })),
    })
}

async fn open_support_conversation(
    State(state): State<FormsState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Json<ApiResult> {
    let mut j = state.journal.lock().await;
    {
        let mgr = ForgeContentManager::new(&mut j);
        let submission = match mgr.get_content(&id) {
            Ok(Some(content)) => match serde_json::from_str::<FormSubmission>(&content.body_json) {
                Ok(sub) => sub,
                Err(e) => {
                    return Json(ApiResult {
                        ok: false,
                        message: format!("Invalid support submission payload: {e}"),
                        data: None,
                    })
                }
            },
            Ok(None) => {
                return Json(ApiResult {
                    ok: false,
                    message: "Submission not found".into(),
                    data: None,
                })
            }
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: e.to_string(),
                    data: None,
                })
            }
        };
        let verifications = support_latest_verification_by_submission(&mgr);
        if support_submission_requires_email_verification(&submission)
            && !support_email_verified_for_submission(&verifications, &id)
        {
            return Json(ApiResult {
                ok: false,
                message:
                    "Email verification is still pending. Ask the sender to use the verification link first."
                        .into(),
                data: Some(serde_json::json!({ "email_verification_status": "pending" })),
            });
        }
    }

    match ensure_support_thread_for_submission(&mut j, &id) {
        Ok(thread_id) => Json(ApiResult {
            ok: true,
            message: "Support conversation ready".into(),
            data: Some(serde_json::json!({ "thread_id": thread_id })),
        }),
        Err(e) => Json(ApiResult {
            ok: false,
            message: e,
            data: None,
        }),
    }
}

/// PUT /api/modules/forms/support/submissions/:id/status — update ticket status
async fn update_submission_status(
    State(state): State<FormsState>,
    axum::extract::Path((_slug, id)): axum::extract::Path<(String, String)>,
    axum::extract::Json(payload): axum::extract::Json<serde_json::Value>,
) -> Json<ApiResult> {
    let new_status = match payload.get("status").and_then(|s| s.as_str()) {
        Some(s) if matches!(s, "new" | "in-progress" | "resolved") => s.to_string(),
        _ => {
            return Json(ApiResult {
                ok: false,
                message: "Invalid status. Must be: new, in-progress, or resolved".into(),
                data: None,
            })
        }
    };

    let mut j = state.journal.lock().await;
    let mut mgr = ForgeContentManager::new(&mut j);

    // Check that the submission exists — use direct get_content instead of a full table scan
    // (the content ID is already known at the call site; list_content 500 would silently miss
    // older submissions once the table grows past 500 rows)
    let exists = mgr
        .get_content(&id)
        .ok()
        .flatten()
        .is_some();

    if !exists {
        return Json(ApiResult {
            ok: false,
            message: "Submission not found".into(),
            data: None,
        });
    }

    // Upsert the status record — look for an existing status entry for this submission
    let existing_status = mgr
        .list_content(Some(SUPPORT_STATUS_TYPE), None, None, 500, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
        .into_iter()
        .find(|(_cid, c)| {
            serde_json::from_str::<serde_json::Value>(&c.body_json)
                .ok()
                .and_then(|v| v.get("submission_id")?.as_str().map(|s| s == id))
                .unwrap_or(false)
        });

    let body_json = serde_json::json!({
        "submission_id": id,
        "status": new_status,
        "updated_at": now_secs(),
    })
    .to_string();

    if let Some((_, existing)) = existing_status {
        if let Err(e) = mgr.update_content(&existing.content_id, None, None, Some(&body_json), None)
        {
            return Json(ApiResult {
                ok: false,
                message: format!("Failed to update status: {e}"),
                data: None,
            });
        }
    } else {
        let content = ForgeContent {
            content_id: ulid::Ulid::new().to_string(),
            content_type: SUPPORT_STATUS_TYPE.into(),
            title: format!("Status for {id}"),
            slug: format!("status-{id}"),
            body_json,
            excerpt: None,
            author_id: "admin".into(),
            status: "published".into(),
            created_at: now_secs(),
            updated_at: now_secs(),
            published_at: Some(now_secs()),
        };
        if let Err(e) = mgr.create_content(&content) {
            return Json(ApiResult {
                ok: false,
                message: format!("Failed to save status: {e}"),
                data: None,
            });
        }
    }

    Json(ApiResult {
        ok: true,
        message: format!("Ticket status updated to '{new_status}'"),
        data: Some(serde_json::json!({ "status": new_status })),
    })
}

#[derive(Deserialize)]
struct SubmitPayload {
    #[serde(flatten)]
    fields: HashMap<String, String>,
}

async fn submit_form(
    State(state): State<FormsState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
    axum::extract::Json(payload): axum::extract::Json<SubmitPayload>,
) -> Json<ApiResult> {
    // Resolve form definition
    let form_def = resolve_form(&state.journal, &slug).await;
    let form_def = match form_def {
        Some(d) => d,
        None => {
            return Json(ApiResult {
                ok: false,
                message: "Form not found".into(),
                data: None,
            })
        }
    };

    // Extract IP and User-Agent from headers.
    // Use rsplit to take the LAST X-Forwarded-For value (proxy-appended, trusted).
    // The first value is attacker-controllable. (luper-guard W5)
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .rsplit(',')
        .next()
        .unwrap_or("unknown")
        .trim()
        .to_string();
    let ua = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let origin = support_request_origin(&headers);

    let mut fields = payload.fields;
    normalize_submission_fields(&slug, &mut fields);

    // Honeypot check — if the honeypot field has a value, it's a bot
    if let Some(hp_val) = fields.get(&form_def.honeypot_field) {
        if !hp_val.is_empty() {
            // Silently accept (don't reveal detection to bots)
            return Json(ApiResult {
                ok: true,
                message: form_def.success_message.clone(),
                data: None,
            });
        }
    }

    {
        let mut j = state.journal.lock().await;
        if support_ip_is_blocked(&mut j, &ip) {
            return Json(ApiResult {
                ok: true,
                message: form_def.success_message.clone(),
                data: None,
            });
        }
    }

    // Rate limiting
    {
        let mut rl = state.rate_limiter.lock().await;
        if !rl.check(&ip, form_def.rate_limit_per_minute) {
            return Json(ApiResult {
                ok: false,
                message: "Too many submissions. Please try again later.".into(),
                data: None,
            });
        }
    }

    // Validate required fields
    for field in &form_def.fields {
        if field.required {
            let val = fields.get(&field.name).map(|s| s.trim()).unwrap_or("");
            if val.is_empty() {
                return Json(ApiResult {
                    ok: false,
                    message: format!("'{}' is required", field.label),
                    data: None,
                });
            }
        }
        // Basic email validation
        if field.field_type == "email" {
            if let Some(val) = fields.get(&field.name) {
                if !val.is_empty() && (!val.contains('@') || !val.contains('.')) {
                    return Json(ApiResult {
                        ok: false,
                        message: format!("'{}' must be a valid email address", field.label),
                        data: None,
                    });
                }
            }
        }
    }

    // Strip honeypot and unknown fields, keep only defined fields
    let mut clean_data = HashMap::new();
    for field in &form_def.fields {
        if let Some(val) = fields.get(&field.name) {
            clean_data.insert(field.name.clone(), val.clone());
        }
    }
    if slug == "support" {
        for field in [
            "topic",
            "topic_label",
            "message",
            "contact_flow",
            "contact_kind",
            "review_policy",
            "trust_account_policy",
            "email_verification_required",
            "verification_policy",
            "connection_flow",
            "group_type",
            "member_label",
            "requested_recipient_id",
            "requested_recipient_name",
        ] {
            if let Some(val) = fields.get(field) {
                clean_data.insert(field.to_string(), val.clone());
            }
        }
    }

    let submission = FormSubmission {
        form_slug: slug.clone(),
        data: clean_data.clone(),
        ip_address: ip,
        user_agent: ua,
        submitted_at: now_secs(),
    };

    // Store submission
    let submission_content_id: String;
    let mut email_verification: Option<SupportEmailVerification> = None;
    {
        let mut j = state.journal.lock().await;
        let mut mgr = ForgeContentManager::new(&mut j);
        let content = ForgeContent {
            content_id: String::new(),
            content_type: FORM_SUB_TYPE.into(),
            title: format!("{} submission", form_def.title),
            slug: format!("{}-{}", slug, ulid::Ulid::new().to_string()),
            body_json: serde_json::to_string(&submission).unwrap_or_default(),
            excerpt: None,
            author_id: "anonymous".into(),
            status: "published".into(),
            created_at: now_secs(),
            updated_at: now_secs(),
            published_at: Some(now_secs()),
        };
        submission_content_id = match mgr.create_content(&content) {
            Ok(id) => id,
            Err(e) => {
                return Json(ApiResult {
                    ok: false,
                    message: format!("Failed to save submission: {e}"),
                    data: None,
                })
            }
        };
        drop(mgr);

        if slug == "support" {
            email_verification =
                create_support_email_verification(&mut j, &submission_content_id, &submission);
            record_support_trust_signal(&mut j, &submission_content_id, &submission, false);
            let _ = ensure_support_thread_for_submission(&mut j, &submission_content_id);
        }
    }

    if let Some(verification) = email_verification.clone() {
        if let Some(email) = support_submission_email(&submission) {
            send_support_verification_email(
                email,
                support_submission_flow_label(&submission).to_string(),
                verification,
                origin,
            );
        }
    }

    // Send notification email (fire-and-forget, don't block response)
    // If the form has an explicit notify_email, use it; otherwise fall back
    // to the site-wide notification config from the WAL.
    let notify_addr = match form_def.notify_email {
        Some(ref e) if !e.is_empty() => Some(e.clone()),
        _ => {
            let j = state.journal.lock().await;
            let email = luperiq_forge::get_notification_email(&j, "admin");
            if email.is_empty() {
                None
            } else {
                Some(email)
            }
        }
    };
    if let Some(notify) = notify_addr {
        let title = form_def.title.clone();
        let data = clean_data.clone();
        tokio::spawn(async move {
            let body = data
                .iter()
                .map(|(k, v)| format!("<p><strong>{k}:</strong> {v}</p>"))
                .collect::<Vec<_>>()
                .join("\n");
            let payload = serde_json::json!({
                "to": notify,
                "subject": format!("New {} submission", title),
                "body": format!("<h2>New {title} submission</h2>\n{body}"),
                "is_html": true,
            });
            // TODO(review): Replace this HTTP loopback call with the platform EmailTransport
            // trait seam (same as send_support_verification_email above, audit item 1).
            let client = reqwest::Client::new();
            let _ = client
                .post("http://127.0.0.1:3000/api/modules/smtp/send")
                .json(&payload)
                .send()
                .await;
        });
    }

    Json(ApiResult {
        ok: true,
        message: form_def.success_message,
        data: Some(serde_json::json!({
            "submission_id": submission_content_id,
            "email_verification_required": email_verification.is_some(),
        })),
    })
}

async fn verify_support_email(
    State(state): State<FormsState>,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> axum::response::Response {
    let token = token.trim().to_ascii_lowercase();
    if token.len() < 20 {
        return support_verification_page(
            axum::http::StatusCode::NOT_FOUND,
            "Verification Link Not Found",
            "That verification link does not match a pending request.",
        );
    }

    let now = now_secs();
    let mut j = state.journal.lock().await;
    let mut mgr = ForgeContentManager::new(&mut j);
    let records = mgr
        .list_content(
            Some(SUPPORT_EMAIL_VERIFICATION_TYPE),
            None,
            None,
            2000,
            0,
            None,
            None,
        )
        .map(|(v, _total)| v)
        .unwrap_or_default();

    for content in records {
        let Ok(mut record) = serde_json::from_str::<SupportEmailVerification>(&content.body_json)
        else {
            continue;
        };
        if record.token != token {
            continue;
        }

        if record.status == "verified" {
            return support_verification_page(
                axum::http::StatusCode::OK,
                "Email Already Verified",
                "This request is already verified and ready for review.",
            );
        }

        if record.expires_at <= now {
            record.status = "expired".to_string();
            let body = serde_json::to_string(&record).unwrap_or_default();
            let _ = mgr.update_content(&content.content_id, None, None, Some(&body), None);
            return support_verification_page(
                axum::http::StatusCode::GONE,
                "Verification Link Expired",
                "That link has expired. Please send the request again if you still need to reach this site.",
            );
        }

        record.status = "verified".to_string();
        record.verified_at = Some(now);
        let sub_for_signal = mgr
            .get_content(&record.submission_id)
            .ok()
            .flatten()
            .and_then(|content| serde_json::from_str::<FormSubmission>(&content.body_json).ok());
        let body = serde_json::to_string(&record).unwrap_or_default();
        let submission_id = record.submission_id.clone();
        match mgr.update_content(&content.content_id, None, None, Some(&body), None) {
            Ok(_) => {
                drop(mgr);
                if let Some(sub) = sub_for_signal {
                    record_support_trust_signal(&mut j, &submission_id, &sub, true);
                }
                return support_verification_page(
                    axum::http::StatusCode::OK,
                    "Email Verified",
                    "Thank you. Your request can now be reviewed and routed by the site owner or admin.",
                );
            }
            Err(e) => {
                return support_verification_page(
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    "Verification Failed",
                    &format!("The verification link was valid, but the update failed: {e}"),
                );
            }
        }
    }

    support_verification_page(
        axum::http::StatusCode::NOT_FOUND,
        "Verification Link Not Found",
        "That verification link does not match a pending request.",
    )
}

fn support_verification_page(
    status: axum::http::StatusCode,
    title: &str,
    message: &str,
) -> axum::response::Response {
    let title = html_escape(title);
    let message = html_escape(message);
    let page = format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
body{{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;margin:0;min-height:100vh;display:grid;place-items:center;background:#f8fafc;color:#0f172a;padding:24px;}}
main{{max-width:560px;background:#fff;border:1px solid #e2e8f0;border-radius:8px;padding:32px;box-shadow:0 16px 40px rgba(15,23,42,.08);}}
h1{{font-size:26px;line-height:1.2;margin:0 0 12px;}}
p{{font-size:16px;line-height:1.6;margin:0;color:#334155;}}
a{{color:#2563eb;font-weight:700;}}
</style>
</head>
<body><main><h1>{title}</h1><p>{message}</p></main></body>
</html>"##
    );
    (status, Html(page)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_support_payload_maps_legacy_widget_fields() {
        let mut fields = HashMap::from([
            ("name".to_string(), "Jane".to_string()),
            ("email".to_string(), "jane@example.com".to_string()),
            ("topic".to_string(), "technical".to_string()),
            (
                "message".to_string(),
                "The page is failing to load for customers.".to_string(),
            ),
            ("liqSupportHoney".to_string(), "".to_string()),
        ]);

        normalize_submission_fields("support", &mut fields);

        assert_eq!(fields.get("priority").map(String::as_str), Some("High"));
        assert_eq!(fields.get("fax_number").map(String::as_str), Some(""));
        let description = fields
            .get("description")
            .expect("description should be normalized");
        assert!(description.contains("Technical Issue"));
        assert!(description.contains("The page is failing to load for customers."));
    }

    #[test]
    fn normalize_support_payload_preserves_existing_canonical_fields() {
        let mut fields = HashMap::from([
            ("priority".to_string(), "Urgent".to_string()),
            (
                "description".to_string(),
                "Canonical description".to_string(),
            ),
            ("message".to_string(), "Legacy message".to_string()),
        ]);

        normalize_submission_fields("support", &mut fields);

        assert_eq!(fields.get("priority").map(String::as_str), Some("Urgent"));
        assert_eq!(
            fields.get("description").map(String::as_str),
            Some("Canonical description")
        );
    }

    #[test]
    fn support_review_first_flows_do_not_route_directly_to_participants() {
        let sub = FormSubmission {
            form_slug: "support".to_string(),
            data: HashMap::from([
                ("name".to_string(), "Reader".to_string()),
                ("email".to_string(), "reader@example.com".to_string()),
                ("topic".to_string(), "story_idea".to_string()),
                ("contact_flow".to_string(), "owner_review_first".to_string()),
                ("contact_kind".to_string(), "blog".to_string()),
            ]),
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            submitted_at: 1,
        };

        assert!(support_submission_is_review_first(&sub));
        assert_eq!(support_submission_flow_label(&sub), "Writer Contact");
        assert!(support_submission_title(&sub).contains("Story Idea"));
        assert!(support_submission_message(&sub).contains("Review policy"));
    }

    #[test]
    fn support_verification_policy_is_opt_in_by_payload() {
        let mut sub = FormSubmission {
            form_slug: "support".to_string(),
            data: HashMap::from([
                ("email".to_string(), "reader@example.com".to_string()),
                ("contact_flow".to_string(), "owner_review_first".to_string()),
            ]),
            ip_address: "127.0.0.1".to_string(),
            user_agent: "test".to_string(),
            submitted_at: 1,
        };

        assert!(!support_submission_requires_email_verification(&sub));

        sub.data.insert(
            "verification_policy".to_string(),
            "email_before_conversation".to_string(),
        );

        assert!(support_submission_requires_email_verification(&sub));
        assert!(support_submission_message(&sub).contains("Email verification: Required"));
    }
}

// ── Public form page render ───────────────────────────────────────────

async fn render_form_page(
    State(state): State<FormsState>,
    axum::extract::Path(slug): axum::extract::Path<String>,
) -> axum::response::Response {
    let form_def = resolve_form(&state.journal, &slug).await;
    let form_def = match form_def {
        Some(d) => d,
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                Html("Form not found".to_string()),
            )
                .into_response();
        }
    };

    // Build form HTML with proper escaping
    let mut fields_html = String::new();
    for field in &form_def.fields {
        let name = html_escape(&field.name);
        let label = html_escape(&field.label);
        let placeholder = html_escape(&field.placeholder);
        let required = if field.required { " required" } else { "" };

        match field.field_type.as_str() {
            "textarea" => {
                fields_html.push_str(&format!(
                    r#"<div class="form-group"><label for="{name}">{label}</label><textarea id="{name}" name="{name}" placeholder="{placeholder}"{required}></textarea></div>"#
                ));
            }
            "select" => {
                let options: String = field
                    .options
                    .iter()
                    .map(|o| {
                        format!(
                            r#"<option value="{}">{}</option>"#,
                            html_escape(o),
                            html_escape(o)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("");
                fields_html.push_str(&format!(
                    r#"<div class="form-group"><label for="{name}">{label}</label><select id="{name}" name="{name}"{required}><option value="">Select...</option>{options}</select></div>"#
                ));
            }
            "checkbox" => {
                fields_html.push_str(&format!(
                    r#"<div class="form-group"><label><input type="checkbox" name="{name}"{required}> {label}</label></div>"#
                ));
            }
            _ => {
                let input_type = if field.field_type == "email" {
                    "email"
                } else {
                    "text"
                };
                fields_html.push_str(&format!(
                    r#"<div class="form-group"><label for="{name}">{label}</label><input type="{input_type}" id="{name}" name="{name}" placeholder="{placeholder}"{required}></div>"#
                ));
            }
        }
    }

    // Honeypot field (hidden via CSS, not type=hidden, to catch more bots)
    let hp = html_escape(&form_def.honeypot_field);
    fields_html.push_str(&format!(
        r#"<div style="position:absolute;left:-9999px;"><label for="{hp}">Leave blank</label><input type="text" id="{hp}" name="{hp}" tabindex="-1" autocomplete="off"></div>"#
    ));

    let title = html_escape(&form_def.title);
    let slug_escaped = html_escape(&form_def.slug);

    let page = format!(
        r##"<!DOCTYPE html>
<html><head>
<title>{title} — LuperIQ</title>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<style>
body {{ font-family: -apple-system, BlinkMacSystemFont, sans-serif; background: #0f172a; color: #e2e8f0; margin: 0; padding: 40px 20px; }}
.form-container {{ max-width: 560px; margin: 0 auto; background: #1e293b; border: 1px solid #334155; border-radius: 12px; padding: 32px; }}
h1 {{ font-size: 24px; margin: 0 0 24px; }}
.form-group {{ margin-bottom: 16px; }}
label {{ display: block; font-size: 13px; font-weight: 500; margin-bottom: 6px; color: #94a3b8; }}
input, textarea, select {{ width: 100%; padding: 10px 12px; background: #0f172a; border: 1px solid #334155; border-radius: 6px; color: #e2e8f0; font-size: 14px; box-sizing: border-box; }}
textarea {{ min-height: 100px; resize: vertical; }}
button {{ background: #3b82f6; color: white; border: none; border-radius: 6px; padding: 12px 24px; font-size: 14px; font-weight: 500; cursor: pointer; margin-top: 8px; }}
button:hover {{ background: #2563eb; }}
button:disabled {{ opacity: 0.6; cursor: not-allowed; }}
.success {{ color: #22c55e; margin-top: 12px; }}
.error {{ color: #ef4444; margin-top: 12px; }}
</style>
</head><body>
<div class="form-container">
<h1>{title}</h1>
<form id="liq-form" onsubmit="return submitForm(event)">
{fields_html}
<button type="submit" id="submit-btn">Submit</button>
<div id="form-status"></div>
</form>
</div>
<script>
async function submitForm(e) {{
    e.preventDefault();
    const btn = document.getElementById('submit-btn');
    const status = document.getElementById('form-status');
    btn.disabled = true;
    btn.textContent = 'Submitting...';
    status.textContent = '';
    status.className = '';

    const fd = new FormData(e.target);
    const data = {{}};
    fd.forEach((v, k) => data[k] = v);

    try {{
        const r = await fetch('/api/modules/forms/{slug_escaped}/submit', {{
            method: 'POST',
            headers: {{ 'Content-Type': 'application/json' }},
            body: JSON.stringify(data),
        }}).then(r => r.json());
        status.textContent = r.message;
        status.className = r.ok ? 'success' : 'error';
        if (r.ok) e.target.reset();
    }} catch(err) {{
        status.textContent = 'Network error. Please try again.';
        status.className = 'error';
    }}
    btn.disabled = false;
    btn.textContent = 'Submit';
    return false;
}}
</script>
</body></html>"##
    );

    Html(page).into_response()
}

use axum::response::IntoResponse;

// ── Helpers ───────────────────────────────────────────────────────────

async fn resolve_form(journal: &SharedJournal, slug: &str) -> Option<FormDefinition> {
    // Check stored forms first
    let mut j = journal.lock().await;
    let mgr = ForgeContentManager::new(&mut j);
    let stored = mgr
        .list_content(Some(FORM_DEF_TYPE), None, None, 100, 0, None, None)
        .map(|(v, _total)| {
            v.into_iter()
                .map(|c| (c.content_id.clone(), c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for (_id, c) in &stored {
        if c.slug == slug {
            if let Ok(def) = serde_json::from_str::<FormDefinition>(&c.body_json) {
                return Some(def);
            }
        }
    }

    // Fall back to builtins
    builtin_forms().into_iter().find(|f| f.slug == slug)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

// ── Admin JS ──────────────────────────────────────────────────────────

const FORMS_ADMIN_JS: &str = r##"
var _role = (window.__CMS && window.__CMS.nexusRole) || '';
var _isPro = _role === 'central' || _role === 'professional' || _role === 'enterprise';
var _isStarter = _isPro || _role === 'starter';

// ── Forms list view ────────────────────────────────────────────────
async function load_forms() {
    const main = document.getElementById('adminMain');
    const r = await fetch('/api/modules/forms').then(r => r.json());
    const forms = r.data || [];

    const el = document.createElement('div');

    var _pc = lqModulePricingCard({name:'Forms',monthly:9,annual:89,lifetime:249,tier:'starter',deps:['SMTP'],slug:'forms'});
    if (_pc) el.appendChild(_pc);

    var _allForms = forms;
    if (!_isStarter && forms.length > 3) {
        forms = forms.slice(0, 3);
    }

    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'Forms';
    toolbar.appendChild(h);
    if (_isStarter) {
        const newBtn = document.createElement('button');
        newBtn.className = 'btn btn-primary';
        newBtn.textContent = '+ New Form';
        newBtn.onclick = () => openFormBuilder();
        toolbar.appendChild(newBtn);
    } else {
        const proBadge = document.createElement('span');
        proBadge.className = 'status-badge status-published';
        proBadge.style.cssText = 'margin-left:8px;background:#f59e0b;color:#000;font-size:11px;';
        proBadge.textContent = 'STARTER';
        h.appendChild(proBadge);
    }
    el.appendChild(toolbar);

    if (!_isStarter && _allForms.length > 3) {
        const gate = document.createElement('div');
        gate.style.cssText = 'background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:12px 16px;margin-bottom:12px;display:flex;align-items:center;justify-content:space-between;';
        const gateText = document.createElement('span');
        gateText.style.cssText = 'font-size:13px;color:var(--text-muted);';
        gateText.textContent = 'Showing 3 of ' + _allForms.length + ' forms — upgrade for full access';
        gate.appendChild(gateText);
        var upBtn = document.createElement('button');
        upBtn.className = 'btn btn-primary btn-sm';
        upBtn.textContent = 'Upgrade';
        upBtn.onclick = function() { navigateTo('store'); };
        gate.appendChild(upBtn);
        el.appendChild(gate);
    }

    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(forms, 'forms.json');
        } else {
            lqExportCSV(forms, ['title','slug','notify_email','builtin'], 'forms.csv');
        }
    }, function() {
        lqImportJSON(async function(data) {
            var arr = Array.isArray(data) ? data : [data];
            var ok = 0;
            for (var i = 0; i < arr.length; i++) {
                try {
                    await fetch('/api/modules/forms', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify(arr[i]) });
                    ok++;
                } catch(e) {}
            }
            showToast('Imported ' + ok + ' of ' + arr.length + ' forms', 'success');
            load_forms();
        });
    });

    const table = document.createElement('div');
    table.className = 'content-table';
    const t = document.createElement('table');
    const hdr = document.createElement('tr');
    ['Form','Fields','Notify','Type',''].forEach(col => {
        const th = document.createElement('th');
        th.textContent = col;
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    forms.forEach(f => {
        const tr = document.createElement('tr');
        const nameTd = document.createElement('td');
        const a = document.createElement('a');
        a.href = '#';
        a.style.color = 'var(--accent)';
        a.textContent = f.title;
        a.onclick = (ev) => { ev.preventDefault(); openFormBuilder(f.slug); };
        nameTd.appendChild(a);
        tr.appendChild(nameTd);

        const fieldsTd = document.createElement('td');
        fieldsTd.textContent = f.fields + ' fields';
        tr.appendChild(fieldsTd);

        const notifyTd = document.createElement('td');
        notifyTd.textContent = f.notify_email || '-';
        tr.appendChild(notifyTd);

        const typeTd = document.createElement('td');
        const badge = document.createElement('span');
        badge.className = 'status-badge ' + (f.builtin ? 'status-draft' : 'status-published');
        badge.textContent = f.builtin ? 'Built-in' : 'Custom';
        typeTd.appendChild(badge);
        tr.appendChild(typeTd);

        const actionTd = document.createElement('td');
        const viewBtn = document.createElement('button');
        viewBtn.className = 'btn btn-ghost btn-sm';
        viewBtn.textContent = 'View';
        viewBtn.onclick = () => window.open('/forms/' + encodeURIComponent(f.slug), '_blank');
        actionTd.appendChild(viewBtn);
        tr.appendChild(actionTd);

        t.appendChild(tr);
    });

    table.appendChild(t);
    el.appendChild(table);
    main.replaceChildren(el);
}

async function openFormBuilder(slug) {
    if (slug) {
        const r = await fetch('/api/modules/forms/' + encodeURIComponent(slug)).then(r => r.json());
        if (r.ok) {
            showFormEditor(r.data);
            return;
        }
    }
    showFormEditor(null);
}

function showFormEditor(form) {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    const h = document.createElement('h2');
    h.textContent = form ? 'Edit Form: ' + form.title : 'New Form';
    el.appendChild(h);

    const card = document.createElement('div');
    card.style.cssText = 'max-width:600px;background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:20px;';

    const addField = (label, id, value, type) => {
        const row = document.createElement('div');
        row.style.marginBottom = '12px';
        const lbl = document.createElement('label');
        lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;';
        lbl.textContent = label;
        row.appendChild(lbl);
        const inp = document.createElement(type === 'textarea' ? 'textarea' : 'input');
        inp.id = id;
        inp.className = 'admin-input';
        inp.value = value || '';
        if (type === 'textarea') inp.rows = 6;
        row.appendChild(inp);
        card.appendChild(row);
    };

    addField('Form Slug', 'form-slug', form?.slug, 'text');
    addField('Form Title', 'form-title', form?.title, 'text');
    addField('Notification Email', 'form-notify', form?.notify_email, 'text');
    addField('Success Message', 'form-success', form?.success_message, 'text');
    addField('Fields (JSON)', 'form-fields', form ? JSON.stringify(form.fields, null, 2) : '[]', 'textarea');

    // AI Improve Form button — available to all tiers
    if (typeof LiqAI !== 'undefined') {
        var aiRow = document.createElement('div');
        aiRow.style.cssText = 'margin-bottom:12px;display:flex;gap:8px;align-items:center;';
        var aiImproveBtn = LiqAI.button({
            label: 'AI Improve Form',
            feature: 'forms_ai_improve',
            credits: 2,
            tier: 'free',
            deferTargetApply: true,
            applyTargetOnKeep: false,
            previewTitle: 'Field improvement preview',
            getInput: function() {
                var f = document.getElementById('form-fields').value.trim();
                if (!f || f === '[]') { showToast('Add some fields first', 'error'); return ''; }
                return 'Form title: ' + (document.getElementById('form-title').value || 'Untitled') + '\nFields: ' + f;
            },
            targetId: 'form-fields',
            renderPreview: function(result, body) {
                if (!Array.isArray(result) || !result.length) {
                    body.textContent = 'No changes suggested.';
                    return;
                }
                result.forEach(function(imp) {
                    var line = document.createElement('div');
                    line.style.marginBottom = '6px';
                    var parts = [imp.field_name || '(unknown field)'];
                    if (imp.label) parts.push('label -> ' + imp.label);
                    if (imp.placeholder) parts.push('placeholder -> ' + imp.placeholder);
                    line.textContent = parts.join(' | ');
                    body.appendChild(line);
                });
            },
            onKeepResult: function(result) {
                if (Array.isArray(result)) {
                    var current = [];
                    try { current = JSON.parse(document.getElementById('form-fields').value); } catch(e) {}
                    result.forEach(function(imp) {
                        var field = current.find(function(f) { return f.name === imp.field_name; });
                        if (field) {
                            if (imp.label) field.label = imp.label;
                            if (imp.placeholder) field.placeholder = imp.placeholder;
                        }
                    });
                    document.getElementById('form-fields').value = JSON.stringify(current, null, 2);
                    showToast('Fields improved by AI', 'success');
                }
            },
        });
        if (aiImproveBtn) aiRow.appendChild(aiImproveBtn);
        var aiFb = document.createElement('div');
        aiFb.id = 'form-fields-ai-fb';
        aiRow.appendChild(aiFb);
        card.appendChild(aiRow);
    }

    const btns = document.createElement('div');
    btns.style.cssText = 'margin-top:16px;display:flex;gap:8px;';
    const saveBtn = document.createElement('button');
    saveBtn.className = 'btn btn-primary';
    saveBtn.textContent = 'Save Form';
    saveBtn.onclick = async () => {
        const slug = document.getElementById('form-slug').value.trim();
        const title = document.getElementById('form-title').value.trim();
        if (!slug || !title) { showToast('Slug and title required', 'error'); return; }
        let fields;
        try { fields = JSON.parse(document.getElementById('form-fields').value); }
        catch(e) { showToast('Invalid fields JSON', 'error'); return; }
        const honeypotField = (form && form.honeypot_field) || (slug === 'support' ? 'fax_number' : 'website_url');
        const r = await fetch('/api/modules/forms', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                slug, title, fields,
                notify_email: document.getElementById('form-notify').value || null,
                success_message: document.getElementById('form-success').value || 'Thank you!',
                honeypot_field: honeypotField,
                rate_limit_per_minute: 3,
            }),
        }).then(r => r.json());
        showToast(r.ok ? 'Form saved!' : r.message, r.ok ? 'success' : 'error');
        if (r.ok) load_forms();
    };
    btns.appendChild(saveBtn);
    const backBtn = document.createElement('button');
    backBtn.className = 'btn btn-ghost';
    backBtn.textContent = 'Back to Forms';
    backBtn.onclick = () => load_forms();
    btns.appendChild(backBtn);
    card.appendChild(btns);

    el.appendChild(card);
    main.replaceChildren(el);
}

// ── Submissions view ───────────────────────────────────────────────
async function load_form_submissions() {
    const main = document.getElementById('adminMain');
    // First get list of forms
    const r = await fetch('/api/modules/forms').then(r => r.json());
    const forms = r.data || [];

    const el = document.createElement('div');
    const h = document.createElement('h2');
    h.textContent = 'Form Submissions';
    el.appendChild(h);

    if (forms.length === 0) {
        const p = document.createElement('p');
        p.style.color = 'var(--text-muted)';
        p.textContent = 'No forms configured yet.';
        el.appendChild(p);
        main.replaceChildren(el);
        return;
    }

    // Form selector
    const selRow = document.createElement('div');
    selRow.style.marginBottom = '16px';
    const sel = document.createElement('select');
    sel.className = 'admin-input';
    sel.style.maxWidth = '300px';
    forms.forEach(f => {
        const opt = document.createElement('option');
        opt.value = f.slug;
        opt.textContent = f.title;
        sel.appendChild(opt);
    });
    sel.onchange = () => loadSubsForForm(sel.value);
    selRow.appendChild(sel);
    el.appendChild(selRow);

    const subsContainer = document.createElement('div');
    subsContainer.id = 'subs-container';
    el.appendChild(subsContainer);

    main.replaceChildren(el);

    // Load first form's submissions
    if (forms.length > 0) loadSubsForForm(forms[0].slug);
}

async function loadSubsForForm(slug) {
    const container = document.getElementById('subs-container');
    const r = await fetch('/api/modules/forms/' + encodeURIComponent(slug) + '/submissions?limit=50').then(r => r.json());
    const subs = r.data || [];

    container.replaceChildren();

    lqAddExportImportBar(container, function(format) {
        if (format === 'json') {
            lqExportJSON(subs, 'form-submissions.json');
        } else {
            var allKeys = new Set();
            subs.forEach(function(s) { Object.keys(s.data || {}).forEach(function(k) { allKeys.add(k); }); });
            var csvHeaders = Array.from(allKeys);
            csvHeaders.push('submitted_at');
            var csvRows = subs.map(function(s) {
                var row = {};
                csvHeaders.forEach(function(k) { row[k] = k === 'submitted_at' ? s.submitted_at : (s.data || {})[k]; });
                return row;
            });
            lqExportCSV(csvRows, csvHeaders, 'form-submissions.csv');
        }
    }, null);

    if (subs.length === 0) {
        const p = document.createElement('p');
        p.style.color = 'var(--text-muted)';
        p.textContent = 'No submissions yet for this form.';
        container.appendChild(p);
        return;
    }

    const table = document.createElement('div');
    table.className = 'content-table';
    const t = document.createElement('table');

    // Get all unique field names from submissions
    const allKeys = new Set();
    subs.forEach(s => Object.keys(s.data || {}).forEach(k => allKeys.add(k)));
    const keys = [...allKeys];

    const hdr = document.createElement('tr');
    keys.forEach(k => { const th = document.createElement('th'); th.textContent = k; hdr.appendChild(th); });
    const thDate = document.createElement('th'); thDate.textContent = 'Date'; hdr.appendChild(thDate);
    const thAct = document.createElement('th'); thAct.textContent = ''; hdr.appendChild(thAct);
    t.appendChild(hdr);

    subs.forEach(function(s) {
        const tr = document.createElement('tr');
        keys.forEach(k => {
            const td = document.createElement('td');
            td.textContent = (s.data || {})[k] || '';
            tr.appendChild(td);
        });
        const dateTd = document.createElement('td');
        dateTd.textContent = new Date(s.submitted_at * 1000).toLocaleString();
        tr.appendChild(dateTd);
        // AI Draft Response action
        var actTd = document.createElement('td');
        if (typeof LiqAI !== 'undefined') {
            var _s = s;
            var aiBtn = LiqAI.button({
                label: 'AI Draft Response',
                feature: 'forms_ai_response',
                credits: 1,
                tier: 'free',
                getInput: function() {
                    var parts = [];
                    Object.keys(_s.data || {}).forEach(function(k) { parts.push(k + ': ' + _s.data[k]); });
                    return 'Form: ' + slug + '\nSubmission:\n' + parts.join('\n');
                },
                onResult: function(result) {
                    if (typeof result === 'string') {
                        try { navigator.clipboard.writeText(result); } catch(e) {}
                        showToast('Response drafted and copied to clipboard', 'success');
                    }
                },
            });
            if (aiBtn) actTd.appendChild(aiBtn);
        }
        tr.appendChild(actTd);
        t.appendChild(tr);
    });

    table.appendChild(t);
    container.appendChild(table);
}

// ── Support Inbox view ────────────────────────────────────────────
async function load_support_inbox() {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    const r = await fetch('/api/modules/forms/support/inbox').then(r => r.json());
    const data = r.data || {};
    const tickets = data.tickets || [];
    const counts = data.counts || {};

    // Header with status badges
    const toolbar = document.createElement('div');
    toolbar.className = 'toolbar';
    const h = document.createElement('h2');
    h.textContent = 'Support Inbox';
    toolbar.appendChild(h);

    const badges = document.createElement('div');
    badges.style.cssText = 'display:flex;gap:8px;align-items:center;';

    function makeBadge(label, count, color, bgColor) {
        const badge = document.createElement('span');
        badge.style.cssText = 'display:inline-flex;align-items:center;gap:4px;padding:4px 10px;border-radius:12px;font-size:12px;font-weight:600;background:' + bgColor + ';color:' + color + ';';
        badge.textContent = count + ' ' + label;
        return badge;
    }
    if (counts.new > 0) badges.appendChild(makeBadge('New', counts.new, '#dc2626', '#fef2f2'));
    if (counts.in_progress > 0) badges.appendChild(makeBadge('In Progress', counts.in_progress, '#d97706', '#fffbeb'));
    if (counts.resolved > 0) badges.appendChild(makeBadge('Resolved', counts.resolved, '#16a34a', '#f0fdf4'));
    toolbar.appendChild(badges);
    el.appendChild(toolbar);

    // Export bar
    lqAddExportImportBar(el, function(format) {
        if (format === 'json') {
            lqExportJSON(tickets, 'support-tickets.json');
        } else {
            lqExportCSV(tickets, ['name','email','topic','priority','message','status','submitted_at'], 'support-tickets.csv');
        }
    }, null);

    if (tickets.length === 0) {
        const p = document.createElement('p');
        p.style.cssText = 'color:var(--text-muted);padding:32px 0;text-align:center;';
        p.textContent = 'No support tickets yet. Tickets arrive when visitors use the support widget.';
        el.appendChild(p);
        main.replaceChildren(el);
        return;
    }

    // Tickets table
    const table = document.createElement('div');
    table.className = 'content-table';
    const t = document.createElement('table');
    const hdr = document.createElement('tr');
    ['Date','Name','Email','Topic','Priority','Message','Status',''].forEach(function(col) {
        const th = document.createElement('th');
        th.textContent = col;
        hdr.appendChild(th);
    });
    t.appendChild(hdr);

    tickets.forEach(function(ticket) {
        const tr = document.createElement('tr');
        tr.style.cursor = 'pointer';

        // Date
        var dateTd = document.createElement('td');
        dateTd.style.whiteSpace = 'nowrap';
        dateTd.textContent = new Date((ticket.submitted_at || 0) * 1000).toLocaleDateString();
        tr.appendChild(dateTd);

        // Name
        var nameTd = document.createElement('td');
        nameTd.textContent = ticket.name || '-';
        tr.appendChild(nameTd);

        // Email
        var emailTd = document.createElement('td');
        var emailLink = document.createElement('a');
        emailLink.href = 'mailto:' + (ticket.email || '');
        emailLink.textContent = ticket.email || '-';
        emailLink.style.color = 'var(--accent)';
        emailLink.onclick = function(e) { e.stopPropagation(); };
        emailTd.appendChild(emailLink);
        tr.appendChild(emailTd);

        // Topic
        var topicTd = document.createElement('td');
        topicTd.textContent = ticket.topic_label || ticket.topic || '-';
        tr.appendChild(topicTd);

        // Priority
        var prioTd = document.createElement('td');
        var prioBadge = document.createElement('span');
        prioBadge.className = 'status-badge';
        var prioColor = { High: '#dc2626', Urgent: '#dc2626', Medium: '#d97706', Low: '#16a34a' }[ticket.priority] || '#6b7280';
        prioBadge.style.cssText = 'background:' + prioColor + '22;color:' + prioColor + ';font-size:11px;padding:2px 8px;border-radius:4px;font-weight:600;';
        prioBadge.textContent = ticket.priority || 'Medium';
        prioTd.appendChild(prioBadge);
        tr.appendChild(prioTd);

        // Message (truncated)
        var msgTd = document.createElement('td');
        msgTd.style.cssText = 'max-width:200px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;';
        msgTd.textContent = (ticket.message || '').substring(0, 80) + (ticket.message && ticket.message.length > 80 ? '...' : '');
        tr.appendChild(msgTd);

        // Status dropdown
        var statusTd = document.createElement('td');
        var statusSelect = document.createElement('select');
        statusSelect.className = 'admin-input';
        statusSelect.style.cssText = 'font-size:12px;padding:4px 8px;min-width:120px;';
        [['new','New'],['in-progress','In Progress'],['resolved','Resolved']].forEach(function(opt) {
            var o = document.createElement('option');
            o.value = opt[0];
            o.textContent = opt[1];
            if (opt[0] === ticket.status) o.selected = true;
            statusSelect.appendChild(o);
        });
        statusSelect.onclick = function(e) { e.stopPropagation(); };
        statusSelect.onchange = function() {
            var newStatus = statusSelect.value;
            fetch('/api/modules/forms/support/submissions/' + encodeURIComponent(ticket.id) + '/status', {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ status: newStatus }),
            }).then(function(r) { return r.json(); }).then(function(res) {
                if (res.ok) {
                    showToast('Status updated to ' + newStatus, 'success');
                    // Refresh to update badges
                    load_support_inbox();
                } else {
                    showToast(res.message || 'Failed to update', 'error');
                }
            }).catch(function() { showToast('Network error', 'error'); });
        };
        statusTd.appendChild(statusSelect);
        tr.appendChild(statusTd);

        // Expand action
        var actTd = document.createElement('td');
        var expandBtn = document.createElement('button');
        expandBtn.className = 'btn btn-ghost btn-sm';
        expandBtn.textContent = 'View';
        expandBtn.onclick = function(e) {
            e.stopPropagation();
            showTicketDetail(ticket);
        };
        actTd.appendChild(expandBtn);
        tr.appendChild(actTd);

        t.appendChild(tr);

        // Click row to expand
        tr.onclick = function() { showTicketDetail(ticket); };
    });

    table.appendChild(t);
    el.appendChild(table);
    main.replaceChildren(el);
}

function showTicketDetail(ticket) {
    const main = document.getElementById('adminMain');
    const el = document.createElement('div');

    // Back button
    var backRow = document.createElement('div');
    backRow.style.marginBottom = '16px';
    var backBtn = document.createElement('button');
    backBtn.className = 'btn btn-ghost';
    backBtn.textContent = 'Back to Inbox';
    backBtn.onclick = function() { load_support_inbox(); };
    backRow.appendChild(backBtn);
    el.appendChild(backRow);

    var h = document.createElement('h2');
    h.textContent = 'Support Ticket';
    el.appendChild(h);

    var card = document.createElement('div');
    card.style.cssText = 'max-width:640px;background:var(--surface);border:1px solid var(--border);border-radius:8px;padding:24px;';

    // Meta row
    var meta = document.createElement('div');
    meta.style.cssText = 'display:flex;gap:16px;flex-wrap:wrap;margin-bottom:20px;font-size:13px;color:var(--text-muted);';
    function addMeta(label, value) {
        var span = document.createElement('span');
        var strong = document.createElement('strong');
        strong.textContent = label + ': ';
        span.appendChild(strong);
        span.appendChild(document.createTextNode(value));
        meta.appendChild(span);
    }
    addMeta('Date', new Date((ticket.submitted_at || 0) * 1000).toLocaleString());
    addMeta('Flow', ticket.flow_label || 'Support Request');
    addMeta('Priority', ticket.priority || 'Medium');
    addMeta('Topic', ticket.topic_label || ticket.topic || '-');
    addMeta('IP', ticket.ip_address || '-');
    if (ticket.review_first) addMeta('Routing', 'Review first');
    if (ticket.email_verification_required) addMeta('Email verification', ticket.email_verified ? 'Verified' : 'Pending');
    card.appendChild(meta);

    // Name + Email
    function addField(label, value) {
        var row = document.createElement('div');
        row.style.marginBottom = '12px';
        var lbl = document.createElement('label');
        lbl.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;text-transform:uppercase;letter-spacing:0.5px;';
        lbl.textContent = label;
        row.appendChild(lbl);
        var val = document.createElement('div');
        val.style.cssText = 'font-size:14px;';
        val.textContent = value || '-';
        row.appendChild(val);
        card.appendChild(row);
    }

    addField('Name', ticket.name);
    addField('Email', ticket.email);

    // Full message
    var msgLabel = document.createElement('label');
    msgLabel.style.cssText = 'font-size:12px;color:var(--text-muted);display:block;margin-bottom:4px;text-transform:uppercase;letter-spacing:0.5px;';
    msgLabel.textContent = 'Message';
    card.appendChild(msgLabel);
    var msgBox = document.createElement('div');
    msgBox.style.cssText = 'background:var(--bg);border:1px solid var(--border);border-radius:6px;padding:16px;font-size:14px;line-height:1.6;white-space:pre-wrap;margin-bottom:16px;';
    msgBox.textContent = ticket.message || '(no message)';
    card.appendChild(msgBox);

    // Status control
    var statusRow = document.createElement('div');
    statusRow.style.cssText = 'display:flex;align-items:center;gap:12px;margin-bottom:16px;';
    var statusLabel = document.createElement('label');
    statusLabel.style.cssText = 'font-size:12px;color:var(--text-muted);text-transform:uppercase;letter-spacing:0.5px;';
    statusLabel.textContent = 'Status';
    statusRow.appendChild(statusLabel);
    var statusSelect = document.createElement('select');
    statusSelect.className = 'admin-input';
    statusSelect.style.cssText = 'max-width:200px;';
    [['new','New'],['in-progress','In Progress'],['resolved','Resolved']].forEach(function(opt) {
        var o = document.createElement('option');
        o.value = opt[0];
        o.textContent = opt[1];
        if (opt[0] === ticket.status) o.selected = true;
        statusSelect.appendChild(o);
    });
    statusSelect.onchange = function() {
        var newStatus = statusSelect.value;
        fetch('/api/modules/forms/support/submissions/' + encodeURIComponent(ticket.id) + '/status', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ status: newStatus }),
        }).then(function(r) { return r.json(); }).then(function(res) {
            if (res.ok) {
                showToast('Status updated to ' + newStatus, 'success');
                ticket.status = newStatus;
            } else {
                showToast(res.message || 'Failed to update', 'error');
            }
        }).catch(function() { showToast('Network error', 'error'); });
    };
    statusRow.appendChild(statusSelect);
    card.appendChild(statusRow);

    // Reply tools
    var actionsRow = document.createElement('div');
    actionsRow.style.cssText = 'display:flex;gap:8px;flex-wrap:wrap;margin-bottom:16px;';
    var convoBtn = document.createElement('button');
    convoBtn.className = 'btn btn-primary';
    convoBtn.textContent = 'Open Conversation';
    if (ticket.email_verification_required && !ticket.email_verified) {
        convoBtn.disabled = true;
        convoBtn.textContent = 'Waiting for email verification';
        convoBtn.title = 'The sender needs to verify their email before this can become a conversation.';
    }
    convoBtn.onclick = async function() {
        if (convoBtn.disabled) return;
        try {
            if (typeof _loadModuleJsWithDeps === 'function') {
                await _loadModuleJsWithDeps('messaging');
            }
        } catch (e) {}
        fetch('/api/modules/forms/support/submissions/' + encodeURIComponent(ticket.id) + '/conversation', {
            method: 'POST'
        }).then(function(r) { return r.json(); }).then(function(res) {
            if (!res.ok || !res.data || !res.data.thread_id) {
                showToast((res && res.message) || 'Unable to open conversation', 'error');
                return;
            }
            if (typeof openThreadDetail === 'function') {
                openThreadDetail(res.data.thread_id);
                return;
            }
            if (typeof navigateTo === 'function') navigateTo('msg-threads');
            showToast('Conversation is ready in Communication -> Threads.', 'success');
        }).catch(function() {
            showToast('Network error', 'error');
        });
    };
    actionsRow.appendChild(convoBtn);
    if (ticket.email) {
        var mailBtn = document.createElement('a');
        mailBtn.className = 'btn btn-ghost';
        mailBtn.textContent = 'Email Reply';
        mailBtn.href = 'mailto:' + ticket.email + '?subject=' + encodeURIComponent((ticket.topic_label || ticket.topic || 'Support Request') + ' - ' + (ticket.name || 'Customer'));
        actionsRow.appendChild(mailBtn);
    }
    card.appendChild(actionsRow);

    // AI Draft Response
    if (typeof LiqAI !== 'undefined') {
        var aiRow = document.createElement('div');
        aiRow.style.cssText = 'margin-top:8px;';
        var aiBtn = LiqAI.button({
            label: 'AI Draft Response',
            feature: 'forms_ai_response',
            credits: 1,
            tier: 'free',
            getInput: function() {
                return (ticket.flow_label || 'Support Ticket') + '\nName: ' + (ticket.name || '') + '\nEmail: ' + (ticket.email || '') + '\nTopic: ' + (ticket.topic_label || ticket.topic || '') + '\nRouting: ' + (ticket.review_first ? 'Review first' : 'Standard') + '\nEmail verification: ' + (ticket.email_verification_required ? (ticket.email_verified ? 'Verified' : 'Pending') : 'Not required') + '\nPriority: ' + (ticket.priority || '') + '\nMessage: ' + (ticket.description || ticket.message || '');
            },
            onResult: function(result) {
                if (typeof result === 'string') {
                    try { navigator.clipboard.writeText(result); } catch(e) {}
                    showToast('Response drafted and copied to clipboard', 'success');
                }
            },
        });
        if (aiBtn) aiRow.appendChild(aiBtn);
        card.appendChild(aiRow);
    }

    el.appendChild(card);
    main.replaceChildren(el);
}
"##;
