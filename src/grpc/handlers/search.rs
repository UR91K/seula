use tracing::{debug, error};
use tonic::{Request, Response, Status};

use super::super::search::*;
use super::utils::convert_live_set_to_proto;
use crate::services::SearchService;

#[derive(Clone)]
pub struct SearchHandler {
    pub service: SearchService,
}

impl SearchHandler {
    pub fn new(service: SearchService) -> Self {
        Self { service }
    }

    pub async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        debug!("Search request: {:?}", request);
        let req = request.into_inner();

        let (results, total_count) = self.service.search(&req.query, req.limit, req.offset).await?;

        let db_arc = self.service.db_handle();
        let mut db = db_arc.lock().await;
        let mut proto_projects = Vec::new();
        for search_result in results {
            match convert_live_set_to_proto(search_result.project, &mut db) {
                Ok(proto_project) => proto_projects.push(proto_project),
                Err(e) => {
                    error!("Failed to convert project to proto: {}", e);
                    return Err(Status::internal(format!("Failed to convert project: {}", e)));
                }
            }
        }

        Ok(Response::new(SearchResponse {
            projects: proto_projects,
            total_count,
        }))
    }
}
