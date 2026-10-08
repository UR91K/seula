//! Domain services shared by the HTTP handlers and the CLI (ADR-0018). Each
//! service owns validation and orchestration for one entity domain; the HTTP handlers
//! and CLI commands both call into these instead of the database directly.

pub mod collections;
pub mod config;
pub mod media;
pub mod plugins;
pub mod project;
pub mod samples;
pub mod scan;
pub mod search;
pub mod statistics;
pub mod system;
pub mod tags;
pub mod tasks;

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::ProjectDatabase;
use crate::error::DatabaseError;
use crate::media::MediaStorageManager;

pub use collections::{CollectionDetail, CollectionsService};
pub use config::ConfigService;
pub use media::MediaService;
pub use plugins::PluginsService;
pub use project::{DeletionScope, ProjectsService};
pub use samples::SamplesService;
pub use scan::{ScanProgress, ScanStatus, WatcherEvent, WatcherEventType};
pub use search::SearchService;
pub use statistics::Statistics;
pub use system::SystemService;
pub use tags::TagsService;
pub use tasks::TasksService;

/// One page of `items`, for the lists that are paged in memory rather than in SQL.
///
/// A negative `offset` or `limit` is a malformed request, so it is refused. It used to
/// be cast to `usize`, which wraps: a negative offset skipped everything and a negative
/// limit took everything.
pub fn paginate<T>(
    items: Vec<T>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<T>, DatabaseError> {
    let offset = match offset {
        Some(n) if n < 0 => {
            return Err(DatabaseError::InvalidOperation(
                "offset must not be negative".to_string(),
            ))
        }
        Some(n) => n as usize,
        None => 0,
    };
    let rest = items.into_iter().skip(offset);
    match limit {
        Some(n) if n < 0 => Err(DatabaseError::InvalidOperation(
            "limit must not be negative".to_string(),
        )),
        Some(n) => Ok(rest.take(n as usize).collect()),
        None => Ok(rest.collect()),
    }
}

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
