//! HTTP wire types for the projects domain (ADR-0024). Independent of the
//! generated proto types (see ADR-0024's rejected alternatives), but the shape
//! deliberately follows `proto/common.proto`'s `Project` message closely -- that
//! shape was already worked out, and there's no reason for the JSON contract to
//! diverge from it just to look different.

use serde::{Deserialize, Serialize};

use crate::database::stats::ProjectStatistics;
use crate::http::dto::tags::TagDto;
use crate::models::{KeySignature, Scale, Tonic};
use crate::project::Project as DomainProject;

#[derive(Serialize)]
pub struct TimeSignatureDto {
    pub numerator: i32,
    pub denominator: i32,
}

/// A key as sent over the wire (ADR-0035). `tonic` and `scale` are the enum names the
/// filters take back as input; `sharp` and `flat` are the two display spellings, and
/// the client shows whichever its sharp/flat switch selects.
#[derive(Serialize)]
pub struct KeySignatureDto {
    pub tonic: String,
    pub scale: String,
    pub sharp: String,
    pub flat: String,
}

impl From<KeySignature> for KeySignatureDto {
    fn from(key: KeySignature) -> Self {
        Self {
            sharp: key.sharp_name(),
            flat: key.flat_name(),
            tonic: key.tonic.to_string(),
            scale: key.scale.to_string(),
        }
    }
}

impl KeySignatureDto {
    /// From the stored enum names, as found in `projects.key_signature_tonic` and
    /// `key_signature_scale` or a proto `KeySignature`. `None` when neither half is a
    /// key, and a half that does not parse counts as `Empty`.
    pub fn from_names(tonic: &str, scale: &str) -> Option<Self> {
        let key = KeySignature {
            tonic: tonic.parse().unwrap_or(Tonic::Empty),
            scale: scale.parse().unwrap_or(Scale::Empty),
        };
        if key.tonic == Tonic::Empty && key.scale == Scale::Empty {
            return None;
        }
        Some(key.into())
    }

    /// From the `"<tonic> <scale>"` strings the statistics queries build in SQL. The
    /// tonic is an enum name and never contains a space, so the first space splits the
    /// two even when an unknown Ableton scale name has spaces of its own.
    pub fn from_joined(joined: &str) -> Option<Self> {
        let (tonic, scale) = joined.split_once(' ')?;
        Self::from_names(tonic, scale)
    }
}

#[derive(Serialize)]
pub struct AbletonVersionDto {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub beta: bool,
}

#[derive(Serialize)]
pub struct PluginDto {
    pub id: String,
    pub dev_identifier: String,
    pub name: String,
    pub format: String,
    pub installed: Option<bool>,
    pub vendor: Option<String>,
    pub version: Option<String>,
}

#[derive(Serialize)]
pub struct SampleDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_present: bool,
}

#[derive(Serialize)]
pub struct TaskDto {
    pub id: String,
    pub project_id: String,
    pub description: String,
    pub completed: bool,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub name: String,
    pub path: String,
    pub hash: String,
    pub notes: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub last_parsed_at: i64,

    pub tempo: f64,
    pub time_signature: TimeSignatureDto,
    pub key_signature: Option<KeySignatureDto>,
    pub duration_seconds: Option<f64>,
    pub furthest_bar: Option<f64>,

    pub ableton_version: AbletonVersionDto,

    pub plugins: Vec<PluginDto>,
    pub samples: Vec<SampleDto>,
    pub tags: Vec<TagDto>,
    pub tasks: Vec<TaskDto>,
    pub collection_ids: Vec<String>,
    pub audio_file_id: Option<String>,
}

