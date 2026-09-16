use log::debug;
use tonic::{Code, Request, Response, Status};

use super::super::common::*;
use super::super::plugins::*;
use super::utils::convert_live_set_to_proto;
use crate::services::PluginsService;

#[derive(Clone)]
pub struct PluginsHandler {
    pub service: PluginsService,
}

impl PluginsHandler {
    pub fn new(service: PluginsService) -> Self {
        Self { service }
    }

    pub async fn get_all_plugins(
        &self,
        request: Request<GetAllPluginsRequest>,
    ) -> Result<Response<GetAllPluginsResponse>, Status> {
        debug!("GetAllPlugins request: {:?}", request);
        let req = request.into_inner();

        let (grpc_plugins, total_count) = self
            .service
            .get_all_plugins(
                req.limit,
                req.offset,
                req.sort_by,
                req.sort_desc,
                req.vendor_filter,
                req.format_filter,
                req.installed_only,
                req.min_usage_count,
            )
            .await?;

        let proto_plugins = grpc_plugins
            .into_iter()
            .map(|grpc_plugin| Plugin {
                id: grpc_plugin.plugin.id.to_string(),
                dev_identifier: grpc_plugin.plugin.dev_identifier,
                name: grpc_plugin.plugin.name,
                format: grpc_plugin.plugin.plugin_format.to_string(),
                installed: grpc_plugin.plugin.installed,
                vendor: grpc_plugin.plugin.vendor,
                version: grpc_plugin.plugin.version,
                usage_count: Some(grpc_plugin.usage_count),
                project_count: Some(grpc_plugin.project_count),
            })
            .collect();

        Ok(Response::new(GetAllPluginsResponse {
            plugins: proto_plugins,
            total_count,
        }))
    }

    pub async fn get_plugin_by_installed_status(
        &self,
        request: Request<GetPluginByInstalledStatusRequest>,
    ) -> Result<Response<GetPluginByInstalledStatusResponse>, Status> {
        debug!("GetPluginByInstalledStatus request: {:?}", request);
        let req = request.into_inner();

        let (plugins, total_count) = self
            .service
            .get_plugins_by_installed_status(req.installed, req.limit, req.offset, req.sort_by, req.sort_desc)
            .await?;

        let proto_plugins = plugins
            .into_iter()
            .map(|plugin| Plugin {
                id: plugin.id.to_string(),
                dev_identifier: plugin.dev_identifier,
                name: plugin.name,
                format: plugin.plugin_format.to_string(),
                installed: plugin.installed,
                vendor: plugin.vendor,
                version: plugin.version,
                usage_count: None, // This method doesn't include usage data
                project_count: None,
            })
            .collect();

        Ok(Response::new(GetPluginByInstalledStatusResponse {
            plugins: proto_plugins,
            total_count,
        }))
    }

    pub async fn search_plugins(
        &self,
        request: Request<SearchPluginsRequest>,
    ) -> Result<Response<SearchPluginsResponse>, Status> {
        debug!("SearchPlugins request: {:?}", request);
        let req = request.into_inner();

        let (plugins, total_count) = self
            .service
            .search_plugins(
                &req.query,
                req.limit,
                req.offset,
                req.installed_only,
                req.vendor_filter,
                req.format_filter,
            )
            .await?;

        let proto_plugins = plugins
            .into_iter()
            .map(|plugin| Plugin {
                id: plugin.id.to_string(),
                dev_identifier: plugin.dev_identifier,
                name: plugin.name,
                format: plugin.plugin_format.to_string(),
                installed: plugin.installed,
                vendor: plugin.vendor,
                version: plugin.version,
                usage_count: None,
                project_count: None,
            })
            .collect();

        Ok(Response::new(SearchPluginsResponse {
            plugins: proto_plugins,
            total_count,
        }))
    }

    pub async fn get_plugin_stats(
        &self,
        _request: Request<GetPluginStatsRequest>,
    ) -> Result<Response<GetPluginStatsResponse>, Status> {
        debug!("GetPluginStats request");

        let stats = self.service.get_plugin_stats().await?;

        Ok(Response::new(GetPluginStatsResponse {
            total_plugins: stats.total_plugins,
            installed_plugins: stats.installed_plugins,
            missing_plugins: stats.missing_plugins,
            unknown_plugins: stats.unknown_plugins,
            unique_vendors: stats.unique_vendors,
            plugins_by_format: stats.plugins_by_format,
            plugins_by_vendor: stats.plugins_by_vendor,
        }))
    }

