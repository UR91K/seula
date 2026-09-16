use log::debug;
use tonic::{Request, Response, Status};

use crate::services::TagsService;
use super::super::tags::*;
use super::super::common::*;

#[derive(Clone)]
pub struct TagsHandler {
    pub service: TagsService,
}

impl TagsHandler {
    pub fn new(service: TagsService) -> Self {
        Self { service }
    }

    fn to_proto(row: (String, String, i64)) -> Tag {
        let (id, name, created_at) = row;
        Tag { id, name, created_at }
    }

    pub async fn get_tags(
        &self,
        _request: Request<GetTagsRequest>,
    ) -> Result<Response<GetTagsResponse>, Status> {
        debug!("GetTags request");

        let tags = self
            .service
            .list_tags()
            .await?
            .into_iter()
            .map(Self::to_proto)
            .collect();

        Ok(Response::new(GetTagsResponse { tags }))
    }

    pub async fn create_tag(
        &self,
        request: Request<CreateTagRequest>,
    ) -> Result<Response<CreateTagResponse>, Status> {
        debug!("CreateTag request: {:?}", request);
        let req = request.into_inner();

        let tag = self.service.create_tag(&req.name).await?;

        debug!("Successfully created tag: {}", req.name);
        Ok(Response::new(CreateTagResponse {
            tag: Some(Self::to_proto(tag)),
        }))
    }

    pub async fn update_tag(
        &self,
        request: Request<UpdateTagRequest>,
    ) -> Result<Response<UpdateTagResponse>, Status> {
        debug!("UpdateTag request: {:?}", request);
        let req = request.into_inner();

        let tag = self.service.update_tag(&req.tag_id, &req.name).await?;

        debug!("Successfully updated tag: {}", req.tag_id);
        Ok(Response::new(UpdateTagResponse {
            tag: Some(Self::to_proto(tag)),
        }))
    }

    pub async fn delete_tag(
        &self,
        request: Request<DeleteTagRequest>,
    ) -> Result<Response<DeleteTagResponse>, Status> {
        debug!("DeleteTag request: {:?}", request);
        let req = request.into_inner();

        self.service.delete_tag(&req.tag_id).await?;

        debug!("Successfully deleted tag: {}", req.tag_id);
        Ok(Response::new(DeleteTagResponse { success: true }))
    }

    pub async fn tag_project(
        &self,
        request: Request<TagProjectRequest>,
    ) -> Result<Response<TagProjectResponse>, Status> {
        debug!("TagProject request: {:?}", request);
        let req = request.into_inner();

        self.service
            .tag_project(&req.project_id, &req.tag_id)
            .await?;

        debug!(
            "Successfully tagged project {} with tag {}",
            req.project_id, req.tag_id
        );
        Ok(Response::new(TagProjectResponse { success: true }))
    }

    pub async fn untag_project(
        &self,
        request: Request<UntagProjectRequest>,
    ) -> Result<Response<UntagProjectResponse>, Status> {
        debug!("UntagProject request: {:?}", request);
        let req = request.into_inner();

        self.service
            .untag_project(&req.project_id, &req.tag_id)
            .await?;

        debug!(
            "Successfully untagged project {} from tag {}",
            req.project_id, req.tag_id
        );
        Ok(Response::new(UntagProjectResponse { success: true }))
    }

