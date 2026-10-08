//! The library statistics `SystemService::get_statistics` assembles (ADR-0045).
//!
//! These used to be the gRPC-generated `GetStatisticsResponse` and its nested messages,
//! and carried the embedded projects already converted to proto. They hold the domain
//! `Project`s now; each adapter converts them its own way.

use super::CollectionDetail;
use crate::database::stats::LibraryCounts;
use crate::project::Project;

/// Every figure counts the projects in the scope it was asked for and what they use.
#[derive(Debug, Default)]
pub struct Statistics {
    // The overview figures. The flat totals are the sums of `counts`, except
    // `total_projects`, which counts the projects in scope.
    pub total_projects: i32,
    pub total_plugins: i32,
    pub total_samples: i32,
    pub total_collections: i32,
    pub total_tags: i32,
    pub total_tasks: i32,
    /// The overview figures split into their states.
    pub counts: LibraryCounts,

    pub top_plugins: Vec<PluginStatistic>,
    pub top_vendors: Vec<VendorStatistic>,

    pub tempo_distribution: Vec<TempoStatistic>,
    pub key_distribution: Vec<KeyStatistic>,
    pub time_signature_distribution: Vec<TimeSignatureStatistic>,

    pub projects_per_year: Vec<YearStatistic>,
    pub projects_per_month: Vec<MonthStatistic>,
    pub average_monthly_projects: f64,

    pub average_project_duration_seconds: f64,
    pub projects_under_40_seconds: i32,
    pub longest_project: Option<Project>,

    pub most_complex_projects: Vec<ProjectComplexity>,
    pub average_plugins_per_project: f64,
    pub average_samples_per_project: f64,

    pub top_samples: Vec<SampleStatistic>,
    pub top_tags: Vec<TagStatistic>,

    pub completed_tasks: i32,
    pub pending_tasks: i32,
    /// 0 to 1, like the trends' rates.
    pub task_completion_rate: f64,

    pub recent_activity: Vec<ActivityTrend>,
    pub ableton_versions: Vec<VersionStatistic>,

    pub average_projects_per_collection: f64,
    pub largest_collection: Option<CollectionDetail>,

    pub task_completion_trends: Vec<TaskCompletionTrend>,
}

#[derive(Debug)]
pub struct PluginStatistic {
    pub name: String,
    pub vendor: String,
    pub usage_count: i32,
}

#[derive(Debug)]
pub struct VendorStatistic {
    pub vendor: String,
    pub plugin_count: i32,
    pub usage_count: i32,
}

#[derive(Debug)]
pub struct TempoStatistic {
    pub tempo: f64,
    pub count: i32,
}

#[derive(Debug)]
pub struct KeyStatistic {
    /// `"<tonic> <scale>"`; absent for projects with no key.
    pub key: Option<String>,
    pub count: i32,
}

#[derive(Debug)]
pub struct TimeSignatureStatistic {
    pub numerator: i32,
    pub denominator: i32,
    pub count: i32,
}

#[derive(Debug)]
pub struct YearStatistic {
    pub year: i32,
    pub count: i32,
}

#[derive(Debug)]
pub struct MonthStatistic {
    pub year: i32,
    pub month: i32,
    pub count: i32,
}

#[derive(Debug)]
pub struct ProjectComplexity {
    pub project: Project,
    pub plugin_count: i32,
    pub sample_count: i32,
    pub complexity_score: i32,
}

#[derive(Debug)]
pub struct SampleStatistic {
    pub name: String,
    pub path: String,
    pub usage_count: i32,
}

#[derive(Debug)]
pub struct TagStatistic {
    pub name: String,
    pub usage_count: i32,
}

#[derive(Debug)]
pub struct ActivityTrend {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub projects_created: i32,
    pub projects_modified: i32,
}

#[derive(Debug)]
pub struct VersionStatistic {
    pub version: String,
    pub count: i32,
}

#[derive(Debug)]
pub struct TaskCompletionTrend {
    pub year: i32,
    pub month: i32,
    pub completed_tasks: i32,
    pub total_tasks: i32,
    pub completion_rate: f64,
}
