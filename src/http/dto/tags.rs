//! HTTP wire types for the tags domain (ADR-0024).
//!
//! `TagStatistics` and `TagUsageInfo` (`src/database/tags.rs`) already derive
//! `Serialize` and are returned as-is: ADR-0024's DTO-independence rule is about
//! not reusing the *generated proto* types, not about wrapping every internal
//! type that happens to already be serde-friendly.

use serde::{Deserialize, Serialize};

use crate::database::tags::TagUsageInfo;
use crate::services::tags::TagRow;

#[derive(Serialize)]
pub struct TagDto {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

impl From<TagRow> for TagDto {
    fn from((id, name, created_at): TagRow) -> Self {
        Self {
            id,
            name,
            created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTagRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateTagRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct TagListResponse {
    pub tags: Vec<TagDto>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Deserialize)]
pub struct ProjectsByTagQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Serialize)]
pub struct TagSearchResponse {
    pub tags: Vec<TagDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct TagsWithUsageQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub min_usage_count: Option<i32>,
}

#[derive(Serialize)]
pub struct TagUsageListResponse {
    pub tags: Vec<TagUsageInfo>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct BatchTagRequest {
    pub project_ids: Vec<String>,
    pub tag_ids: Vec<String>,
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
