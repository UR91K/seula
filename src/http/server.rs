//! HTTP route table (ADR-0024).
//!
//! `/health` plus the tags domain (the pattern-proof domain). Remaining CRUD
//! domains and the streaming endpoints are added incrementally, one at a time,
//! per the ADR's implementation order.

use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde_json::json;
use tower_http::cors::{AllowOrigin, CorsLayer};

use super::handlers::projects as project_handlers;
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
        .route(
            "/api/v1/projects",
            get(project_handlers::list_projects),
        )
        .route(
            "/api/v1/projects/statistics",
            get(project_handlers::get_statistics),
        )
        .route(
            "/api/v1/projects/batch-archive",
            post(project_handlers::batch_mark_archived),
        )
        .route(
            "/api/v1/projects/batch-delete",
            post(project_handlers::batch_delete),
        )
        .route(
            "/api/v1/projects/:project_id",
            get(project_handlers::get_project).delete(project_handlers::mark_project_deleted),
        )
        .route(
            "/api/v1/projects/:project_id/permanent",
            axum::routing::delete(project_handlers::permanently_delete_project),
        )
        .route(
            "/api/v1/projects/:project_id/notes",
            put(project_handlers::update_project_notes),
        )
        .route(
            "/api/v1/projects/:project_id/name",
            put(project_handlers::update_project_name),
        )
        .route(
            "/api/v1/projects/:project_id/reactivate",
            post(project_handlers::reactivate_project),
        )
        .route(
            "/api/v1/projects/:project_id/rescan",
            post(project_handlers::rescan_project),
        )
        .layer(cors)
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}
