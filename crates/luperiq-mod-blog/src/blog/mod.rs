//! Blog Module — public blog index and single-post pages.
//!
//! Renders `/blog` (listing of published posts) and `/blog/{slug}` (individual
//! post view). Integrates with Theme Studio, SEO meta, Google Analytics, and
//! the admin toolbar for logged-in editors.
//!
//! Previously lived in `routes/blog.rs` as core code; extracted into a module
//! so it can be toggled, updated, and managed independently of the CMS core.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use axum_extra::extract::cookie::CookieJar;
use std::sync::Arc;
use tera::{Context, Tera};

use luperiq_module_api::{AdminView, AppContext, CmsModule, SharedJournal};

// ── Provider trait ──────────────────────────────────────────────────

/// Trait abstracting CMS-specific helpers that the blog module needs.
///
/// The CMS wires in the real implementation; this keeps the crate
/// decoupled from CMS internals (routes::pages, middleware::session, etc.).
pub trait BlogCmsProvider: Send + Sync + 'static {
    /// Return the current asset version string (for cache busting).
    fn asset_version(&self) -> &str;

    /// Inject theme studio variables into a Tera context.
    fn inject_theme_studio(&self, ctx: &mut Context, journal: &mut luperiq_forge::ForgeJournal);

    /// Inject Google Analytics/Tag Manager tags into a Tera context.
    fn inject_google_tags(&self, ctx: &mut Context, journal: &mut luperiq_forge::ForgeJournal);

    /// Convert body_json (block editor content) to HTML.
    fn body_to_html(&self, body_json: &str) -> String;

    /// Check if the current request is from an authenticated admin.
    ///
    /// Accepts cloneable values so the returned future can be `'static`.
    fn is_admin_session(
        &self,
        jar: CookieJar,
        journal: SharedJournal,
        jwt_secret: String,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>;

    /// Get the effective public base URL from headers.
    fn effective_public_base_url(&self, headers: &HeaderMap) -> String;

    /// Look up SEO meta for a content ID.
    fn lookup_seo_meta(
        &self,
        journal: &mut luperiq_forge::ForgeJournal,
        content_id: &str,
    ) -> Option<SeoMetaSnapshot>;

    /// Render admin toolbar HTML.
    fn admin_toolbar_html(
        &self,
        content_id: Option<&str>,
        seo_title: Option<&str>,
        seo_description: Option<&str>,
        seo_score: Option<u8>,
        robots: Option<&str>,
        extra: &str,
    ) -> String;
}

/// Snapshot of SEO meta used by the blog for toolbar display.
pub struct SeoMetaSnapshot {
    pub title: String,
    pub description: String,
    pub seo_score: u8,
    pub robots: String,
}

/// Shared handle to the CMS provider.
pub type BlogProvider = Arc<dyn BlogCmsProvider>;

// ── Module Definition ─────────────────────────────────────────────

pub struct BlogModule {
    pub provider: BlogProvider,
}

impl CmsModule for BlogModule {
    fn slug(&self) -> &str {
        "blog"
    }

    fn name(&self) -> &str {
        "Blog"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn description(&self) -> &str {
        "Public blog with SEO, Theme Studio integration, and admin toolbar for inline editing."
    }

    fn category(&self) -> &str {
        "Content"
    }

    fn routes(&self, ctx: &AppContext) -> Option<Router> {
        let tera = ctx.tera.clone()?;
        let state = BlogState {
            journal: ctx.journal.clone(),
            tera,
            theme_css: ctx.theme_css.clone(),
            site_name: ctx.site_name.clone(),
            jwt_secret: Arc::new(ctx.jwt_secret.clone()),
            provider: self.provider.clone(),
        };
        Some(
            Router::new()
                .route("/blog", get(blog_index))
                .route(
                    "/blog/",
                    get(|| async { axum::response::Redirect::permanent("/blog") }),
                )
                .route("/blog/{slug}", get(blog_single))
                .with_state(state),
        )
    }

    fn admin_views(&self) -> Vec<AdminView> {
        // Intentionally empty: blog admin surface is provided by the CMS
        // content manager (Page Studio / Content Studio) rather than a
        // dedicated sidebar entry. An AdminView pointing to the post list
        // could be added here once a direct /admin/blog route exists.
        vec![]
    }
}

// ── State ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct BlogState {
    journal: SharedJournal,
    tera: Arc<Tera>,
    theme_css: Arc<String>,
    site_name: Arc<String>,
    jwt_secret: Arc<String>,
    provider: BlogProvider,
}

// ── Serialisable types ────────────────────────────────────────────

#[derive(serde::Serialize)]
struct NavItem {
    title: String,
    url: String,
}

#[derive(serde::Serialize)]
struct PostSummary {
    title: String,
    slug: String,
    excerpt: String,
    published_at: String,
}

