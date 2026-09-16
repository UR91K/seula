use tracing::debug;
use tonic::{Request, Response, Status};

use super::super::common::*;
use super::super::tasks::*;
use crate::services::TasksService;

#[derive(Clone)]
pub struct TasksHandler {
    pub service: TasksService,
}

impl TasksHandler {
    pub fn new(service: TasksService) -> Self {
        Self { service }
    }

    fn project_task_to_proto(project_id: &str, row: (String, String, bool, i64)) -> Task {
        let (id, description, completed, created_at) = row;
        Task {
            id,
            project_id: project_id.to_string(),
            description,
            completed,
            created_at,
        }
    }

    fn task_to_proto(row: (String, String, String, bool, i64)) -> Task {
        let (id, project_id, description, completed, created_at) = row;
        Task {
            id,
            project_id,
            description,
            completed,
            created_at,
        }
    }

    pub async fn get_project_tasks(
        &self,
        request: Request<GetProjectTasksRequest>,
    ) -> Result<Response<GetProjectTasksResponse>, Status> {
        debug!("GetProjectTasks request: {:?}", request);
        let req = request.into_inner();

        let task_data = self.service.get_project_tasks(&req.project_id).await?;
        let tasks = task_data
            .into_iter()
            .map(|row| Self::project_task_to_proto(&req.project_id, row))
            .collect();

        Ok(Response::new(GetProjectTasksResponse { tasks }))
    }

    pub async fn create_task(
        &self,
        request: Request<CreateTaskRequest>,
    ) -> Result<Response<CreateTaskResponse>, Status> {
        debug!("CreateTask request: {:?}", request);
        let req = request.into_inner();

        let task = self.service.create_task(&req.project_id, &req.description).await?;

        Ok(Response::new(CreateTaskResponse {
            task: Some(Self::task_to_proto(task)),
        }))
    }

    pub async fn update_task(
        &self,
        request: Request<UpdateTaskRequest>,
    ) -> Result<Response<UpdateTaskResponse>, Status> {
        debug!("UpdateTask request: {:?}", request);
        let req = request.into_inner();

        let task = self
            .service
            .update_task(&req.task_id, req.description.as_deref(), req.completed)
            .await?;

        Ok(Response::new(UpdateTaskResponse {
            task: Some(Self::task_to_proto(task)),
        }))
    }

    pub async fn delete_task(
        &self,
        request: Request<DeleteTaskRequest>,
    ) -> Result<Response<DeleteTaskResponse>, Status> {
        debug!("DeleteTask request: {:?}", request);
        let req = request.into_inner();

        self.service.delete_task(&req.task_id).await?;

        debug!("Successfully deleted task: {}", req.task_id);
        Ok(Response::new(DeleteTaskResponse { success: true }))
    }

    // Batch Task Operations
    pub async fn batch_update_task_status(
        &self,
        request: Request<BatchUpdateTaskStatusRequest>,
    ) -> Result<Response<BatchUpdateTaskStatusResponse>, Status> {
        debug!("BatchUpdateTaskStatus request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_update_task_status(&req.task_ids, req.completed)
            .await?;

        let (successful_count, failed_count) = results
            .iter()
            .fold((0, 0), |(s, f), (_, r)| if r.is_ok() { (s + 1, f) } else { (s, f + 1) });
        let batch_results = results
            .into_iter()
            .map(|(id, result)| BatchOperationResult {
                id,
                success: result.is_ok(),
                error_message: result.err().map(|e| e.to_string()),
            })
            .collect();

        Ok(Response::new(BatchUpdateTaskStatusResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn batch_delete_tasks(
        &self,
        request: Request<BatchDeleteTasksRequest>,
    ) -> Result<Response<BatchDeleteTasksResponse>, Status> {
        debug!("BatchDeleteTasks request: {:?}", request);
        let req = request.into_inner();

        let results = self.service.batch_delete_tasks(&req.task_ids).await?;

        let (successful_count, failed_count) = results
            .iter()
            .fold((0, 0), |(s, f), (_, r)| if r.is_ok() { (s + 1, f) } else { (s, f + 1) });
        let batch_results = results
            .into_iter()
            .map(|(id, result)| BatchOperationResult {
                id,
                success: result.is_ok(),
                error_message: result.err().map(|e| e.to_string()),
            })
            .collect();

        Ok(Response::new(BatchDeleteTasksResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn search_tasks(
        &self,
        request: Request<SearchTasksRequest>,
    ) -> Result<Response<SearchTasksResponse>, Status> {
        debug!("SearchTasks request: {:?}", request);
        let req = request.into_inner();

        let (task_data, total_count) = self
            .service
            .search_tasks(
                &req.project_id,
                &req.query,
                req.limit,
                req.offset,
                req.completed_only,
                req.pending_only,
            )
            .await?;

        let tasks = task_data
            .into_iter()
            .map(|row| Self::project_task_to_proto(&req.project_id, row))
            .collect();

        Ok(Response::new(SearchTasksResponse { tasks, total_count }))
    }

    pub async fn get_task_statistics(
        &self,
        request: Request<GetTaskStatisticsRequest>,
    ) -> Result<Response<GetTaskStatisticsResponse>, Status> {
        debug!("GetTaskStatistics request: {:?}", request);
        let req = request.into_inner();

        let stats = self.service.get_task_statistics(req.project_id.as_deref()).await?;

        let monthly_trends = stats
            .monthly_trends
            .into_iter()
            .map(|(year, month, completed_tasks, total_tasks, completion_rate)| {
                super::super::tasks::TaskTrend {
                    year,
                    month,
                    completed_tasks,
                    total_tasks,
                    completion_rate: completion_rate * 100.0, // Convert to percentage
                }
            })
            .collect();

        let proto_stats = super::super::tasks::TaskStatistics {
            total_tasks: stats.total_tasks,
            completed_tasks: stats.completed_tasks,
            pending_tasks: stats.pending_tasks,
            completion_rate: stats.completion_rate,
            tasks_created_this_week: stats.tasks_created_this_week,
            tasks_completed_this_week: stats.tasks_completed_this_week,
            tasks_created_this_month: stats.tasks_created_this_month,
            tasks_completed_this_month: stats.tasks_completed_this_month,
            monthly_trends,
        };

        Ok(Response::new(GetTaskStatisticsResponse {
            statistics: Some(proto_stats),
        }))
    }
}
