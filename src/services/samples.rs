use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::samples::{
    ExtensionAnalytics, SampleAnalytics, SampleRefreshResult, SampleStats, SampleUsageInfo,
};
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::models::Sample;
use crate::project::Project;

#[derive(Clone)]
pub struct SamplesService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl SamplesService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_all_samples(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        present_only: Option<bool>,
        missing_only: Option<bool>,
        extension_filter: Option<String>,
        min_usage_count: Option<i32>,
        max_usage_count: Option<i32>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_all_samples(
            limit,
            offset,
            sort_by,
            sort_desc,
            present_only,
            missing_only,
            extension_filter,
            min_usage_count,
            max_usage_count,
        )
    }

    pub async fn get_sample(&self, sample_id: &str) -> Result<Option<Sample>, DatabaseError> {
        let db = self.db.lock().await;
        db.get_sample_by_id(sample_id)
    }

    pub async fn get_samples_by_presence(
        &self,
        is_present: bool,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_samples_by_presence(is_present, limit, offset, sort_by, sort_desc)
    }

    pub async fn search_samples(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        present_only: Option<bool>,
        extension_filter: Option<String>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.search_samples(query, limit, offset, present_only, extension_filter)
    }

    pub async fn get_sample_stats(&self) -> Result<SampleStats, DatabaseError> {
        let db = self.db.lock().await;
        db.get_sample_stats()
    }

    pub async fn get_all_sample_usage_numbers(&self) -> Result<Vec<SampleUsageInfo>, DatabaseError> {
        let db = self.db.lock().await;
        db.get_all_sample_usage_numbers()
    }

    pub async fn get_projects_by_sample(
        &self,
        sample_id: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<Project>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_projects_by_sample_id(sample_id, limit, offset)
    }

    pub async fn refresh_sample_presence_status(&self) -> Result<SampleRefreshResult, DatabaseError> {
        let mut db = self.db.lock().await;
        db.refresh_sample_presence_status()
    }

    pub async fn get_sample_analytics(&self) -> Result<SampleAnalytics, DatabaseError> {
        let db = self.db.lock().await;
        db.get_sample_analytics()
    }

    pub async fn get_sample_extensions(
        &self,
    ) -> Result<std::collections::HashMap<String, ExtensionAnalytics>, DatabaseError> {
        let db = self.db.lock().await;
        db.get_sample_extensions()
    }
}
