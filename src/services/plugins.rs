use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::plugins::{
    FormatInfo, InstallState, PluginRefreshResult, PluginStats, VendorInfo,
};
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::models::GrpcPlugin;
use crate::project::Project;

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

    pub async fn refresh_plugin_installation_status(&self) -> Result<PluginRefreshResult, DatabaseError> {
        let mut db = self.db.lock().await;
        db.refresh_plugin_installation_status()
    }
}
