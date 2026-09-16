//! Tags domain HTTP handlers (ADR-0024) -- the pattern-proof domain. Thin in the
//! sense ADR-0018 specified: parse the request, call one `Services` method,
//! convert the result. No business logic, no database access.
//!
//! `GetProjectsByTag` (the gRPC handlers' name) is deliberately not ported yet:
//! it returns full `Project` values, and there is no HTTP DTO for projects until
//! that domain's own pass. Porting it here would mean inventing a throwaway
//! shape ahead of that work.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::tags::{
    BatchOperationResponse, BatchTagRequest, CreateTagRequest, SearchQuery, TagDto,
    TagListResponse, TagSearchResponse, TagsWithUsageQuery, UpdateTagRequest,
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
