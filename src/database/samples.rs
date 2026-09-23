use crate::error::DatabaseError;
use crate::models::{Sample, SampleFormat};
use crate::scan::sample_check::SampleFileState;
use std::collections::HashMap;
use rusqlite::params;
use std::path::PathBuf;
use uuid::Uuid;

use super::ProjectDatabase;

/// The filters the sample list and search routes share, for counting what they return.
/// A field left `None` does not filter. `query` matches the way `search_samples` does:
/// a substring of the name or path.
#[derive(Debug, Default, Clone)]
pub struct SampleFilter {
    pub query: Option<String>,
    /// A format id, one of its extensions, or `other` (ADR-0039).
    pub format: Option<String>,
    pub present: Option<bool>,
}

impl SampleFilter {
    /// The `WHERE` conditions over `samples s`, and the values they bind in order.
    fn conditions(&self) -> (Vec<String>, Vec<Box<dyn rusqlite::ToSql>>) {
        let mut conditions = Vec::new();
        let mut bound: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(query) = self.query.as_deref().filter(|q| !q.is_empty()) {
            conditions.push("(s.name LIKE ? OR s.path LIKE ?)".to_string());
            let like = format!("%{}%", query);
            bound.push(Box::new(like.clone()));
            bound.push(Box::new(like));
        }
        if let Some(format) = &self.format {
            conditions.push(SampleFormat::sql_filter(format, "s.path").unwrap_or_else(|| "0".into()));
        }
        if let Some(present) = self.present {
            conditions.push("s.is_present = ?".to_string());
            bound.push(Box::new(present));
        }
        (conditions, bound)
    }
}

impl ProjectDatabase {
    /// Sizes the last sample check measured, keyed by sample id. Samples never measured
    /// are absent.
    pub fn sample_sizes(&self, ids: &[String]) -> Result<HashMap<String, i64>, DatabaseError> {
        let mut sizes = HashMap::new();
        for chunk in ids.chunks(500) {
            let placeholders = vec!["?"; chunk.len()].join(", ");
            let mut stmt = self.conn.prepare(&format!(
                "SELECT sample_id, size_bytes FROM sample_files WHERE sample_id IN ({placeholders})"
            ))?;
            let rows = stmt.query_map(rusqlite::params_from_iter(chunk), |row| Ok((row.get(0)?, row.get(1)?)))?;
            for row in rows {
                let (id, size) = row?;
                sizes.insert(id, size);
            }
        }
        Ok(sizes)
    }