    // Batch Tag Operations
    pub async fn batch_tag_projects(
        &self,
        request: Request<BatchTagProjectsRequest>,
    ) -> Result<Response<BatchTagProjectsResponse>, Status> {
        debug!("BatchTagProjects request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_tag_projects(&req.project_ids, &req.tag_ids)
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

        Ok(Response::new(BatchTagProjectsResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn batch_untag_projects(
        &self,
        request: Request<BatchUntagProjectsRequest>,
    ) -> Result<Response<BatchUntagProjectsResponse>, Status> {
        debug!("BatchUntagProjects request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_untag_projects(&req.project_ids, &req.tag_ids)
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

        Ok(Response::new(BatchUntagProjectsResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn get_tag(
        &self,
        request: Request<GetTagRequest>,
    ) -> Result<Response<GetTagResponse>, Status> {
        debug!("GetTag request: {:?}", request);
        let req = request.into_inner();

        let tag = self.service.get_tag(&req.tag_id).await?.map(Self::to_proto);
        if tag.is_none() {
            debug!("Tag not found: {}", req.tag_id);
        }

        Ok(Response::new(GetTagResponse { tag }))
    }

    pub async fn search_tags(
        &self,
        request: Request<SearchTagsRequest>,
    ) -> Result<Response<SearchTagsResponse>, Status> {
        debug!("SearchTags request: {:?}", request);
        let req = request.into_inner();

        let (tag_data, total_count) = self
            .service
            .search_tags(&req.query, req.limit, req.offset)
            .await?;

        let tags = tag_data.into_iter().map(Self::to_proto).collect();
        Ok(Response::new(SearchTagsResponse { tags, total_count }))
    }

    pub async fn get_projects_by_tag(
        &self,
        request: Request<GetProjectsByTagRequest>,
    ) -> Result<Response<GetProjectsByTagResponse>, Status> {
        debug!("GetProjectsByTag request: {:?}", request);
        let req = request.into_inner();

        let live_sets = self.service.get_projects_by_tag(&req.tag_id).await?;

        // Convert LiveSets to proto Projects. Each conversion needs its own DB lock,
        // so this happens outside the service (matches prior behavior).
        let mut projects = Vec::new();
        {
            let db_arc = self.service.db_handle();
            let mut db = db_arc.lock().await;
            for live_set in live_sets {
                match super::utils::convert_live_set_to_proto(live_set, &mut db) {
                    Ok(project) => projects.push(project),
                    Err(e) => {
                        log::error!("Failed to convert LiveSet to proto: {:?}", e);
                        continue;
                    }
                }
            }
        }

        let total_count = projects.len() as i32;
        let offset = req.offset.unwrap_or(0) as usize;
        let limit = req.limit.map(|l| l as usize);

        let paginated_projects = if let Some(limit_val) = limit {
            projects.into_iter().skip(offset).take(limit_val).collect()
        } else {
            projects.into_iter().skip(offset).collect()
        };

        Ok(Response::new(GetProjectsByTagResponse {
            projects: paginated_projects,
            total_count,
        }))
    }

    pub async fn get_tag_statistics(
        &self,
        _request: Request<GetTagStatisticsRequest>,
    ) -> Result<Response<GetTagStatisticsResponse>, Status> {
        debug!("GetTagStatistics request");

        let stats = self.service.get_tag_statistics().await?;

        let most_used_tags = stats
            .most_used_tags
            .into_iter()
            .map(|info| super::super::tags::TagUsageInfo {
                tag_id: info.tag_id,
                name: info.name,
                project_count: info.project_count,
                usage_percentage: info.usage_percentage,
            })
            .collect();

        let least_used_tags = stats
            .least_used_tags
            .into_iter()
            .map(|info| super::super::tags::TagUsageInfo {
                tag_id: info.tag_id,
                name: info.name,
                project_count: info.project_count,
                usage_percentage: info.usage_percentage,
            })
            .collect();

        let proto_stats = super::super::tags::TagStatistics {
            total_tags: stats.total_tags,
            tags_in_use: stats.tags_in_use,
            unused_tags: stats.unused_tags,
            average_tags_per_project: stats.average_tags_per_project,
            most_used_tags,
            least_used_tags,
            projects_with_no_tags: stats.projects_with_no_tags,
            projects_with_tags: stats.projects_with_tags,
        };

        Ok(Response::new(GetTagStatisticsResponse {
            statistics: Some(proto_stats),
        }))
    }

    pub async fn get_all_tags_with_usage(
        &self,
        request: Request<GetAllTagsWithUsageRequest>,
    ) -> Result<Response<GetAllTagsWithUsageResponse>, Status> {
        debug!("GetAllTagsWithUsage request: {:?}", request);
        let req = request.into_inner();

        let (tag_data, total_count) = self
            .service
            .get_all_tags_with_usage(
                req.limit,
                req.offset,
                req.sort_by,
                req.sort_desc,
                req.min_usage_count,
            )
            .await?;

        let tags = tag_data
            .into_iter()
            .map(|info| super::super::tags::TagUsageInfo {
                tag_id: info.tag_id,
                name: info.name,
                project_count: info.project_count,
                usage_percentage: info.usage_percentage,
            })
            .collect();

        Ok(Response::new(GetAllTagsWithUsageResponse { tags, total_count }))
    }
}
