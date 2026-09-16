//! Projects domain HTTP handlers (ADR-0024). Thin over `ProjectsService`,
//! mirroring `src/grpc/handlers/projects.rs`.
//!
//! `GetProjectsByDeletionStatus` (the gRPC handlers' name) is deliberately not
//! given its own route: it is a strict subset of `list_projects` (deletion
//! status with no other filters), and ADR-0024 says the HTTP surface is not
//! required to mirror the gRPC method names. `GET /api/v1/projects?scope=deleted`
//! covers it.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::error::DatabaseError;
use crate::http::dto::projects::{
    project_to_dto, BatchArchiveRequest, BatchOperationResponse, BatchProjectIdsRequest,
    ListProjectsQuery, ProjectListResponse, ProjectStatisticsDto, RescanRequest, RescanResponse,
    StatisticsQuery, UpdateNameRequest, UpdateNotesRequest,
};
use crate::http::error::ApiError;
use crate::http::state::AppState;
use crate::services::DeletionScope;
use crate::Project;

fn parse_scope(scope: Option<&str>) -> DeletionScope {
    match scope {
        Some("deleted") => DeletionScope::DeletedOnly,
        Some("all") => DeletionScope::All,
        _ => DeletionScope::ActiveOnly,
    }
}

async fn projects_to_dtos(
    state: &AppState,
    projects: Vec<Project>,
) -> Result<Vec<crate::http::dto::projects::ProjectDto>, DatabaseError> {
    let db_arc = state.services.projects.db_handle();
    let mut db = db_arc.lock().await;
    projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect()
}

pub async fn list_projects(
    State(state): State<AppState>,
    Query(query): Query<ListProjectsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = parse_scope(query.scope.as_deref());

    let (projects, total_count) = state
        .services
        .projects
        .list_projects(
            scope,
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
            query.min_tempo,
            query.max_tempo,
            query.key_signature_tonic,
            query.key_signature_scale,
            query.time_signature_numerator,
            query.time_signature_denominator,
            query.ableton_version_major,
            query.ableton_version_minor,
            query.ableton_version_patch,
            query.created_after,
            query.created_before,
            query.modified_after,
            query.modified_before,
            query.has_audio_file,
        )
        .await?;

    let projects = projects_to_dtos(&state, projects).await?;
    Ok(Json(ProjectListResponse { projects, total_count }))
}

pub async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let project = state
        .services
        .projects
        .get_project(&project_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Project {} not found", project_id)))?;

    let dto = projects_to_dtos(&state, vec![project]).await?.remove(0);
    Ok(Json(dto))
}

pub async fn update_project_notes(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateNotesRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .projects
        .update_project(&project_id, None, Some(&req.notes))
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_project_name(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<UpdateNameRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .projects
        .update_project(&project_id, Some(&req.name), None)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn mark_project_deleted(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.projects.mark_deleted(&project_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn reactivate_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.projects.reactivate(&project_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn permanently_delete_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    match state.services.projects.permanently_delete(&project_id).await {
        Ok(()) => Ok(axum::http::StatusCode::NO_CONTENT),
        Err(DatabaseError::InvalidOperation(msg))
            if msg == "Cannot permanently delete an active project" =>
        {
            Err(ApiError::InvalidRequest(msg))
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn batch_mark_archived(
    State(state): State<AppState>,
    Json(req): Json<BatchArchiveRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .projects
        .batch_mark_archived(&req.project_ids, req.archived)
        .await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn batch_delete(
    State(state): State<AppState>,
    Json(req): Json<BatchProjectIdsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state.services.projects.batch_delete(&req.project_ids).await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn get_statistics(
    State(state): State<AppState>,
    Query(query): Query<StatisticsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .services
        .projects
        .get_statistics(
            query.min_tempo,
            query.max_tempo,
            query.key_signature_tonic,
            query.key_signature_scale,
            query.time_signature_numerator,
            query.time_signature_denominator,
            query.ableton_version_major,
            query.ableton_version_minor,
            query.ableton_version_patch,
            query.created_after,
            query.created_before,
            query.has_audio_file,
        )
        .await?;

    Ok(Json(ProjectStatisticsDto::from(stats)))
}

pub async fn rescan_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<RescanRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .services
        .projects
        .rescan(&project_id, req.force_rescan.unwrap_or(false))
        .await?;

    let updated_project = match result.updated_project {
        Some(project) => Some(projects_to_dtos(&state, vec![project]).await?.remove(0)),
        None => None,
    };

    Ok(Json(RescanResponse {
        success: result.success,
        was_updated: result.was_updated,
        scan_summary: result.scan_summary,
        error_message: result.error_message,
        updated_project,
    }))
}
