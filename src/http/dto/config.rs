//! HTTP wire types for the config domain (ADR-0024).
//!
//! Mutations here follow `src/grpc/handlers/config.rs`'s convention, not the
//! `ApiError`/`?` pattern the other domains use: a failed update is a normal
//! 200 response with `success: false` and an `error_message`, not a 4xx/5xx.
//! That's a deliberate existing choice (config errors are user-facing
//! validation feedback, not exceptional failures) and preserved as-is rather
//! than folded into the generic error mapping as part of this port.

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
pub struct ConfigDto {
    pub paths: Vec<String>,
    pub database_path: Option<String>,
    pub grpc_port: u32,
    pub log_level: String,
    pub media_storage_dir: String,
    pub max_cover_art_size_mb: Option<u32>,
    pub max_audio_file_size_mb: Option<u32>,
    pub needs_setup: bool,
    pub status_message: String,
}

impl From<&Config> for ConfigDto {
    fn from(config: &Config) -> Self {
        Self {
            paths: config.paths.clone(),
            database_path: config.database_path.clone(),
            grpc_port: config.grpc_port as u32,
            log_level: config.log_level.clone(),
            media_storage_dir: config.media_storage_dir.clone(),
            max_cover_art_size_mb: config.max_cover_art_size_mb,
            max_audio_file_size_mb: config.max_audio_file_size_mb,
            needs_setup: config.needs_setup(),
            status_message: config.get_status_message(),
        }
    }
}

#[derive(Serialize)]
pub struct GetConfigResponse {
    pub config: ConfigDto,
}

#[derive(Serialize)]
pub struct ConfigStatusResponse {
    pub needs_setup: bool,
    pub is_ready_for_operation: bool,
    pub status_message: String,
    pub configured_paths_count: i32,
}

#[derive(Deserialize)]
pub struct UpdatePathsRequest {
    pub paths: Vec<String>,
}

#[derive(Deserialize)]
pub struct AddPathRequest {
    pub path: String,
}

#[derive(Deserialize)]
pub struct RemovePathRequest {
    pub path: String,
}

#[derive(Serialize)]
pub struct PathMutationResponse {
    pub success: bool,
    pub error_message: Option<String>,
    pub validation_warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct RemovePathResponse {
    pub success: bool,
    pub error_message: Option<String>,
    pub remaining_paths_count: i32,
}

#[derive(Deserialize)]
pub struct UpdateSettingsRequest {
    pub database_path: Option<String>,
    pub grpc_port: Option<u16>,
    pub log_level: Option<String>,
    pub media_storage_dir: Option<String>,
    pub max_cover_art_size_mb: Option<Option<u32>>,
    pub max_audio_file_size_mb: Option<Option<u32>>,
}

#[derive(Serialize)]
pub struct ReloadConfigResponse {
    pub success: bool,
    pub error_message: Option<String>,
    pub validation_warnings: Vec<String>,
    pub config: Option<ConfigDto>,
}

#[derive(Serialize)]
pub struct ValidateConfigResponse {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub needs_setup: bool,
}