/// Mirrors `src/grpc/handlers/utils.rs::convert_live_set_to_proto`, but builds
/// the HTTP DTO directly instead of the proto type. Needs a database lock for
/// the same reason that function does: notes, audio file, collections, tags and
/// tasks are not fields of `Project` itself -- they're stored, and loaded,
/// separately.
pub fn project_to_dto(
    live_set: DomainProject,
    db: &mut crate::database::ProjectDatabase,
) -> Result<ProjectDto, crate::error::DatabaseError> {
    let project_id = live_set.id.to_string();

    let notes = db.get_project_notes(&project_id)?.unwrap_or_default();
    let audio_file_id = db
        .get_project_audio_file(&project_id)?
        .map(|media_file| media_file.id);
    let collection_ids = db.get_collections_for_project(&project_id)?;
    let tag_data = db.get_project_tag_data(&project_id)?;
    let tasks = db
        .get_project_tasks(&project_id)?
        .into_iter()
        .map(|(task_id, description, completed, created_at)| TaskDto {
            id: task_id,
            project_id: project_id.clone(),
            description,
            completed,
            created_at,
        })
        .collect();
    let tags = tag_data.into_iter().map(TagDto::from).collect();

    Ok(ProjectDto {
        id: project_id,
        name: live_set.name,
        path: live_set.file_path.to_string_lossy().to_string(),
        hash: live_set.file_hash,
        notes,
        created_at: live_set.created_time.timestamp(),
        modified_at: live_set.modified_time.timestamp(),
        last_parsed_at: live_set.last_parsed_timestamp.timestamp(),

        tempo: live_set.tempo,
        time_signature: TimeSignatureDto {
            numerator: live_set.time_signature.numerator as i32,
            denominator: live_set.time_signature.denominator as i32,
        },
        key_signature: live_set.key_signature.map(KeySignatureDto::from),
        duration_seconds: live_set.estimated_duration.map(|d| d.num_seconds() as f64),
        furthest_bar: live_set.furthest_bar,

        ableton_version: AbletonVersionDto {
            major: live_set.ableton_metadata.major,
            minor: live_set.ableton_metadata.minor,
            patch: live_set.ableton_metadata.patch,
            beta: live_set.ableton_metadata.beta,
        },

        plugins: live_set
            .plugins
            .into_iter()
            .map(|p| PluginDto {
                id: p.id.to_string(),
                dev_identifier: p.dev_identifier,
                name: p.name,
                format: p.plugin_format.to_string(),
                installed: p.installed,
                vendor: p.vendor,
                version: p.version,
            })
            .collect(),

        samples: live_set
            .samples
            .into_iter()
            .map(|s| SampleDto {
                id: s.id.to_string(),
                name: s.name,
                path: s.path.to_string_lossy().to_string(),
                is_present: s.is_present,
            })
            .collect(),

        tags,
        tasks,
        collection_ids,
        audio_file_id,
    })
}

#[derive(Deserialize)]
pub struct ListProjectsQuery {
    /// "active" (default), "deleted", or "all".
    pub scope: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub min_tempo: Option<f64>,
    pub max_tempo: Option<f64>,
    pub key_signature_tonic: Option<String>,
    pub key_signature_scale: Option<String>,
    pub time_signature_numerator: Option<i32>,
    pub time_signature_denominator: Option<i32>,
    pub ableton_version_major: Option<i32>,
    pub ableton_version_minor: Option<i32>,
    pub ableton_version_patch: Option<i32>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub has_audio_file: Option<bool>,
}

#[derive(Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct UpdateNotesRequest {
    pub notes: String,
}

#[derive(Deserialize)]
pub struct UpdateNameRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct BatchArchiveRequest {
    pub project_ids: Vec<String>,
    pub archived: bool,
}

