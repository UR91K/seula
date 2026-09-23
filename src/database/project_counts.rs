//! How many projects use each of a page of plugins or samples (ADR-0034), counting
//! archived projects or not (ADR-0040).
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

/// Which projects a plugin's or sample's usage counts, and its used-in list shows
/// (ADR-0040). The default leaves archived projects out, as the projects view does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProjectScope {
    #[default]
    Active,
    /// Archived projects too.
    All,
}

impl ProjectScope {
    /// A join that keeps only rows whose project is in scope, for a query over a
    /// junction table. `project_id` is the junction's project column, such as
    /// `pp.project_id`. Empty for `All`.
    pub fn join(self, project_id: &str) -> String {
        match self {
            ProjectScope::Active => format!(
                "JOIN projects scope_p ON scope_p.id = {} AND scope_p.is_active = 1",
                project_id
            ),
            ProjectScope::All => String::new(),
        }
    }

    /// A condition to AND onto a query over `projects p`. Empty for `All`.
    pub fn and_projects(self) -> &'static str {
        match self {
            ProjectScope::Active => "AND p.is_active = 1",
            ProjectScope::All => "",
        }
    }
}

impl ProjectDatabase {
    /// Project counts for the given sample ids. Ids used by no project map to 0.
    pub fn sample_project_counts(
        &self,
        ids: &[String],
        scope: ProjectScope,
    ) -> Result<HashMap<String, i32>, DatabaseError> {
        self.project_counts("project_samples", "sample_id", ids, scope)
    }

    /// Project counts for the given plugin ids. Ids used by no project map to 0.
    pub fn plugin_project_counts(
        &self,
        ids: &[String],
        scope: ProjectScope,
    ) -> Result<HashMap<String, i32>, DatabaseError> {
        self.project_counts("project_plugins", "plugin_id", ids, scope)
    }

    fn project_counts(
        &self,
        table: &'static str,
        column: &'static str,
        ids: &[String],
        scope: ProjectScope,
    ) -> Result<HashMap<String, i32>, DatabaseError> {
        let join = scope.join("j.project_id");
        let mut counts: HashMap<String, i32> = ids.iter().map(|id| (id.clone(), 0)).collect();
        for chunk in ids.chunks(CHUNK) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let sql = format!(
                "SELECT j.{column}, COUNT(*) FROM {table} j {join} WHERE j.{column} IN ({placeholders}) GROUP BY j.{column}"
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
