//! HTTP wire types for the media domain (ADR-0024).
//!
//! Uploads are a plain request body (raw bytes) with metadata as query
//! parameters, not multipart -- axum's multipart support is an opt-in feature
//! this crate doesn't otherwise need, and a raw-bytes body is simpler for any
//! HTTP client (`fetch`, curl) to produce than a multipart envelope for a
//! single file. Downloads read the whole file and return it as one response
//! with `Content-Type`/`Content-Length` set, per ADR-0024's Streaming section
//! -- which, note, is what the gRPC handler already does internally
//! (`tokio::fs::read` loads the whole file before it re-chunks it for
//! transmission), so this isn't a behavior change, just a simpler wire shape.

use serde::{Deserialize, Serialize};

use crate::media::MediaFile as DomainMediaFile;

#[derive(Serialize)]
pub struct MediaFileDto {
    pub id: String,
    pub original_filename: String,
    pub file_extension: String,
    pub media_type: String,
    pub file_size_bytes: i64,
    pub mime_type: String,
    pub uploaded_at: i64,
    pub checksum: String,
}

impl From<DomainMediaFile> for MediaFileDto {
    fn from(file: DomainMediaFile) -> Self {
        Self {
            id: file.id,
            original_filename: file.original_filename,
            file_extension: file.file_extension,
            media_type: file.media_type.as_str().to_string(),
            file_size_bytes: file.file_size_bytes as i64,
            mime_type: file.mime_type,
            uploaded_at: file.uploaded_at.timestamp(),
            checksum: file.checksum,
        }
    }
}

#[derive(Deserialize)]
pub struct UploadCoverArtQuery {
    pub collection_id: String,
    pub filename: String,
}

#[derive(Deserialize)]
pub struct UploadAudioFileQuery {
    pub project_id: String,
    pub filename: String,
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub media_file_id: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Serialize)]
pub struct MutationResponse {
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Deserialize)]
pub struct SetCoverArtRequest {
    pub media_file_id: String,
}

#[derive(Deserialize)]
pub struct SetAudioFileRequest {
    pub media_file_id: String,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Deserialize)]
pub struct MediaByTypeQuery {
    pub media_type: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Serialize)]
pub struct MediaFileListResponse {
    pub media_files: Vec<MediaFileDto>,
    pub total_count: i32,
}

#[derive(Serialize)]
pub struct MediaStatisticsResponse {
    pub total_files: i32,
    pub total_size_bytes: i64,
    pub cover_art_count: i32,
    pub audio_file_count: i32,
    pub orphaned_files_count: i32,
    pub orphaned_files_size_bytes: i64,
    pub files_by_type: std::collections::HashMap<String, i32>,
}

#[derive(Deserialize)]
pub struct CleanupQuery {
    pub dry_run: bool,
}

#[derive(Serialize)]
pub struct CleanupResponse {
    pub files_cleaned: i32,
    pub bytes_freed: i64,
    pub deleted_file_ids: Vec<String>,
    pub success: bool,
    pub error_message: Option<String>,
}
