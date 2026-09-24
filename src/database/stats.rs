//! Library statistics for the stats view, and the filtered project statistics behind
//! `/api/v1/projects/statistics`.
//!
//! Every library statistic takes a project scope (ADR-0045): with `Active`, archived
//! projects and everything only they use drop out of every figure on the page, not
//! just the project count. Time series come back oldest first with empty periods as
//! zero, and bins are equal width with empty bins as zero, so a chart drawn from them
//! shows a gap where there is one.

use chrono::{Datelike, NaiveDate};
use rusqlite::OptionalExtension;

use crate::error::DatabaseError;

use super::{ProjectDatabase, ProjectScope};

/// Width of a tempo histogram bin, in BPM.
const TEMPO_BIN: i32 = 10;

/// The overview figures, each split into its states. The totals are the sums, so a
/// total can never disagree with its parts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LibraryCounts {
    /// Both project figures ignore the scope: they are what the scope chooses between.
    pub projects_active: i32,
    pub projects_archived: i32,
    /// Distinct plugins used by projects in scope, by the tri-state installed flag
    /// (ADR-0012).
    pub plugins_installed: i32,
    pub plugins_missing: i32,
    pub plugins_not_scanned: i32,
    /// Distinct samples used by projects in scope.
    pub samples_present: i32,
    pub samples_missing: i32,
    /// Collections holding at least one project in scope, and the rest.
    pub collections_with_projects: i32,
    pub collections_empty: i32,
    /// Tags on at least one project in scope, and the rest.
    pub tags_in_use: i32,
    pub tags_unused: i32,
    /// Tasks on projects in scope.
    pub tasks_completed: i32,
    pub tasks_pending: i32,
}

/// The first day of each of the `months` calendar months ending with `today`'s,
/// oldest first.
pub fn calendar_months(today: NaiveDate, months: u32) -> Vec<(i32, u32)> {
    let mut year = today.year();
    let mut month = today.month();
    let mut out = Vec::with_capacity(months as usize);
    for _ in 0..months {
        out.push((year, month));
        if month == 1 {
            month = 12;
            year -= 1;
        } else {
            month -= 1;
        }
    }
    out.reverse();
    out
}

