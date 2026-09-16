//! HTTP wire types for the tasks domain (ADR-0024).

use serde::{Deserialize, Serialize};

use crate::database::tasks::TaskAnalytics;
use crate::services::tasks::{ProjectTaskRow, TaskRow};

#[derive(Serialize)]
pub struct TaskDto {
    pub id: String,
    pub project_id: String,
    pub description: String,
    pub completed: bool,
    pub created_at: i64,
}

impl From<TaskRow> for TaskDto {
    fn from((id, project_id, description, completed, created_at): TaskRow) -> Self {
        Self {
            id,
            project_id,
            description,
            completed,
            created_at,
        }
    }
}

impl TaskDto {
    pub fn from_project_row(project_id: &str, row: ProjectTaskRow) -> Self {
        let (id, description, completed, created_at) = row;
        Self {
            id,
            project_id: project_id.to_string(),
            description,
            completed,
            created_at,
        }
    }
}

#[derive(Serialize)]
pub struct ProjectTasksResponse {
    pub tasks: Vec<TaskDto>,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub description: String,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub description: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct SearchTasksQuery {
    pub query: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    pub completed_only: Option<bool>,
    pub pending_only: Option<bool>,
}

#[derive(Serialize)]
pub struct TaskSearchResponse {
    pub tasks: Vec<TaskDto>,
    pub total_count: i32,
}

#[derive(Deserialize)]
pub struct BatchTaskIdsRequest {
    pub task_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct BatchUpdateTaskStatusRequest {
    pub task_ids: Vec<String>,
    pub completed: bool,
}

#[derive(Serialize)]
pub struct BatchOperationResultDto {
    pub id: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Serialize)]
pub struct BatchOperationResponse {
    pub results: Vec<BatchOperationResultDto>,
    pub successful_count: i32,
    pub failed_count: i32,
}

impl BatchOperationResponse {
    pub fn from_results(results: Vec<(String, Result<(), crate::error::DatabaseError>)>) -> Self {
        let (successful_count, failed_count) = results
            .iter()
            .fold((0, 0), |(s, f), (_, r)| if r.is_ok() { (s + 1, f) } else { (s, f + 1) });

        let results = results
            .into_iter()
            .map(|(id, result)| BatchOperationResultDto {
                id,
                success: result.is_ok(),
                error_message: result.err().map(|e| e.to_string()),
            })
            .collect();

        Self {
            results,
            successful_count,
            failed_count,
        }
    }
}

#[derive(Deserialize)]
pub struct TaskStatisticsQuery {
    pub project_id: Option<String>,
}

#[derive(Serialize)]
pub struct TaskTrendDto {
    pub year: i32,
    pub month: i32,
    pub completed_tasks: i32,
    pub total_tasks: i32,
    pub completion_rate: f64,
}

#[derive(Serialize)]
pub struct TaskStatisticsDto {
    pub total_tasks: i32,
    pub completed_tasks: i32,
    pub pending_tasks: i32,
    pub completion_rate: f64,
    pub tasks_created_this_week: i32,
    pub tasks_completed_this_week: i32,
    pub tasks_created_this_month: i32,
    pub tasks_completed_this_month: i32,
    pub monthly_trends: Vec<TaskTrendDto>,
}

impl From<TaskAnalytics> for TaskStatisticsDto {
    fn from(stats: TaskAnalytics) -> Self {
        Self {
            total_tasks: stats.total_tasks,
            completed_tasks: stats.completed_tasks,
            pending_tasks: stats.pending_tasks,
            completion_rate: stats.completion_rate,
            tasks_created_this_week: stats.tasks_created_this_week,
            tasks_completed_this_week: stats.tasks_completed_this_week,
            tasks_created_this_month: stats.tasks_created_this_month,
            tasks_completed_this_month: stats.tasks_completed_this_month,
            monthly_trends: stats
                .monthly_trends
                .into_iter()
                .map(|(year, month, completed_tasks, total_tasks, completion_rate)| TaskTrendDto {
                    year,
                    month,
                    completed_tasks,
                    total_tasks,
                    // Matches src/grpc/handlers/tasks.rs: monthly trend rates are
                    // converted to a percentage, unlike the top-level
                    // completion_rate above, which isn't. Preserved as-is.
                    completion_rate: completion_rate * 100.0,
                })
                .collect(),
        }
    }
}
