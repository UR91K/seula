//! Media domain HTTP handlers (ADR-0024). Thin over `MediaService`, mirroring
//! `src/grpc/handlers/media.rs` -- including its two error conventions: an
//! empty upload or a missing required field is a real rejection (400), but a
//! storage/database failure during store/delete/set is a normal 200 with
//! `success: false`, matching the gRPC handler exactly.
//!
//! The project audio-list routes (ADR-0037) have no gRPC counterpart to mirror, so they
//! use ordinary HTTP errors (404, 400) and return the resulting list on success.

use axum::body::{boxed, Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, Request};
use axum::response::{IntoResponse, Response};
use axum::Json;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::http::dto::media::{
    CleanupQuery, CleanupResponse, MediaByTypeQuery, MediaFileDto, MediaFileListResponse,
    MediaStatisticsResponse, MutationResponse, PaginationQuery, SetAudioFileRequest,
    SetCoverArtRequest, UploadAudioFileQuery, UploadCoverArtQuery, UploadResponse,
};
use crate::http::dto::projects::{AudioFileDto, AudioFileListResponse};
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

/// Streams the stored file with `Range` support, so an `<audio>` element can seek in
/// the audition audio (ADR-0033). `ServeFile` also answers conditional requests.
pub async fn download_media(
    State(state): State<AppState>,
    Path(media_file_id): Path<String>,
    request: Request<Body>,
) -> Result<Response, ApiError> {
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

    let mime: mime::Mime = media_file
        .mime_type
        .parse()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM);

    // ServeFile's error type is Infallible; a missing file comes back as a 404 response.
    let mut response = ServeFile::new_with_mime(&file_path, &mime)
        .oneshot(request)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to serve file: {}", e)))?;

    // Browsers ignore this for <img> and <audio>; it makes opening the URL directly
    // download the file under its original name.
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        content_disposition(&media_file.original_filename),
    );

    Ok(response.map(boxed))
}

/// `attachment` with the original file name, RFC 6266 style: an ASCII fallback plus a
/// percent-encoded UTF-8 form, since a header value cannot carry raw non-ASCII.
fn content_disposition(filename: &str) -> HeaderValue {
    let ascii: String = filename
        .chars()
        .map(|c| if (c.is_ascii_graphic() && c != '"' && c != '\\') || c == ' ' { c } else { '_' })
        .collect();
    let encoded: String = filename
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{:02X}", b)
            }
        })
        .collect();
    HeaderValue::from_str(&format!("attachment; filename=\"{}\"; filename*=UTF-8''{}", ascii, encoded))
        .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
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

/// A project's audition audios, in list order (ADR-0037).
pub async fn list_project_audio_files(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Result<Json<AudioFileListResponse>, ApiError> {
    let audio_files = state
        .services
        .media
        .project_audio_files(&project_id)
        .await?
        .into_iter()
        .map(|(media, primary)| AudioFileDto::new(media, primary))
        .collect();
    Ok(Json(AudioFileListResponse { audio_files }))
}

/// Attach an already-uploaded audio to a project's list. It does not become the
/// primary; that is `PUT /api/v1/projects/:id/audio-file`. Returns the new list.
pub async fn add_project_audio_file(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(req): Json<SetAudioFileRequest>,
) -> Result<Json<AudioFileListResponse>, ApiError> {
    state.services.media.add_project_audio_file(&project_id, &req.media_file_id).await?;
    list_project_audio_files(State(state), Path(project_id)).await
}

/// Take an audio off a project's list; the next one becomes primary if it was.
/// Returns the new list, or 404 when the audio was not on it.
pub async fn remove_project_audio_file_from_list(
    State(state): State<AppState>,
    Path((project_id, media_file_id)): Path<(String, String)>,
) -> Result<Json<AudioFileListResponse>, ApiError> {
    let removed = state
        .services
        .media
        .remove_project_audio_file_from_list(&project_id, &media_file_id)
        .await?;
    if !removed {
        return Err(ApiError::NotFound(format!(
            "Audio {} is not on project {}",
            media_file_id, project_id
        )));
    }
    list_project_audio_files(State(state), Path(project_id)).await
}
