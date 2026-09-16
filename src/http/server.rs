//! HTTP route table (ADR-0024).
//!
//! Skeleton step: only `/health` exists. Domain routes (tags, projects, ...) are
//! added incrementally in later steps, one domain at a time, per the ADR's
//! implementation order.

use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::state::AppState;

/// Builds the axum router for the HTTP adapter.
///
/// CORS is scoped permissively to localhost origins, per ADR-0024: the only
/// consumers are a Tauri webview and a local browser page, neither of which shares
/// the server's origin, and nothing here is intended to cross a real network.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
            origin
                .to_str()
                .map(|s| {
                    s.starts_with("http://localhost")
                        || s.starts_with("http://127.0.0.1")
                        || s.starts_with("https://localhost")
                        || s.starts_with("https://127.0.0.1")
                        || s.starts_with("tauri://")
                        || s.starts_with("https://tauri.localhost")
                })
                .unwrap_or(false)
        }))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    Router::new()
        .route("/health", get(health))
        .layer(cors)
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}
