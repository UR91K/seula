//! HTTP route table (ADR-0024).
//!
//! `/health` plus the tags domain (the pattern-proof domain). Remaining CRUD
//! domains and the streaming endpoints are added incrementally, one at a time,
//! per the ADR's implementation order.

use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::handlers::tags as tag_handlers;
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
        .route(
            "/api/v1/tags",
            get(tag_handlers::list_tags).post(tag_handlers::create_tag),
        )
        .route("/api/v1/tags/search", get(tag_handlers::search_tags))
        .route(
            "/api/v1/tags/statistics",
            get(tag_handlers::get_tag_statistics),
        )
        .route(
            "/api/v1/tags/with-usage",
            get(tag_handlers::get_all_tags_with_usage),
        )
        .route(
            "/api/v1/tags/batch-tag",
            post(tag_handlers::batch_tag_projects),
        )
        .route(
            "/api/v1/tags/batch-untag",
            post(tag_handlers::batch_untag_projects),
        )
        .route(
            "/api/v1/tags/:tag_id",
            get(tag_handlers::get_tag)
                .put(tag_handlers::update_tag)
                .delete(tag_handlers::delete_tag),
        )
        .route(
            "/api/v1/projects/:project_id/tags/:tag_id",
            post(tag_handlers::tag_project).delete(tag_handlers::untag_project),
        )
        .layer(cors)
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}
