//! Plugins domain HTTP handlers (ADR-0024). Thin over `PluginsService`,
//! mirroring `src/grpc/handlers/plugins.rs`.

use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use axum::Json;

use crate::http::dto::plugins::{
    parse_install_states, ByInstalledStatusQuery, FormatListResponse, GetAllPluginsQuery,
    GetPluginResponse, PaginationQuery, PluginDto, PluginListResponse, ProjectsByPluginQuery,
    SearchPluginsQuery, VendorListResponse,
};
use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::error::ApiError;
use crate::http::state::AppState;

pub async fn get_all_plugins(
    State(state): State<AppState>,
    Query(query): Query<GetAllPluginsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let install_states = parse_install_states(query.install_states.as_deref());

    let (plugins, total_count) = state
        .services
        .plugins
        .get_all_plugins(
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
            query.vendor_filter,
            query.format_filter,
            &install_states,
            query.min_usage_count,
        )
        .await?;

    Ok(Json(PluginListResponse {
        plugins: plugins.into_iter().map(PluginDto::from).collect(),
        total_count,
    }))
}

pub async fn get_plugins_by_installed_status(
    State(state): State<AppState>,
    Query(query): Query<ByInstalledStatusQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let install_states = parse_install_states(query.install_states.as_deref());

    let (plugins, total_count) = state
        .services
        .plugins
        .get_plugins_by_installed_status(
            &install_states,
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
        )
        .await?;

    Ok(Json(PluginListResponse {
        plugins: plugins.into_iter().map(PluginDto::from).collect(),
        total_count,
    }))
}

pub async fn search_plugins(
    State(state): State<AppState>,
    Query(query): Query<SearchPluginsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let install_states = parse_install_states(query.install_states.as_deref());

    let (plugins, total_count) = state
        .services
        .plugins
        .search_plugins(
            &query.query,
            query.limit,
            query.offset,
            &install_states,
            query.vendor_filter,
            query.format_filter,
        )
        .await?;

    Ok(Json(PluginListResponse {
        plugins: plugins.into_iter().map(PluginDto::from).collect(),
        total_count,
    }))
}

pub async fn get_plugin_stats(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let stats = state.services.plugins.get_plugin_stats().await?;
    Ok(Json(stats))
}

pub async fn get_plugin_vendors(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (vendors, total_count) = state
        .services
        .plugins
        .get_plugin_vendors(query.limit, query.offset, query.sort_by, query.sort_desc)
        .await?;
    Ok(Json(VendorListResponse { vendors, total_count }))
}

pub async fn get_plugin_formats(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (formats, total_count) = state
        .services
        .plugins
        .get_plugin_formats(query.limit, query.offset, query.sort_by, query.sort_desc)
        .await?;
    Ok(Json(FormatListResponse { formats, total_count }))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let grpc_plugin = state
        .services
        .plugins
        .get_plugin(&plugin_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Plugin with ID {} not found", plugin_id)))?;

    let usage_count = grpc_plugin.usage_count;
    let project_count = grpc_plugin.project_count;

    Ok(Json(GetPluginResponse {
        plugin: PluginDto::from(grpc_plugin),
        usage_count,
        project_count,
    }))
}

pub async fn get_projects_by_plugin(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    Query(query): Query<ProjectsByPluginQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (projects, total_count) = state
        .services
        .plugins
        .get_projects_by_plugin(&plugin_id, query.limit, query.offset)
        .await?;

    let db_arc = state.services.plugins.db_handle();
    let mut db = db_arc.lock().await;
    let projects = projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ProjectListResponse { projects, total_count }))
}

pub async fn refresh_plugin_installation_status(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.services.plugins.refresh_plugin_installation_status().await?;
    Ok(Json(result))
}
