//! `DirectoryHostGate` — a tower `Layer`/`Service` that 404s any request whose
//! `Host` header is not in the canonical allowlist.
//!
//! Phase 1 (directory hardening): the public pest-control directory must only be
//! served on `pestcontroller.org`. The same apex binary serves many hosts off one
//! router (host-global routes), so without this gate the `/directory/*` pages would
//! leak onto every brand. This layer wraps `directory_routes(..)` at merge time and
//! short-circuits non-canonical hosts with an empty `404` before any handler runs.
//!
//! Behaviour-neutral for legitimate traffic: `Host: pestcontroller.org` (with or
//! without an explicit `:port`, case-insensitive) passes straight through.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use tower_layer::Layer;
use tower_service::Service;

/// Canonical hosts allowed to serve the directory.
const ALLOWED_HOSTS: &[&str] = &["pestcontroller.org", "www.pestcontroller.org"];

/// Returns true if `host` (a raw `Host` header value) is canonical.
/// Strips any `:port` suffix and compares case-insensitively.
fn host_allowed(host: &str) -> bool {
    let bare = host.split(':').next().unwrap_or(host).trim();
    ALLOWED_HOSTS.iter().any(|h| bare.eq_ignore_ascii_case(h))
}

/// Layer that wraps an inner service with [`DirectoryHostGateService`].
#[derive(Clone, Copy, Default)]
pub struct DirectoryHostGate;

impl DirectoryHostGate {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for DirectoryHostGate {
    type Service = DirectoryHostGateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        DirectoryHostGateService { inner }
    }
}

/// Service produced by [`DirectoryHostGate`].
#[derive(Clone)]
pub struct DirectoryHostGateService<S> {
    inner: S,
}

impl<S> Service<Request<Body>> for DirectoryHostGateService<S>
where
    S: Service<Request<Body>, Response = Response<Body>> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let ok = req
            .headers()
            .get(axum::http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .map(host_allowed)
            .unwrap_or(false);

        if ok {
            let fut = self.inner.call(req);
            Box::pin(fut)
        } else {
            Box::pin(async move {
                let resp = Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .expect("static 404 response builds");
                Ok(resp)
            })
        }
    }
}
