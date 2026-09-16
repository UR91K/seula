//! Tasks domain HTTP handlers (ADR-0024). Thin over `TasksService`, mirroring
//! `src/grpc/handlers/tasks.rs`.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::tasks::{
    BatchOperationResponse, BatchTaskIdsRequest, BatchUpdateTaskStatusRequest,
    CreateTaskRequest, ProjectTasksResponse, SearchTasksQuery, TaskDto, TaskSearchResponse,
    TaskStatisticsDto, TaskStatisticsQuery, UpdateTaskRequest,
};
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn get_project_tasks(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let task_data = state.services.tasks.get_project_tasks(&project_id).await?;
    let tasks = task_data
        .into_iter()
        .map(|row| TaskDto::from_project_row(&project_id, row))
        .collect();

    Ok(Json(ProjectTasksResponse { tasks }))
}

pub async fn create_task(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let task = state.services.tasks.create_task(&project_id, &req.description).await?;
    Ok(Json(TaskDto::from(task)))
}

pub async fn update_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let task = state
        .services
        .tasks
        .update_task(&task_id, req.description.as_deref(), req.completed)
        .await?;
    Ok(Json(TaskDto::from(task)))
}

pub async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.tasks.delete_task(&task_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn batch_update_task_status(
    State(state): State<AppState>,
    Json(req): Json<BatchUpdateTaskStatusRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .tasks
        .batch_update_task_status(&req.task_ids, req.completed)
        .await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn batch_delete_tasks(
    State(state): State<AppState>,
    Json(req): Json<BatchTaskIdsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state.services.tasks.batch_delete_tasks(&req.task_ids).await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn search_tasks(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<SearchTasksQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (task_data, total_count) = state
        .services
        .tasks
        .search_tasks(
            &project_id,
            &query.query,
            query.limit,
            query.offset,
            query.completed_only,
            query.pending_only,
        )
        .await?;

    let tasks = task_data
        .into_iter()
        .map(|row| TaskDto::from_project_row(&project_id, row))
        .collect();

    Ok(Json(TaskSearchResponse { tasks, total_count }))
}

pub async fn get_task_statistics(
    State(state): State<AppState>,
    Query(query): Query<TaskStatisticsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .services
        .tasks
        .get_task_statistics(query.project_id.as_deref())
        .await?;
    Ok(Json(TaskStatisticsDto::from(stats)))
}
