use tracing::debug;
use tonic::{Code, Request, Response, Status};

use super::super::common::*;
use super::super::samples::*;
use super::utils::convert_live_set_to_proto;
use crate::services::SamplesService;

fn to_proto(sample: crate::models::Sample) -> Sample {
    Sample {
        id: sample.id.to_string(),
        name: sample.name,
        path: sample.path.to_string_lossy().to_string(),
        is_present: sample.is_present,
    }
}

#[derive(Clone)]
pub struct SamplesHandler {
    pub service: SamplesService,
}

impl SamplesHandler {
    pub fn new(service: SamplesService) -> Self {
        Self { service }
    }

    pub async fn get_all_samples(
        &self,
        request: Request<GetAllSamplesRequest>,
    ) -> Result<Response<GetAllSamplesResponse>, Status> {
        debug!("GetAllSamples request: {:?}", request);
        let req = request.into_inner();

        let (samples, total_count) = self
            .service
            .get_all_samples(
                req.limit,
                req.offset,
                req.sort_by,
                req.sort_desc,
                req.present_only,
                req.missing_only,
                req.extension_filter,
                req.min_usage_count,
                req.max_usage_count,
            )
            .await?;

        Ok(Response::new(GetAllSamplesResponse {
            samples: samples.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    pub async fn get_sample(
        &self,
        request: Request<GetSampleRequest>,
    ) -> Result<Response<GetSampleResponse>, Status> {
        debug!("GetSample request: {:?}", request);
        let req = request.into_inner();

        match self.service.get_sample(&req.sample_id).await? {
            Some(sample) => Ok(Response::new(GetSampleResponse {
                sample: Some(to_proto(sample)),
            })),
            None => {
                debug!("Sample not found with ID: {}", req.sample_id);
                Err(Status::new(
                    Code::NotFound,
                    format!("Sample not found with ID: {}", req.sample_id),
                ))
            }
        }
    }

    pub async fn get_sample_by_presence(
        &self,
        request: Request<GetSampleByPresenceRequest>,
    ) -> Result<Response<GetSampleByPresenceResponse>, Status> {
        debug!("GetSampleByPresence request: {:?}", request);
        let req = request.into_inner();

        let (samples, total_count) = self
            .service
            .get_samples_by_presence(req.is_present, req.limit, req.offset, req.sort_by, req.sort_desc)
            .await?;

        Ok(Response::new(GetSampleByPresenceResponse {
            samples: samples.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    pub async fn search_samples(
        &self,
        request: Request<SearchSamplesRequest>,
    ) -> Result<Response<SearchSamplesResponse>, Status> {
        debug!("SearchSamples request: {:?}", request);
        let req = request.into_inner();

        let (samples, total_count) = self
            .service
            .search_samples(&req.query, req.limit, req.offset, req.present_only, req.extension_filter)
            .await?;

        Ok(Response::new(SearchSamplesResponse {
            samples: samples.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    pub async fn get_sample_stats(
        &self,
        _request: Request<GetSampleStatsRequest>,
    ) -> Result<Response<GetSampleStatsResponse>, Status> {
        debug!("GetSampleStats request");

        let stats = self.service.get_sample_stats().await?;

        Ok(Response::new(GetSampleStatsResponse {
            total_samples: stats.total_samples,
            present_samples: stats.present_samples,
            missing_samples: stats.missing_samples,
            unique_paths: stats.unique_paths,
            samples_by_extension: stats.samples_by_extension,
            total_estimated_size_bytes: stats.total_estimated_size_bytes,
        }))
    }

    pub async fn get_all_sample_usage_numbers(
        &self,
        _request: Request<GetAllSampleUsageNumbersRequest>,
    ) -> Result<Response<GetAllSampleUsageNumbersResponse>, Status> {
        debug!("GetAllSampleUsageNumbers request");

        let usage_info = self.service.get_all_sample_usage_numbers().await?;

        let sample_usages = usage_info
            .into_iter()
            .map(|info| SampleUsage {
                sample_id: info.sample_id,
                name: info.name,
                path: info.path,
                usage_count: info.usage_count,
                project_count: info.project_count,
            })
            .collect();

        Ok(Response::new(GetAllSampleUsageNumbersResponse { sample_usages }))
    }

    pub async fn get_projects_by_sample(
        &self,
        request: Request<GetProjectsBySampleRequest>,
    ) -> Result<Response<GetProjectsBySampleResponse>, Status> {
        debug!("GetProjectsBySample request: {:?}", request);
        let req = request.into_inner();

        let (projects, total_count) = self
            .service
            .get_projects_by_sample(&req.sample_id, req.limit, req.offset)
            .await?;

        let db_arc = self.service.db_handle();
        let mut db = db_arc.lock().await;
        let mut proto_projects = Vec::new();
        for project in projects {
            match convert_live_set_to_proto(project, &mut db) {
                Ok(proto_project) => proto_projects.push(proto_project),
                Err(e) => return Err(Status::internal(format!("Database error: {}", e))),
            }
        }

        Ok(Response::new(GetProjectsBySampleResponse {
            projects: proto_projects,
            total_count,
        }))
    }

    pub async fn refresh_sample_presence_status(
        &self,
        _request: Request<RefreshSamplePresenceStatusRequest>,
    ) -> Result<Response<RefreshSamplePresenceStatusResponse>, Status> {
        debug!("RefreshSamplePresenceStatus request");

        let result = self.service.refresh_sample_presence_status().await?;

        Ok(Response::new(RefreshSamplePresenceStatusResponse {
            total_samples_checked: result.total_samples_checked,
            samples_now_present: result.samples_now_present,
            samples_now_missing: result.samples_now_missing,
            samples_unchanged: result.samples_unchanged,
            success: true,
            error_message: None,
        }))
    }

    pub async fn get_sample_analytics(
        &self,
        _request: Request<GetSampleAnalyticsRequest>,
    ) -> Result<Response<GetSampleAnalyticsResponse>, Status> {
        debug!("GetSampleAnalytics request");

        let analytics = self.service.get_sample_analytics().await?;

        let top_used_samples = analytics
            .top_used_samples
            .into_iter()
            .map(|info| SampleUsage {
                sample_id: info.sample_id,
                name: info.name,
                path: info.path,
                usage_count: info.usage_count,
                project_count: info.project_count,
            })
            .collect();

        let extensions = analytics
            .extensions
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    ExtensionAnalytics {
                        count: value.count,
                        total_size_bytes: value.total_size_bytes,
                        present_count: value.present_count,
                        missing_count: value.missing_count,
                        average_usage_count: value.average_usage_count,
                    },
                )
            })
            .collect();

        let proto_analytics = super::super::samples::SampleAnalytics {
            most_used_samples_count: analytics.most_used_samples_count,
            moderately_used_samples_count: analytics.moderately_used_samples_count,
            rarely_used_samples_count: analytics.rarely_used_samples_count,
            unused_samples_count: analytics.unused_samples_count,
            extensions,
            missing_samples_percentage: analytics.missing_samples_percentage,
            present_samples_percentage: analytics.present_samples_percentage,
            total_storage_bytes: analytics.total_storage_bytes,
            present_storage_bytes: analytics.present_storage_bytes,
            missing_storage_bytes: analytics.missing_storage_bytes,
            top_used_samples,
            recently_added_samples: analytics.recently_added_samples,
        };

        Ok(Response::new(GetSampleAnalyticsResponse {
            analytics: Some(proto_analytics),
        }))
    }

    pub async fn get_sample_extensions(
        &self,
        _request: Request<GetSampleExtensionsRequest>,
    ) -> Result<Response<GetSampleExtensionsResponse>, Status> {
        debug!("GetSampleExtensions request");

        let extensions = self.service.get_sample_extensions().await?;

        let proto_extensions = extensions
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    ExtensionAnalytics {
                        count: value.count,
                        total_size_bytes: value.total_size_bytes,
                        present_count: value.present_count,
                        missing_count: value.missing_count,
                        average_usage_count: value.average_usage_count,
                    },
                )
            })
            .collect();

        Ok(Response::new(GetSampleExtensionsResponse {
            extensions: proto_extensions,
        }))
    }
}
