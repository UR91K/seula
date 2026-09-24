//! Samples domain HTTP handlers (ADR-0024). Thin over `SamplesService`,
//! mirroring `src/grpc/handlers/samples.rs`.

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use tokio_stream::wrappers::ReceiverStream;

use crate::database::samples::SampleFilter;

use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::dto::samples::{
    sample_sort_key, ByPresenceQuery, GetAllSamplesQuery, ProjectsBySampleQuery, SampleDto,
    SampleCheckEventDto, SampleDetailDto, SampleFileDto, SampleFormatDto, SampleFormatListResponse, SampleListResponse,
    SampleStatsQuery, ScopeQuery, SearchSamplesQuery,
};
use crate::database::ProjectScope;
use crate::http::dto::parse_project_scope;
use crate::models::{OTHER_SAMPLE_FORMAT, SAMPLE_FORMATS};
use crate::models::Sample;
use crate::http::error::ApiError;
use crate::http::state::AppState;
use crate::services::system::send_scan_update;

/// Attach each sample's project count (ADR-0034), counting the projects in `scope`
/// (ADR-0040), and its measured size (ADR-0041), with one query each for the page.
async fn with_counts(
    state: &AppState,
    samples: Vec<Sample>,
    scope: ProjectScope,
) -> Result<Vec<SampleDto>, ApiError> {
    let ids: Vec<String> = samples.iter().map(|s| s.id.to_string()).collect();
    let counts = state.services.samples.project_counts(&ids, scope).await?;
    let sizes = state.services.samples.sizes(&ids).await?;
    Ok(samples
        .into_iter()
        .map(|s| {
            let id = s.id.to_string();
            let count = counts.get(&id).copied().unwrap_or(0);
            SampleDto::new(s, count, sizes.get(&id).copied())
        })
        .collect())
}

pub async fn get_all_samples(
    State(state): State<AppState>,
    Query(query): Query<GetAllSamplesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = parse_project_scope(query.scope.as_deref())?;
    let (samples, total_count) = state
        .services
        .samples
        .get_all_samples(
            query.limit,
            query.offset,
            sample_sort_key(query.sort_by),
            query.sort_desc,
            query.present_only,
            query.missing_only,
            query.format_filter,
            query.min_project_count,
            query.max_project_count,
            scope,
        )
        .await?;

    Ok(Json(SampleListResponse {
        samples: with_counts(&state, samples, scope).await?,
        total_count,
    }))
}

pub async fn get_sample(
    State(state): State<AppState>,
    Path(sample_id): Path<String>,
    Query(query): Query<ScopeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = parse_project_scope(query.scope.as_deref())?;
    let sample = state
        .services
        .samples
        .get_sample(&sample_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Sample not found with ID: {}", sample_id)))?;

    let mut dtos = with_counts(&state, vec![sample], scope).await?;
    let file = state
        .services
        .samples
        .file(&sample_id)
        .await?
        .map(|(size_bytes, modified_at, checked_at)| SampleFileDto { size_bytes, modified_at, checked_at });
    Ok(Json(SampleDetailDto { sample: dtos.remove(0), file }))
}

pub async fn get_samples_by_presence(
    State(state): State<AppState>,
    Query(query): Query<ByPresenceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (samples, total_count) = state
        .services
        .samples
        .get_samples_by_presence(
            query.is_present,
            query.limit,
            query.offset,
            sample_sort_key(query.sort_by),
            query.sort_desc,
        )
        .await?;

    Ok(Json(SampleListResponse {
        samples: with_counts(&state, samples, ProjectScope::Active).await?,
        total_count,
    }))
}

pub async fn search_samples(
    State(state): State<AppState>,
    Query(query): Query<SearchSamplesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = parse_project_scope(query.scope.as_deref())?;
    let (samples, total_count) = state
        .services
        .samples
        .search_samples(&query.query, query.limit, query.offset, query.present_only, query.format_filter)
        .await?;

    Ok(Json(SampleListResponse {
        samples: with_counts(&state, samples, scope).await?,
        total_count,
    }))
}

/// Counts over the samples the same filters list, or over every sample with none.
pub async fn get_sample_stats(
    State(state): State<AppState>,
    Query(query): Query<SampleStatsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = SampleFilter {
        query: query.query,
        format: query.format_filter,
        present: query.present_only.or(query.missing_only.map(|m| !m)),
    };
    let stats = state.services.samples.get_sample_stats_filtered(&filter).await?;
    Ok(Json(stats))
}

/// Check every sample file in the background, streaming progress as Server-Sent Events
/// like the other scans (ADR-0041). The progress also shows at
/// `GET /api/v1/system/scan-status`. 409 when a scan is already running.
pub async fn check_samples(
    State(state): State<AppState>,
) -> Result<Sse<ReceiverStream<Result<Event, Infallible>>>, ApiError> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    let started = state
        .system
        .start_sample_check(move |response, result| {
            let status = response.status;
            let dto = SampleCheckEventDto {
                progress: response.into(),
                result,
            };
            if let Ok(json) = serde_json::to_string(&dto) {
                send_scan_update(&tx, status, Ok(Event::default().data(json)));
            }
        })
        .await;
    if !started {
        return Err(ApiError::Conflict("A scan is already running".to_string()));
    }

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default()))
}