fn month_start_epoch(year: i32, month: u32) -> i64 {
    NaiveDate::from_ymd_opt(year, month, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
        .unwrap_or(0)
}

fn day_epoch(day: NaiveDate) -> i64 {
    day.and_hms_opt(0, 0, 0).map(|dt| dt.and_utc().timestamp()).unwrap_or(0)
}

impl ProjectDatabase {
    // Statistics methods

    /// Unscoped totals for the CLI's system summary.
    pub fn get_basic_counts(&self) -> Result<(i32, i32, i32, i32, i32, i32), DatabaseError> {
        let total_projects: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM projects WHERE is_active = true",
            [],
            |row| row.get(0),
        )?;
        let total_plugins: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))?;
        let total_samples: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))?;
        let total_collections: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM collections", [], |row| row.get(0))?;
        let total_tags: i32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))?;
        let total_tasks: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM project_tasks", [], |row| row.get(0))?;

        Ok((
            total_projects,
            total_plugins,
            total_samples,
            total_collections,
            total_tags,
            total_tasks,
        ))
    }

    fn count(&self, sql: &str) -> Result<i32, DatabaseError> {
        Ok(self.conn.query_row(sql, [], |row| row.get(0))?)
    }

    pub fn get_library_counts(&self, scope: ProjectScope) -> Result<LibraryCounts, DatabaseError> {
        let in_scope = scope.and_projects();
        let pp = scope.join("pp.project_id");
        let ps = scope.join("ps.project_id");

        let (projects_active, projects_archived): (i32, i32) = self.conn.query_row(
            "SELECT COALESCE(SUM(is_active = 1), 0), COALESCE(SUM(is_active = 0), 0) FROM projects",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let (plugins_installed, plugins_missing, plugins_not_scanned): (i32, i32, i32) =
            self.conn.query_row(
                &format!(
                    "SELECT COALESCE(SUM(installed = 1), 0), COALESCE(SUM(installed = 0), 0),
                            COALESCE(SUM(installed IS NULL), 0)
                     FROM plugins WHERE id IN (SELECT pp.plugin_id FROM project_plugins pp {pp})"
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

        let (samples_present, samples_missing): (i32, i32) = self.conn.query_row(
            &format!(
                "SELECT COALESCE(SUM(is_present = 1), 0), COALESCE(SUM(is_present = 0), 0)
                 FROM samples WHERE id IN (SELECT ps.sample_id FROM project_samples ps {ps})"
            ),
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let collections = self.count("SELECT COUNT(*) FROM collections")?;
        let collections_with_projects = self.count(&format!(
            "SELECT COUNT(DISTINCT cp.collection_id) FROM collection_projects cp {}",
            scope.join("cp.project_id")
        ))?;

        let tags = self.count("SELECT COUNT(*) FROM tags")?;
        let tags_in_use = self.count(&format!(
            "SELECT COUNT(DISTINCT pt.tag_id) FROM project_tags pt {}",
            scope.join("pt.project_id")
        ))?;

        let (tasks_completed, tasks_pending): (i32, i32) = self.conn.query_row(
            &format!(
                "SELECT COALESCE(SUM(t.completed = 1), 0), COALESCE(SUM(t.completed = 0), 0)
                 FROM project_tasks t JOIN projects p ON p.id = t.project_id WHERE 1 = 1 {in_scope}"
            ),
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        Ok(LibraryCounts {
            projects_active,
            projects_archived,
            plugins_installed,
            plugins_missing,
            plugins_not_scanned,
            samples_present,
            samples_missing,
            collections_with_projects,
            collections_empty: collections - collections_with_projects,
            tags_in_use,
            tags_unused: tags - tags_in_use,
            tasks_completed,
            tasks_pending,
        })
    }

    pub fn get_top_plugins(
        &self,
        limit: i32,
        scope: ProjectScope,
    ) -> Result<Vec<(String, String, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT p.name, COALESCE(p.vendor, 'Unknown'), COUNT(*) as usage_count
             FROM plugins p
             JOIN project_plugins pp ON p.id = pp.plugin_id
             {}
             GROUP BY p.id
             ORDER BY usage_count DESC
             LIMIT ?",
            scope.join("pp.project_id")
        ))?;

        let rows = stmt.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut plugins = Vec::new();
        for row in rows {
            plugins.push(row?);
        }
        Ok(plugins)
    }

    pub fn get_top_vendors(
        &self,
        limit: i32,
        scope: ProjectScope,
    ) -> Result<Vec<(String, i32, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                COALESCE(p.vendor, 'Unknown') as vendor,
                COUNT(DISTINCT p.id) as plugin_count,
                COUNT(*) as usage_count
             FROM plugins p
             JOIN project_plugins pp ON p.id = pp.plugin_id
             {}
             GROUP BY vendor
             ORDER BY usage_count DESC
             LIMIT ?",
            scope.join("pp.project_id")
        ))?;

        let rows = stmt.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut vendors = Vec::new();
        for row in rows {
            vendors.push(row?);
        }
        Ok(vendors)
    }

    /// Equal `TEMPO_BIN`-wide bins, each labelled by its lower edge, from the lowest
    /// tempo used to the highest, with empty bins as zero.
    pub fn get_tempo_distribution(&self, scope: ProjectScope) -> Result<Vec<(f64, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT CAST(p.tempo / {TEMPO_BIN} AS INTEGER) * {TEMPO_BIN} as bin, COUNT(*)
             FROM projects p
             WHERE p.tempo > 0 {}
             GROUP BY bin
             ORDER BY bin",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)))?;
        let mut found = Vec::new();
        for row in rows {
            found.push(row?);
        }

        let (Some(&(first, _)), Some(&(last, _))) = (found.first(), found.last()) else {
            return Ok(Vec::new());
        };
        let mut found = found.into_iter().peekable();
        let mut distribution = Vec::new();
        for bin in (first..=last).step_by(TEMPO_BIN as usize) {
            let count = match found.peek() {
                Some(&(b, c)) if b == bin => {
                    found.next();
                    c
                }
                _ => 0,
            };
            distribution.push((bin as f64, count));
        }
        Ok(distribution)
    }

    /// `"<tonic> <scale>"` per key, most common first. Projects with no key are one
    /// entry with `None`, not a key named "Unknown".
    pub fn get_key_distribution(
        &self,
        scope: ProjectScope,
    ) -> Result<Vec<(Option<String>, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                CASE
                    WHEN p.key_signature_tonic IS NOT NULL AND p.key_signature_scale IS NOT NULL
                    THEN p.key_signature_tonic || ' ' || p.key_signature_scale
                END as key_sig,
                COUNT(*) as count
             FROM projects p
             WHERE 1 = 1 {}
             GROUP BY key_sig
             ORDER BY count DESC",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i32>(1)?))
        })?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    pub fn get_time_signature_distribution(
        &self,
        scope: ProjectScope,
    ) -> Result<Vec<(i32, i32, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT p.time_signature_numerator, p.time_signature_denominator, COUNT(*) as count
             FROM projects p
             WHERE 1 = 1 {}
             GROUP BY p.time_signature_numerator, p.time_signature_denominator
             ORDER BY count DESC",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    /// Every year from the first project's to the last's, oldest first, with empty
    /// years as zero.
    pub fn get_projects_per_year(&self, scope: ProjectScope) -> Result<Vec<(i32, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                CAST(strftime('%Y', datetime(p.created_at, 'unixepoch')) AS INTEGER) as year,
                COUNT(*) as count
             FROM projects p
             WHERE p.created_at IS NOT NULL {}
             GROUP BY year
             ORDER BY year",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?)))?;
        let mut found = std::collections::BTreeMap::new();
        for row in rows {
            let (year, count) = row?;
            found.insert(year, count);
        }

        let (Some(&first), Some(&last)) = (found.keys().next(), found.keys().next_back()) else {
            return Ok(Vec::new());
        };
        Ok((first..=last).map(|y| (y, found.get(&y).copied().unwrap_or(0))).collect())
    }

    /// The `months` calendar months ending with `today`'s, oldest first, with empty
    /// months as zero.
    pub fn get_projects_per_month(
        &self,
        months: u32,
        today: NaiveDate,
        scope: ProjectScope,
    ) -> Result<Vec<(i32, i32, i32)>, DatabaseError> {
        let window = calendar_months(today, months);
        let Some(&(start_year, start_month)) = window.first() else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                CAST(strftime('%Y', datetime(p.created_at, 'unixepoch')) AS INTEGER) as year,
                CAST(strftime('%m', datetime(p.created_at, 'unixepoch')) AS INTEGER) as month,
                COUNT(*) as count
             FROM projects p
             WHERE p.created_at >= ? {}
             GROUP BY year, month",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([month_start_epoch(start_year, start_month)], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, u32>(1)?, row.get::<_, i32>(2)?))
        })?;
        let mut found = std::collections::HashMap::new();
        for row in rows {
            let (year, month, count) = row?;
            found.insert((year, month), count);
        }

        Ok(window
            .into_iter()
            .map(|(y, m)| (y, m as i32, found.get(&(y, m)).copied().unwrap_or(0)))
            .collect())
    }

    pub fn get_duration_analytics(
        &self,
        scope: ProjectScope,
    ) -> Result<(f64, i32, Option<String>), DatabaseError> {
        let in_scope = scope.and_projects();
        let avg_duration: f64 = self.conn.query_row(
            &format!("SELECT AVG(CAST(p.duration_seconds AS REAL)) FROM projects p WHERE p.duration_seconds IS NOT NULL {in_scope}"),
            [],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let short_projects: i32 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM projects p WHERE p.duration_seconds IS NOT NULL AND p.duration_seconds < 40 {in_scope}"),
            [],
            |row| row.get(0)
        )?;

        let longest_project: Option<String> = self.conn.query_row(
            &format!("SELECT p.id FROM projects p WHERE p.duration_seconds IS NOT NULL {in_scope} ORDER BY p.duration_seconds DESC LIMIT 1"),
            [],
            |row| row.get(0)
        ).optional()?;

        Ok((avg_duration, short_projects, longest_project))
    }

    /// Plugins and samples per project, averaged over every project in scope,
    /// including those with none.
    pub fn get_complexity_metrics(&self, scope: ProjectScope) -> Result<(f64, f64), DatabaseError> {
        let projects = self.count(&format!(
            "SELECT COUNT(*) FROM projects p WHERE 1 = 1 {}",
            scope.and_projects()
        ))?;
        if projects == 0 {
            return Ok((0.0, 0.0));
        }
        let plugin_uses = self.count(&format!(
            "SELECT COUNT(*) FROM project_plugins pp {}",
            scope.join("pp.project_id")
        ))?;
        let sample_uses = self.count(&format!(
            "SELECT COUNT(*) FROM project_samples ps {}",
            scope.join("ps.project_id")
        ))?;

        Ok((
            plugin_uses as f64 / projects as f64,
            sample_uses as f64 / projects as f64,
        ))
    }

    pub fn get_most_complex_projects(
        &self,
        limit: i32,
        scope: ProjectScope,
    ) -> Result<Vec<(String, i32, i32, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT
                p.id,
                COALESCE(plugin_counts.plugin_count, 0) as plugin_count,
                COALESCE(sample_counts.sample_count, 0) as sample_count,
                (COALESCE(plugin_counts.plugin_count, 0) + COALESCE(sample_counts.sample_count, 0)) as complexity_score
             FROM projects p
             LEFT JOIN (
                 SELECT project_id, COUNT(*) as plugin_count
                 FROM project_plugins
                 GROUP BY project_id
             ) plugin_counts ON p.id = plugin_counts.project_id
             LEFT JOIN (
                 SELECT project_id, COUNT(*) as sample_count
                 FROM project_samples
                 GROUP BY project_id
             ) sample_counts ON p.id = sample_counts.project_id
             WHERE 1 = 1 {}
             ORDER BY complexity_score DESC
             LIMIT ?",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row?);
        }
        Ok(projects)
    }

    pub fn get_top_samples(
        &self,
        limit: i32,
        scope: ProjectScope,
    ) -> Result<Vec<(String, String, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT s.name, s.path, COUNT(*) as usage_count
             FROM samples s
             JOIN project_samples ps ON s.id = ps.sample_id
             {}
             GROUP BY s.id
             ORDER BY usage_count DESC
             LIMIT ?",
            scope.join("ps.project_id")
        ))?;

        let rows = stmt.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
            ))
        })?;

        let mut samples = Vec::new();
        for row in rows {
            samples.push(row?);
        }
        Ok(samples)
    }

    pub fn get_top_tags(&self, limit: i32, scope: ProjectScope) -> Result<Vec<(String, i32)>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT t.name, COUNT(*) as usage_count
             FROM tags t
             JOIN project_tags pt ON t.id = pt.tag_id
             {}
             GROUP BY t.id
             ORDER BY usage_count DESC
             LIMIT ?",
            scope.join("pt.project_id")
        ))?;

        let rows = stmt.query_map([limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }

    /// Completed and pending tasks on projects in scope, and the completed share as a
    /// fraction from 0 to 1, the same unit as the monthly trends.
    pub fn get_task_statistics(&self, scope: ProjectScope) -> Result<(i32, i32, f64), DatabaseError> {
        let counts = self.get_library_counts(scope)?;
        let (completed_tasks, pending_tasks) = (counts.tasks_completed, counts.tasks_pending);

        let total_tasks = completed_tasks + pending_tasks;
        let completion_rate = if total_tasks > 0 {
            completed_tasks as f64 / total_tasks as f64
        } else {
            0.0
        };

        Ok((completed_tasks, pending_tasks, completion_rate))
    }

    /// Projects created and modified on each of the `days` days ending `today`, oldest
    /// first, with quiet days as zero.
    pub fn get_recent_activity(
        &self,
        days: u32,
        today: NaiveDate,
        scope: ProjectScope,
    ) -> Result<Vec<(i32, i32, i32, i32, i32)>, DatabaseError> {
        let start = today - chrono::Duration::days(days.saturating_sub(1) as i64);
        let in_scope = scope.and_projects();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT date, SUM(projects_created), SUM(projects_modified)
             FROM (
                 SELECT DATE(datetime(p.created_at, 'unixepoch')) as date, COUNT(*) as projects_created, 0 as projects_modified
                 FROM projects p
                 WHERE p.created_at >= ?1 {in_scope}
                 GROUP BY date
                 UNION ALL
                 SELECT DATE(datetime(p.modified_at, 'unixepoch')) as date, 0, COUNT(*)
                 FROM projects p
                 WHERE p.modified_at >= ?1 {in_scope}
                 GROUP BY date
             )
             WHERE date IS NOT NULL
             GROUP BY date"
        ))?;

        let rows = stmt.query_map([day_epoch(start)], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?))
        })?;
        let mut found = std::collections::HashMap::new();
        for row in rows {
            let (date, created, modified) = row?;
            if let Ok(date) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
                found.insert(date, (created, modified));
            }
        }

        Ok(start
            .iter_days()
            .take_while(|d| *d <= today)
            .map(|d| {
                let (created, modified) = found.get(&d).copied().unwrap_or((0, 0));
                (d.year(), d.month() as i32, d.day() as i32, created, modified)
            })
            .collect())
    }

    pub fn get_ableton_version_stats(&self, scope: ProjectScope) -> Result<Vec<(String, i32)>, DatabaseError> {
        // daw_version_display already includes the beta suffix (ADR-0015,
        // AbletonVersion::Display), so no per-column reconstruction is needed here.
        let mut stmt = self.conn.prepare(&format!(
            "SELECT p.daw_version_display as version, COUNT(*) as count
             FROM projects p
             WHERE 1 = 1 {}
             GROUP BY version
             ORDER BY count DESC",
            scope.and_projects()
        ))?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        let mut versions = Vec::new();
        for row in rows {
            versions.push(row?);
        }
        Ok(versions)
    }

    /// Projects in scope per collection, averaged over every collection including
    /// empty ones, and the collection holding the most.
    pub fn get_collection_analytics(
        &self,
        scope: ProjectScope,
    ) -> Result<(f64, Option<String>), DatabaseError> {
        let collections = self.count("SELECT COUNT(*) FROM collections")?;
        if collections == 0 {
            return Ok((0.0, None));
        }
        let join = scope.join("cp.project_id");
        let memberships = self.count(&format!("SELECT COUNT(*) FROM collection_projects cp {join}"))?;

        let largest_collection_id: Option<String> = self
            .conn
            .query_row(
                &format!(
                    "SELECT cp.collection_id FROM collection_projects cp {join}
                     GROUP BY cp.collection_id
                     ORDER BY COUNT(*) DESC
                     LIMIT 1"
                ),
                [],
                |row| row.get(0),
            )
            .optional()?;

        Ok((memberships as f64 / collections as f64, largest_collection_id))
    }

    // Project-specific statistics methods
    pub fn get_project_statistics(
        &self,
        min_tempo: Option<f64>,
        max_tempo: Option<f64>,
        key_signature_tonic: Option<String>,
        key_signature_scale: Option<String>,
        time_signature_numerator: Option<i32>,
        time_signature_denominator: Option<i32>,
        ableton_version_major: Option<i32>,
        ableton_version_minor: Option<i32>,
        ableton_version_patch: Option<i32>,
        created_after: Option<i64>,
        created_before: Option<i64>,
        has_audio_file: Option<bool>,
    ) -> Result<ProjectStatistics, DatabaseError> {
        // Build WHERE conditions for filtering
        let mut conditions = vec!["is_active = true"];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(min_tempo_val) = min_tempo {
            conditions.push("tempo >= ?");
            params.push(Box::new(min_tempo_val));
        }

        if let Some(max_tempo_val) = max_tempo {
            conditions.push("tempo <= ?");
            params.push(Box::new(max_tempo_val));
        }

        if let Some(ref tonic) = key_signature_tonic {
            conditions.push("key_signature_tonic = ?");
            params.push(Box::new(tonic.clone()));
        }

        if let Some(ref scale) = key_signature_scale {
            conditions.push("key_signature_scale = ?");
            params.push(Box::new(scale.clone()));
        }

        if let Some(numerator) = time_signature_numerator {
            conditions.push("time_signature_numerator = ?");
            params.push(Box::new(numerator));
        }

        if let Some(denominator) = time_signature_denominator {
            conditions.push("time_signature_denominator = ?");
            params.push(Box::new(denominator));
        }

        // A subquery rather than a join: `where_clause` below is reused verbatim
        // across many queries with different FROM shapes (some already joined,
        // most not), so the version filter must not require any of them to add
        // project_ableton_metadata to their own FROM clause.
        if let Some(major) = ableton_version_major {
            conditions.push("id IN (SELECT project_id FROM project_ableton_metadata WHERE version_major = ?)");
            params.push(Box::new(major));
        }

        if let Some(minor) = ableton_version_minor {
            conditions.push("id IN (SELECT project_id FROM project_ableton_metadata WHERE version_minor = ?)");
            params.push(Box::new(minor));
        }

        if let Some(patch) = ableton_version_patch {
            conditions.push("id IN (SELECT project_id FROM project_ableton_metadata WHERE version_patch = ?)");
            params.push(Box::new(patch));
        }

        if let Some(after) = created_after {
            conditions.push("created_at >= ?");
            params.push(Box::new(after));
        }

        if let Some(before) = created_before {
            conditions.push("created_at <= ?");
            params.push(Box::new(before));
        }

        if let Some(has_audio) = has_audio_file {
            if has_audio {
                // ADR-0037: any listed audio, not just a primary.
                conditions.push("EXISTS (SELECT 1 FROM project_audio_files pa WHERE pa.project_id = projects.id)");
            } else {
                conditions.push("NOT EXISTS (SELECT 1 FROM project_audio_files pa WHERE pa.project_id = projects.id)");
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Basic counts
        let total_projects: i32 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM projects {}", where_clause),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        let projects_with_audio_files: i32 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM projects {} AND EXISTS (SELECT 1 FROM project_audio_files pa WHERE pa.project_id = projects.id)", where_clause),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        let projects_without_audio_files = total_projects - projects_with_audio_files;

        // Musical statistics
        let (average_tempo, min_tempo, max_tempo): (Option<f64>, Option<f64>, Option<f64>) = self.conn.query_row(
            &format!("SELECT AVG(tempo), MIN(tempo), MAX(tempo) FROM projects {}", where_clause),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        // Duration statistics
        let (average_duration, min_duration, max_duration): (Option<f64>, Option<f64>, Option<f64>) = self.conn.query_row(
            &format!("SELECT AVG(duration_seconds), MIN(duration_seconds), MAX(duration_seconds) FROM projects {}", where_clause),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;

        // Complexity statistics
        let average_plugins_per_project: Option<f64> = self.conn.query_row(
            &format!(
                "SELECT AVG(plugin_count) FROM (
                    SELECT COUNT(*) as plugin_count 
                    FROM project_plugins pp 
                    JOIN projects p ON pp.project_id = p.id 
                    {} 
                    GROUP BY p.id
                )",
                where_clause
            ),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        let average_samples_per_project: Option<f64> = self.conn.query_row(
            &format!(
                "SELECT AVG(sample_count) FROM (
                    SELECT COUNT(*) as sample_count 
                    FROM project_samples ps 
                    JOIN projects p ON ps.project_id = p.id 
                    {} 
                    GROUP BY p.id
                )",
                where_clause
            ),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        let average_tags_per_project: Option<f64> = self.conn.query_row(
            &format!(
                "SELECT AVG(tag_count) FROM (
                    SELECT COUNT(*) as tag_count 
                    FROM project_tags pt 
                    JOIN projects p ON pt.project_id = p.id 
                    {} 
                    GROUP BY p.id
                )",
                where_clause
            ),
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| row.get(0),
        )?;

        // Get distributions
        let tempo_distribution = self.get_project_tempo_distribution(&where_clause, &params)?;
        let key_signature_distribution = self.get_project_key_signature_distribution(&where_clause, &params)?;
        let time_signature_distribution = self.get_project_time_signature_distribution(&where_clause, &params)?;
        let ableton_version_distribution = self.get_project_ableton_version_distribution(&where_clause, &params)?;
        let projects_per_year = self.get_project_year_distribution(&where_clause, &params)?;
        let projects_per_month = self.get_project_month_distribution(&where_clause, &params)?;
        let most_complex_projects = self.get_project_complexity_statistics(&where_clause, &params)?;

        Ok(ProjectStatistics {
            total_projects,
            projects_with_audio_files,
            projects_without_audio_files,
            average_tempo: average_tempo.unwrap_or(0.0),
            min_tempo: min_tempo.unwrap_or(0.0),
            max_tempo: max_tempo.unwrap_or(0.0),
            tempo_distribution,
            key_signature_distribution,
            time_signature_distribution,
            ableton_version_distribution,
            average_duration_seconds: average_duration.unwrap_or(0.0),
            min_duration_seconds: min_duration.unwrap_or(0.0),
            max_duration_seconds: max_duration.unwrap_or(0.0),
            average_plugins_per_project: average_plugins_per_project.unwrap_or(0.0),
            average_samples_per_project: average_samples_per_project.unwrap_or(0.0),
            average_tags_per_project: average_tags_per_project.unwrap_or(0.0),
            projects_per_year,
            projects_per_month,
            most_complex_projects,
        })
    }

    fn get_project_tempo_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(String, i32)>, DatabaseError> {
        let query = format!(
            "SELECT 
                CASE 
                    WHEN tempo < 80 THEN '60-80 BPM'
                    WHEN tempo < 100 THEN '80-100 BPM'
                    WHEN tempo < 120 THEN '100-120 BPM'
                    WHEN tempo < 140 THEN '120-140 BPM'
                    WHEN tempo < 160 THEN '140-160 BPM'
                    WHEN tempo < 180 THEN '160-180 BPM'
                    ELSE '180+ BPM'
                END as tempo_range,
                COUNT(*) as count
             FROM projects {}
             GROUP BY tempo_range
             ORDER BY MIN(tempo)",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_key_signature_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(String, i32)>, DatabaseError> {
        let query = format!(
            "SELECT 
                CASE 
                    WHEN key_signature_tonic IS NOT NULL AND key_signature_scale IS NOT NULL 
                    THEN key_signature_tonic || ' ' || key_signature_scale
                    ELSE 'Unknown'
                END as key_signature,
                COUNT(*) as count
             FROM projects {}
             GROUP BY key_signature
             ORDER BY count DESC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_time_signature_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(i32, i32, i32)>, DatabaseError> {
        let query = format!(
            "SELECT 
                time_signature_numerator,
                time_signature_denominator,
                COUNT(*) as count
             FROM projects {}
             GROUP BY time_signature_numerator, time_signature_denominator
             ORDER BY count DESC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get::<_, i32>(0)?, row.get::<_, i32>(1)?, row.get::<_, i32>(2)?)),
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_ableton_version_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(String, i32)>, DatabaseError> {
        // Numeric columns from the side table are required here (unlike
        // get_ableton_version_stats) because sorting needs real integer order, which
        // a display string can't give correctly (e.g. "9.1.0" vs "10.0.0" as text).
        // No beta suffix, preserved as-is from prior behavior.
        let query = format!(
            "SELECT
                CAST(a.version_major AS TEXT) || '.' ||
                CAST(a.version_minor AS TEXT) || '.' ||
                CAST(a.version_patch AS TEXT) as version,
                COUNT(*) as count
             FROM projects p
             JOIN project_ableton_metadata a ON a.project_id = p.id
             {}
             GROUP BY version
             ORDER BY a.version_major DESC, a.version_minor DESC, a.version_patch DESC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)),
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_year_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(i32, i32)>, DatabaseError> {
        let query = format!(
            "SELECT 
                strftime('%Y', datetime(created_at, 'unixepoch')) as year,
                COUNT(*) as count
             FROM projects {}
             GROUP BY year
             ORDER BY year DESC",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| {
                let year_str: String = row.get(0)?;
                let year: i32 = year_str.parse().unwrap_or(0);
                Ok((year, row.get::<_, i32>(1)?))
            },
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_month_distribution(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(i32, i32, i32)>, DatabaseError> {
        let query = format!(
            "SELECT 
                strftime('%Y', datetime(created_at, 'unixepoch')) as year,
                strftime('%m', datetime(created_at, 'unixepoch')) as month,
                COUNT(*) as count
             FROM projects {}
             GROUP BY year, month
             ORDER BY year DESC, month DESC
             LIMIT 24",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| {
                let year_str: String = row.get(0)?;
                let month_str: String = row.get(1)?;
                let year: i32 = year_str.parse().unwrap_or(0);
                let month: i32 = month_str.parse().unwrap_or(0);
                Ok((year, month, row.get::<_, i32>(2)?))
            },
        )?;

        let mut distribution = Vec::new();
        for row in rows {
            distribution.push(row?);
        }
        Ok(distribution)
    }

    fn get_project_complexity_statistics(
        &self,
        where_clause: &str,
        params: &[Box<dyn rusqlite::ToSql>],
    ) -> Result<Vec<(String, String, i32, i32, i32, f64)>, DatabaseError> {
        let query = format!(
            "SELECT 
                p.id,
                p.name,
                COALESCE(plugin_count, 0) as plugin_count,
                COALESCE(sample_count, 0) as sample_count,
                COALESCE(tag_count, 0) as tag_count,
                (COALESCE(plugin_count, 0) + COALESCE(sample_count, 0) + COALESCE(tag_count, 0)) as complexity_score
             FROM projects p
             LEFT JOIN (
                SELECT project_id, COUNT(*) as plugin_count
                FROM project_plugins
                GROUP BY project_id
             ) pp ON pp.project_id = p.id
             LEFT JOIN (
                SELECT project_id, COUNT(*) as sample_count
                FROM project_samples
                GROUP BY project_id
             ) ps ON ps.project_id = p.id
             LEFT JOIN (
                SELECT project_id, COUNT(*) as tag_count
                FROM project_tags
                GROUP BY project_id
             ) pt ON pt.project_id = p.id
             {}
             ORDER BY complexity_score DESC
             LIMIT 10",
            where_clause
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i32>(4)?,
                row.get::<_, f64>(5)?,
            )),
        )?;

        let mut statistics = Vec::new();
        for row in rows {
            statistics.push(row?);
        }
        Ok(statistics)
    }
}

#[derive(Debug)]
pub struct ProjectStatistics {
    pub total_projects: i32,
    pub projects_with_audio_files: i32,
    pub projects_without_audio_files: i32,
    pub average_tempo: f64,
    pub min_tempo: f64,
    pub max_tempo: f64,
    pub tempo_distribution: Vec<(String, i32)>,
    pub key_signature_distribution: Vec<(String, i32)>,
    pub time_signature_distribution: Vec<(i32, i32, i32)>,
    pub ableton_version_distribution: Vec<(String, i32)>,
    pub average_duration_seconds: f64,
    pub min_duration_seconds: f64,
    pub max_duration_seconds: f64,
    pub average_plugins_per_project: f64,
    pub average_samples_per_project: f64,
    pub average_tags_per_project: f64,
    pub projects_per_year: Vec<(i32, i32)>,
    pub projects_per_month: Vec<(i32, i32, i32)>,
    pub most_complex_projects: Vec<(String, String, i32, i32, i32, f64)>,
}
