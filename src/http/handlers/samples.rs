//! Samples domain HTTP handlers (ADR-0024). Thin over `SamplesService`,
//! mirroring `src/grpc/handlers/samples.rs`.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::dto::samples::{
    sample_sort_key, ByPresenceQuery, GetAllSamplesQuery, ProjectsBySampleQuery, SampleDto,
    SampleFormatDto, SampleFormatListResponse, SampleListResponse, SearchSamplesQuery,
};
use crate::models::{OTHER_SAMPLE_FORMAT, SAMPLE_FORMATS};
use crate::models::Sample;
use crate::http::error::ApiError;
use crate::http::state::AppState;

/// Attach each sample's project count with one query for the page (ADR-0034).
async fn with_counts(state: &AppState, samples: Vec<Sample>) -> Result<Vec<SampleDto>, ApiError> {
    let ids: Vec<String> = samples.iter().map(|s| s.id.to_string()).collect();
    let counts = state.services.samples.project_counts(&ids).await?;
    Ok(samples
        .into_iter()
        .map(|s| {
            let count = counts.get(&s.id.to_string()).copied().unwrap_or(0);
            SampleDto::new(s, count)
        })
        .collect())
}

pub async fn get_all_samples(
    State(state): State<AppState>,
    Query(query): Query<GetAllSamplesQuery>,
) -> Result<impl IntoResponse, ApiError> {
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
        )
        .await?;

    Ok(Json(SampleListResponse {
        samples: with_counts(&state, samples).await?,
        total_count,
    }))
}

pub async fn get_sample(
    State(state): State<AppState>,
    Path(sample_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let sample = state
        .services
        .samples
        .get_sample(&sample_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Sample not found with ID: {}", sample_id)))?;

    let mut dtos = with_counts(&state, vec![sample]).await?;
    Ok(Json(dtos.remove(0)))
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
        samples: with_counts(&state, samples).await?,
        total_count,
    }))
}

pub async fn search_samples(
    State(state): State<AppState>,
    Query(query): Query<SearchSamplesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (samples, total_count) = state
        .services
        .samples
        .search_samples(&query.query, query.limit, query.offset, query.present_only, query.format_filter)
        .await?;

    Ok(Json(SampleListResponse {
        samples: with_counts(&state, samples).await?,
        total_count,
    }))
}

pub async fn get_sample_stats(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let stats = state.services.samples.get_sample_stats().await?;
    Ok(Json(stats))
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
        .get_projects_by_sample(&sample_id, query.limit, query.offset)
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
