use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::{ProjectDatabase, ProjectScope};
use crate::error::DatabaseError;
use crate::models::CollectionStatistics;
use crate::project::Project;

/// A collection plus the detail gRPC/CLI both need to display it: derived
/// project-count/duration stats, bundled here so callers don't repeat the
/// get-then-stat two-step that used to live in the gRPC handler.
///
/// `project_ids`, `project_count` and `total_duration_seconds` all count the same
/// projects: those in the scope the detail was loaded with (ADR-0043).
pub struct CollectionDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub project_ids: Vec<String>,
    pub cover_art_id: Option<String>,
    pub total_duration_seconds: Option<f64>,
    pub project_count: i32,
}

#[derive(Clone)]
pub struct CollectionsService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl CollectionsService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    pub async fn list_collections(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        scope: ProjectScope,
    ) -> Result<(Vec<CollectionDetail>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        let (rows, total_count) = db.list_collections(limit, offset, sort_by, sort_desc, scope)?;
        let detailed = rows
            .into_iter()
            .filter_map(|(id, _, _)| Self::load_detail(&mut db, &id, scope).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        Ok((detailed, total_count))
    }

    pub async fn search_collections(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        scope: ProjectScope,
    ) -> Result<(Vec<CollectionDetail>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        let (rows, total_count) = db.search_collections(query, limit, offset)?;
        let detailed = rows
            .into_iter()
            .filter_map(|(id, _, _)| Self::load_detail(&mut db, &id, scope).transpose())
            .collect::<Result<Vec<_>, _>>()?;
        Ok((detailed, total_count))
    }

    pub async fn get_collection(
        &self,
        id: &str,
        scope: ProjectScope,
    ) -> Result<Option<CollectionDetail>, DatabaseError> {
        let mut db = self.db.lock().await;
        Self::load_detail(&mut db, id, scope)
    }

    pub async fn get_collection_projects(
        &self,
        id: &str,
        scope: ProjectScope,
    ) -> Result<Vec<Project>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_collection_projects(id, scope)
    }

    pub async fn create_collection(
        &self,
        name: &str,
        description: Option<&str>,
        notes: Option<&str>,
    ) -> Result<CollectionDetail, DatabaseError> {
        let mut db = self.db.lock().await;
        let collection_id = db.create_collection(name, description, notes)?;
        Self::load_detail(&mut db, &collection_id, ProjectScope::default())?.ok_or_else(|| {
            DatabaseError::QueryError(format!(
                "Collection {} was created but not found",
                collection_id
            ))
        })
    }

    pub async fn update_collection(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        notes: Option<&str>,
    ) -> Result<CollectionDetail, DatabaseError> {
        let mut db = self.db.lock().await;
        db.update_collection(id, name, description, notes)?;
        Self::load_detail(&mut db, id, ProjectScope::default())?
            .ok_or_else(|| DatabaseError::NotFound(format!("Collection {} not found", id)))
    }

    pub async fn delete_collection(&self, id: &str) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        db.delete_collection(id)
    }

    pub async fn duplicate_collection(
        &self,
        id: &str,
        new_name: &str,
        new_description: Option<&str>,
        new_notes: Option<&str>,
    ) -> Result<CollectionDetail, DatabaseError> {
        let mut db = self.db.lock().await;
        let new_id = db.duplicate_collection(id, new_name, new_description, new_notes)?;
        Self::load_detail(&mut db, &new_id, ProjectScope::default())?.ok_or_else(|| {
            DatabaseError::QueryError(format!("Collection {} was duplicated but not found", new_id))
        })
    }

    /// Adds a project to a collection, validating both exist first -- unlike the
    /// raw database method, which only debug-logs whether the project exists and
    /// never checks the collection at all.
    pub async fn add_project_to_collection(
        &self,
        collection_id: &str,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        Self::require_collection(&mut db, collection_id)?;
        Self::require_project(&mut db, project_id)?;
        db.add_project_to_collection(collection_id, project_id)
    }

    pub async fn remove_project_from_collection(
        &self,
        collection_id: &str,
        project_id: &str,
    ) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        Self::require_collection(&mut db, collection_id)?;
        db.remove_project_from_collection(collection_id, project_id)
    }

    /// Reorders a collection's projects to the given order, which must be exactly
    /// the collection's current project set in `scope` (checked here, not left to the
    /// caller). Under `ProjectScope::Active` that is the list the caller was shown:
    /// the archived members keep their slots, and the active ones fill the remaining
    /// slots in the new order, so unarchiving one puts it back where it was (ADR-0043).
    pub async fn reorder_collection(
        &self,
        collection_id: &str,
        project_ids: &[String],
        scope: ProjectScope,
    ) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;

        let not_found = || DatabaseError::NotFound("Collection not found".to_string());
        let (_, _, _, _, _, _, all_ids, _) = db
            .get_collection_by_id(collection_id, ProjectScope::All)?
            .ok_or_else(not_found)?;
        let (_, _, _, _, _, _, scoped_ids, _) = db
            .get_collection_by_id(collection_id, scope)?
            .ok_or_else(not_found)?;

        let current_set: std::collections::HashSet<_> = scoped_ids.iter().collect();
        let requested_set: std::collections::HashSet<_> = project_ids.iter().collect();
        if current_set != requested_set || project_ids.len() != scoped_ids.len() {
            return Err(DatabaseError::InvalidOperation(
                "Project IDs must match exactly with the collection's projects".to_string(),
            ));
        }

        let mut reordered = project_ids.iter();
        let full_order: Vec<&String> = all_ids
            .iter()
            .map(|id| if current_set.contains(id) { reordered.next().unwrap_or(id) } else { id })
            .collect();

        for (new_position, project_id) in full_order.into_iter().enumerate() {
            db.reorder_project_in_collection(collection_id, project_id, new_position as i32)?;
        }

        Ok(())
    }

    /// `(task id, project id, project name, description, completed, created_at)`.
    pub async fn get_collection_tasks(
        &self,
        collection_id: &str,
        scope: ProjectScope,
    ) -> Result<Vec<(String, String, String, String, bool, i64)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_collection_tasks(collection_id, scope)
    }

    pub async fn get_collection_statistics(
        &self,
        collection_id: &str,
        scope: ProjectScope,
    ) -> Result<CollectionStatistics, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_collection_detailed_statistics(collection_id, scope)
    }

    pub async fn batch_add_to_collection(
        &self,
        project_ids: &[String],
        collection_id: &str,
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_add_projects_to_collection(project_ids, collection_id)
    }

    pub async fn batch_remove_from_collection(
        &self,
        project_ids: &[String],
        collection_id: &str,
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_remove_projects_from_collection(project_ids, collection_id)
    }

    #[allow(clippy::type_complexity)]
    pub async fn batch_create_collection_from(
        &self,
        name: &str,
        project_ids: &[String],
        description: Option<&str>,
        notes: Option<&str>,
    ) -> Result<
        (
            Option<CollectionDetail>,
            Vec<(String, Result<(), DatabaseError>)>,
        ),
        DatabaseError,
    > {
        let mut db = self.db.lock().await;
        let (collection_id, results) =
            db.batch_create_collection_from_projects(name, project_ids, description, notes)?;
        let detail = Self::load_detail(&mut db, &collection_id, ProjectScope::default()).unwrap_or(None);
        Ok((detail, results))
    }

    fn load_detail(
        db: &mut ProjectDatabase,
        id: &str,
        scope: ProjectScope,
    ) -> Result<Option<CollectionDetail>, DatabaseError> {
        let Some((id, name, description, notes, created_at, modified_at, project_ids, cover_art_id)) =
            db.get_collection_by_id(id, scope)?
        else {
            return Ok(None);
        };
        let (total_duration_seconds, project_count) =
            db.get_collection_statistics(&id, scope).unwrap_or((None, 0));
        Ok(Some(CollectionDetail {
            id,
            name,
            description,
            notes,
            created_at,
            modified_at,
            project_ids,
            cover_art_id,
            total_duration_seconds,
            project_count,
        }))
    }

    fn require_collection(db: &mut ProjectDatabase, collection_id: &str) -> Result<(), DatabaseError> {
        db.get_collection_by_id(collection_id, ProjectScope::All)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Collection {} not found", collection_id)))
            .map(|_| ())
    }

    fn require_project(db: &mut ProjectDatabase, project_id: &str) -> Result<(), DatabaseError> {
        db.get_project_by_id(project_id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Project {} not found", project_id)))
            .map(|_| ())
    }
}