    /// What the last check found for one sample: (size, file modified, checked at).
    /// `None` when no check has found the file.
    pub fn sample_file(&self, sample_id: &str) -> Result<Option<(i64, Option<i64>, i64)>, DatabaseError> {
        use rusqlite::OptionalExtension;
        Ok(self
            .conn
            .query_row(
                "SELECT size_bytes, modified_at, checked_at FROM sample_files WHERE sample_id = ?",
                [sample_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?)
    }

    /// Get all samples with pagination and sorting
    pub fn get_all_samples(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        present_only: Option<bool>,
        missing_only: Option<bool>,
        format_filter: Option<String>,
        min_usage_count: Option<i32>,
        max_usage_count: Option<i32>,
        scope: super::project_counts::ProjectScope,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("name") => "s.name",
            Some("path") => "s.path",
            Some("present") => "s.is_present",
            Some("usage_count") => "usage_count",
            _ => "s.name", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        // Build WHERE conditions for filtering
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Presence filters (mutually exclusive)
        if let Some(present) = present_only {
            conditions.push("s.is_present = ?".into());
            params.push(Box::new(present));
        } else if let Some(missing) = missing_only {
            conditions.push("s.is_present = ?".into());
            params.push(Box::new(!missing));
        }

        // Format filter: a format id, one of its extensions, or "other". A value that is
        // none of those matches nothing rather than being ignored.
        if let Some(format) = format_filter {
            conditions.push(SampleFormat::sql_filter(&format, "s.path").unwrap_or_else(|| "0".into()));
        }

        // Project counts, in scope (ADR-0040). Always joined: the list is sorted and
        // filtered by it, and a page of samples is cheap to count.
        let usage = format!(
            "LEFT JOIN (
                SELECT ps.sample_id, COUNT(*) AS usage_count
                FROM project_samples ps {}
                GROUP BY ps.sample_id
            ) usage_stats ON s.id = usage_stats.sample_id",
            scope.join("ps.project_id")
        );
        if let Some(min_count) = min_usage_count {
            conditions.push("COALESCE(usage_stats.usage_count, 0) >= ?".into());
            params.push(Box::new(min_count));
        }
        if let Some(max_count) = max_usage_count {
            conditions.push("COALESCE(usage_stats.usage_count, 0) <= ?".into());
            params.push(Box::new(max_count));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let count_query = format!("SELECT COUNT(*) FROM samples s {} {}", usage, where_clause);
        let mut count_stmt = self.conn.prepare(&count_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let total_count: i32 = count_stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

        let main_query = format!(
            "SELECT s.*, COALESCE(usage_stats.usage_count, 0) AS usage_count FROM samples s {} {}
             ORDER BY {} {}, s.name ASC LIMIT ? OFFSET ?",
            usage, where_clause, sort_column, sort_order
        );

        // Add pagination parameters
        params.push(Box::new(limit.unwrap_or(1000)));
        params.push(Box::new(offset.unwrap_or(0)));

        let mut stmt = self.conn.prepare(&main_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let id_str: String = row.get("id")?;
            Ok(Sample {
                id: Uuid::parse_str(&id_str).map_err(|_e| rusqlite::Error::InvalidColumnType(0, "id".to_string(), rusqlite::types::Type::Text))?,
                name: row.get("name")?,
                path: PathBuf::from(row.get::<_, String>("path")?),
                is_present: row.get("is_present")?,
            })
        })?;

        let samples: Result<Vec<Sample>, _> = rows.collect();
        Ok((samples?, total_count))
    }

    /// Get a single sample by ID
    pub fn get_sample_by_id(&self, sample_id: &str) -> Result<Option<Sample>, DatabaseError> {
        let mut stmt = self.conn.prepare("SELECT * FROM samples WHERE id = ?")?;
        let result = stmt.query_row(params![sample_id], |row| {
            let id_str: String = row.get("id")?;
            Ok(Sample {
                id: Uuid::parse_str(&id_str).map_err(|_e| rusqlite::Error::InvalidColumnType(0, "id".to_string(), rusqlite::types::Type::Text))?,
                name: row.get("name")?,
                path: PathBuf::from(row.get::<_, String>("path")?),
                is_present: row.get("is_present")?,
            })
        });

        match result {
            Ok(sample) => Ok(Some(sample)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::from(e)),
        }
    }

    /// Get samples filtered by presence status
    pub fn get_samples_by_presence(
        &self,
        is_present: bool,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("name") => "name",
            Some("path") => "path",
            Some("present") => "is_present",
            _ => "name", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        // Get total count
        let total_count: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM samples WHERE is_present = ?",
            params![is_present],
            |row| row.get(0),
        )?;

        // Build query with pagination
        let query = format!(
            "SELECT * FROM samples WHERE is_present = ? ORDER BY {} {} LIMIT ? OFFSET ?",
            sort_column, sort_order
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            params![is_present, limit.unwrap_or(1000), offset.unwrap_or(0)],
            |row| {
                let id_str: String = row.get("id")?;
                Ok(Sample {
                    id: Uuid::parse_str(&id_str).map_err(|_e| rusqlite::Error::InvalidColumnType(0, "id".to_string(), rusqlite::types::Type::Text))?,
                    name: row.get("name")?,
                    path: PathBuf::from(row.get::<_, String>("path")?),
                    is_present: row.get("is_present")?,
                })
            },
        )?;

        let samples: Result<Vec<Sample>, _> = rows.collect();
        Ok((samples?, total_count))
    }

    /// Search samples by name or path
    pub fn search_samples(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        present_only: Option<bool>,
        format_filter: Option<String>,
    ) -> Result<(Vec<Sample>, i32), DatabaseError> {
        let mut conditions: Vec<String> = vec!["(name LIKE ? OR path LIKE ?)".into()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(format!("%{}%", query)),
            Box::new(format!("%{}%", query)),
        ];

        if let Some(present) = present_only {
            conditions.push("is_present = ?".into());
            params.push(Box::new(present));
        }

        if let Some(format) = format_filter {
            conditions.push(SampleFormat::sql_filter(&format, "path").unwrap_or_else(|| "0".into()));
        }

        let where_clause = conditions.join(" AND ");

        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM samples WHERE {}", where_clause);
        let mut count_stmt = self.conn.prepare(&count_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let total_count: i32 = count_stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

        // Build main query
        let main_query = format!(
            "SELECT * FROM samples WHERE {} ORDER BY name ASC LIMIT ? OFFSET ?",
            where_clause
        );

        params.push(Box::new(limit.unwrap_or(1000)));
        params.push(Box::new(offset.unwrap_or(0)));

        let mut stmt = self.conn.prepare(&main_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let id_str: String = row.get("id")?;
            Ok(Sample {
                id: Uuid::parse_str(&id_str).map_err(|_e| rusqlite::Error::InvalidColumnType(0, "id".to_string(), rusqlite::types::Type::Text))?,
                name: row.get("name")?,
                path: PathBuf::from(row.get::<_, String>("path")?),
                is_present: row.get("is_present")?,
            })
        })?;

        let samples: Result<Vec<Sample>, _> = rows.collect();
        Ok((samples?, total_count))
    }

    /// Sample counts for the status bar, over every sample.
    pub fn get_sample_stats(&self) -> Result<SampleStats, DatabaseError> {
        self.get_sample_stats_filtered(&SampleFilter::default())
    }

    /// Sample counts over the samples a list or search with the same filter returns,
    /// so a status bar can describe what is on screen. Sizes are the ones a sample
    /// check measured (ADR-0041); a present sample no check has measured yet adds
    /// nothing, and `sized_samples` says how many were.
    pub fn get_sample_stats_filtered(&self, filter: &SampleFilter) -> Result<SampleStats, DatabaseError> {
        let (conditions, bound) = filter.conditions();
        let params: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|b| b.as_ref()).collect();
        let where_with = |extra: &str| {
            let mut all = conditions.clone();
            if !extra.is_empty() {
                all.push(extra.to_string());
            }
            if all.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", all.join(" AND "))
            }
        };