pub async fn get_all_sample_usage_numbers(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let usage_info = state.services.samples.get_all_sample_usage_numbers().await?;
    Ok(Json(usage_info))
}

pub async fn get_projects_by_sample(
    State(state): State<AppState>,
    Path(sample_id): Path<String>,
    Query(query): Query<ProjectsBySampleQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (projects, total_count) = state
        .services
        .samples
        .get_projects_by_sample(&sample_id, query.limit, query.offset, parse_project_scope(query.scope.as_deref())?)
        .await?;

    let db_arc = state.services.samples.db_handle();
    let mut db = db_arc.lock().await;
    let projects = projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ProjectListResponse { projects, total_count }))
}

pub async fn refresh_sample_presence_status(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.services.samples.refresh_sample_presence_status().await?;
    Ok(Json(result))
}

pub async fn get_sample_analytics(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let analytics = state.services.samples.get_sample_analytics().await?;
    Ok(Json(analytics))
}

/// The format dropdown: every format Live loads, with its counts, then `other`.
pub async fn get_sample_formats(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let mut counts = state.services.samples.get_sample_extensions().await?;
    let mut formats: Vec<SampleFormatDto> = SAMPLE_FORMATS
        .iter()
        .map(|f| {
            let c = counts.remove(f.id);
            SampleFormatDto {
                format: f.id.to_string(),
                name: f.name.to_string(),
                extensions: f.extensions.iter().map(|e| e.to_string()).collect(),
                count: c.as_ref().map_or(0, |c| c.count),
                present_count: c.as_ref().map_or(0, |c| c.present_count),
                missing_count: c.as_ref().map_or(0, |c| c.missing_count),
                total_size_bytes: c.as_ref().map_or(0, |c| c.total_size_bytes),
            }
        })
        .collect();
    if let Some(c) = counts.remove(OTHER_SAMPLE_FORMAT) {
        formats.push(SampleFormatDto {
            format: OTHER_SAMPLE_FORMAT.to_string(),
            name: "Other".to_string(),
            extensions: Vec::new(),
            count: c.count,
            present_count: c.present_count,
            missing_count: c.missing_count,
            total_size_bytes: c.total_size_bytes,
        });
    }
    Ok(Json(SampleFormatListResponse { formats }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ProjectDatabase;
    use crate::grpc::common::ScanStatus;
    use crate::media::{MediaConfig, MediaStorageManager};
    use crate::services::{Services, SystemService};
    use axum::body::HttpBody;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Mutex;

    /// Streams sent with `try_send` into a channel of 100, so a client more than 100
    /// updates behind lost them, the last one included, and that one carries the
    /// result. This check sends about 300 and nothing reads until it has finished.
    #[tokio::test]
    async fn a_slow_client_still_gets_the_final_update() {
        let db = ProjectDatabase::new(":memory:".into()).unwrap();
        for i in 0..300 {
            db.conn
                .execute(
                    "INSERT INTO samples (id, name, path, is_present) VALUES (?, 'kick.wav', ?, 1)",
                    rusqlite::params![format!("sample-{i}"), format!("Z:/nowhere/folder {i}/kick.wav")],
                )
                .unwrap();
        }
        let db = Arc::new(Mutex::new(db));
        let media_dir = tempfile::tempdir().unwrap();
        let media = Arc::new(MediaStorageManager::new(media_dir.path().to_path_buf(), MediaConfig::default()).unwrap());
        let status = Arc::new(Mutex::new(ScanStatus::ScanUnknown));
        let system = SystemService::new(
            Arc::clone(&db),
            Arc::clone(&status),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(None)),
            Instant::now(),
        );
        let state = AppState::new(Services::new(db, media), system);

        let mut body = check_samples(State(state)).await.unwrap().into_response().into_body();

        let deadline = Instant::now() + Duration::from_secs(30);
        while !matches!(*status.lock().await, ScanStatus::ScanCompleted | ScanStatus::ScanError) {
            assert!(Instant::now() < deadline, "the check never finished");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        // The status is written just before the final update is sent.
        tokio::time::sleep(Duration::from_millis(200)).await;

        let mut text = String::new();
        while let Some(chunk) = body.data().await {
            text.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
        }
        let last = text.lines().filter_map(|l| l.strip_prefix("data:")).last().unwrap();
        let last: serde_json::Value = serde_json::from_str(last.trim()).unwrap();
        assert!(!last["result"].is_null(), "the last update received is not the final one: {last}");
    }
}
