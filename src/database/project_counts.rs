//! How many projects use each of a page of plugins or samples (ADR-0034).
//!
//! One query per page rather than a join in every list query, so the list functions
//! keep their signatures and the gRPC and CLI callers are untouched. The junction
//! tables hold one row per (project, item) pair, so a row count is a project count.

use std::collections::HashMap;

use crate::error::DatabaseError;

use super::ProjectDatabase;

/// SQLite's bound-parameter limit is far higher in current builds, but older ones
/// stop at 999.
const CHUNK: usize = 500;

impl ProjectDatabase {
    /// Project counts for the given sample ids. Ids used by no project map to 0.
    pub fn sample_project_counts(&self, ids: &[String]) -> Result<HashMap<String, i32>, DatabaseError> {
        self.project_counts("project_samples", "sample_id", ids)
    }

    /// Project counts for the given plugin ids. Ids used by no project map to 0.
    pub fn plugin_project_counts(&self, ids: &[String]) -> Result<HashMap<String, i32>, DatabaseError> {
        self.project_counts("project_plugins", "plugin_id", ids)
    }

    fn project_counts(
        &self,
        table: &'static str,
        column: &'static str,
        ids: &[String],
    ) -> Result<HashMap<String, i32>, DatabaseError> {
        let mut counts: HashMap<String, i32> = ids.iter().map(|id| (id.clone(), 0)).collect();
        for chunk in ids.chunks(CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT {column}, COUNT(*) FROM {table} WHERE {column} IN ({placeholders}) GROUP BY {column}"
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
            })?;
            for row in rows {
                let (id, count) = row?;
                counts.insert(id, count);
            }
        }
        Ok(counts)
    }
}