        let (total_samples, present_samples, sized_samples, total_size_bytes): (i32, i32, i32, i64) =
            self.conn.query_row(
                &format!(
                    "SELECT COUNT(*),
                            COALESCE(SUM(s.is_present), 0),
                            COALESCE(SUM(s.is_present AND f.size_bytes IS NOT NULL), 0),
                            COALESCE(SUM(CASE WHEN s.is_present THEN f.size_bytes END), 0)
                     FROM samples s LEFT JOIN sample_files f ON f.sample_id = s.id {}",
                    where_with("")
                ),
                params.as_slice(),
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;

        let mut samples_by_extension = HashMap::new();
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {} AS format, COUNT(*) FROM samples s {} GROUP BY format",
            SampleFormat::sql_case("s.path"),
            where_with("")
        ))?;
        let rows = stmt.query_map(params.as_slice(), |row| Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?)))?;
        for row in rows {
            let (format, count) = row?;
            samples_by_extension.insert(format, count);
        }

        Ok(SampleStats {
            total_samples,
            present_samples,
            missing_samples: total_samples - present_samples,
            // `path` is unique, so this always equals `total_samples`. Kept for gRPC.
            unique_paths: total_samples,
            samples_by_extension,
            total_size_bytes,
            sized_samples,
        })
    }

    /// Get sample usage numbers
    pub fn get_all_sample_usage_numbers(&self) -> Result<Vec<SampleUsageInfo>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                s.id,
                s.name,
                s.path,
                COUNT(ps.project_id) as usage_count,
                COUNT(DISTINCT ps.project_id) as project_count
            FROM samples s
            LEFT JOIN project_samples ps ON ps.sample_id = s.id
            GROUP BY s.id, s.name, s.path
            ORDER BY usage_count DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SampleUsageInfo {
                sample_id: row.get("id")?,
                name: row.get("name")?,
                path: row.get("path")?,
                usage_count: row.get("usage_count")?,
                project_count: row.get("project_count")?,
            })
        })?;

        let usage_info: Result<Vec<SampleUsageInfo>, _> = rows.collect();
        Ok(usage_info?)
    }

    /// Every sample's path, for a check to look at (ADR-0041).
    pub fn sample_paths(&self) -> Result<Vec<String>, DatabaseError> {
        let mut stmt = self.conn.prepare("SELECT path FROM samples")?;
        let paths = stmt.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(paths)
    }

    /// Write what a check found, in one transaction (ADR-0041). The check itself runs
    /// before this, without the database. `found` maps a path to its file, or `None`
    /// when it is not there; paths it does not mention, such as samples added while it
    /// ran, are left alone.
    ///
    /// A found file's size and time are recorded. A missing one keeps the size it last
    /// had, so a missing sample still says how big it was.
    pub fn record_sample_check(
        &mut self,
        found: &HashMap<String, Option<SampleFileState>>,
    ) -> Result<SampleRefreshResult, DatabaseError> {
        let now = chrono::Utc::now().timestamp();
        let tx = self.conn.transaction()?;
        let mut result = SampleRefreshResult {
            total_samples_checked: 0,
            samples_now_present: 0,
            samples_now_missing: 0,
            samples_unchanged: 0,
        };
        {
            let mut read = tx.prepare("SELECT id, path, is_present FROM samples")?;
            let rows: Vec<(String, String, bool)> = read
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<Result<_, _>>()?;
            let mut set_present = tx.prepare("UPDATE samples SET is_present = ? WHERE id = ?")?;
            let mut set_file = tx.prepare(
                "INSERT INTO sample_files (sample_id, size_bytes, modified_at, checked_at)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(sample_id) DO UPDATE SET
                    size_bytes = EXCLUDED.size_bytes,
                    modified_at = EXCLUDED.modified_at,
                    checked_at = EXCLUDED.checked_at",
            )?;
            for (id, path, was_present) in rows {
                let Some(state) = found.get(&path) else { continue };
                result.total_samples_checked += 1;
                let present = state.is_some();
                if present != was_present {
                    set_present.execute(params![present, id])?;
                    if present {
                        result.samples_now_present += 1;
                    } else {
                        result.samples_now_missing += 1;
                    }
                } else {
                    result.samples_unchanged += 1;
                }
                if let Some(file) = state {
                    set_file.execute(params![id, file.size_bytes as i64, file.modified_at, now])?;
                }
            }
        }
        tx.commit()?;
        Ok(result)
    }

    /// Get comprehensive sample analytics
    pub fn get_sample_analytics(&self) -> Result<SampleAnalytics, DatabaseError> {
        // Get usage distribution
        let usage_distribution = self.get_usage_distribution()?;
        
        // Get extension analytics
        let extensions = self.get_extension_analytics()?;
        
        // Get missing vs present percentages
        let (missing_percentage, present_percentage) = self.get_presence_percentages()?;
        
        // Get storage usage
        let (total_storage, present_storage, missing_storage) = self.get_storage_usage()?;
        
        // Get top used samples
        let top_used_samples = self.get_top_used_samples(10)?;
        
        // Get recently added samples (last 30 days)
        let recently_added = self.get_recently_added_samples()?;

        Ok(SampleAnalytics {
            most_used_samples_count: usage_distribution.most_used,
            moderately_used_samples_count: usage_distribution.moderately_used,
            rarely_used_samples_count: usage_distribution.rarely_used,
            unused_samples_count: usage_distribution.unused,
            extensions,
            missing_samples_percentage: missing_percentage,
            present_samples_percentage: present_percentage,
            total_storage_bytes: total_storage,
            present_storage_bytes: present_storage,
            missing_storage_bytes: missing_storage,
            top_used_samples,
            recently_added_samples: recently_added,
        })
    }

    /// Get usage distribution statistics
    fn get_usage_distribution(&self) -> Result<UsageDistribution, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                CASE 
                    WHEN usage_count >= 5 THEN 'most_used'
                    WHEN usage_count >= 2 THEN 'moderately_used'
                    WHEN usage_count = 1 THEN 'rarely_used'
                    ELSE 'unused'
                END as usage_category,
                COUNT(*) as count
            FROM (
                SELECT s.id, COALESCE(usage_stats.usage_count, 0) as usage_count
                FROM samples s
                LEFT JOIN (
                    SELECT sample_id, COUNT(*) as usage_count
                    FROM project_samples
                    GROUP BY sample_id
                ) usage_stats ON s.id = usage_stats.sample_id
            )
            GROUP BY usage_category
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        let mut distribution = UsageDistribution {
            most_used: 0,
            moderately_used: 0,
            rarely_used: 0,
            unused: 0,
        };

        for row in rows {
            let (category, count) = row?;
            match category.as_str() {
                "most_used" => distribution.most_used = count,
                "moderately_used" => distribution.moderately_used = count,
                "rarely_used" => distribution.rarely_used = count,
                "unused" => distribution.unused = count,
                _ => {}
            }
        }

        Ok(distribution)
    }

    /// Get sample extension statistics for filtering UI and storage analysis
    pub fn get_sample_extensions(&self) -> Result<std::collections::HashMap<String, ExtensionAnalytics>, DatabaseError> {
        self.get_extension_analytics()
    }

    /// Get extension analytics with detailed statistics
    fn get_extension_analytics(&self) -> Result<std::collections::HashMap<String, ExtensionAnalytics>, DatabaseError> {
        let mut stmt = self.conn.prepare(&format!(
            r#"
            SELECT 
                {format_case} as extension,
                COUNT(*) as count,
                SUM(CASE WHEN is_present THEN 1 ELSE 0 END) as present_count,
                SUM(CASE WHEN NOT is_present THEN 1 ELSE 0 END) as missing_count,
                AVG(COALESCE(usage_count, 0)) as avg_usage_count,
                -- Measured by the last check, present samples only (ADR-0041)
                COALESCE(SUM(CASE WHEN is_present THEN size_bytes END), 0) as total_size_bytes
            FROM (
                SELECT s.*, COALESCE(usage_stats.usage_count, 0) as usage_count, f.size_bytes
                FROM samples s
                LEFT JOIN sample_files f ON f.sample_id = s.id
                LEFT JOIN (
                    SELECT sample_id, COUNT(*) as usage_count
                    FROM project_samples
                    GROUP BY sample_id
                ) usage_stats ON s.id = usage_stats.sample_id
            )
            GROUP BY extension
            "#,
            format_case = SampleFormat::sql_case("path")
        ))?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, i64>(5)?,
            ))
        })?;

        let mut extensions = std::collections::HashMap::new();
        for row in rows {
            let (extension, count, present_count, missing_count, avg_usage_count, total_size_bytes) = row?;
            extensions.insert(extension, ExtensionAnalytics {
                count,
                total_size_bytes,
                present_count,
                missing_count,
                average_usage_count: avg_usage_count,
            });
        }

        Ok(extensions)
    }

    /// Get missing vs present percentages
    fn get_presence_percentages(&self) -> Result<(i32, i32), DatabaseError> {
        let total_samples: i32 = self.conn.query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))?;
        let present_samples: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM samples WHERE is_present = true",
            [],
            |row| row.get(0),
        )?;

        let missing_samples = total_samples - present_samples;
        let missing_percentage = if total_samples > 0 {
            ((missing_samples as f64 / total_samples as f64) * 100.0) as i32
        } else {
            0
        };
        let present_percentage = if total_samples > 0 {
            ((present_samples as f64 / total_samples as f64) * 100.0) as i32
        } else {
            0
        };

        Ok((missing_percentage, present_percentage))
    }

    /// Storage as the last check measured it (ADR-0041). Missing samples count at the
    /// size they last had.
    fn get_storage_usage(&self) -> Result<(i64, i64, i64), DatabaseError> {
        let (total_storage, present_storage) = self.conn.query_row(
            "SELECT SUM(f.size_bytes), SUM(CASE WHEN s.is_present THEN f.size_bytes ELSE 0 END)
             FROM samples s JOIN sample_files f ON f.sample_id = s.id",
            [],
            |row| Ok((row.get::<_, Option<i64>>(0)?.unwrap_or(0), row.get::<_, Option<i64>>(1)?.unwrap_or(0))),
        )?;

        let missing_storage = total_storage - present_storage;
        Ok((total_storage, present_storage, missing_storage))
    }

    /// Get top used samples
    fn get_top_used_samples(&self, limit: i32) -> Result<Vec<SampleUsageInfo>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                s.id,
                s.name,
                s.path,
                COUNT(ps.project_id) as usage_count,
                COUNT(DISTINCT ps.project_id) as project_count
            FROM samples s
            LEFT JOIN project_samples ps ON ps.sample_id = s.id
            GROUP BY s.id, s.name, s.path
            ORDER BY usage_count DESC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(SampleUsageInfo {
                sample_id: row.get("id")?,
                name: row.get("name")?,
                path: row.get("path")?,
                usage_count: row.get("usage_count")?,
                project_count: row.get("project_count")?,
            })
        })?;

        let usage_info: Result<Vec<SampleUsageInfo>, _> = rows.collect();
        Ok(usage_info?)
    }

    /// Get recently added samples (last 30 days)
    fn get_recently_added_samples(&self) -> Result<i32, DatabaseError> {
        // Since we don't have a created_at field in samples table, we'll estimate
        // based on the assumption that samples are added when projects are scanned
        // For now, we'll return 0 as a placeholder
        // TODO: Add created_at field to samples table in future migration
        Ok(0)
    }
}