#[derive(Deserialize)]
pub struct BatchProjectIdsRequest {
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
pub struct RescanRequest {
    pub force_rescan: Option<bool>,
}

#[derive(Serialize)]
pub struct RescanResponse {
    pub success: bool,
    pub was_updated: bool,
    pub scan_summary: String,
    pub error_message: Option<String>,
    pub updated_project: Option<ProjectDto>,
}

#[derive(Deserialize)]
pub struct StatisticsQuery {
    pub min_tempo: Option<f64>,
    pub max_tempo: Option<f64>,
    pub key_signature_tonic: Option<String>,
    pub key_signature_scale: Option<String>,
    pub time_signature_numerator: Option<i32>,
    pub time_signature_denominator: Option<i32>,
    pub ableton_version_major: Option<i32>,
    pub ableton_version_minor: Option<i32>,
    pub ableton_version_patch: Option<i32>,
    pub created_after: Option<i64>,
    pub created_before: Option<i64>,
    pub has_audio_file: Option<bool>,
}

#[derive(Serialize)]
pub struct TempoRangeStatisticDto {
    pub range: String,
    pub count: i32,
}

/// `key_signature` is `null` for projects with no detected key.
#[derive(Serialize)]
pub struct KeySignatureStatisticDto {
    pub key_signature: Option<KeySignatureDto>,
    pub count: i32,
}

#[derive(Serialize)]
pub struct TimeSignatureStatisticDto {
    pub numerator: i32,
    pub denominator: i32,
    pub count: i32,
}

#[derive(Serialize)]
pub struct AbletonVersionStatisticDto {
    pub version: String,
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
    pub project_id: String,
    pub project_name: String,
    pub plugin_count: i32,
    pub sample_count: i32,
    pub tag_count: i32,
    pub complexity_score: f64,
}

#[derive(Serialize)]
pub struct ProjectStatisticsDto {
    pub total_projects: i32,
    pub projects_with_audio_files: i32,
    pub projects_without_audio_files: i32,
    pub average_tempo: f64,
    pub min_tempo: f64,
    pub max_tempo: f64,
    pub tempo_distribution: Vec<TempoRangeStatisticDto>,
    pub key_signature_distribution: Vec<KeySignatureStatisticDto>,
    pub time_signature_distribution: Vec<TimeSignatureStatisticDto>,
    pub ableton_version_distribution: Vec<AbletonVersionStatisticDto>,
    pub average_duration_seconds: f64,
    pub min_duration_seconds: f64,
    pub max_duration_seconds: f64,
    pub average_plugins_per_project: f64,
    pub average_samples_per_project: f64,
    pub average_tags_per_project: f64,
    pub projects_per_year: Vec<YearStatisticDto>,
    pub projects_per_month: Vec<MonthStatisticDto>,
    pub most_complex_projects: Vec<ProjectComplexityStatisticDto>,
}

impl From<ProjectStatistics> for ProjectStatisticsDto {
    fn from(stats: ProjectStatistics) -> Self {
        Self {
            total_projects: stats.total_projects,
            projects_with_audio_files: stats.projects_with_audio_files,
            projects_without_audio_files: stats.projects_without_audio_files,
            average_tempo: stats.average_tempo,
            min_tempo: stats.min_tempo,
            max_tempo: stats.max_tempo,
            tempo_distribution: stats
                .tempo_distribution
                .into_iter()
                .map(|(range, count)| TempoRangeStatisticDto { range, count })
                .collect(),
            key_signature_distribution: stats
                .key_signature_distribution
                .into_iter()
                .map(|(key_signature, count)| KeySignatureStatisticDto {
                    key_signature: KeySignatureDto::from_joined(&key_signature),
                    count,
                })
                .collect(),
            time_signature_distribution: stats
                .time_signature_distribution
                .into_iter()
                .map(|(numerator, denominator, count)| TimeSignatureStatisticDto {
                    numerator,
                    denominator,
                    count,
                })
                .collect(),
            ableton_version_distribution: stats
                .ableton_version_distribution
                .into_iter()
                .map(|(version, count)| AbletonVersionStatisticDto { version, count })
                .collect(),
            average_duration_seconds: stats.average_duration_seconds,
            min_duration_seconds: stats.min_duration_seconds,
            max_duration_seconds: stats.max_duration_seconds,
            average_plugins_per_project: stats.average_plugins_per_project,
            average_samples_per_project: stats.average_samples_per_project,
            average_tags_per_project: stats.average_tags_per_project,
            projects_per_year: stats
                .projects_per_year
                .into_iter()
                .map(|(year, count)| YearStatisticDto { year, count })
                .collect(),
            projects_per_month: stats
                .projects_per_month
                .into_iter()
                .map(|(year, month, count)| MonthStatisticDto { year, month, count })
                .collect(),
            most_complex_projects: stats
                .most_complex_projects
                .into_iter()
                .map(
                    |(project_id, project_name, plugin_count, sample_count, tag_count, complexity_score)| {
                        ProjectComplexityStatisticDto {
                            project_id,
                            project_name,
                            plugin_count,
                            sample_count,
                            tag_count,
                            complexity_score,
                        }
                    },
                )
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_key_strings_split_on_the_first_space_only() {
        let dto = KeySignatureDto::from_joined("GSharp Some Future Scale").unwrap();
        assert_eq!(dto.tonic, "GSharp");
        assert_eq!(dto.scale, "Some Future Scale");
        assert_eq!(dto.flat, "A\u{266D} Some Future Scale");

        assert!(KeySignatureDto::from_joined("Unknown").is_none());
        assert!(KeySignatureDto::from_joined("Empty Empty").is_none());
    }
}
