//! Collections domain HTTP handlers (ADR-0024). Thin over `CollectionsService`,
//! mirroring `src/grpc/handlers/collections.rs`.
//!
//! `get_collection_projects` (returning full `Project` values) is exposed here
//! even though the gRPC surface never did -- only the CLI used it. It's
//! possible now because the projects domain's `ProjectDto`/`project_to_dto`
//! already exist; the tags domain's equivalent (`GetProjectsByTag`) had to
//! wait for exactly this, and now it can be revisited too.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::collections::{
    BatchCollectionProjectsRequest, BatchCreateCollectionFromRequest,
    BatchCreateCollectionFromResponse, BatchOperationResponse, CollectionDto,
    CollectionStatisticsDto,
    CollectionListResponse, CollectionTasksResponse, CreateCollectionRequest,
    DuplicateCollectionRequest, ListCollectionsQuery, ReorderCollectionRequest,
    SearchCollectionsQuery, TaskDto, UpdateCollectionRequest,
};
use crate::http::dto::projects::project_to_dto;
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn list_collections(
    State(state): State<AppState>,
    Query(query): Query<ListCollectionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (collections, total_count) = state
        .services
        .collections
        .list_collections(query.limit, query.offset, query.sort_by, query.sort_desc)
        .await?;

    Ok(Json(CollectionListResponse {
        collections: collections.into_iter().map(CollectionDto::from).collect(),
        total_count,
    }))
}

pub async fn search_collections(
    State(state): State<AppState>,
    Query(query): Query<SearchCollectionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (collections, total_count) = state
        .services
        .collections
        .search_collections(&query.query, query.limit, query.offset)
        .await?;

    Ok(Json(CollectionListResponse {
        collections: collections.into_iter().map(CollectionDto::from).collect(),
        total_count,
    }))
}

pub async fn get_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state
        .services
        .collections
        .get_collection(&collection_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Collection {} not found", collection_id)))?;

    Ok(Json(CollectionDto::from(collection)))
}

pub async fn get_collection_projects(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let projects = state
        .services
        .collections
        .get_collection_projects(&collection_id)
        .await?;

    let db_arc = state.services.projects.db_handle();
    let mut db = db_arc.lock().await;
    let projects = projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(projects))
}

pub async fn create_collection(
    State(state): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state
        .services
        .collections
        .create_collection(&req.name, req.description.as_deref(), req.notes.as_deref())
        .await?;

    Ok(Json(CollectionDto::from(collection)))
}

pub async fn update_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<UpdateCollectionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state
        .services
        .collections
        .update_collection(
            &collection_id,
            req.name.as_deref(),
            req.description.as_deref(),
            req.notes.as_deref(),
        )
        .await?;

    Ok(Json(CollectionDto::from(collection)))
}

pub async fn delete_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.services.collections.delete_collection(&collection_id).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn duplicate_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<DuplicateCollectionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let collection = state
        .services
        .collections
        .duplicate_collection(
            &collection_id,
            &req.new_name,
            req.new_description.as_deref(),
            req.new_notes.as_deref(),
        )
        .await?;

    Ok(Json(CollectionDto::from(collection)))
}

pub async fn add_project_to_collection(
    State(state): State<AppState>,
    Path((collection_id, project_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .collections
        .add_project_to_collection(&collection_id, &project_id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn remove_project_from_collection(
    State(state): State<AppState>,
    Path((collection_id, project_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .collections
        .remove_project_from_collection(&collection_id, &project_id)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn reorder_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<ReorderCollectionRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .services
        .collections
        .reorder_collection(&collection_id, &req.project_ids)
        .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn get_collection_tasks(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let tasks_data = state.services.collections.get_collection_tasks(&collection_id).await?;

    let mut tasks = Vec::new();
    let mut completed_count = 0;
    for (id, project_name, description, completed, created_at) in tasks_data {
        if completed {
            completed_count += 1;
        }
        // project_name, not project_id, per the gRPC handler this mirrors: it
        // shows which project the task belongs to rather than an id the caller
        // would have to resolve.
        tasks.push(TaskDto {
            id,
            project_id: project_name,
            description,
            completed,
            created_at,
        });
    }

    let total_tasks = tasks.len() as i32;
    let pending_tasks = total_tasks - completed_count;
    let completion_rate = if total_tasks > 0 {
        completed_count as f64 / total_tasks as f64
    } else {
        0.0
    };

    Ok(Json(CollectionTasksResponse {
        tasks,
        total_tasks,
        completed_tasks: completed_count,
        pending_tasks,
        completion_rate,
    }))
}

pub async fn get_collection_statistics(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .services
        .collections
        .get_collection_statistics(&collection_id)
        .await?;
    Ok(Json(CollectionStatisticsDto::from(stats)))
}

pub async fn batch_add_to_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<BatchCollectionProjectsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .collections
        .batch_add_to_collection(&req.project_ids, &collection_id)
        .await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn batch_remove_from_collection(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<BatchCollectionProjectsRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let results = state
        .services
        .collections
        .batch_remove_from_collection(&req.project_ids, &collection_id)
        .await?;
    Ok(Json(BatchOperationResponse::from_results(results)))
}

pub async fn batch_create_collection_from(
    State(state): State<AppState>,
    Json(req): Json<BatchCreateCollectionFromRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let (collection, results) = state
        .services
        .collections
        .batch_create_collection_from(
            &req.collection_name,
            &req.project_ids,
            req.description.as_deref(),
            req.notes.as_deref(),
        )
        .await?;

    let batch = BatchOperationResponse::from_results(results);
    Ok(Json(BatchCreateCollectionFromResponse {
        collection: collection.map(CollectionDto::from),
        results: batch.results,
        successful_count: batch.successful_count,
        failed_count: batch.failed_count,
    }))
}
