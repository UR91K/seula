//! Domain services shared by the gRPC handlers and the CLI/TUI (ADR-0018). Each
//! service owns validation and orchestration for one entity domain; gRPC handlers
//! and CLI commands both call into these instead of the database directly.

pub mod collections;
pub mod config;
pub mod media;
pub mod plugins;
pub mod project;
pub mod samples;
pub mod search;
pub mod system;
pub mod tags;
pub mod tasks;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::ProjectDatabase;
use crate::media::MediaStorageManager;

pub use collections::{CollectionDetail, CollectionsService};
pub use config::ConfigService;
pub use media::MediaService;
pub use plugins::PluginsService;
pub use project::{DeletionScope, ProjectsService};
pub use samples::SamplesService;
pub use search::SearchService;
pub use system::SystemService;
pub use tags::TagsService;
pub use tasks::TasksService;

#[derive(Clone)]
pub struct Services {
    pub tags: TagsService,
    pub projects: ProjectsService,
    pub collections: CollectionsService,
    pub search: SearchService,
    pub tasks: TasksService,
    pub plugins: PluginsService,
    pub samples: SamplesService,
    pub config: ConfigService,
    pub media: MediaService,
}

impl Services {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>, media_storage: Arc<MediaStorageManager>) -> Self {
        Self {
            tags: TagsService::new(Arc::clone(&db)),
            projects: ProjectsService::new(Arc::clone(&db)),
            collections: CollectionsService::new(Arc::clone(&db)),
            search: SearchService::new(Arc::clone(&db)),
            tasks: TasksService::new(Arc::clone(&db)),
            plugins: PluginsService::new(Arc::clone(&db)),
            samples: SamplesService::new(Arc::clone(&db)),
            config: ConfigService::new(),
            media: MediaService::new(Arc::clone(&db), media_storage),
        }
    }
}
