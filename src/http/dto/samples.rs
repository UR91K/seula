//! HTTP wire types for the samples domain (ADR-0024).
//!
//! `SampleStats`, `SampleUsageInfo`, `SampleRefreshResult`, `SampleAnalytics`
//! and `ExtensionAnalytics` (`src/database/samples.rs`) already derive
//! `Serialize` and are returned as-is, same reasoning as the other domains'
//! reuse of already-serde types.

use serde::{Deserialize, Serialize};

use crate::models::Sample;

#[derive(Serialize)]
pub struct SampleDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_present: bool,
}

impl From<Sample> for SampleDto {
    fn from(sample: Sample) -> Self {
        Self {
            id: sample.id.to_string(),
            name: sample.name,
            path: sample.path.to_string_lossy().to_string(),
            is_present: sample.is_present,
        }
    }
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
    pub min_usage_count: Option<i32>,
    pub max_usage_count: Option<i32>,
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