// ── Helpers ───────────────────────────────────────────────────────

fn load_nav_items(journal: &mut luperiq_forge::ForgeJournal) -> Vec<NavItem> {
    let mgr = luperiq_forge::ForgeMenuManager::new(journal);
    let menu = match mgr.get_menu_by_slug("main-nav") {
        Ok(Some(m)) => m,
        _ => return Vec::new(),
    };
    match mgr.get_menu_items(&menu.menu_id) {
        Ok(items) => items
            .into_iter()
            .filter(|i| i.is_visible)
            .map(|i| NavItem {
                title: i.title,
                url: i.url,
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn base_context(
    journal: &mut luperiq_forge::ForgeJournal,
    theme_css: &str,
    site_name: &str,
    provider: &dyn BlogCmsProvider,
) -> Context {
    let mut ctx = Context::new();
    ctx.insert("site_name", site_name);
    ctx.insert("nav_items", &load_nav_items(journal));
    ctx.insert("theme_css", theme_css);
    ctx.insert("asset_version", provider.asset_version());
    provider.inject_theme_studio(&mut ctx, journal);
    provider.inject_google_tags(&mut ctx, journal);
    ctx
}

pub fn truncate_html(html: &str, max_len: usize) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            text.push(ch);
            if text.len() >= max_len {
                text.push_str("...");
                break;
            }
        }
    }
    text
}

/// Format a Unix timestamp (in **seconds** — NOT milliseconds) as `YYYY-MM-DD`.
///
/// # Note
/// Most platform timestamp fields use milliseconds. This function expects
/// seconds because `ForgeContentManager` returns `published_at` / `created_at`
/// as seconds-since-epoch. Do not pass millisecond values without dividing by
/// 1000 first, or the computed date will be wrong by many decades.
pub fn format_timestamp(ts: u64) -> String {
    let secs = ts as i64;
    let days = secs / 86400;
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0usize;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    format!("{}-{:02}-{:02}", y, m + 1, remaining + 1)
}

// ── Handlers ──────────────────────────────────────────────────────

async fn blog_index(
    State(state): State<BlogState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Response {
    let is_admin = state
        .provider
        .is_admin_session(
            jar.clone(),
            state.journal.clone(),
            (*state.jwt_secret).clone(),
        )
        .await;

    let mut journal = state.journal.lock().await;

    let mgr = luperiq_forge::ForgeContentManager::new(&mut journal);
    let posts: Vec<PostSummary> =
        match mgr.list_content(Some("post"), Some("published"), None, 50, 0, None, None) {
            Ok((items, _total)) => items
                .into_iter()
                .map(|p| {
                    let excerpt = p.excerpt.clone().unwrap_or_else(|| {
                        truncate_html(&state.provider.body_to_html(&p.body_json), 160)
                    });
                    let ts = p.published_at.unwrap_or(p.created_at);
                    PostSummary {
                        title: p.title,
                        slug: p.slug,
                        excerpt,
                        published_at: format_timestamp(ts),
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        };

    let mut ctx = base_context(
        &mut journal,
        &state.theme_css,
        &state.site_name,
        &*state.provider,
    );
    let base_url = state.provider.effective_public_base_url(&headers);
    let canonical = format!("{}/blog", base_url.trim_end_matches('/'));
    let normalized_base_url = base_url.trim_end_matches('/').to_ascii_lowercase();
    // REVIEW: `is_luperiq_apex` is an apex-site detection hack baked into a
    // "universal" module. The hostname/site_name comparison ties this engine to
    // a specific deployment identity. The correct fix is to add
    // `fn is_apex_site(&self) -> bool` to `BlogCmsProvider` and have the host
    // return the value — keeping this engine truly deployment-agnostic.
    // Until that trait change lands, leave the logic unchanged but flagged.
    let is_luperiq_apex = normalized_base_url == "https://luperiq.com"
        || normalized_base_url == "http://luperiq.com"
        || state.site_name.eq_ignore_ascii_case("LuperIQ");
    ctx.insert("page_title", "Blog");
    ctx.insert(
        "seo_description",
        "Read the latest posts on CMS architecture, AI workflows, launches, and growth.",
    );
    ctx.insert("seo_canonical", &canonical);
    ctx.insert("is_luperiq_apex", &is_luperiq_apex);
    ctx.insert("posts", &posts);
    ctx.insert("is_admin", &is_admin);

    // Sidebar data: recent posts (up to 5), post count, site description
    let recent: Vec<&PostSummary> = posts.iter().take(5).collect();
    ctx.insert("recent_posts", &recent);
    ctx.insert("post_count", &posts.len());

    // Load site description from company profile if available.
    // NOTE: "CompProf:Profile" is the aggregate key owned by the company-profile
    // engine. If that key is ever renamed this lookup silently returns empty.
    // TODO: import a shared const (e.g. AGG_COMPANY_PROFILE) once the owning
    // crate exposes one, to eliminate this raw-string coupling.
    let site_desc = journal
        .get_latest("CompProf:Profile", "singleton")
        .and_then(|e| serde_json::from_slice::<serde_json::Value>(&e.payload).ok())
        .and_then(|v| v.get("tagline").and_then(|t| t.as_str().map(String::from)))
        .unwrap_or_default();
    ctx.insert("site_description", &site_desc);

    if is_admin {
        let toolbar = state
            .provider
            .admin_toolbar_html(None, None, None, None, None, "");
        ctx.insert("admin_toolbar", &toolbar);
    }

    match state.tera.render("blog/index.html", &ctx) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("[blog] Tera render error for blog/index.html: {e:?}");
            Html(format!(
                r#"<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Blog — {name}</title>
<style>body{{font-family:system-ui,-apple-system,sans-serif;margin:0;background:#fafafa;color:#1a1a2e;}}
.blog-fb{{max-width:700px;margin:80px auto;padding:0 24px;text-align:center;}}
.blog-fb h1{{font-size:2rem;font-weight:800;margin-bottom:12px;}}
.blog-fb p{{color:#64748b;font-size:1.05rem;line-height:1.6;}}
.blog-fb .cta{{display:inline-block;margin-top:24px;padding:14px 32px;background:#2563eb;color:#fff;border-radius:10px;text-decoration:none;font-weight:700;}}
.blog-fb .pencil{{font-size:3rem;margin-bottom:16px;}}
</style></head><body>
<div class="blog-fb">
<div class="pencil">&#9998;</div>
<h1>Blog</h1>
<p>New posts are on the way. Check back soon!</p>
<a href="/" class="cta">Back to Home</a>
</div>
</body></html>"#,
                name = state.site_name
            ))
            .into_response()
        }
    }
}

async fn blog_single(
    State(state): State<BlogState>,
    jar: CookieJar,
    headers: HeaderMap,
    Path(slug): Path<String>,
) -> Response {
    let is_admin = state
        .provider
        .is_admin_session(
            jar.clone(),
            state.journal.clone(),
            (*state.jwt_secret).clone(),
        )
        .await;

    let mut journal = state.journal.lock().await;

    let mgr = luperiq_forge::ForgeContentManager::new(&mut journal);
    if let Ok((posts, _)) =
        mgr.list_content(Some("post"), Some("published"), None, 100, 0, None, None)
    {
        for post in &posts {
            if post.slug == slug {
                let mut ctx = base_context(
                    &mut journal,
                    &state.theme_css,
                    &state.site_name,
                    &*state.provider,
                );
                ctx.insert("page_title", &post.title);
                ctx.insert("page_body", &state.provider.body_to_html(&post.body_json));
                ctx.insert("page_slug", &post.slug);
                if let Some(ref excerpt) = post.excerpt {
                    ctx.insert("page_excerpt", excerpt);
                }
                ctx.insert("content_id", &post.content_id);
                let ts = post.published_at.unwrap_or(post.created_at);
                ctx.insert("published_at", &format_timestamp(ts));
                let canonical = format!(
                    "{}/blog/{}/",
                    state
                        .provider
                        .effective_public_base_url(&headers)
                        .trim_end_matches('/'),
                    post.slug
                );
                ctx.insert("seo_canonical", &canonical);
                ctx.insert("is_admin", &is_admin);

                if is_admin {
                    let seo_meta = state
                        .provider
                        .lookup_seo_meta(&mut journal, &post.content_id);
                    let toolbar = state.provider.admin_toolbar_html(
                        Some(&post.content_id),
                        seo_meta.as_ref().map(|s| s.title.as_str()),
                        seo_meta.as_ref().map(|s| s.description.as_str()),
                        seo_meta.as_ref().map(|s| s.seo_score),
                        seo_meta.as_ref().map(|s| s.robots.as_str()),
                        "",
                    );
                    ctx.insert("admin_toolbar", &toolbar);
                }

                return match state.tera.render("blog/single.html", &ctx) {
                    Ok(html) => Html(html).into_response(),
                    Err(_) => Html(format!(
                        "<h1>{}</h1><div>{}</div>",
                        post.title,
                        state.provider.body_to_html(&post.body_json)
                    ))
                    .into_response(),
                };
            }
        }
    }

    // 404
    let mut ctx = base_context(
        &mut journal,
        &state.theme_css,
        &state.site_name,
        &*state.provider,
    );
    ctx.insert("requested_slug", &slug);
    match state.tera.render("pages/404.html", &ctx) {
        Ok(html) => (axum::http::StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(_) => (
            axum::http::StatusCode::NOT_FOUND,
            Html(format!("Post not found: {}", slug)),
        )
            .into_response(),
    }
}
