//! Plugins domain HTTP handlers (ADR-0024). Thin over `PluginsService`,
//! mirroring `src/grpc/handlers/plugins.rs`.

use std::convert::Infallible;

use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Json;
use tokio_stream::wrappers::ReceiverStream;

use crate::http::dto::plugins::{
    parse_install_states, plugin_sort_key, ByInstalledStatusQuery, FormatListResponse,
    GetAllPluginsQuery, GetPluginResponse, PaginationQuery, PluginDto, PluginListResponse,
    PluginScanEventDto, PluginStatsQuery, ProjectsByPluginQuery, ScopeQuery, SearchPluginsQuery,
    VendorListResponse,
};
use crate::database::plugins::PluginFilter;
use crate::database::ProjectScope;
use crate::http::dto::parse_project_scope;
use crate::http::dto::projects::{project_to_dto, ProjectListResponse};
use crate::http::error::ApiError;
use crate::http::state::AppState;
use crate::models::Plugin as DomainPlugin;

/// Attach each plugin's project count with one query for the page (ADR-0034), counting
/// the projects in `scope` (ADR-0040).
async fn with_counts(
    state: &AppState,
    plugins: Vec<DomainPlugin>,
    scope: ProjectScope,
) -> Result<Vec<PluginDto>, ApiError> {
    let ids: Vec<String> = plugins.iter().map(|p| p.id.to_string()).collect();
    let counts = state.services.plugins.project_counts(&ids, scope).await?;
    Ok(plugins
        .into_iter()
        .map(|p| {
            let count = counts.get(&p.id.to_string()).copied().unwrap_or(0);
            PluginDto::new(p, count)
        })
        .collect())
}

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
            plugin_sort_key(query.sort_by),
            query.sort_desc,
            query.vendor_filter,
            query.format_filter,
            &install_states,
            query.min_project_count,
            parse_project_scope(query.scope.as_deref())?,
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
    let scope = parse_project_scope(query.scope.as_deref())?;

    let (plugins, total_count) = state
        .services
        .plugins
        .get_plugins_by_installed_status(
            &install_states,
            query.limit,
            query.offset,
            plugin_sort_key(query.sort_by),
            query.sort_desc,
        )
        .await?;

    Ok(Json(PluginListResponse {
        plugins: with_counts(&state, plugins, scope).await?,
        total_count,
    }))
}

pub async fn search_plugins(
    State(state): State<AppState>,
    Query(query): Query<SearchPluginsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let install_states = parse_install_states(query.install_states.as_deref());
    let scope = parse_project_scope(query.scope.as_deref())?;

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
        plugins: with_counts(&state, plugins, scope).await?,
        total_count,
    }))
}

/// Counts over the plugins the same filters list, or over every plugin with none.
pub async fn get_plugin_stats(
    State(state): State<AppState>,
    Query(query): Query<PluginStatsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let filter = PluginFilter {
        query: query.query,
        vendor: query.vendor_filter,
        format: query.format_filter,
        install_states: parse_install_states(query.install_states.as_deref()),
    };
    let stats = state.services.plugins.get_plugin_stats_filtered(&filter).await?;
    Ok(Json(stats))
}

pub async fn get_plugin_vendors(
    State(state): State<AppState>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (vendors, total_count) = state
        .services
        .plugins
        .get_plugin_vendors(
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
            parse_project_scope(query.scope.as_deref())?,
        )
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
        .get_plugin_formats(
            query.limit,
            query.offset,
            query.sort_by,
            query.sort_desc,
            parse_project_scope(query.scope.as_deref())?,
        )
        .await?;
    Ok(Json(FormatListResponse { formats, total_count }))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(plugin_id): Path<String>,
    Query(query): Query<ScopeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let scope = parse_project_scope(query.scope.as_deref())?;
    let not_found = || ApiError::NotFound(format!("Plugin with ID {} not found", plugin_id));
    let grpc_plugin = state.services.plugins.get_plugin(&plugin_id, scope).await?.ok_or_else(not_found)?;
    let details = state
        .services
        .plugins
        .get_plugin_details(&plugin_id)
        .await?
        .ok_or_else(not_found)?;

    Ok(Json(GetPluginResponse {
        plugin: PluginDto::from(grpc_plugin),
        details,
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
        .get_projects_by_plugin(&plugin_id, query.limit, query.offset, parse_project_scope(query.scope.as_deref())?)
        .await?;

    let db_arc = state.services.plugins.db_handle();
    let mut db = db_arc.lock().await;
    let projects = projects
        .into_iter()
        .map(|p| project_to_dto(p, &mut db))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ProjectListResponse { projects, total_count }))
}

/// Rescan the system's plugins in the background, streaming progress as Server-Sent
/// Events like `POST /api/v1/system/scan` (ADR-0038). The progress also shows at
/// `GET /api/v1/system/scan-status`, so a client that did not start the scan can follow
/// it. 409 when a scan is already running.
pub async fn scan_plugins(
    State(state): State<AppState>,
) -> Result<Sse<ReceiverStream<Result<Event, Infallible>>>, ApiError> {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    let started = state
        .system
        .start_plugin_scan(move |response, result| {
            let dto = PluginScanEventDto {
                progress: response.into(),
                result,
            };
            if let Ok(json) = serde_json::to_string(&dto) {
                let _ = tx.try_send(Ok(Event::default().data(json)));
            }
        })
        .await;
    if !started {
        return Err(ApiError::Conflict("A scan is already running".to_string()));
    }

    Ok(Sse::new(ReceiverStream::new(rx)).keep_alive(KeepAlive::default()))
}

/// The same rescan, answered only when it is finished (minutes on a real library).
/// Kept for scripts; the plugins view uses `scan_plugins`.
pub async fn refresh_plugin_installation_status(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state.services.plugins.refresh_plugin_installation_status().await?;
    Ok(Json(result))
}
