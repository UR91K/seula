//! HTTP wire types for the system domain (ADR-0024).
//!
//! `SystemService::get_statistics` returns the generated proto
//! `GetStatisticsResponse` directly (see that method's own doc comment: the
//! data is proto-shaped by nature and had no other consumer). ADR-0024's
//! rejected alternatives rule out deriving serde on generated types and
//! reusing them as the HTTP wire format, so everything below is a hand-written
//! mirror of that message and its nested types, built by converting the proto
//! response field by field -- including the two embedded `common::Project`s
//! and the embedded `common::Collection`, converted from their already-proto
//! form rather than from the domain type (the service has already done that
//! conversion once by the time HTTP sees it).

use serde::{Deserialize, Serialize};

use crate::grpc::common::Project as ProtoProject;
use crate::grpc::system as proto;
use crate::http::dto::collections::CollectionDto;
use crate::http::dto::projects::{
    AbletonVersionDto, KeySignatureDto, PluginDto, ProjectDto, SampleDto, TaskDto,
    TimeSignatureDto,
};
use crate::http::dto::tags::TagDto;

fn proto_project_to_dto(p: ProtoProject) -> ProjectDto {
    ProjectDto {
        id: p.id,
        // The proto Project has no active flag. The statistics these come from are
        // computed over active projects only.
        is_active: true,
        name: p.name,
        path: p.path,
        hash: p.hash,
        notes: p.notes,
        created_at: p.created_at,
        modified_at: p.modified_at,
        last_parsed_at: p.last_parsed_at,
        tempo: p.tempo,
        time_signature: p
            .time_signature
            .map(|ts| TimeSignatureDto {
                numerator: ts.numerator,
                denominator: ts.denominator,
            })
            .unwrap_or(TimeSignatureDto {
                numerator: 0,
                denominator: 0,
            }),
        key_signature: p
            .key_signature
            .and_then(|ks| KeySignatureDto::from_names(&ks.tonic, &ks.scale)),
        duration_seconds: p.duration_seconds,
        furthest_bar: p.furthest_bar,
        ableton_version: p
            .ableton_version
            .map(|v| AbletonVersionDto {
                major: v.major,
                minor: v.minor,
                patch: v.patch,
                beta: v.beta,
            })
            .unwrap_or(AbletonVersionDto {
                major: 0,
                minor: 0,
                patch: 0,
                beta: false,
            }),
        plugins: p
            .plugins
            .into_iter()
            .map(|pl| PluginDto {
                id: pl.id,
                dev_identifier: pl.dev_identifier,
                name: pl.name,
                format: pl.format,
                installed: pl.installed,
                vendor: pl.vendor,
                version: pl.version,
            })
            .collect(),
        samples: p
            .samples
            .into_iter()
            .map(|s| SampleDto {
                id: s.id,
                name: s.name,
                path: s.path,
                is_present: s.is_present,
            })
            .collect(),
        tags: p
            .tags
            .into_iter()
            .map(|t| TagDto::from((t.id, t.name, t.created_at)))
            .collect(),
        tasks: p
            .tasks
            .into_iter()
            .map(|t| TaskDto {
                id: t.id,
                project_id: t.project_id,
                description: t.description,
                completed: t.completed,
                created_at: t.created_at,
            })
            .collect(),
        collection_ids: p.collection_ids,
        audio_file_id: p.audio_file_id,
        // The proto Project carries only the primary (gRPC is not extended, ADR-0028/0037),
        // so the system statistics' embedded projects have no audio list.
        audio_files: Vec::new(),
    }
}

fn proto_collection_to_dto(c: crate::grpc::common::Collection) -> CollectionDto {
    CollectionDto {
        id: c.id,
        name: c.name,
        description: c.description,
        notes: c.notes,
        created_at: c.created_at,
        modified_at: c.modified_at,
        project_ids: c.project_ids,
        cover_art_id: c.cover_art_id,
        total_duration_seconds: c.total_duration_seconds,
        project_count: c.project_count,
    }
}

#[derive(Serialize)]
pub struct SystemInfoResponse {
    pub version: String,
    pub watch_paths: Vec<String>,
    pub watcher_active: bool,
    pub uptime_seconds: i64,
}

#[derive(Serialize)]
pub struct ScanStatusResponse {
    pub status: String,
    pub current_progress: Option<ScanProgressDto>,
}

#[derive(Serialize, Clone)]
pub struct ScanProgressDto {
    pub completed: u32,
    pub total: u32,
    pub progress: f32,
    pub message: String,
    pub status: String,
}

