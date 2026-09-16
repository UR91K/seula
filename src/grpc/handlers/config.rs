use log::{debug, error};
use tonic::{Code, Request, Response, Status};

use super::super::config::*;
use crate::config::Config;
use crate::services::ConfigService;

#[derive(Clone)]
pub struct ConfigHandler {
    pub service: ConfigService,
}

impl ConfigHandler {
    pub fn new(service: ConfigService) -> Self {
        Self { service }
    }

    fn to_proto(config: &Config) -> ConfigData {
        ConfigData {
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

    pub async fn get_config(
        &self,
        _request: Request<GetConfigRequest>,
    ) -> Result<Response<GetConfigResponse>, Status> {
        debug!("GetConfig request");

        let config = self
            .service
            .get()
            .map_err(|e| Status::new(Code::Internal, format!("Failed to load config: {}", e)))?;

        Ok(Response::new(GetConfigResponse {
            config: Some(Self::to_proto(config)),
        }))
    }

    pub async fn get_config_status(
        &self,
        _request: Request<GetConfigStatusRequest>,
    ) -> Result<Response<GetConfigStatusResponse>, Status> {
        debug!("GetConfigStatus request");

        let config = self
            .service
            .get()
            .map_err(|e| Status::new(Code::Internal, format!("Failed to load config: {}", e)))?;

        Ok(Response::new(GetConfigStatusResponse {
            needs_setup: config.needs_setup(),
            is_ready_for_operation: config.is_ready_for_operation(),
            status_message: config.get_status_message(),
            configured_paths_count: config.paths.len() as i32,
        }))
    }

    pub async fn update_paths(
        &self,
        request: Request<UpdatePathsRequest>,
    ) -> Result<Response<UpdatePathsResponse>, Status> {
        debug!("UpdatePaths request: {:?}", request);
        let req = request.into_inner();

        match self.service.update_paths(req.paths) {
            Ok((_, warnings)) => {
                debug!("Successfully updated paths");
                Ok(Response::new(UpdatePathsResponse {
                    success: true,
                    error_message: None,
                    validation_warnings: warnings,
                }))
            }
            Err(e) => {
                error!("Failed to update paths: {:?}", e);
                Ok(Response::new(UpdatePathsResponse {
                    success: false,
                    error_message: Some(e.to_string()),
                    validation_warnings: vec![],
                }))
            }
        }
    }

    pub async fn add_path(
        &self,
        request: Request<AddPathRequest>,
    ) -> Result<Response<AddPathResponse>, Status> {
        debug!("AddPath request: {:?}", request);
        let req = request.into_inner();

        match self.service.add_path(req.path) {
            Ok((_, warnings)) => {
                debug!("Successfully added path");
                Ok(Response::new(AddPathResponse {
                    success: true,
                    error_message: None,
                    validation_warnings: warnings,
                }))
            }
            Err(e) => {
                error!("Failed to add path: {:?}", e);
                Ok(Response::new(AddPathResponse {
                    success: false,
                    error_message: Some(e.to_string()),
                    validation_warnings: vec![],
                }))
            }
        }
    }

    pub async fn remove_path(
        &self,
        request: Request<RemovePathRequest>,
    ) -> Result<Response<RemovePathResponse>, Status> {
        debug!("RemovePath request: {:?}", request);
        let req = request.into_inner();

        match self.service.remove_path(&req.path) {
            Ok(config) => {
                debug!("Successfully removed path");
                Ok(Response::new(RemovePathResponse {
                    success: true,
                    error_message: None,
                    remaining_paths_count: config.paths.len() as i32,
                }))
            }
            Err(e) => {
                error!("Failed to remove path: {:?}", e);
                // Match prior behavior: report the (unchanged) current path count
                // even on failure, rather than leaving the field unset.
                let remaining = self.service.get().map(|c| c.paths.len() as i32).unwrap_or(0);
                Ok(Response::new(RemovePathResponse {
                    success: false,
                    error_message: Some(e.to_string()),
                    remaining_paths_count: remaining,
                }))
            }
        }
    }

    pub async fn update_settings(
        &self,
        request: Request<UpdateSettingsRequest>,
    ) -> Result<Response<UpdateSettingsResponse>, Status> {
        debug!("UpdateSettings request: {:?}", request);
        let req = request.into_inner();

        match self.service.update_settings(
            req.database_path,
            req.grpc_port.map(|p| p as u16),
            req.log_level,
            req.media_storage_dir,
            req.max_cover_art_size_mb.map(Some),
            req.max_audio_file_size_mb.map(Some),
        ) {
            Ok((_, warnings)) => {
                debug!("Successfully updated settings");
                Ok(Response::new(UpdateSettingsResponse {
                    success: true,
                    error_message: None,
                    validation_warnings: warnings,
                }))
            }
            Err(e) => {
                error!("Failed to update settings: {:?}", e);
                Ok(Response::new(UpdateSettingsResponse {
                    success: false,
                    error_message: Some(e.to_string()),
                    validation_warnings: vec![],
                }))
            }
        }
    }

    pub async fn reload_config(
        &self,
        _request: Request<ReloadConfigRequest>,
    ) -> Result<Response<ReloadConfigResponse>, Status> {
        debug!("ReloadConfig request");

        match self.service.reload() {
            Ok((new_config, warnings)) => {
                debug!("Successfully reloaded config");
                Ok(Response::new(ReloadConfigResponse {
                    success: true,
                    error_message: None,
                    validation_warnings: warnings,
                    config: Some(Self::to_proto(&new_config)),
                }))
            }
            Err(e) => {
                error!("Failed to reload config: {:?}", e);
                Ok(Response::new(ReloadConfigResponse {
                    success: false,
                    error_message: Some(e.to_string()),
                    validation_warnings: vec![],
                    config: None,
                }))
            }
        }
    }

    pub async fn validate_config(
        &self,
        _request: Request<ValidateConfigRequest>,
    ) -> Result<Response<ValidateConfigResponse>, Status> {
        debug!("ValidateConfig request");

        let needs_setup = self
            .service
            .get()
            .map_err(|e| Status::new(Code::Internal, format!("Failed to load config: {}", e)))?
            .needs_setup();

        match self.service.validate() {
            Ok(warnings) => {
                debug!("Config validation successful with {} warnings", warnings.len());
                Ok(Response::new(ValidateConfigResponse {
                    is_valid: true,
                    warnings,
                    errors: vec![],
                    needs_setup,
                }))
            }
            Err(e) => {
                debug!("Config validation failed: {:?}", e);
                Ok(Response::new(ValidateConfigResponse {
                    is_valid: false,
                    warnings: vec![],
                    errors: vec![e.to_string()],
                    needs_setup,
                }))
            }
        }
    }
}
