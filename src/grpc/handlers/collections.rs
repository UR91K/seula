use tracing::debug;
use tonic::{Code, Request, Response, Status};

use super::super::collections::*;
use super::super::common::*;
use crate::services::{CollectionDetail, CollectionsService};

impl From<CollectionDetail> for Collection {
    fn from(d: CollectionDetail) -> Self {
        Collection {
            id: d.id,
            name: d.name,
            description: d.description,
            notes: d.notes,
            created_at: d.created_at,
            modified_at: d.modified_at,
            project_ids: d.project_ids,
            cover_art_id: d.cover_art_id,
            total_duration_seconds: d.total_duration_seconds,
            project_count: d.project_count,
        }
    }
}

#[derive(Clone)]
pub struct CollectionsHandler {
    pub service: CollectionsService,
}

impl CollectionsHandler {
    pub fn new(service: CollectionsService) -> Self {
        Self { service }
    }

    pub async fn get_collections(
        &self,
        request: Request<GetCollectionsRequest>,
    ) -> Result<Response<GetCollectionsResponse>, Status> {
        debug!("GetCollections request: {:?}", request);
        let req = request.into_inner();

        let (collections, total_count) = self
            .service
            .list_collections(req.limit, req.offset, req.sort_by, req.sort_desc)
            .await?;

        Ok(Response::new(GetCollectionsResponse {
            collections: collections.into_iter().map(Into::into).collect(),
            total_count,
        }))
    }

    pub async fn get_collection(
        &self,
        request: Request<GetCollectionRequest>,
    ) -> Result<Response<GetCollectionResponse>, Status> {
        debug!("GetCollection request: {:?}", request);
        let req = request.into_inner();

        let collection = self.service.get_collection(&req.collection_id).await?;
        if collection.is_none() {
            debug!("Collection {} not found", req.collection_id);
        }

        Ok(Response::new(GetCollectionResponse {
            collection: collection.map(Into::into),
        }))
    }

    pub async fn create_collection(
        &self,
        request: Request<CreateCollectionRequest>,
    ) -> Result<Response<CreateCollectionResponse>, Status> {
        debug!("CreateCollection request: {:?}", request);
        let req = request.into_inner();

        let collection = self
            .service
            .create_collection(&req.name, req.description.as_deref(), req.notes.as_deref())
            .await?;

        Ok(Response::new(CreateCollectionResponse {
            collection: Some(collection.into()),
        }))
    }

    pub async fn update_collection(
        &self,
        request: Request<UpdateCollectionRequest>,
    ) -> Result<Response<UpdateCollectionResponse>, Status> {
        debug!("UpdateCollection request: {:?}", request);
        let req = request.into_inner();

        let collection = self
            .service
            .update_collection(
                &req.collection_id,
                req.name.as_deref(),
                req.description.as_deref(),
                req.notes.as_deref(),
            )
            .await?;

        Ok(Response::new(UpdateCollectionResponse {
            collection: Some(collection.into()),
        }))
    }

    pub async fn delete_collection(
        &self,
        request: Request<DeleteCollectionRequest>,
    ) -> Result<Response<DeleteCollectionResponse>, Status> {
        debug!("DeleteCollection request: {:?}", request);
        let req = request.into_inner();

        self.service.delete_collection(&req.collection_id).await?;

        debug!("Successfully deleted collection: {}", req.collection_id);
        Ok(Response::new(DeleteCollectionResponse { success: true }))
    }

    pub async fn duplicate_collection(
        &self,
        request: Request<DuplicateCollectionRequest>,
    ) -> Result<Response<DuplicateCollectionResponse>, Status> {
        debug!("DuplicateCollection request: {:?}", request);
        let req = request.into_inner();

        let collection = self
            .service
            .duplicate_collection(
                &req.collection_id,
                &req.new_name,
                req.new_description.as_deref(),
                req.new_notes.as_deref(),
            )
            .await?;

        Ok(Response::new(DuplicateCollectionResponse {
            collection: Some(collection.into()),
        }))
    }

    pub async fn add_project_to_collection(
        &self,
        request: Request<AddProjectToCollectionRequest>,
    ) -> Result<Response<AddProjectToCollectionResponse>, Status> {
        debug!("AddProjectToCollection request: {:?}", request);
        let req = request.into_inner();

        self.service
            .add_project_to_collection(&req.collection_id, &req.project_id)
            .await?;

        debug!(
            "Successfully added project {} to collection {}",
            req.project_id, req.collection_id
        );
        Ok(Response::new(AddProjectToCollectionResponse { success: true }))
    }

    pub async fn remove_project_from_collection(
        &self,
        request: Request<RemoveProjectFromCollectionRequest>,
    ) -> Result<Response<RemoveProjectFromCollectionResponse>, Status> {
        debug!("RemoveProjectFromCollection request: {:?}", request);
        let req = request.into_inner();

        self.service
            .remove_project_from_collection(&req.collection_id, &req.project_id)
            .await?;

        debug!(
            "Successfully removed project {} from collection {}",
            req.project_id, req.collection_id
        );
        Ok(Response::new(RemoveProjectFromCollectionResponse { success: true }))
    }

