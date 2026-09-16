//! Media domain HTTP handlers (ADR-0024). Thin over `MediaService`, mirroring
//! `src/grpc/handlers/media.rs` -- including its two error conventions: an
//! empty upload or a missing required field is a real rejection (400), but a
//! storage/database failure during store/delete/set is a normal 200 with
//! `success: false`, matching the gRPC handler exactly.

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::media::{
    CleanupQuery, CleanupResponse, MediaByTypeQuery, MediaFileDto, MediaFileListResponse,
    MediaStatisticsResponse, MutationResponse, PaginationQuery, SetAudioFileRequest,
    SetCoverArtRequest, UploadAudioFileQuery, UploadCoverArtQuery, UploadResponse,
};
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn upload_cover_art(
    State(state): State<AppState>,
    Query(query): Query<UploadCoverArtQuery>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    if body.is_empty() {
        return Err(ApiError::InvalidRequest("No file data received".to_string()));
    }

    let response = match state
        .services
        .media
        .store_cover_art(&body, &query.filename, &query.collection_id)
        .await
    {
        Ok(media_file) => UploadResponse {
            media_file_id: media_file.id,
            success: true,
            error_message: None,
        },
        Err(e) => UploadResponse {
            media_file_id: String::new(),
            success: false,
            error_message: Some(e.to_string()),
        },
    };

    Ok(Json(response))
}

pub async fn upload_audio_file(
    State(state): State<AppState>,
    Query(query): Query<UploadAudioFileQuery>,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    if body.is_empty() {
        return Err(ApiError::InvalidRequest("No file data received".to_string()));
    }

    let response = match state
        .services
        .media
        .store_audio_file(&body, &query.filename, &query.project_id)
        .await
    {
        Ok(media_file) => UploadResponse {
            media_file_id: media_file.id,
            success: true,
            error_message: None,
        },
        Err(e) => UploadResponse {
            media_file_id: String::new(),
            success: false,
            error_message: Some(e.to_string()),
        },
    };

    Ok(Json(response))
}

pub async fn download_media(
    State(state): State<AppState>,
    Path(media_file_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let media_file = state
        .services
        .media
        .get_media_file(&media_file_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Media file not found".to_string()))?;

    let file_path = state
        .services
        .media
        .file_path(&media_file)
        .map_err(|e| ApiError::Internal(format!("Failed to get file path: {}", e)))?;

    let file_data = tokio::fs::read(&file_path)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to read file: {}", e)))?;

    let headers = [
        (header::CONTENT_TYPE, media_file.mime_type.clone()),
        (header::CONTENT_LENGTH, file_data.len().to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", media_file.original_filename),
        ),
    ];

    Ok((StatusCode::OK, headers, file_data))
}

pub async fn delete_media(
    State(state): State<AppState>,
    Path(media_file_id): Path<String>,
) -> Json<MutationResponse> {
    match state.services.media.delete_media(&media_file_id).await {
        Ok(()) => Json(MutationResponse {
            success: true,
            error_message: None,
        }),
        Err(e) => Json(MutationResponse {
            success: false,
            error_message: Some(e.to_string()),
        }),
    }
}

pub async fn set_collection_cover_art(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
    Json(req): Json<SetCoverArtRequest>,
) -> Json<MutationResponse> {
    match state
        .services
        .media
        .set_collection_cover_art(&collection_id, &req.media_file_id)
        .await
    {
        Ok(()) => Json(MutationResponse {
            success: true,
            error_message: None,
        }),
        Err(e) => Json(MutationResponse {
            success: false,
            error_message: Some(format!("Database error: {}", e)),
        }),
    }
}

pub async fn remove_collection_cover_art(
    State(state): State<AppState>,
    Path(collection_id): Path<String>,
) -> Json<MutationResponse> {
    match state.services.media.remove_collection_cover_art(&collection_id).await {
        Ok(()) => Json(MutationResponse {
            success: true,
            error_message: None,
        }),
        Err(e) => Json(MutationResponse {
            success: false,
            error_message: Some(format!("Database error: {}", e)),
        }),
    }
}

pub async fn set_project_audio_file(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<SetAudioFileRequest>,
) -> Json<MutationResponse> {
    match state
        .services
        .media
        .set_project_audio_file(&project_id, &req.media_file_id)
        .await
    {
        Ok(()) => Json(MutationResponse {
            success: true,
            error_message: None,
        }),
        Err(e) => Json(MutationResponse {
            success: false,
            error_message: Some(format!("Database error: {}", e)),
        }),
    }
}

pub async fn remove_project_audio_file(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Json<MutationResponse> {
    match state.services.media.remove_project_audio_file(&project_id).await {
        Ok(()) => Json(MutationResponse {
            success: true,
            error_message: None,
        }),
        Err(e) => Json(MutationResponse {
            success: false,
            error_message: Some(format!("Database error: {}", e)),
        }),
    }
}

pub async fn list_media_files(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (media_files, total_count) = state.services.media.list_media_files(query.limit, query.offset).await?;
    Ok(Json(MediaFileListResponse {
        media_files: media_files.into_iter().map(MediaFileDto::from).collect(),
        total_count,
    }))
}

pub async fn get_media_files_by_type(
    State(state): State<AppState>,
    Query(query): Query<MediaByTypeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (media_files, total_count) = state
        .services
        .media
        .get_media_files_by_type(&query.media_type, query.limit, query.offset)
        .await?;
    Ok(Json(MediaFileListResponse {
        media_files: media_files.into_iter().map(MediaFileDto::from).collect(),
        total_count,
    }))
}

pub async fn get_orphaned_media_files(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (media_files, total_count) = state
        .services
        .media
        .get_orphaned_media_files(query.limit, query.offset)
        .await?;
    Ok(Json(MediaFileListResponse {
        media_files: media_files.into_iter().map(MediaFileDto::from).collect(),
        total_count,
    }))
}

pub async fn get_media_statistics(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let (total_files, total_size, cover_art_count, audio_file_count, orphaned_count, orphaned_size) =
        state.services.media.get_media_statistics().await?;

    let mut files_by_type = std::collections::HashMap::new();
    files_by_type.insert("cover_art".to_string(), cover_art_count);
    files_by_type.insert("audio_file".to_string(), audio_file_count);

    Ok(Json(MediaStatisticsResponse {
        total_files,
        total_size_bytes: total_size,
        cover_art_count,
        audio_file_count,
        orphaned_files_count: orphaned_count,
        orphaned_files_size_bytes: orphaned_size,
        files_by_type,
    }))
}

pub async fn cleanup_orphaned_media(
    State(state): State<AppState>,
    Query(query): Query<CleanupQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (deleted_file_ids, bytes_freed) = state.services.media.cleanup_orphaned_media(query.dry_run).await?;

    Ok(Json(CleanupResponse {
        files_cleaned: deleted_file_ids.len() as i32,
        bytes_freed,
        deleted_file_ids,
        success: true,
        error_message: None,
    }))
}
