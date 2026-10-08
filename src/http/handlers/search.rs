//! Search domain HTTP handlers (ADR-0024). Thin over `SearchService`. A gap: `SearchResult`
//! carries a relevance `rank` and `match_reason`, and this surface does not return
//! them.

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::dto::search::SearchQuery;
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (results, total_count) = state
        .services
        .search
        .search(&query.query, query.limit, query.offset)
        .await?;

    let db_arc = state.services.search.db_handle();
    let mut db = db_arc.lock().await;
    let projects = results
        .into_iter()
        .map(|r| project_to_dto(r.project, &mut db))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ProjectListResponse {
        projects,
        total_count,
    }))
}
