use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::search::{SearchQuery, SearchResult};
use crate::database::ProjectDatabase;
use crate::error::DatabaseError;

#[derive(Clone)]
pub struct SearchService {
    db: Arc<Mutex<ProjectDatabase>>,
}

impl SearchService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>) -> Self {
        Self { db }
    }

    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    /// Runs the query and paginates in-process -- `search_fts` itself has no
    /// limit/offset, both callers used to skip/take identically after the fact.
    pub async fn search(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<SearchResult>, i32), DatabaseError> {
        let mut db = self.db.lock().await;
        let search_query = SearchQuery::parse(query);
        let results = db.search_fts(&search_query)?;
        let total_count = results.len() as i32;

        let iter = results.into_iter().skip(offset.unwrap_or(0) as usize);
        let page = if let Some(limit) = limit {
            iter.take(limit as usize).collect()
        } else {
            iter.collect()
        };

        Ok((page, total_count))
    }
}
