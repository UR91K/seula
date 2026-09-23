//! HTTP wire types for the plugins domain (ADR-0024).
//!
//! `PluginStats`, `VendorInfo`, `FormatInfo` and `PluginRefreshResult`
//! (`src/database/plugins.rs`) already derive `Serialize` and are returned
//! as-is, same reasoning as the other domains' reuse of already-serde types.

use serde::{Deserialize, Serialize};

use crate::database::plugins::InstallState;
use crate::models::{GrpcPlugin, Plugin as DomainPlugin};

/// Parses a comma-separated `install_states` query value into the database
/// layer's tri-state set (ADR-0025). Unrecognised values are dropped rather
/// than rejected, matching `src/grpc/handlers/plugins.rs::install_states`: an
/// empty set means "no filtering", which is what a request naming no state (or
/// only invalid ones) is asking for.
pub fn parse_install_states(raw: Option<&str>) -> Vec<InstallState> {
    let Some(raw) = raw else {
        return Vec::new();
    };
    raw.split(',')
        .filter_map(|s| match s.trim() {
            "installed" => Some(InstallState::Installed),
            "absent" => Some(InstallState::Absent),
            "unscanned" => Some(InstallState::Unscanned),
            _ => None,
        })
        .collect()
}

/// `project_count` is always populated (ADR-0034); the old `usage_count` always equalled
/// it and is gone.
#[derive(Serialize)]
pub struct PluginDto {
    pub id: String,
    pub dev_identifier: String,
    pub name: String,
    pub format: String,
    pub installed: Option<bool>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub project_count: i32,
}

impl From<GrpcPlugin> for PluginDto {
    fn from(grpc_plugin: GrpcPlugin) -> Self {
        Self {
            id: grpc_plugin.plugin.id.to_string(),
            dev_identifier: grpc_plugin.plugin.dev_identifier,
            name: grpc_plugin.plugin.name,
            format: grpc_plugin.plugin.plugin_format.to_string(),
            installed: grpc_plugin.plugin.installed,
            vendor: grpc_plugin.plugin.vendor,
            version: grpc_plugin.plugin.version,
            project_count: grpc_plugin.project_count,
        }
    }
}

impl PluginDto {
    /// For the routes that return plain plugins; the handler supplies the count.
    pub fn new(plugin: DomainPlugin, project_count: i32) -> Self {
        Self {
            id: plugin.id.to_string(),
            dev_identifier: plugin.dev_identifier,
            name: plugin.name,
            format: plugin.plugin_format.to_string(),
            installed: plugin.installed,
            vendor: plugin.vendor,
            version: plugin.version,
            project_count,
        }
    }
}

#[derive(Deserialize)]
pub struct GetAllPluginsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
    pub vendor_filter: Option<String>,
    pub format_filter: Option<String>,
    pub install_states: Option<String>,
    pub min_project_count: Option<i32>,
}

/// Maps the HTTP sort key onto the database layer's, which still calls it
/// `usage_count` (ADR-0034).
pub fn plugin_sort_key(sort_by: Option<String>) -> Option<String> {
    sort_by.map(|s| if s == "project_count" { "usage_count".to_string() } else { s })
}

#[derive(Deserialize)]
pub struct ByInstalledStatusQuery {
    pub install_states: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchPluginsQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub install_states: Option<String>,
    pub vendor_filter: Option<String>,
    pub format_filter: Option<String>,
}

#[derive(Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub sort_by: Option<String>,
    pub sort_desc: Option<bool>,
}

#[derive(Serialize)]
pub struct VendorListResponse {
    pub vendors: Vec<crate::database::plugins::VendorInfo>,
    pub total_count: i32,
}

#[derive(Serialize)]
pub struct FormatListResponse {
    pub formats: Vec<crate::database::plugins::FormatInfo>,
    pub total_count: i32,
}

#[derive(Serialize)]
pub struct GetPluginResponse {
    pub plugin: PluginDto,
}

#[derive(Deserialize)]
pub struct ProjectsByPluginQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}
