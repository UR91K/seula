use tracing::{debug, error};
use tonic::{Request, Response, Status};

use super::super::common::*;
use super::super::common::Project as ProtoProject;
use super::super::projects::*;
use super::utils::convert_live_set_to_proto;
use crate::error::DatabaseError;
use crate::services::{DeletionScope, ProjectsService};
use crate::Project;

fn deletion_scope_from_proto(value: i32) -> DeletionScope {
    match value {
        1 => DeletionScope::DeletedOnly,
        2 => DeletionScope::All,
        _ => DeletionScope::ActiveOnly,
    }
}

#[derive(Clone)]
pub struct ProjectsHandler {
    pub service: ProjectsService,
}

impl ProjectsHandler {
    pub fn new(service: ProjectsService) -> Self {
        Self { service }
    }

    async fn projects_to_proto(&self, projects: Vec<Project>) -> Result<Vec<ProtoProject>, Status> {
        let db_arc = self.service.db_handle();
        let mut db = db_arc.lock().await;
        let mut proto_projects = Vec::new();
        for project in projects {
            match convert_live_set_to_proto(project, &mut db) {
                Ok(proto_project) => proto_projects.push(proto_project),
                Err(e) => {
                    error!("Failed to convert project to proto: {:?}", e);
                    return Err(Status::internal(format!("Database error: {}", e)));
                }
            }
        }
        Ok(proto_projects)
    }

    pub async fn get_projects(
        &self,
        request: Request<GetProjectsRequest>,
    ) -> Result<Response<GetProjectsResponse>, Status> {
        debug!("GetProjects request: {:?}", request);
        let req = request.into_inner();
        let scope = deletion_scope_from_proto(req.deletion_scope);

        let (projects, total_count) = self
            .service
            .list_projects(
                scope,
                req.limit,
                req.offset,
                req.sort_by,
                req.sort_desc,
                req.min_tempo,
                req.max_tempo,
                req.key_signature_tonic,
                req.key_signature_scale,
                req.time_signature_numerator,
                req.time_signature_denominator,
                req.ableton_version_major,
                req.ableton_version_minor,
                req.ableton_version_patch,
                req.created_after,
                req.created_before,
                req.modified_after,
                req.modified_before,
                req.has_audio_file,
            )
            .await?;

        let proto_projects = self.projects_to_proto(projects).await?;

        Ok(Response::new(GetProjectsResponse {
            projects: proto_projects,
            total_count,
        }))
    }

    pub async fn get_project(
        &self,
        request: Request<GetProjectRequest>,
    ) -> Result<Response<GetProjectResponse>, Status> {
        debug!("GetProject request: {:?}", request);
        let req = request.into_inner();

        match self.service.get_project(&req.project_id).await? {
            Some(project) => {
                let proto_project = self.projects_to_proto(vec![project]).await?.remove(0);
                Ok(Response::new(GetProjectResponse {
                    project: Some(proto_project),
                }))
            }
            None => Ok(Response::new(GetProjectResponse { project: None })),
        }
    }

    pub async fn update_project_notes(
        &self,
        request: Request<UpdateProjectNotesRequest>,
    ) -> Result<Response<UpdateProjectNotesResponse>, Status> {
        debug!("UpdateProjectNotes request: {:?}", request);
        let req = request.into_inner();

        self.service
            .update_project(&req.project_id, None, Some(&req.notes))
            .await?;

        Ok(Response::new(UpdateProjectNotesResponse { success: true }))
    }

    pub async fn update_project_name(
        &self,
        request: Request<UpdateProjectNameRequest>,
    ) -> Result<Response<UpdateProjectNameResponse>, Status> {
        debug!("UpdateProjectName request: {:?}", request);
        let req = request.into_inner();

        self.service
            .update_project(&req.project_id, Some(&req.name), None)
            .await?;

        Ok(Response::new(UpdateProjectNameResponse { success: true }))
    }