    pub async fn get_plugin_vendors(
        &self,
        request: Request<GetPluginVendorsRequest>,
    ) -> Result<Response<GetPluginVendorsResponse>, Status> {
        debug!("GetPluginVendors request: {:?}", request);
        let req = request.into_inner();

        let (vendors, total_count) = self
            .service
            .get_plugin_vendors(req.limit, req.offset, req.sort_by, req.sort_desc)
            .await?;

        let proto_vendors = vendors
            .into_iter()
            .map(|vendor| VendorInfo {
                vendor: vendor.vendor,
                plugin_count: vendor.plugin_count,
                installed_plugins: vendor.installed_plugins,
                missing_plugins: vendor.missing_plugins,
                total_usage_count: vendor.total_usage_count,
                unique_projects_using: vendor.unique_projects_using,
                plugins_by_format: vendor.plugins_by_format,
            })
            .collect();

        Ok(Response::new(GetPluginVendorsResponse {
            vendors: proto_vendors,
            total_count,
        }))
    }

    pub async fn get_plugin_formats(
        &self,
        request: Request<GetPluginFormatsRequest>,
    ) -> Result<Response<GetPluginFormatsResponse>, Status> {
        debug!("GetPluginFormats request: {:?}", request);
        let req = request.into_inner();

        let (formats, total_count) = self
            .service
            .get_plugin_formats(req.limit, req.offset, req.sort_by, req.sort_desc)
            .await?;

        let proto_formats = formats
            .into_iter()
            .map(|format| FormatInfo {
                format: format.format,
                plugin_count: format.plugin_count,
                installed_plugins: format.installed_plugins,
                missing_plugins: format.missing_plugins,
                total_usage_count: format.total_usage_count,
                unique_projects_using: format.unique_projects_using,
                plugins_by_vendor: format.plugins_by_vendor,
            })
            .collect();

        Ok(Response::new(GetPluginFormatsResponse {
            formats: proto_formats,
            total_count,
        }))
    }

    pub async fn get_plugin(
        &self,
        request: Request<GetPluginRequest>,
    ) -> Result<Response<GetPluginResponse>, Status> {
        debug!("GetPlugin request: {:?}", request);
        let req = request.into_inner();

        match self.service.get_plugin(&req.plugin_id).await? {
            Some(grpc_plugin) => {
                let proto_plugin = Plugin {
                    id: grpc_plugin.plugin.id.to_string(),
                    dev_identifier: grpc_plugin.plugin.dev_identifier,
                    name: grpc_plugin.plugin.name,
                    format: grpc_plugin.plugin.plugin_format.to_string(),
                    installed: grpc_plugin.plugin.installed,
                    vendor: grpc_plugin.plugin.vendor,
                    version: grpc_plugin.plugin.version,
                    usage_count: Some(grpc_plugin.usage_count),
                    project_count: Some(grpc_plugin.project_count),
                };

                Ok(Response::new(GetPluginResponse {
                    plugin: Some(proto_plugin),
                    usage_count: grpc_plugin.usage_count,
                    project_count: grpc_plugin.project_count,
                }))
            }
            None => Err(Status::new(
                Code::NotFound,
                format!("Plugin with ID {} not found", req.plugin_id),
            )),
        }
    }

    pub async fn get_projects_by_plugin(
        &self,
        request: Request<GetProjectsByPluginRequest>,
    ) -> Result<Response<GetProjectsByPluginResponse>, Status> {
        debug!("GetProjectsByPlugin request: {:?}", request);
        let req = request.into_inner();

        let (projects, total_count) = self
            .service
            .get_projects_by_plugin(&req.plugin_id, req.limit, req.offset)
            .await?;

        let db_arc = self.service.db_handle();
        let mut db = db_arc.lock().await;
        let mut proto_projects = Vec::new();
        for project in projects {
            match convert_live_set_to_proto(project, &mut db) {
                Ok(proto_project) => proto_projects.push(proto_project),
                Err(e) => {
                    return Err(Status::internal(format!("Database error: {}", e)));
                }
            }
        }

        Ok(Response::new(GetProjectsByPluginResponse {
            projects: proto_projects,
            total_count,
        }))
    }

    pub async fn refresh_plugin_installation_status(
        &self,
        _request: Request<RefreshPluginInstallationStatusRequest>,
    ) -> Result<Response<RefreshPluginInstallationStatusResponse>, Status> {
        debug!("RefreshPluginInstallationStatus request");

        let result = self.service.refresh_plugin_installation_status().await?;

        Ok(Response::new(RefreshPluginInstallationStatusResponse {
            candidates_scanned: result.candidates_scanned,
            plugins_installed: result.plugins_installed,
            plugins_missing: result.plugins_missing,
            plugins_reconciled: result.plugins_reconciled,
            scan_failures: result.scan_failures,
            success: true,
            error_message: None,
        }))
    }
}
