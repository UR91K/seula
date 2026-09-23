//! HTTP wire types for the collections domain (ADR-0024).
//!
//! `CollectionStatistics` (`src/models.rs`) is mirrored as `CollectionStatisticsDto`
//! only so that `most_common_key` goes out in the ADR-0035 key shape rather than as
//! the `"<tonic> <scale>"` string the database builds.

use serde::{Deserialize, Serialize};

use crate::http::dto::projects::KeySignatureDto;
use crate::models::CollectionStatistics;
use crate::services::CollectionDetail;

#[derive(Serialize)]
pub struct CollectionStatisticsDto {
    pub project_count: i32,
    pub total_duration_seconds: Option<f64>,
    pub average_tempo: Option<f64>,
    pub total_plugins: i32,
    pub total_samples: i32,
    pub total_tags: i32,
    pub most_common_key: Option<KeySignatureDto>,
    pub most_common_time_signature: Option<String>,
}

impl From<CollectionStatistics> for CollectionStatisticsDto {
    fn from(s: CollectionStatistics) -> Self {
        Self {
            project_count: s.project_count,
            total_duration_seconds: s.total_duration_seconds,
            average_tempo: s.average_tempo,
            total_plugins: s.total_plugins,
            total_samples: s.total_samples,
            total_tags: s.total_tags,
            most_common_key: s.most_common_key.as_deref().and_then(KeySignatureDto::from_joined),
            most_common_time_signature: s.most_common_time_signature,
        }
    }
}

#[derive(Serialize)]
pub struct CollectionDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub project_ids: Vec<String>,
    pub cover_art_id: Option<String>,
    pub total_duration_seconds: Option<f64>,
    pub project_count: i32,
}

impl From<CollectionDetail> for CollectionDto {
    fn from(d: CollectionDetail) -> Self {
        Self {
            id: d.id,
            name: d.name,
            description: d.description,
            notes: d.notes,
            created_at: d.created_at,
            modified_at: d.modified_at,
            project_ids: d.project_ids,
            cover_art_id: d.cover_art_id,
            total_duration_seconds: d.total_duration_seconds,
            project_count: d.project_count,
        }
    }
}

#[derive(Deserialize)]
pub struct ListCollectionsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    /// `name`, `description`, `created_at`, `modified_at`, `project_count` or
    /// `total_duration`; the last two count in `scope`.
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    /// `active` (default) or `all`; see `parse_project_scope` (ADR-0043).
    pub scope: Option<String>,
}

#[derive(Deserialize)]
pub struct SearchCollectionsQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub scope: Option<String>,
}

/// Routes whose only option is the project scope (ADR-0043).
#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

#[derive(Serialize)]
pub struct CollectionListResponse {
    pub collections: Vec<CollectionDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct CreateCollectionRequest {
    pub name: String,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCollectionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Deserialize)]
pub struct DuplicateCollectionRequest {
    pub new_name: String,
    pub new_description: Option<String>,
    pub new_notes: Option<String>,
}

#[derive(Deserialize)]
pub struct ReorderCollectionRequest {
    pub project_ids: Vec<String>,
}

/// A task in a collection's consolidated list. It carries its project's id, so the
/// UI can reach the project, and its name, so it can say which project it is without
/// a lookup. (The gRPC surface put the name in `project_id` instead.)
#[derive(Serialize)]
pub struct TaskDto {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub description: String,
    pub completed: bool,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct CollectionTasksResponse {
    pub tasks: Vec<TaskDto>,
    pub total_tasks: i32,
    pub completed_tasks: i32,
    pub pending_tasks: i32,
    pub completion_rate: f64,
}

#[derive(Deserialize)]
pub struct BatchCollectionProjectsRequest {
    pub project_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct BatchOperationResultDto {
    pub id: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Serialize)]
pub struct BatchOperationResponse {
    pub results: Vec<BatchOperationResultDto>,
    pub successful_count: i32,
    pub failed_count: i32,
}

impl BatchOperationResponse {
    pub fn from_results(results: Vec<(String, Result<(), crate::error::DatabaseError>)>) -> Self {
        let (successful_count, failed_count) = results
            .iter()
            .fold((0, 0), |(s, f), (_, r)| if r.is_ok() { (s + 1, f) } else { (s, f + 1) });

        let results = results
            .into_iter()
            .map(|(id, result)| BatchOperationResultDto {
                id,
                success: result.is_ok(),
                error_message: result.err().map(|e| e.to_string()),
            })
            .collect();

        Self {
            results,
            successful_count,
            failed_count,
        }
    }
}

#[derive(Deserialize)]
pub struct BatchCreateCollectionFromRequest {
    pub collection_name: String,
    pub project_ids: Vec<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct BatchCreateCollectionFromResponse {
    pub collection: Option<CollectionDto>,
    pub results: Vec<BatchOperationResultDto>,
    pub successful_count: i32,
    pub failed_count: i32,
}