    pub async fn reorder_collection(
        &self,
        request: Request<ReorderCollectionRequest>,
    ) -> Result<Response<ReorderCollectionResponse>, Status> {
        debug!("ReorderCollection request: {:?}", request);
        let req = request.into_inner();

        self.service
            .reorder_collection(&req.collection_id, &req.project_ids)
            .await
            .map_err(|e| match e {
                crate::error::DatabaseError::InvalidOperation(msg) => {
                    Status::new(Code::InvalidArgument, msg)
                }
                other => other.into(),
            })?;

        debug!(
            "Successfully reordered collection {} with {} projects",
            req.collection_id,
            req.project_ids.len()
        );
        Ok(Response::new(ReorderCollectionResponse { success: true }))
    }

    pub async fn get_collection_tasks(
        &self,
        request: Request<GetCollectionTasksRequest>,
    ) -> Result<Response<GetCollectionTasksResponse>, Status> {
        debug!("GetCollectionTasks request: {:?}", request);
        let req = request.into_inner();

        let tasks_data = self.service.get_collection_tasks(&req.collection_id).await?;

        let mut tasks = Vec::new();
        let mut completed_count = 0;
        for (id, project_name, description, completed, created_at) in tasks_data {
            if completed {
                completed_count += 1;
            }
            tasks.push(Task {
                id,
                project_id: project_name, // Using project_name in project_id field to show which project the task belongs to
                description,
                completed,
                created_at,
            });
        }

        let total_tasks = tasks.len() as i32;
        let pending_tasks = total_tasks - completed_count;
        let completion_rate = if total_tasks > 0 {
            completed_count as f64 / total_tasks as f64
        } else {
            0.0
        };

        debug!(
            "Successfully retrieved {} tasks for collection {}",
            total_tasks, req.collection_id
        );
        Ok(Response::new(GetCollectionTasksResponse {
            tasks,
            total_tasks,
            completed_tasks: completed_count,
            pending_tasks,
            completion_rate,
        }))
    }

    pub async fn search_collections(
        &self,
        request: Request<SearchCollectionsRequest>,
    ) -> Result<Response<SearchCollectionsResponse>, Status> {
        debug!("SearchCollections request: {:?}", request);
        let req = request.into_inner();

        let (collections, total_count) = self
            .service
            .search_collections(&req.query, req.limit, req.offset)
            .await?;

        Ok(Response::new(SearchCollectionsResponse {
            collections: collections.into_iter().map(Into::into).collect(),
            total_count,
        }))
    }

    pub async fn get_collection_statistics(
        &self,
        request: Request<GetCollectionStatisticsRequest>,
    ) -> Result<Response<GetCollectionStatisticsResponse>, Status> {
        debug!("GetCollectionStatistics request: {:?}", request);
        let req = request.into_inner();

        let stats = self
            .service
            .get_collection_statistics(&req.collection_id)
            .await?;

        Ok(Response::new(GetCollectionStatisticsResponse {
            project_count: stats.project_count,
            total_duration_seconds: stats.total_duration_seconds,
            average_tempo: stats.average_tempo,
            total_plugins: stats.total_plugins,
            total_samples: stats.total_samples,
            total_tags: stats.total_tags,
            most_common_key: stats.most_common_key,
            most_common_time_signature: stats.most_common_time_signature,
        }))
    }

    // Batch Collection Operations
    pub async fn batch_add_to_collection(
        &self,
        request: Request<BatchAddToCollectionRequest>,
    ) -> Result<Response<BatchAddToCollectionResponse>, Status> {
        debug!("BatchAddToCollection request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_add_to_collection(&req.project_ids, &req.collection_id)
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

        Ok(Response::new(BatchAddToCollectionResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn batch_remove_from_collection(
        &self,
        request: Request<BatchRemoveFromCollectionRequest>,
    ) -> Result<Response<BatchRemoveFromCollectionResponse>, Status> {
        debug!("BatchRemoveFromCollection request: {:?}", request);
        let req = request.into_inner();

        let results = self
            .service
            .batch_remove_from_collection(&req.project_ids, &req.collection_id)
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

        Ok(Response::new(BatchRemoveFromCollectionResponse {
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }

    pub async fn batch_create_collection_from(
        &self,
        request: Request<BatchCreateCollectionFromRequest>,
    ) -> Result<Response<BatchCreateCollectionFromResponse>, Status> {
        debug!("BatchCreateCollectionFrom request: {:?}", request);
        let req = request.into_inner();

        let (collection, results) = self
            .service
            .batch_create_collection_from(
                &req.collection_name,
                &req.project_ids,
                req.description.as_deref(),
                req.notes.as_deref(),
            )
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

        Ok(Response::new(BatchCreateCollectionFromResponse {
            collection: collection.map(Into::into),
            results: batch_results,
            successful_count,
            failed_count,
        }))
    }
}