    pub async fn mark_project_deleted(
        &self,
        request: Request<MarkProjectDeletedRequest>,
    ) -> Result<Response<MarkProjectDeletedResponse>, Status> {
        debug!("MarkProjectDeleted request: {:?}", request);
        let req = request.into_inner();

        self.service.mark_deleted(&req.project_id).await?;

        debug!("Successfully marked project {} as deleted", req.project_id);
        Ok(Response::new(MarkProjectDeletedResponse { success: true }))
    }

    pub async fn reactivate_project(
        &self,
        request: Request<ReactivateProjectRequest>,
    ) -> Result<Response<ReactivateProjectResponse>, Status> {
        debug!("ReactivateProject request: {:?}", request);
        let req = request.into_inner();

        self.service.reactivate(&req.project_id).await?;

        debug!("Successfully reactivated project {}", req.project_id);
        Ok(Response::new(ReactivateProjectResponse { success: true }))
    }

    pub async fn get_projects_by_deletion_status(
        &self,
        request: Request<GetProjectsByDeletionStatusRequest>,
    ) -> Result<Response<GetProjectsByDeletionStatusResponse>, Status> {
        debug!("GetProjectsByDeletionStatus request: {:?}", request);
        let req = request.into_inner();
        let scope = if req.is_deleted {
            DeletionScope::DeletedOnly
        } else {
            DeletionScope::ActiveOnly
        };

        let (projects, total_count) = self
            .service
            .list_projects(
                scope, req.limit, req.offset, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            )
            .await?;

        let proto_projects = self.projects_to_proto(projects).await?;

        Ok(Response::new(GetProjectsByDeletionStatusResponse {
            projects: proto_projects,
            total_count,
        }))
    }

    pub async fn permanently_delete_project(
        &self,
        request: Request<PermanentlyDeleteProjectRequest>,
    ) -> Result<Response<PermanentlyDeleteProjectResponse>, Status> {
        debug!("PermanentlyDeleteProject request: {:?}", request);
        let req = request.into_inner();

        match self.service.permanently_delete(&req.project_id).await {
            Ok(()) => {
                debug!("Successfully permanently deleted project {}", req.project_id);
                Ok(Response::new(PermanentlyDeleteProjectResponse { success: true }))
            }
            Err(DatabaseError::InvalidOperation(msg))
                if msg == "Cannot permanently delete an active project" =>
            {
                debug!("Cannot permanently delete active project {}", req.project_id);
                Ok(Response::new(PermanentlyDeleteProjectResponse { success: false }))
            }
            Err(e) => {
                error!(
                    "Failed to permanently delete project {}: {:?}",
                    req.project_id, e
                );
                Err(e.into())
            }
        }
    }

