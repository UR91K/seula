//! Domain services shared by the gRPC handlers and the CLI/TUI (ADR pending: shared
//! service layer). Each service owns validation and orchestration for one entity
//! domain; gRPC handlers and CLI commands both call into these instead of the
//! database directly. See docs/decisions for the audit that motivated this.

pub mod tags;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::ProjectDatabase;

pub use tags::TagsService;

#[derive(Clone)]
pub struct Services {
    pub tags: TagsService,
}

impl Services {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self {
            tags: TagsService::new(Arc::clone(&db)),
        }
    }
}
