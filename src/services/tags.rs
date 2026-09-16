use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::tags::{TagStatistics, TagUsageInfo};
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::project::Project;

pub type TagRow = (String, String, i64);

#[derive(Clone)]
pub struct TagsService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl TagsService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    /// Escape hatch for callers that need a raw `&mut ProjectDatabase` lock, e.g.
    /// gRPC's LiveSet-to-proto conversion helper. Prefer the typed methods above.
    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    pub async fn list_tags(&self) -> Result<Vec<TagRow>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.list_tags()
    }

    pub async fn create_tag(&self, name: &str) -> Result<TagRow, DatabaseError> {
        let mut db = self.db.lock().await;
        let tag_id = db.add_tag(name)?;
        db.get_tag_by_id(&tag_id)?.ok_or_else(|| {
            DatabaseError::NotFound(format!("Created tag {} not found after creation", tag_id))
        })
    }

    pub async fn update_tag(&self, tag_id: &str, name: &str) -> Result<TagRow, DatabaseError> {
        let mut db = self.db.lock().await;
        db.update_tag(tag_id, name)?;
        db.get_tag_by_id(tag_id)?.ok_or_else(|| {
            DatabaseError::NotFound(format!("Tag {} not found after update", tag_id))
        })
    }

    pub async fn delete_tag(&self, tag_id: &str) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        db.remove_tag(tag_id)
    }

    pub async fn get_tag(&self, tag_id: &str) -> Result<Option<TagRow>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_tag_by_id(tag_id)
    }

    pub async fn search_tags(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<TagRow>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        db.search_tags(query, limit, offset)
    }

    pub async fn get_projects_by_tag(&self, tag_id: &str) -> Result<Vec<Project>, DatabaseError> {
        let mut db = self.db.lock().await;
        Self::require_tag(&mut db, tag_id)?;
        db.get_projects_by_tag(tag_id)
    }

    pub async fn get_tag_statistics(&self) -> Result<TagStatistics, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_tag_statistics()
    }

    pub async fn get_all_tags_with_usage(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        min_usage_count: Option<i32>,
    ) -> Result<(Vec<TagUsageInfo>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_all_tags_with_usage(limit, offset, sort_by, sort_desc, min_usage_count)
    }

    /// Tags a project, validating both sides exist first — unlike the raw database
    /// method (`INSERT OR IGNORE`), which silently no-ops on a nonexistent tag or
    /// project. Returns the tag row so callers that need the name (e.g. CLI display)
    /// don't have to fetch it again.
    pub async fn tag_project(&self, project_id: &str, tag_id: &str) -> Result<TagRow, DatabaseError> {
        let mut db = self.db.lock().await;
        Self::require_project(&mut db, project_id)?;
        let tag = Self::require_tag(&mut db, tag_id)?;
        db.tag_project(project_id, tag_id)?;
        Ok(tag)
    }

    pub async fn untag_project(&self, project_id: &str, tag_id: &str) -> Result<TagRow, DatabaseError> {
        let mut db = self.db.lock().await;
        Self::require_project(&mut db, project_id)?;
        let tag = Self::require_tag(&mut db, tag_id)?;
        db.untag_project(project_id, tag_id)?;
        Ok(tag)
    }

    pub async fn batch_tag_projects(
        &self,
        project_ids: &[String],
        tag_ids: &[String],
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_tag_projects(project_ids, tag_ids)
    }

    pub async fn batch_untag_projects(
        &self,
        project_ids: &[String],
        tag_ids: &[String],
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_untag_projects(project_ids, tag_ids)
    }

    fn require_tag(db: &mut ProjectDatabase, tag_id: &str) -> Result<TagRow, DatabaseError> {
        db.get_tag_by_id(tag_id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Tag {} not found", tag_id)))
    }

    fn require_project(db: &mut ProjectDatabase, project_id: &str) -> Result<(), DatabaseError> {
        db.get_project_by_id(project_id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Project {} not found", project_id)))
            .map(|_| ())
    }
}
