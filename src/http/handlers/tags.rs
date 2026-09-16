//! Tags domain HTTP handlers (ADR-0024) -- the pattern-proof domain. Thin in the
//! sense ADR-0018 specified: parse the request, call one `Services` method,
//! convert the result. No business logic, no database access.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::dto::tags::{
    BatchOperationResponse, BatchTagRequest, CreateTagRequest, ProjectsByTagQuery, SearchQuery,
    TagDto, TagListResponse, TagSearchResponse, TagsWithUsageQuery, UpdateTagRequest,
};
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn list_tags(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tags = state
        .services
        .tags
        .list_tags()
        .await?
        .into_iter()
        .map(TagDto::from)
        .collect();

    Ok(Json(TagListResponse { tags }))
}

pub async fn create_tag(
    State(state): State<AppState>,
    Json(req): Json<CreateTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let tag = state.services.tags.create_tag(&req.name).await?;
    Ok(Json(TagDto::from(tag)))
}

pub async fn get_tag(
    State(state): State<AppState>,
    Path(tag_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let tag = state
        .services
        .tags
        .get_tag(&tag_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Tag {} not found", tag_id)))?;

    Ok(Json(TagDto::from(tag)))
}

pub async fn update_tag(
    State(state): State<AppState>,
    Path(tag_id): Path<String>,
    Json(req): Json<UpdateTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let tag = state.services.tags.update_tag(&tag_id, &req.name).await?;
    Ok(Json(TagDto::from(tag)))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    Path(tag_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.tags.delete_tag(&tag_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn search_tags(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (tags, total_count) = state
        .services
        .tags
        .search_tags(&query.query, query.limit, query.offset)
        .await?;

    Ok(Json(TagSearchResponse {
        tags: tags.into_iter().map(TagDto::from).collect(),
        total_count,
    }))
}

pub async fn get_tag_statistics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.services.tags.get_tag_statistics().await?;
    Ok(Json(stats))
}

pub async fn get_all_tags_with_usage(
    State(state): State<AppState>,
    Query(query): Query<TagsWithUsageQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (tags, total_count) = state
        .services
        .tags
        .get_all_tags_with_usage(
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
            query.min_usage_count,
        )
        .await?;

    Ok(Json(crate::http::dto::tags::TagUsageListResponse {
        tags,
        total_count,
    }))
}

/// Mirrors `src/grpc/handlers/tags.rs::get_projects_by_tag`: the service
/// returns every tagged project, and pagination is applied here rather than in
/// SQL, matching the existing (gRPC) behavior rather than changing it as part
/// of this port.
pub async fn get_projects_by_tag(
    State(state): State<AppState>,
    Path(tag_id): Path<String>,
    Query(query): Query<ProjectsByTagQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let projects = state.services.tags.get_projects_by_tag(&tag_id).await?;
    let total_count = projects.len() as i32;

    let db_arc = state.services.tags.db_handle();
    let mut db = db_arc.lock().await;
    let projects = projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect::<Result<Vec<_>, _>>()?;
    drop(db);

    let offset = query.offset.unwrap_or(0) as usize;
    let projects = if let Some(limit) = query.limit {
        projects.into_iter().skip(offset).take(limit as usize).collect()
    } else {
        projects.into_iter().skip(offset).collect()
    };

    Ok(Json(ProjectListResponse { projects, total_count }))
}

pub async fn tag_project(
    State(state): State<AppState>,
    Path((project_id, tag_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let tag = state.services.tags.tag_project(&project_id, &tag_id).await?;
    Ok(Json(TagDto::from(tag)))
}

pub async fn untag_project(
    State(state): State<AppState>,
    Path((project_id, tag_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.tags.untag_project(&project_id, &tag_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn batch_tag_projects(
    State(state): State<AppState>,
    Json(req): Json<BatchTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .tags
        .batch_tag_projects(&req.project_ids, &req.tag_ids)
        .await?;

    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn batch_untag_projects(
    State(state): State<AppState>,
    Json(req): Json<BatchTagRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .tags
        .batch_untag_projects(&req.project_ids, &req.tag_ids)
        .await?;

    Ok(Json(BatchOperationResponse::from_results(results)))
}
