use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::samples::{
    ExtensionAnalytics, SampleAnalytics, SampleFilter, SampleRefreshResult, SampleStats, SampleUsageInfo,
};
use crate::database::{ProjectDatabase, ProjectScope};
use crate::error::DatabaseError;
use crate::models::Sample;
use crate::project::Project;
use crate::scan::sample_check::{check_sample_files, default_threads};

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

    /// How many projects use each sample, keyed by id (ADR-0034).
    pub async fn project_counts(
        &self,
        ids: &[String],
        scope: ProjectScope,
    ) -> Result<std::collections::HashMap<String, i32>, DatabaseError> {
        self.db.lock().await.sample_project_counts(ids, scope)
    }

    /// Sizes the last sample check measured, keyed by id (ADR-0041).
    pub async fn sizes(&self, ids: &[String]) -> Result<std::collections::HashMap<String, i64>, DatabaseError> {
        self.db.lock().await.sample_sizes(ids)
    }

    /// Sample counts over the samples `filter` selects.
    pub async fn get_sample_stats_filtered(&self, filter: &SampleFilter) -> Result<SampleStats, DatabaseError> {
        self.db.lock().await.get_sample_stats_filtered(filter)
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
        format_filter: Option<String>,
        min_usage_count: Option<i32>,
        max_usage_count: Option<i32>,
        scope: ProjectScope,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_all_samples(
            limit,
            offset,
            sort_by,
            sort_desc,
            present_only,
            missing_only,
            format_filter,
            min_usage_count,
            max_usage_count,
            scope,
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
        format_filter: Option<String>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.search_samples(query, limit, offset, present_only, format_filter)
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
        scope: ProjectScope,
    ) -> Result<(Vec<Project>, i32), DatabaseError> {
        let db = self.db.lock().await;
        db.get_projects_by_sample_id(sample_id, limit, offset, scope)
    }

    /// Check every sample file and record what was found, answering when it is done.
    /// The database is locked only to read the paths and to write the result
    /// (ADR-0041). The HTTP API's background check is `SystemService::start_sample_check`.
    pub async fn refresh_sample_presence_status(&self) -> Result<SampleRefreshResult, DatabaseError> {
        let paths = self.db.lock().await.sample_paths()?;
        let found = tokio::task::spawn_blocking(move || {
            check_sample_files(&paths, default_threads(), &mut |_, _, _| {})
        })
        .await
        .map_err(|e| DatabaseError::ConnectionError(format!("Sample check task failed: {}", e)))?;
        self.db.lock().await.record_sample_check(&found)
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
