//! HTTP wire types for the samples domain (ADR-0024).
//!
//! `SampleStats`, `SampleUsageInfo`, `SampleRefreshResult`, `SampleAnalytics`
//! and `ExtensionAnalytics` (`src/database/samples.rs`) already derive
//! `Serialize` and are returned as-is, same reasoning as the other domains'
//! reuse of already-serde types.

use serde::{Deserialize, Serialize};

use crate::models::Sample;

/// `project_count` is always populated (ADR-0034). There is no separate usage count:
/// a project either uses a sample or does not, so the two would always agree.
#[derive(Serialize)]
pub struct SampleDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_present: bool,
    pub project_count: i32,
}

impl SampleDto {
    pub fn new(sample: Sample, project_count: i32) -> Self {
        Self {
            id: sample.id.to_string(),
            name: sample.name,
            path: sample.path.to_string_lossy().to_string(),
            is_present: sample.is_present,
            project_count,
        }
    }
}

/// Maps the HTTP sort key onto the database layer's, which still calls it
/// `usage_count` (ADR-0034).
pub fn sample_sort_key(sort_by: Option<String>) -> Option<String> {
    sort_by.map(|s| if s == "project_count" { "usage_count".to_string() } else { s })
}

#[derive(Serialize)]
pub struct SampleListResponse {
    pub samples: Vec<SampleDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct GetAllSamplesQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub present_only: Option<bool>,
    pub missing_only: Option<bool>,
    pub extension_filter: Option<String>,
    pub min_project_count: Option<i32>,
    pub max_project_count: Option<i32>,
}

#[derive(Deserialize)]
pub struct ByPresenceQuery {
    pub is_present: bool,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchSamplesQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub present_only: Option<bool>,
    pub extension_filter: Option<String>,
}

#[derive(Deserialize)]
pub struct ProjectsBySampleQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
