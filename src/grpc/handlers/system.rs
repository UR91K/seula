use log::{debug, error, info};
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::super::common::*;
use super::super::scanning::*;
use super::super::system::*;
use super::super::watcher::*;
use super::utils::convert_live_set_to_proto;
use crate::services::SystemService;

#[derive(Clone)]
pub struct SystemHandler {
    pub service: SystemService,
}

impl SystemHandler {
    pub fn new(service: SystemService) -> Self {
        Self { service }
    }

    pub async fn scan_directories(
        &self,
        request: Request<ScanDirectoriesRequest>,
    ) -> Result<Response<ReceiverStream<Result<ScanProgressResponse, Status>>>, Status> {
        info!("ScanDirectories request: {:?}", request);
        let _req = request.into_inner();

        let (tx, rx) = mpsc::channel(100);
        let tx_for_callback = tx.clone();

        self.service
            .start_scan(move |response| {
                if let Err(e) = tx_for_callback.try_send(Ok(response)) {
                    error!("Failed to send progress update: {:?}", e);
                }
            })
            .await;

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    pub async fn get_scan_status(
        &self,
        _request: Request<GetScanStatusRequest>,
    ) -> Result<Response<GetScanStatusResponse>, Status> {
        let (status, current_progress) = self.service.get_scan_status().await;

        Ok(Response::new(GetScanStatusResponse {
            status: status as i32,
            current_progress,
        }))
    }

    pub async fn add_single_project(
        &self,
        request: Request<AddSingleProjectRequest>,
    ) -> Result<Response<AddSingleProjectResponse>, Status> {
        debug!("AddSingleProject request: {:?}", request);
        let req = request.into_inner();
        let file_path = PathBuf::from(&req.file_path);

        match self.service.add_single_project(&file_path).await {
            Ok(project) => {
                let db_arc = self.service.db_handle();
                let mut db = db_arc.lock().await;
                match convert_live_set_to_proto(project, &mut db) {
                    Ok(proto_project) => {
                        info!("Successfully added single project: {}", req.file_path);
                        Ok(Response::new(AddSingleProjectResponse {
                            success: true,
                            project: Some(proto_project),
                            error_message: None,
                        }))
                    }
                    Err(e) => {
                        error!("Failed to convert project to proto: {:?}", e);
                        Ok(Response::new(AddSingleProjectResponse {
                            success: false,
                            project: None,
                            error_message: Some(format!("Failed to convert project: {}", e)),
                        }))
                    }
                }
            }
            Err(message) => Ok(Response::new(AddSingleProjectResponse {
                success: false,
                project: None,
                error_message: Some(message),
            })),
        }
    }

    pub async fn add_multiple_projects(
        &self,
        request: Request<AddMultipleProjectsRequest>,
    ) -> Result<Response<AddMultipleProjectsResponse>, Status> {
        debug!("AddMultipleProjects request: {:?}", request);
        let req = request.into_inner();
        let total_requested = req.file_paths.len() as i32;

        let (successes, failures) = self.service.add_multiple_projects(req.file_paths).await;

        let mut failed_paths: Vec<String> = failures.iter().map(|(path, _)| path.clone()).collect();
        let mut error_messages: Vec<String> = failures.into_iter().map(|(_, msg)| msg).collect();

        let mut proto_projects = Vec::new();
        if !successes.is_empty() {
            let db_arc = self.service.db_handle();
            let mut db = db_arc.lock().await;
            for (path, project) in successes {
                match convert_live_set_to_proto(project, &mut db) {
                    Ok(proto_project) => {
                        proto_projects.push(proto_project);
                        info!("Successfully added project: {}", path);
                    }
                    Err(e) => {
                        error!("Failed to convert project to proto: {:?}", e);
                        failed_paths.push(path);
                        error_messages.push(format!("Failed to convert project: {}", e));
                    }
                }
            }
        }

        let successful_imports = proto_projects.len() as i32;
        let failed_imports = total_requested - successful_imports;
        let success = failed_imports == 0;

        Ok(Response::new(AddMultipleProjectsResponse {
            success,
            projects: proto_projects,
            failed_paths,
            error_messages,
            total_requested,
            successful_imports,
            failed_imports,
        }))
    }

    pub async fn start_watcher(
        &self,
        _request: Request<StartWatcherRequest>,
    ) -> Result<Response<StartWatcherResponse>, Status> {
        debug!("Starting file watcher");

        match self.service.start_watcher().await {
            Ok(success) => {
                info!("File watcher started successfully");
                Ok(Response::new(StartWatcherResponse { success }))
            }
            Err(e) => {
                error!("Failed to start watcher: {}", e);
                Err(Status::internal(format!("Failed to start watcher: {}", e)))
            }
        }
    }

    pub async fn stop_watcher(
        &self,
        _request: Request<StopWatcherRequest>,
    ) -> Result<Response<StopWatcherResponse>, Status> {
        debug!("Stopping file watcher");
        self.service.stop_watcher().await;
        info!("File watcher stopped successfully");
        Ok(Response::new(StopWatcherResponse { success: true }))
    }

    pub async fn get_watcher_events(
        &self,
        _request: Request<GetWatcherEventsRequest>,
    ) -> Result<Response<ReceiverStream<Result<WatcherEventResponse, Status>>>, Status> {
        debug!("Getting watcher events stream");

        let (tx, rx) = mpsc::channel(100);
        let started = self
            .service
            .start_watcher_event_stream(move |event| {
                if tx.try_send(Ok(event)).is_err() {
                    debug!("Watcher event receiver dropped");
                }
            })
            .await;

        if started {
            Ok(Response::new(ReceiverStream::new(rx)))
        } else {
            Err(Status::failed_precondition("Watcher not active"))
        }
    }

    pub async fn get_system_info(
        &self,
        _request: Request<GetSystemInfoRequest>,
    ) -> Result<Response<GetSystemInfoResponse>, Status> {
        let (version, watch_paths, watcher_active, uptime_seconds) = self
            .service
            .get_system_info()
            .await
            .map_err(|e| Status::internal(format!("Config error: {}", e)))?;

        Ok(Response::new(GetSystemInfoResponse {
            version,
            watch_paths,
            watcher_active,
            uptime_seconds,
        }))
    }

    pub async fn get_statistics(
        &self,
        request: Request<GetStatisticsRequest>,
    ) -> Result<Response<GetStatisticsResponse>, Status> {
        debug!("Getting comprehensive statistics: {:?}", request);
        // TODO: Implement filtering based on request.date_range, collection_ids,
        // tag_ids, ableton_version_filter -- unfiltered for now (unchanged).

        let stats = self
            .service
            .get_statistics()
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        debug!("Successfully gathered comprehensive statistics");
        Ok(Response::new(stats))
    }

    pub async fn export_statistics(
        &self,
        request: Request<ExportStatisticsRequest>,
    ) -> Result<Response<ExportStatisticsResponse>, Status> {
        debug!("ExportStatistics request: {:?}", request);
        let req = request.into_inner();

        let stats_request = req.filters.clone().unwrap_or_default();
        let stats_response = self.get_statistics(Request::new(stats_request)).await?;
        let stats = stats_response.into_inner();

        match req.format() {
            ExportFormat::ExportCsv => {
                let csv_data = self.generate_csv_export(stats).await?;
                let filename = format!("statistics_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

                Ok(Response::new(ExportStatisticsResponse {
                    data: csv_data,
                    filename,
                    success: true,
                    error_message: None,
                }))
            }
        }
    }

    async fn generate_csv_export(&self, stats: GetStatisticsResponse) -> Result<Vec<u8>, Status> {
        let mut csv_content = String::new();

        csv_content.push_str("Category,Value\n");
        csv_content.push_str(&format!("Total Projects,{}\n", stats.total_projects));
        csv_content.push_str(&format!("Total Plugins,{}\n", stats.total_plugins));
        csv_content.push_str(&format!("Total Samples,{}\n", stats.total_samples));
        csv_content.push_str(&format!("Total Collections,{}\n", stats.total_collections));
        csv_content.push_str(&format!("Total Tags,{}\n", stats.total_tags));
        csv_content.push_str(&format!("Total Tasks,{}\n", stats.total_tasks));
        csv_content.push_str(&format!("Completed Tasks,{}\n", stats.completed_tasks));
        csv_content.push_str(&format!("Pending Tasks,{}\n", stats.pending_tasks));
        csv_content.push_str(&format!(
            "Task Completion Rate,{:.2}%\n",
            stats.task_completion_rate * 100.0
        ));
        csv_content.push_str(&format!(
            "Average Project Duration,{:.2} seconds\n",
            stats.average_project_duration_seconds
        ));
        csv_content.push_str(&format!(
            "Average Projects per Collection,{:.2}\n",
            stats.average_projects_per_collection
        ));
        csv_content.push_str(&format!(
            "Average Plugins per Project,{:.2}\n",
            stats.average_plugins_per_project
        ));
        csv_content.push_str(&format!(
            "Average Samples per Project,{:.2}\n",
            stats.average_samples_per_project
        ));
        csv_content.push_str("\n");

        csv_content.push_str("Top Plugins\n");
        csv_content.push_str("Plugin Name,Vendor,Usage Count\n");
        for plugin in stats.top_plugins {
            csv_content.push_str(&format!(
                "{},{},{}\n",
                plugin.name, plugin.vendor, plugin.usage_count
            ));
        }
        csv_content.push_str("\n");

        csv_content.push_str("Tempo Distribution\n");
        csv_content.push_str("Tempo,Count\n");
        for tempo in stats.tempo_distribution {
            csv_content.push_str(&format!("{},{}\n", tempo.tempo, tempo.count));
        }
        csv_content.push_str("\n");

        csv_content.push_str("Key Distribution\n");
        csv_content.push_str("Key,Count\n");
        for key in stats.key_distribution {
            csv_content.push_str(&format!("{},{}\n", key.key, key.count));
        }

        Ok(csv_content.into_bytes())
    }
}
