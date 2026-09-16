use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::tasks::TaskAnalytics;
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;

pub type TaskRow = (String, String, String, bool, i64); // id, project_id, description, completed, created_at
pub type ProjectTaskRow = (String, String, bool, i64); // id, description, completed, created_at

#[derive(Clone)]
pub struct TasksService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl TasksService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    pub async fn get_project_tasks(&self, project_id: &str) -> Result<Vec<ProjectTaskRow>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_project_tasks(project_id)
    }

    pub async fn create_task(&self, project_id: &str, description: &str) -> Result<TaskRow, DatabaseError> {
        let mut db = self.db.lock().await;
        let task_id = db.add_task(project_id, description)?;
        db.get_task(&task_id)?.ok_or_else(|| {
            DatabaseError::NotFound(format!("Task {} created but could not be retrieved", task_id))
        })
    }

    pub async fn update_task(
        &self,
        task_id: &str,
        description: Option<&str>,
        completed: Option<bool>,
    ) -> Result<TaskRow, DatabaseError> {
        let mut db = self.db.lock().await;
        if let Some(description) = description {
            db.update_task_description(task_id, description)?;
        }
        if let Some(completed) = completed {
            db.complete_task(task_id, completed)?;
        }
        db.get_task(task_id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Task not found: {}", task_id)))
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<TaskRow>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_task(task_id)
    }

    pub async fn delete_task(&self, task_id: &str) -> Result<TaskRow, DatabaseError> {
        let mut db = self.db.lock().await;
        let task = db
            .get_task(task_id)?
            .ok_or_else(|| DatabaseError::NotFound(format!("Task not found: {}", task_id)))?;
        db.remove_task(task_id)?;
        Ok(task)
    }

    pub async fn batch_update_task_status(
        &self,
        task_ids: &[String],
        completed: bool,
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_update_task_status(task_ids, completed)
    }

    pub async fn batch_delete_tasks(
        &self,
        task_ids: &[String],
    ) -> Result<Vec<(String, Result<(), DatabaseError>)>, DatabaseError> {
        let mut db = self.db.lock().await;
        db.batch_delete_tasks(task_ids)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search_tasks(
        &self,
        project_id: &str,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        completed_only: Option<bool>,
        pending_only: Option<bool>,
    ) -> Result<(Vec<ProjectTaskRow>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        db.search_tasks(project_id, query, limit, offset, completed_only, pending_only)
    }

    pub async fn get_task_statistics(&self, project_id: Option<&str>) -> Result<TaskAnalytics, DatabaseError> {
        let mut db = self.db.lock().await;
        db.get_task_analytics(project_id)
    }
}
