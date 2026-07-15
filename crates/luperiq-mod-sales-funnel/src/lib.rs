//! Platform-level sales funnel for LuperIQ's SaaS/trial growth loop.
//! Handles free trial signup, trial lifecycle, lead tracking with UTM
//! attribution, lifetime entitlement detection, trial reminder emails,
//! demo site banner generation, business enrichment, and promo codes.
//! See [`sales_funnel`] for the full implementation.
pub mod sales_funnel;
pub use sales_funnel::SalesFunnelModule;

/// Public routes that do not require authentication.
/// Mount separately in main.rs without auth middleware.
pub fn public_routes() -> axum::Router {
    axum::Router::new()
        .route(
            "/api/enrich-business",
            axum::routing::post(sales_funnel::enrich::enrich_handler),
        )
        .route(
            "/api/modules/sales-funnel/promo/validate",
            axum::routing::post(sales_funnel::promo::validate_handler),
        )
}
