//! Config domain HTTP handlers (ADR-0024). Thin over `ConfigService`, mirroring
//! `src/grpc/handlers/config.rs` -- including its two different error
//! conventions: a config that fails to *load* is a 500 (`ApiError::Internal`,
//! matching the gRPC handler's `Status::Internal`), but a mutation that fails
//! (bad path, invalid setting) is a normal 200 with `success: false`.

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::config::{
    AddPathRequest, ConfigDto, ConfigStatusResponse, GetConfigResponse, PathMutationResponse,
    ReloadConfigResponse, RemovePathRequest, RemovePathResponse, UpdatePathsRequest,
    UpdateSettingsRequest, ValidateConfigResponse,
};
use crate::http::error::ApiError;
use crate::http::state::AppState;

fn load_error(e: impl std::fmt::Display) -> ApiError {
    ApiError::Internal(format!("Failed to load config: {}", e))
}

pub async fn get_config(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let config = state.services.config.get().map_err(load_error)?;
    Ok(Json(GetConfigResponse {
        config: ConfigDto::from(config),
    }))
}

pub async fn get_config_status(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let config = state.services.config.get().map_err(load_error)?;
    Ok(Json(ConfigStatusResponse {
        needs_setup: config.needs_setup(),
        is_ready_for_operation: config.is_ready_for_operation(),
        status_message: config.get_status_message(),
        configured_paths_count: config.paths.len() as i32,
    }))
}

pub async fn update_paths(
    State(state): State<AppState>,
    Json(req): Json<UpdatePathsRequest>,
) -> Json<PathMutationResponse> {
    match state.services.config.update_paths(req.paths) {
        Ok((_, warnings)) => Json(PathMutationResponse {
            success: true,
            error_message: None,
            validation_warnings: warnings,
        }),
        Err(e) => Json(PathMutationResponse {
            success: false,
            error_message: Some(e.to_string()),
            validation_warnings: vec![],
        }),
    }
}

pub async fn add_path(
    State(state): State<AppState>,
    Json(req): Json<AddPathRequest>,
) -> Json<PathMutationResponse> {
    match state.services.config.add_path(req.path) {
        Ok((_, warnings)) => Json(PathMutationResponse {
            success: true,
            error_message: None,
            validation_warnings: warnings,
        }),
        Err(e) => Json(PathMutationResponse {
            success: false,
            error_message: Some(e.to_string()),
            validation_warnings: vec![],
        }),
    }
}

pub async fn remove_path(
    State(state): State<AppState>,
    Json(req): Json<RemovePathRequest>,
) -> Json<RemovePathResponse> {
    match state.services.config.remove_path(&req.path) {
        Ok(config) => Json(RemovePathResponse {
            success: true,
            error_message: None,
            remaining_paths_count: config.paths.len() as i32,
        }),
        Err(e) => {
            // Match prior (gRPC) behavior: report the (unchanged) current path
            // count even on failure, rather than leaving the field unset.
            let remaining = state.services.config.get().map(|c| c.paths.len() as i32).unwrap_or(0);
            Json(RemovePathResponse {
                success: false,
                error_message: Some(e.to_string()),
                remaining_paths_count: remaining,
            })
        }
    }
}

pub async fn update_settings(
    State(state): State<AppState>,
    Json(req): Json<UpdateSettingsRequest>,
) -> Json<PathMutationResponse> {
    match state.services.config.update_settings(
        req.database_path,
        req.grpc_port,
        req.log_level,
        req.media_storage_dir,
        req.max_cover_art_size_mb,
        req.max_audio_file_size_mb,
    ) {
        Ok((_, warnings)) => Json(PathMutationResponse {
            success: true,
            error_message: None,
            validation_warnings: warnings,
        }),
        Err(e) => Json(PathMutationResponse {
            success: false,
            error_message: Some(e.to_string()),
            validation_warnings: vec![],
        }),
    }
}

pub async fn reload_config(State(state): State<AppState>) -> Json<ReloadConfigResponse> {
    match state.services.config.reload() {
        Ok((new_config, warnings)) => Json(ReloadConfigResponse {
            success: true,
            error_message: None,
            validation_warnings: warnings,
            config: Some(ConfigDto::from(&new_config)),
        }),
        Err(e) => Json(ReloadConfigResponse {
            success: false,
            error_message: Some(e.to_string()),
            validation_warnings: vec![],
            config: None,
        }),
    }
}

pub async fn validate_config(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let needs_setup = state.services.config.get().map_err(load_error)?.needs_setup();

    Ok(Json(match state.services.config.validate() {
        Ok(warnings) => ValidateConfigResponse {
            is_valid: true,
            warnings,
            errors: vec![],
            needs_setup,
        },
        Err(e) => ValidateConfigResponse {
            is_valid: false,
            warnings: vec![],
            errors: vec![e.to_string()],
            needs_setup,
        },
    }))
}
