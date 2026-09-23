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
    /// As the last sample check measured it (ADR-0041); `null` if no check has found
    /// the file. A missing sample keeps the size it last had.
    pub size_bytes: Option<i64>,
}

impl SampleDto {
    pub fn new(sample: Sample, project_count: i32, size_bytes: Option<i64>) -> Self {
        Self {
            id: sample.id.to_string(),
            name: sample.name,
            path: sample.path.to_string_lossy().to_string(),
            is_present: sample.is_present,
            project_count,
            size_bytes,
        }
    }
}

/// The list and search filters, so the status bar counts what the list shows. All
/// optional; none given counts every sample.
#[derive(Deserialize)]
pub struct SampleStatsQuery {
    pub query: Option<String>,
    pub format_filter: Option<String>,
    pub present_only: Option<bool>,
    pub missing_only: Option<bool>,
}

/// One event of the sample check stream (ADR-0041): the same progress fields as the
/// other scans, and on the final `completed` event, what the check changed.
#[derive(Serialize)]
pub struct SampleCheckEventDto {
    #[serde(flatten)]
    pub progress: crate::http::dto::system::ScanProgressDto,
    pub result: Option<crate::database::samples::SampleRefreshResult>,
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
    /// A format id (`aiff`), one of its extensions (`aif`), or `other`.
    pub format_filter: Option<String>,
    pub min_project_count: Option<i32>,
    pub max_project_count: Option<i32>,
    /// `active` (default) or `all`; see `parse_project_scope`.
    pub scope: Option<String>,
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
    pub format_filter: Option<String>,
    pub scope: Option<String>,
}

/// Routes whose only option is the project scope.
#[derive(Deserialize)]
pub struct ScopeQuery {
    pub scope: Option<String>,
}

/// One sample format and its counts (`GET /api/v1/samples/formats`). Every known
/// format is listed, in a fixed order, even when no sample has it; `other` is listed
/// only when some sample has it.
#[derive(Serialize)]
pub struct SampleFormatDto {
    pub format: String,
    pub name: String,
    pub extensions: Vec<String>,
    pub count: i32,
    pub present_count: i32,
    pub missing_count: i32,
    pub total_size_bytes: i64,
}

#[derive(Serialize)]
pub struct SampleFormatListResponse {
    pub formats: Vec<SampleFormatDto>,
}

#[derive(Deserialize)]
pub struct ProjectsBySampleQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    /// `all` includes archived projects, each marked by its `is_active`.
    pub scope: Option<String>,
}