pub fn scan_status_name(status: crate::grpc::common::ScanStatus) -> String {
    use crate::grpc::common::ScanStatus::*;
    match status {
        ScanUnknown => "unknown",
        ScanStarting => "starting",
        ScanScanningPlugins => "scanning_plugins",
        ScanDiscovering => "discovering",
        ScanParsing => "parsing",
        ScanInserting => "inserting",
        ScanCompleted => "completed",
        ScanError => "error",
    }
    .to_string()
}

impl From<crate::grpc::scanning::ScanProgressResponse> for ScanProgressDto {
    fn from(p: crate::grpc::scanning::ScanProgressResponse) -> Self {
        let status = crate::grpc::common::ScanStatus::try_from(p.status)
            .unwrap_or(crate::grpc::common::ScanStatus::ScanUnknown);
        Self {
            completed: p.completed,
            total: p.total,
            progress: p.progress,
            message: p.message,
            status: scan_status_name(status),
        }
    }
}

#[derive(Serialize)]
pub struct WatcherEventDto {
    pub event_type: String,
    pub path: String,
    pub new_path: Option<String>,
    pub timestamp: i64,
}

#[derive(Serialize)]
pub struct AddProjectResponse {
    pub success: bool,
    pub project: Option<ProjectDto>,
    pub error_message: Option<String>,
}

#[derive(Deserialize)]
pub struct AddSingleProjectRequest {
    pub file_path: String,
}

#[derive(Deserialize)]
pub struct AddMultipleProjectsRequest {
    pub file_paths: Vec<String>,
}

#[derive(Serialize)]
pub struct AddMultipleProjectsResponse {
    pub success: bool,
    pub projects: Vec<ProjectDto>,
    pub failed_paths: Vec<String>,
    pub error_messages: Vec<String>,
    pub total_requested: i32,
    pub successful_imports: i32,
    pub failed_imports: i32,
}

#[derive(Serialize)]
pub struct WatcherActionResponse {
    pub success: bool,
}

#[derive(Serialize)]
pub struct PluginStatisticDto {
    pub name: String,
    pub vendor: String,
    pub usage_count: i32,
}

#[derive(Serialize)]
pub struct VendorStatisticDto {
    pub vendor: String,
    pub plugin_count: i32,
    pub usage_count: i32,
}

#[derive(Serialize)]
pub struct TempoStatisticDto {
    pub tempo: f64,
    pub count: i32,
}

#[derive(Serialize)]
pub struct KeyStatisticDto {
    pub key: String,
    pub count: i32,
}

#[derive(Serialize)]
pub struct TimeSignatureStatisticDto {
    pub numerator: i32,
    pub denominator: i32,
    pub count: i32,
}

#[derive(Serialize)]
pub struct YearStatisticDto {
    pub year: i32,
    pub count: i32,
}

#[derive(Serialize)]
pub struct MonthStatisticDto {
    pub year: i32,
    pub month: i32,
    pub count: i32,
}

#[derive(Serialize)]
pub struct ProjectComplexityStatisticDto {
    pub project: Option<ProjectDto>,
    pub plugin_count: i32,
    pub sample_count: i32,
    pub complexity_score: i32,
}

#[derive(Serialize)]
pub struct SampleStatisticDto {
    pub name: String,
    pub path: String,
    pub usage_count: i32,
}

#[derive(Serialize)]
pub struct TagStatisticDto {
    pub name: String,
    pub usage_count: i32,
}

#[derive(Serialize)]
pub struct ActivityTrendStatisticDto {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub projects_created: i32,
    pub projects_modified: i32,
}

#[derive(Serialize)]
pub struct VersionStatisticDto {
    pub version: String,
    pub count: i32,
}

#[derive(Serialize)]
pub struct TaskCompletionTrendStatisticDto {
    pub year: i32,
    pub month: i32,
    pub completed_tasks: i32,
    pub total_tasks: i32,
    pub completion_rate: f64,
}

#[derive(Serialize)]
pub struct StatisticsDto {
    pub total_projects: i32,
    pub total_plugins: i32,
    pub total_samples: i32,
    pub total_collections: i32,
    pub total_tags: i32,
    pub total_tasks: i32,
    pub top_plugins: Vec<PluginStatisticDto>,
    pub top_vendors: Vec<VendorStatisticDto>,
    pub tempo_distribution: Vec<TempoStatisticDto>,
    pub key_distribution: Vec<KeyStatisticDto>,
    pub time_signature_distribution: Vec<TimeSignatureStatisticDto>,
    pub projects_per_year: Vec<YearStatisticDto>,
    pub projects_per_month: Vec<MonthStatisticDto>,
    pub average_monthly_projects: f64,
    pub average_project_duration_seconds: f64,
    pub projects_under_40_seconds: i32,
    pub longest_project: Option<ProjectDto>,
    pub most_complex_projects: Vec<ProjectComplexityStatisticDto>,
    pub average_plugins_per_project: f64,
    pub average_samples_per_project: f64,
    pub top_samples: Vec<SampleStatisticDto>,
    pub top_tags: Vec<TagStatisticDto>,
    pub completed_tasks: i32,
    pub pending_tasks: i32,
    pub task_completion_rate: f64,
    pub recent_activity: Vec<ActivityTrendStatisticDto>,
    pub ableton_versions: Vec<VersionStatisticDto>,
    pub average_projects_per_collection: f64,
    pub largest_collection: Option<CollectionDto>,
    pub task_completion_trends: Vec<TaskCompletionTrendStatisticDto>,
}