    // Batch Project Operations
    pub async fn batch_mark_projects_as_archived(
        &self,
        request: Request<BatchMarkProjectsAsArchivedRequest>,
    ) -> Result<Response<BatchMarkProjectsAsArchivedResponse>, Status> {
        debug!("BatchMarkProjectsAsArchived request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_mark_archived(&req.project_ids, req.archived)
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

        debug!(
            "Batch archive operation completed: {} successful, {} failed",
            successful_count, failed_count
        );
        Ok(Response::new(BatchMarkProjectsAsArchivedResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn batch_delete_projects(
        &self,
        request: Request<BatchDeleteProjectsRequest>,
    ) -> Result<Response<BatchDeleteProjectsResponse>, Status> {
        debug!("BatchDeleteProjects request: {:?}", request);
        let req = request.into_inner();

        let results = self.service.batch_delete(&req.project_ids).await?;

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

        debug!(
            "Batch delete operation completed: {} successful, {} failed",
            successful_count, failed_count
        );
        Ok(Response::new(BatchDeleteProjectsResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn get_project_statistics(
        &self,
        request: Request<GetProjectStatisticsRequest>,
    ) -> Result<Response<GetProjectStatisticsResponse>, Status> {
        debug!("GetProjectStatistics request: {:?}", request);
        let req = request.into_inner();

        let stats = self
            .service
            .get_statistics(
                req.min_tempo,
                req.max_tempo,
                req.key_signature_tonic,
                req.key_signature_scale,
                req.time_signature_numerator,
                req.time_signature_denominator,
                req.ableton_version_major,
                req.ableton_version_minor,
                req.ableton_version_patch,
                req.created_after,
                req.created_before,
                req.has_audio_file,
            )
            .await?;

        let tempo_distribution = stats
            .tempo_distribution
            .into_iter()
            .map(|(range, count)| TempoRangeStatistic { range, count })
            .collect();

        let key_signature_distribution = stats
            .key_signature_distribution
            .into_iter()
            .map(|(key_signature, count)| KeySignatureStatistic { key_signature, count })
            .collect();

        let time_signature_distribution = stats
            .time_signature_distribution
            .into_iter()
            .map(|(numerator, denominator, count)| TimeSignatureStatistic {
                numerator,
                denominator,
                count,
            })
            .collect();

        let ableton_version_distribution = stats
            .ableton_version_distribution
            .into_iter()
            .map(|(version, count)| AbletonVersionStatistic { version, count })
            .collect();

        let projects_per_year = stats
            .projects_per_year
            .into_iter()
            .map(|(year, count)| YearStatistic { year, count })
            .collect();

        let projects_per_month = stats
            .projects_per_month
            .into_iter()
            .map(|(year, month, count)| MonthStatistic { year, month, count })
            .collect();

        let most_complex_projects = stats
            .most_complex_projects
            .into_iter()
            .map(
                |(project_id, project_name, plugin_count, sample_count, tag_count, complexity_score)| {
                    ProjectComplexityStatistic {
                        project_id,
                        project_name,
                        plugin_count,
                        sample_count,
                        tag_count,
                        complexity_score,
                    }
                },
            )
            .collect();

        Ok(Response::new(GetProjectStatisticsResponse {
            total_projects: stats.total_projects,
            projects_with_audio_files: stats.projects_with_audio_files,
            projects_without_audio_files: stats.projects_without_audio_files,
            average_tempo: stats.average_tempo,
            min_tempo: stats.min_tempo,
            max_tempo: stats.max_tempo,
            tempo_distribution,
            key_signature_distribution,
            time_signature_distribution,
            ableton_version_distribution,
            average_duration_seconds: stats.average_duration_seconds,
            min_duration_seconds: stats.min_duration_seconds,
            max_duration_seconds: stats.max_duration_seconds,
            average_plugins_per_project: stats.average_plugins_per_project,
            average_samples_per_project: stats.average_samples_per_project,
            average_tags_per_project: stats.average_tags_per_project,
            projects_per_year,
            projects_per_month,
            most_complex_projects,
        }))
    }

    pub async fn rescan_project(
        &self,
        request: Request<RescanProjectRequest>,
    ) -> Result<Response<RescanProjectResponse>, Status> {
        debug!("RescanProject request: {:?}", request);
        let req = request.into_inner();

        // Not routed through `?` (which maps InvalidOperation to InvalidArgument):
        // an unrecognized project_id here isn't a malformed request, it's a lookup
        // miss, so this stays Internal to match the RPC's existing error contract.
        let result = self
            .service
            .rescan(&req.project_id, req.force_rescan.unwrap_or(false))
            .await
            .map_err(|e| {
                error!("Failed to rescan project {}: {:?}", req.project_id, e);
                Status::internal(format!("Database error: {}", e))
            })?;

        let updated_project = if let Some(project) = result.updated_project {
            Some(self.projects_to_proto(vec![project]).await?.remove(0))
        } else {
            None
        };

        Ok(Response::new(RescanProjectResponse {
            success: result.success,
            updated_project,
            error_message: result.error_message,
            was_updated: result.was_updated,
            scan_summary: result.scan_summary,
        }))
    }
}