#[derive(serde::Serialize)]
pub struct SampleStats {
    pub total_samples: i32,
    pub present_samples: i32,
    pub missing_samples: i32,
    pub unique_paths: i32,
    /// Keyed by format id (ADR-0039), despite the name.
    pub samples_by_extension: std::collections::HashMap<String, i32>,
    /// Measured by the last sample check, over present samples (ADR-0041). It used to
    /// be an estimate from a guessed size per extension.
    pub total_size_bytes: i64,
    /// Present samples a check has measured, of `present_samples`.
    pub sized_samples: i32,
}

#[derive(serde::Serialize)]
pub struct SampleUsageInfo {
    pub sample_id: String,
    pub name: String,
    pub path: String,
    pub usage_count: i32,
    pub project_count: i32,
}

#[derive(serde::Serialize)]
pub struct SampleRefreshResult {
    pub total_samples_checked: i32,
    pub samples_now_present: i32,
    pub samples_now_missing: i32,
    pub samples_unchanged: i32,
}

#[derive(serde::Serialize)]
pub struct SampleAnalytics {
    pub most_used_samples_count: i32,
    pub moderately_used_samples_count: i32,
    pub rarely_used_samples_count: i32,
    pub unused_samples_count: i32,
    pub extensions: std::collections::HashMap<String, ExtensionAnalytics>,
    pub missing_samples_percentage: i32,
    pub present_samples_percentage: i32,
    pub total_storage_bytes: i64,
    pub present_storage_bytes: i64,
    pub missing_storage_bytes: i64,
    pub top_used_samples: Vec<SampleUsageInfo>,
    pub recently_added_samples: i32,
}

pub struct UsageDistribution {
    pub most_used: i32,
    pub moderately_used: i32,
    pub rarely_used: i32,
    pub unused: i32,
}

#[derive(serde::Serialize)]
pub struct ExtensionAnalytics {
    pub count: i32,
    pub total_size_bytes: i64,
    pub present_count: i32,
    pub missing_count: i32,
    pub average_usage_count: f64,
}
