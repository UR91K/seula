use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::database::projects::RescanProjectResult;
use crate::database::stats::ProjectStatistics;
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::project::Project;

/// Which projects to include by deletion status. Defaults to `ActiveOnly` — the
/// unfiltered gRPC `GetProjects` used to return active + deleted with no way to
/// opt out, while the CLI defaulted to active-only; this is the single default
/// both surfaces now share. See the service-layer audit / ADR discussion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DeletionScope {
    #[default]
    ActiveOnly,
    DeletedOnly,
    All,
}

impl DeletionScope {
    fn to_is_active(self) -> Option<bool> {
        match self {
            DeletionScope::ActiveOnly => Some(true),
            DeletionScope::DeletedOnly => Some(false),
            DeletionScope::All => None,
        }
    }
}

#[derive(Clone)]
pub struct ProjectsService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl ProjectsService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    /// Escape hatch for callers that need a raw `&mut ProjectDatabase` lock, e.g.
    /// gRPC's LiveSet-to-proto conversion helper. Prefer the typed methods below.
    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_projects(
        &self,
        scope: DeletionScope,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        min_tempo: Option<f64>,
        max_tempo: Option<f64>,
        key_signature_tonic: Option<String>,
        key_signature_scale: Option<String>,
        time_signature_numerator: Option<i32>,
        time_signature_denominator: Option<i32>,
        ableton_version_major: Option<i32>,
        ableton_version_minor: Option<i32>,
        ableton_version_patch: Option<i32>,
        created_after: Option<i64>,
        created_before: Option<i64>,
        modified_after: Option<i64>,
        modified_before: Option<i64>,
        has_audio_file: Option<bool>,
    ) -> Result<(Vec<Project>, i32), DatabaseError> {
        let has_filters = min_tempo.is_some()
            || max_tempo.is_some()
            || key_signature_tonic.is_some()
            || key_signature_scale.is_some()
            || time_signature_numerator.is_some()
            || time_signature_denominator.is_some()
            || ableton_version_major.is_some()
            || ableton_version_minor.is_some()
            || ableton_version_patch.is_some()
            || created_after.is_some()
            || created_before.is_some()
            || modified_after.is_some()
            || modified_before.is_some()
            || has_audio_file.is_some();

        let db = self.db.lock().await;

        if has_filters {
            db.get_projects_with_filters(
                scope.to_is_active(),
                limit,
                offset,
                sort_by,
                sort_desc,
                min_tempo,
                max_tempo,
                key_signature_tonic,
                key_signature_scale,
                time_signature_numerator,
                time_signature_denominator,
                ableton_version_major,
                ableton_version_minor,
                ableton_version_patch,
                created_after,
                created_before,
                modified_after,
                modified_before,
                has_audio_file,
            )
        } else {
            let mut projects = db.get_all_projects_with_status(scope.to_is_active())?;
            let total_count = projects.len() as i32;
            let offset = offset.unwrap_or(0) as usize;
            let drained = projects.drain(..).skip(offset);
            let page = if let Some(limit) = limit {
                drained.take(limit as usize).collect()
            } else {
                drained.collect()
            };
            Ok((page, total_count))
        }
    }

    pub async fn get_project(&self, id: &str) -> Result<Option<Project>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_project_by_id(id)
    }

    pub async fn get_project_any_status(&self, id: &str) -> Result<Option<Project>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_project_by_id_any_status(id)
    }

    /// Updates name and/or notes through the real setters (`set_project_name`,
    /// `set_project_notes`), which bump `modified_at`. Previously the CLI ran raw
    /// SQL that bypassed these entirely.
    pub async fn update_project(
        &self,
        id: &str,
        name: Option<&str>,
        notes: Option<&str>,
    ) -> Result<(), DatabaseError> {
        let mut db = self.db.lock().await;
        if let Some(name) = name {
            db.set_project_name(id, name)?;
        }
        if let Some(notes) = notes {
            db.set_project_notes(id, notes)?;
        }
        Ok(())
    }

    pub async fn mark_deleted(&self, id: &str) -> Result<(), DatabaseError> {
        let uuid = Self::parse_uuid(id)?;
        let mut db = self.db.lock().await;
        db.mark_project_deleted(&uuid)
    }

    /// Reactivates a soft-deleted project. Looks up its stored path first, since
    /// `reactivate_project` needs one.
    pub async fn reactivate(&self, id: &str) -> Result<(), DatabaseError> {
        let uuid = Self::parse_uuid(id)?;
        let mut db = self.db.lock().await;
        let project = db
            .get_project_by_id_any_status(id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Project {} not found", id)))?;
        db.reactivate_project(&uuid, &project.file_path)
    }

    pub async fn permanently_delete(&self, id: &str) -> Result<(), DatabaseError> {
        let uuid = Self::parse_uuid(id)?;
        let mut db = self.db.lock().await;
        db.permanently_delete_project(&uuid)
    }

    pub async fn batch_mark_archived(
        &self,
        project_ids: &[String],
        archived: bool,
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_mark_projects_archived(project_ids, archived)
    }

    pub async fn batch_delete(
        &self,
        project_ids: &[String],
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_delete_projects(project_ids)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_statistics(
        &self,
        min_tempo: Option<f64>,
        max_tempo: Option<f64>,
        key_signature_tonic: Option<String>,
        key_signature_scale: Option<String>,
        time_signature_numerator: Option<i32>,
        time_signature_denominator: Option<i32>,
        ableton_version_major: Option<i32>,
        ableton_version_minor: Option<i32>,
        ableton_version_patch: Option<i32>,
        created_after: Option<i64>,
        created_before: Option<i64>,
        has_audio_file: Option<bool>,
    ) -> Result<ProjectStatistics, DatabaseError> {
        let db = self.db.lock().await;
        db.get_project_statistics(
            min_tempo,
            max_tempo,
            key_signature_tonic,
            key_signature_scale,
            time_signature_numerator,
            time_signature_denominator,
            ableton_version_major,
            ableton_version_minor,
            ableton_version_patch,
            created_after,
            created_before,
            has_audio_file,
        )
    }

    pub async fn rescan(
        &self,
        id: &str,
        force_rescan: bool,
    ) -> Result<RescanProjectResult, DatabaseError> {
        let mut db = self.db.lock().await;
        db.rescan_project(id, force_rescan)
    }

    fn parse_uuid(id: &str) -> Result<Uuid, DatabaseError> {
        Uuid::parse_str(id)
            .map_err(|e| DatabaseError::InvalidOperation(format!("Invalid project ID format: {}", e)))
    }
}
