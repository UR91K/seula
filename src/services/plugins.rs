use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

use crate::database::plugin_details::PluginDetails;
use crate::database::plugins::{
    FormatInfo, InstallState, PluginFilter, PluginRefreshResult, PluginStats, VendorInfo,
};
use crate::config::CONFIG;
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::models::GrpcPlugin;
use crate::project::Project;
use crate::scan::plugins::{scan_system_with_progress, ScanReport};

#[derive(Clone)]
pub struct PluginsService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl PluginsService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    /// How many projects use each plugin, keyed by id (ADR-0034).
    pub async fn project_counts(
        &self,
        ids: &[String],
    ) -> Result<std::collections::HashMap<String, i32>, DatabaseError> {
        self.db.lock().await.plugin_project_counts(ids)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_all_plugins(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        vendor_filter: Option<String>,
        format_filter: Option<String>,
        install_states: &[InstallState],
        min_usage_count: Option<i32>,
    ) -> Result<(Vec<GrpcPlugin>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_all_plugins(
            limit,
            offset,
            sort_by,
            sort_desc,
            vendor_filter,
            format_filter,
            install_states,
            min_usage_count,
        )
    }

    pub async fn get_plugins_by_installed_status(
        &self,
        install_states: &[InstallState],
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<crate::models::Plugin>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugins_by_installed_status(install_states, limit, offset, sort_by, sort_desc)
    }

    pub async fn search_plugins(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        install_states: &[InstallState],
        vendor_filter: Option<String>,
        format_filter: Option<String>,
    ) -> Result<(Vec<crate::models::Plugin>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.search_plugins(query, limit, offset, install_states, vendor_filter, format_filter)
    }

    pub async fn get_plugin_stats(&self) -> Result<PluginStats, DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_stats()
    }

    /// The same counts over only the plugins `filter` selects.
    pub async fn get_plugin_stats_filtered(&self, filter: &PluginFilter) -> Result<PluginStats, DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_stats_filtered(filter)
    }

    pub async fn get_plugin_details(&self, plugin_id: &str) -> Result<Option<PluginDetails>, DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_details(plugin_id)
    }

    pub async fn get_plugin_vendors(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<VendorInfo>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_vendors(limit, offset, sort_by, sort_desc)
    }

    pub async fn get_plugin_formats(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<FormatInfo>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_formats(limit, offset, sort_by, sort_desc)
    }

    pub async fn get_plugin(&self, plugin_id: &str) -> Result<Option<GrpcPlugin>, DatabaseError> {
        let db = self.db.lock().await;
        db.get_plugin_by_id(plugin_id)
    }

    pub async fn get_projects_by_plugin(
        &self,
        plugin_id: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<Project>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_projects_by_plugin_id(plugin_id, limit, offset)
    }

    /// Rescan the system's plugins and record the result. Returns when the scan is
    /// done, which takes minutes; the database is locked only to write the result
    /// (ADR-0038). The HTTP API's background scan is `SystemService::start_plugin_scan`.
    pub async fn refresh_plugin_installation_status(&self) -> Result<PluginRefreshResult, DatabaseError> {
        let report = tokio::task::spawn_blocking(|| scan_configured_plugins(&mut |_, _, _| {}))
            .await
            .map_err(|e| DatabaseError::ConnectionError(format!("Plugin scan task failed: {}", e)))??;
        self.db.lock().await.record_plugin_refresh(&report)
    }
}

/// Scan the configured plugin search paths with the configured timeout, reporting each
/// plugin as it is attempted. Blocking and slow: run it on a blocking thread, and never
/// while holding the database.
pub(crate) fn scan_configured_plugins(
    on_progress: &mut dyn FnMut(usize, usize, &Path),
) -> Result<ScanReport, DatabaseError> {
    let config = CONFIG
        .as_ref()
        .map_err(|e| DatabaseError::ConfigError(e.clone()))?;
    let roots: Vec<PathBuf> = config.vst_search_paths.iter().map(PathBuf::from).collect();
    let timeout = Duration::from_secs(config.vst_scan_timeout_secs);
    scan_system_with_progress(&roots, timeout, on_progress)
        .map_err(|e| DatabaseError::ConnectionError(e.to_string()))
}
