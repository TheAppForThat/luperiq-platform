//! Session extraction utility for extracted module crates.
//!
//! Provides JWT session validation from request cookies, usable by any
//! module crate that depends on luperiq-module-api.

use crate::SharedJournal;
use axum_extra::extract::cookie::CookieJar;

pub const SESSION_COOKIE: &str = "liq_session";

/// Default session TTL/refresh values.
///
/// Exposed as `pub const` so the host (`luperiq-cms`) can reference the same
/// values when constructing `AuthConfig`, preventing silent drift between the
/// session validation path and the host's configured TTL.
pub const DEFAULT_SESSION_TTL: u64 = 86400;
pub const DEFAULT_REFRESH_WINDOW: u64 = 3600;

/// Extract and validate a JWT session from the request cookies.
/// Returns the session claims if valid, None otherwise.
pub async fn extract_session(
    jar: &CookieJar,
    shared: &SharedJournal,
    jwt_secret: &str,
) -> Option<luperiq_forge::SessionClaims> {
    let token = jar.get(SESSION_COOKIE)?.value().to_string();
    if token.is_empty() {
        return None;
    }

    let mut journal = shared.lock().await;
    let auth_config = luperiq_forge::AuthConfig {
        jwt_secret: jwt_secret.to_string(),
        session_ttl_secs: DEFAULT_SESSION_TTL,
        refresh_window_secs: DEFAULT_REFRESH_WINDOW,
    };
    let auth = luperiq_forge::ForgeAuthManager::new(&mut journal, auth_config).ok()?;
    auth.validate_session(&token).ok()
}