impl From<proto::GetStatisticsResponse> for StatisticsDto {
    fn from(s: proto::GetStatisticsResponse) -> Self {
        Self {
            total_projects: s.total_projects,
            total_plugins: s.total_plugins,
            total_samples: s.total_samples,
            total_collections: s.total_collections,
            total_tags: s.total_tags,
            total_tasks: s.total_tasks,
            top_plugins: s
                .top_plugins
                .into_iter()
                .map(|p| PluginStatisticDto {
                    name: p.name,
                    vendor: p.vendor,
                    usage_count: p.usage_count,
                })
                .collect(),
            top_vendors: s
                .top_vendors
                .into_iter()
                .map(|v| VendorStatisticDto {
                    vendor: v.vendor,
                    plugin_count: v.plugin_count,
                    usage_count: v.usage_count,
                })
                .collect(),
            tempo_distribution: s
                .tempo_distribution
                .into_iter()
                .map(|t| TempoStatisticDto {
                    tempo: t.tempo,
                    count: t.count,
                })
                .collect(),
            key_distribution: s
                .key_distribution
                .into_iter()
                .map(|k| KeyStatisticDto {
                    key: k.key,
                    count: k.count,
                })
                .collect(),
            time_signature_distribution: s
                .time_signature_distribution
                .into_iter()
                .map(|t| TimeSignatureStatisticDto {
                    numerator: t.numerator,
                    denominator: t.denominator,
                    count: t.count,
                })
                .collect(),
            projects_per_year: s
                .projects_per_year
                .into_iter()
                .map(|y| YearStatisticDto {
                    year: y.year,
                    count: y.count,
                })
                .collect(),
            projects_per_month: s
                .projects_per_month
                .into_iter()
                .map(|m| MonthStatisticDto {
                    year: m.year,
                    month: m.month,
                    count: m.count,
                })
                .collect(),
            average_monthly_projects: s.average_monthly_projects,
            average_project_duration_seconds: s.average_project_duration_seconds,
            projects_under_40_seconds: s.projects_under_40_seconds,
            longest_project: s.longest_project.map(proto_project_to_dto),
            most_complex_projects: s
                .most_complex_projects
                .into_iter()
                .map(|c| ProjectComplexityStatisticDto {
                    project: c.project.map(proto_project_to_dto),
                    plugin_count: c.plugin_count,
                    sample_count: c.sample_count,
                    complexity_score: c.complexity_score,
                })
                .collect(),
            average_plugins_per_project: s.average_plugins_per_project,
            average_samples_per_project: s.average_samples_per_project,
            top_samples: s
                .top_samples
                .into_iter()
                .map(|sm| SampleStatisticDto {
                    name: sm.name,
                    path: sm.path,
                    usage_count: sm.usage_count,
                })
                .collect(),
            top_tags: s
                .top_tags
                .into_iter()
                .map(|t| TagStatisticDto {
                    name: t.name,
                    usage_count: t.usage_count,
                })
                .collect(),
            completed_tasks: s.completed_tasks,
            pending_tasks: s.pending_tasks,
            task_completion_rate: s.task_completion_rate,
            recent_activity: s
                .recent_activity
                .into_iter()
                .map(|a| ActivityTrendStatisticDto {
                    year: a.year,
                    month: a.month,
                    day: a.day,
                    projects_created: a.projects_created,
                    projects_modified: a.projects_modified,
                })
                .collect(),
            ableton_versions: s
                .ableton_versions
                .into_iter()
                .map(|v| VersionStatisticDto {
                    version: v.version,
                    count: v.count,
                })
                .collect(),
            average_projects_per_collection: s.average_projects_per_collection,
            largest_collection: s.largest_collection.map(proto_collection_to_dto),
            task_completion_trends: s
                .task_completion_trends
                .into_iter()
                .map(|t| TaskCompletionTrendStatisticDto {
                    year: t.year,
                    month: t.month,
                    completed_tasks: t.completed_tasks,
                    total_tasks: t.total_tasks,
                    completion_rate: t.completion_rate,
                })
                .collect(),
        }
    }
}

#[derive(Deserialize)]
pub struct ExportStatisticsQuery {
    pub format: Option<String>,
}
