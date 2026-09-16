use crate::error::DatabaseError;
use crate::models::{Plugin, GrpcPlugin};
use rusqlite::params;

use super::ProjectDatabase;

/// Which of the three `plugins.installed` categories a query should return.
///
/// `installed` is tri-state (ADR-0012): `1` means a scan found the plugin, `0` means a
/// scan looked and did not, and `NULL` means no scan has looked yet. Filters take a
/// *set* of these rather than a single value because the useful questions are unions:
/// "everything I cannot load" is `Absent + Unscanned`, "everything a scan has ruled on"
/// is `Installed + Absent`. An empty set means no filtering at all. See ADR-0025.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstallState {
    /// A scan confirmed the plugin is installed on this machine.
    Installed,
    /// A scan looked for the plugin and did not find it.
    Absent,
    /// No scan has looked. Deliberately not the same as `Absent`.
    Unscanned,
}

impl InstallState {
    /// The SQL predicate for one state.
    ///
    /// Spelled out per variant rather than as `installed = ?`, because in SQL
    /// `NULL = 0` is not true — which is exactly how the never-scanned rows used to be
    /// dropped silently.
    fn predicate(self, column: &str) -> String {
        match self {
            InstallState::Installed => format!("{} = 1", column),
            InstallState::Absent => format!("{} = 0", column),
            InstallState::Unscanned => format!("{} IS NULL", column),
        }
    }
}

/// Build the `WHERE` fragment for a set of install states, or `None` when the set does
/// not constrain anything (empty, or naming all three categories).
///
/// Binds no parameters, so callers do not have to keep a parameter list in step.
fn install_state_condition(states: &[InstallState], column: &str) -> Option<String> {
    if states.is_empty() {
        return None;
    }

    let mut wanted: Vec<InstallState> = Vec::with_capacity(3);
    for state in states {
        if !wanted.contains(state) {
            wanted.push(*state);
        }
    }

    // All three categories together are every row, so filtering would be a no-op.
    if wanted.len() == 3 {
        return None;
    }

    Some(format!(
        "({})",
        wanted
            .iter()
            .map(|state| state.predicate(column))
            .collect::<Vec<_>>()
            .join(" OR ")
    ))
}

impl ProjectDatabase {
    /// Get all plugins with pagination, sorting, and filtering, including usage data
    pub fn get_all_plugins(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
        vendor_filter: Option<String>,
        format_filter: Option<String>,
        install_states: &[InstallState],
        min_usage_count: Option<i32>,
    ) -> Result<(Vec<GrpcPlugin>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("name") => "name",
            Some("vendor") => "vendor",
            Some("installed") => "installed",
            Some("format") => "format",
            Some("usage_count") => "usage_count",
            _ => "name", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        // Build WHERE conditions for filtering
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref vendor) = vendor_filter {
            conditions.push("p.vendor = ?".to_string());
            params.push(Box::new(vendor.clone()));
        }

        if let Some(ref format) = format_filter {
            conditions.push("p.format = ?".to_string());
            params.push(Box::new(format.clone()));
        }

        // Binds no parameter, so it does not appear in the count/main parameter lists.
        if let Some(condition) = install_state_condition(install_states, "p.installed") {
            conditions.push(condition);
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let where_prefix = if conditions.is_empty() { "WHERE" } else { "AND" };

        // Get total count with filters
        let count_query = if min_usage_count.is_some() {
            format!(
                r#"
                SELECT COUNT(*) FROM plugins p
                LEFT JOIN (
                    SELECT 
                        pp.plugin_id,
                        COUNT(pp.project_id) as usage_count
                    FROM project_plugins pp
                    GROUP BY pp.plugin_id
                ) usage_stats ON usage_stats.plugin_id = p.id
                {} 
                {} COALESCE(usage_stats.usage_count, 0) >= ?
                "#,
                where_clause,
                where_prefix
            )
        } else {
            format!(
                r#"
                SELECT COUNT(*) FROM plugins p
                {}
                "#,
                where_clause
            )
        };

        // Build parameters for count query
        let mut count_params = Vec::new();
        if let Some(vendor) = &vendor_filter {
            count_params.push(vendor as &dyn rusqlite::ToSql);
        }
        if let Some(format) = &format_filter {
            count_params.push(format as &dyn rusqlite::ToSql);
        }
        if let Some(min_usage) = &min_usage_count {
            count_params.push(min_usage as &dyn rusqlite::ToSql);
        }

        let total_count: i32 = if count_params.is_empty() {
            self.conn.query_row(&count_query, [], |row| row.get(0))?
        } else {
            self.conn.query_row(&count_query, count_params.as_slice(), |row| row.get(0))?
        };

        // Build main query with pagination, filtering, and usage data
        let query = if min_usage_count.is_some() {
            format!(
                r#"
                SELECT 
                    p.*,
                    COALESCE(usage_stats.usage_count, 0) as usage_count,
                    COALESCE(usage_stats.project_count, 0) as project_count
                FROM plugins p
                LEFT JOIN (
                    SELECT 
                        pp.plugin_id,
                        COUNT(pp.project_id) as usage_count,
                        COUNT(DISTINCT pp.project_id) as project_count
                    FROM project_plugins pp
                    GROUP BY pp.plugin_id
                ) usage_stats ON usage_stats.plugin_id = p.id
                {} 
                {} COALESCE(usage_stats.usage_count, 0) >= ?
                ORDER BY {} {} LIMIT ? OFFSET ?
                "#,
                where_clause,
                where_prefix,
                sort_column, sort_order
            )
        } else {
            format!(
                r#"
                SELECT 
                    p.*,
                    COALESCE(usage_stats.usage_count, 0) as usage_count,
                    COALESCE(usage_stats.project_count, 0) as project_count
                FROM plugins p
                LEFT JOIN (
                    SELECT 
                        pp.plugin_id,
                        COUNT(pp.project_id) as usage_count,
                        COUNT(DISTINCT pp.project_id) as project_count
                    FROM project_plugins pp
                    GROUP BY pp.plugin_id
                ) usage_stats ON usage_stats.plugin_id = p.id
                {}
                ORDER BY {} {} LIMIT ? OFFSET ?
                "#,
                where_clause,
                sort_column, sort_order
            )
        };

        // Add min_usage_count parameter if needed
        if let Some(min_usage) = min_usage_count {
            params.push(Box::new(min_usage));
        }

        // Add pagination parameters
        params.push(Box::new(limit.unwrap_or(1000)));
        params.push(Box::new(offset.unwrap_or(0)));

        let mut stmt = self.conn.prepare(&query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            let plugin = crate::database::helpers::row_to_plugin(row)?;
            
            Ok(GrpcPlugin {
                plugin,
                usage_count: row.get("usage_count")?,
                project_count: row.get("project_count")?,
            })
        })?;

        let grpc_plugins: Result<Vec<GrpcPlugin>, _> = rows.collect();
        Ok((grpc_plugins?, total_count))
    }

    /// Refresh plugin installation status by checking against Ableton's database
    /// Rescan the plugins installed on this system and write the results.
    ///
    /// This replaces an earlier implementation that asked Ableton's database whether
    /// each plugin existed. That version tested `get_plugin_by_dev_identifier(..).is_ok()`,
    /// which is `true` for `Ok(None)` — so it marked every plugin installed regardless
    /// of the answer (ADR-0006).
    ///
    /// Re-resolving references is part of the job: a plugin installed since the last
    /// scan is picked up here without reparsing any project file (ADR-0009).
    pub fn refresh_plugin_installation_status(
        &mut self,
    ) -> Result<PluginRefreshResult, DatabaseError> {
        use crate::config::CONFIG;
        use crate::scan::plugins::scan_system;
        use std::path::PathBuf;
        use std::time::Duration;

        let config = CONFIG
            .as_ref()
            .map_err(|e| DatabaseError::ConfigError(e.clone()))?;

        let roots: Vec<PathBuf> = config.vst_search_paths.iter().map(PathBuf::from).collect();
        let timeout = Duration::from_secs(config.vst_scan_timeout_secs);

        let report = scan_system(&roots, timeout)
            .map_err(|e| DatabaseError::ConnectionError(e.to_string()))?;

        // Only a scan that covered every configured search path may declare anything
        // missing. A truncated one reports what it saw and leaves the rest alone.
        let full_scan = !report.budget_exhausted;

        let candidates_scanned = report.results.len() as i32;
        let scan_failures = report.failed() as i32;

        let persisted = self.persist_plugin_scan(&report, full_scan)?;

        Ok(PluginRefreshResult {
            candidates_scanned,
            plugins_installed: (persisted.inserted + persisted.updated) as i32,
            plugins_missing: persisted.marked_missing as i32,
            plugins_reconciled: persisted.reconciled as i32,
            scan_failures,
        })
    }

    /// Get plugins filtered by installation status.
    ///
    /// `install_states` is a set: passing several returns their union, and passing none
    /// returns everything. See ADR-0025.
    pub fn get_plugins_by_installed_status(
        &self,
        install_states: &[InstallState],
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<Plugin>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("name") => "name",
            Some("vendor") => "vendor",
            Some("installed") => "installed",
            Some("format") => "format",
            _ => "name", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        let where_clause = match install_state_condition(install_states, "installed") {
            Some(condition) => format!("WHERE {}", condition),
            None => String::new(),
        };

        // Get total count
        let total_count: i32 = self.conn.query_row(
            &format!("SELECT COUNT(*) FROM plugins {}", where_clause),
            [],
            |row| row.get(0),
        )?;

        // Build query with pagination
        let query = format!(
            "SELECT * FROM plugins {} ORDER BY {} {} LIMIT ? OFFSET ?",
            where_clause, sort_column, sort_order
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            params![limit.unwrap_or(1000), offset.unwrap_or(0)],
            |row| {
                Ok(crate::database::helpers::row_to_plugin(row)?)
            },
        )?;

        let plugins: Result<Vec<Plugin>, _> = rows.collect();
        Ok((plugins?, total_count))
    }

    /// Search plugins by name, vendor, or format
    pub fn search_plugins(
        &self,
        query: &str,
        limit: Option<i32>,
        offset: Option<i32>,
        install_states: &[InstallState],
        vendor_filter: Option<String>,
        format_filter: Option<String>,
    ) -> Result<(Vec<Plugin>, i32), DatabaseError> {
        let mut conditions: Vec<String> =
            vec!["(name LIKE ? OR vendor LIKE ? OR format LIKE ?)".to_string()];
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(format!("%{}%", query)),
            Box::new(format!("%{}%", query)),
            Box::new(format!("%{}%", query)),
        ];

        // Binds no parameter, so the parameter list stays in step on its own.
        if let Some(condition) = install_state_condition(install_states, "installed") {
            conditions.push(condition);
        }

        if let Some(vendor) = vendor_filter {
            conditions.push("vendor = ?".to_string());
            params.push(Box::new(vendor));
        }

        if let Some(format) = format_filter {
            conditions.push("format = ?".to_string());
            params.push(Box::new(format));
        }

        let where_clause = conditions.join(" AND ");

        // Get total count
        let count_query = format!("SELECT COUNT(*) FROM plugins WHERE {}", where_clause);
        let mut count_stmt = self.conn.prepare(&count_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let total_count: i32 = count_stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

        // Build main query
        let main_query = format!(
            "SELECT * FROM plugins WHERE {} ORDER BY name ASC LIMIT ? OFFSET ?",
            where_clause
        );

        params.push(Box::new(limit.unwrap_or(1000)));
        params.push(Box::new(offset.unwrap_or(0)));

        let mut stmt = self.conn.prepare(&main_query)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(param_refs.as_slice(), |row| {
            Ok(crate::database::helpers::row_to_plugin(row)?)
        })?;

        let plugins: Result<Vec<Plugin>, _> = rows.collect();
        Ok((plugins?, total_count))
    }

    /// Get plugin statistics for status bar
    pub fn get_plugin_stats(&self) -> Result<PluginStats, DatabaseError> {
        let total_plugins: i32 =
            self.conn
                .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))?;

        let installed_plugins: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM plugins WHERE installed = true",
            [],
            |row| row.get(0),
        )?;

        let missing_plugins: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM plugins WHERE installed = false",
            [],
            |row| row.get(0),
        )?;

        // Never scanned for. Not the same as looked-for-and-absent, and deriving it as
        // `total - installed` would quietly merge the two.
        let unknown_plugins: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM plugins WHERE installed IS NULL",
            [],
            |row| row.get(0),
        )?;

        let unique_vendors: i32 = self.conn.query_row(
            "SELECT COUNT(DISTINCT vendor) FROM plugins WHERE vendor IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        // Get plugins by format
        let mut plugins_by_format = std::collections::HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT format, COUNT(*) FROM plugins GROUP BY format")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        for row in rows {
            let (format, count) = row?;
            plugins_by_format.insert(format, count);
        }

        // Get plugins by vendor
        let mut plugins_by_vendor = std::collections::HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT vendor, COUNT(*) FROM plugins WHERE vendor IS NOT NULL GROUP BY vendor ORDER BY COUNT(*) DESC LIMIT 10"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
        })?;

        for row in rows {
            let (vendor, count) = row?;
            plugins_by_vendor.insert(vendor, count);
        }

        Ok(PluginStats {
            total_plugins,
            installed_plugins,
            missing_plugins,
            unknown_plugins,
            unique_vendors,
            plugins_by_format,
            plugins_by_vendor,
        })
    }

    /// Get plugin vendors with their usage statistics
    pub fn get_plugin_vendors(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<VendorInfo>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("vendor") => "vendor",
            Some("plugin_count") => "plugin_count",
            Some("usage_count") => "total_usage_count",
            _ => "vendor", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        // Get total count
        let total_count: i32 = self.conn.query_row(
            "SELECT COUNT(DISTINCT vendor) FROM plugins WHERE vendor IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        // Build query with vendor statistics
        let query = format!(
            r#"
            WITH vendor_stats AS (
                SELECT
                    p.vendor,
                    -- Distinct plugin ids: usage_stats has one row per (plugin, project)
                    -- pair, so a plugin used in several projects joins to several rows
                    -- here. COUNT(*)/plain SUM would count it once per project instead
                    -- of once; COUNT(DISTINCT ... p.id) and the CASE-guarded variants
                    -- below collapse back to one count per plugin regardless.
                    COUNT(DISTINCT p.id) as plugin_count,
                    COUNT(DISTINCT CASE WHEN p.installed = 1 THEN p.id END) as installed_plugins,
                    COUNT(DISTINCT CASE WHEN p.installed = 0 THEN p.id END) as missing_plugins,
                    -- Never scanned for (ADR-0012). Counted explicitly rather than left
                    -- out, so installed + missing + unknown == plugin_count.
                    COUNT(DISTINCT CASE WHEN p.installed IS NULL THEN p.id END) as unknown_plugins,
                    COALESCE(SUM(usage_stats.usage_count), 0) as total_usage_count,
                    COALESCE(COUNT(DISTINCT usage_stats.project_id), 0) as unique_projects_using
                FROM plugins p
                LEFT JOIN (
                    SELECT
                        pp.plugin_id,
                        COUNT(pp.project_id) as usage_count,
                        pp.project_id
                    FROM project_plugins pp
                    GROUP BY pp.plugin_id, pp.project_id
                ) usage_stats ON usage_stats.plugin_id = p.id
                WHERE p.vendor IS NOT NULL
                GROUP BY p.vendor
            )
            SELECT
                vendor,
                plugin_count,
                installed_plugins,
                missing_plugins,
                unknown_plugins,
                total_usage_count,
                unique_projects_using
            FROM vendor_stats
            ORDER BY {} {} LIMIT ? OFFSET ?
            "#,
            sort_column, sort_order
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            params![limit.unwrap_or(1000), offset.unwrap_or(0)],
            |row| {
                let vendor: String = row.get(0)?;
                
                // Get plugins by format for this vendor
                let mut plugins_by_format = std::collections::HashMap::new();
                let mut format_stmt = self.conn.prepare(
                    "SELECT format, COUNT(*) FROM plugins WHERE vendor = ? GROUP BY format"
                )?;
                let format_rows = format_stmt.query_map(params![&vendor], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
                })?;

                for format_result in format_rows {
                    let (format, count) = format_result?;
                    plugins_by_format.insert(format, count);
                }

                Ok(VendorInfo {
                    vendor,
                    plugin_count: row.get(1)?,
                    installed_plugins: row.get(2)?,
                    missing_plugins: row.get(3)?,
                    unknown_plugins: row.get(4)?,
                    total_usage_count: row.get(5)?,
                    unique_projects_using: row.get(6)?,
                    plugins_by_format,
                })
            },
        )?;

        let vendors: Result<Vec<VendorInfo>, _> = rows.collect();
        Ok((vendors?, total_count))
    }

    /// Get plugin formats with their usage statistics
    pub fn get_plugin_formats(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
        sort_by: Option<String>,
        sort_desc: Option<bool>,
    ) -> Result<(Vec<FormatInfo>, i32), DatabaseError> {
        let sort_column = match sort_by.as_deref() {
            Some("format") => "format",
            Some("plugin_count") => "plugin_count",
            Some("usage_count") => "total_usage_count",
            _ => "format", // default sort
        };

        let sort_order = if sort_desc.unwrap_or(false) {
            "DESC"
        } else {
            "ASC"
        };

        // Get total count
        let total_count: i32 = self.conn.query_row(
            "SELECT COUNT(DISTINCT format) FROM plugins",
            [],
            |row| row.get(0),
        )?;

        // Build query with format statistics
        let query = format!(
            r#"
            WITH format_stats AS (
                SELECT
                    p.format,
                    -- Distinct plugin ids: usage_stats has one row per (plugin, project)
                    -- pair, so a plugin used in several projects joins to several rows
                    -- here. COUNT(*)/plain SUM would count it once per project instead
                    -- of once; COUNT(DISTINCT ... p.id) and the CASE-guarded variants
                    -- below collapse back to one count per plugin regardless.
                    COUNT(DISTINCT p.id) as plugin_count,
                    COUNT(DISTINCT CASE WHEN p.installed = 1 THEN p.id END) as installed_plugins,
                    COUNT(DISTINCT CASE WHEN p.installed = 0 THEN p.id END) as missing_plugins,
                    -- Never scanned for (ADR-0012). Counted explicitly rather than left
                    -- out, so installed + missing + unknown == plugin_count.
                    COUNT(DISTINCT CASE WHEN p.installed IS NULL THEN p.id END) as unknown_plugins,
                    COALESCE(SUM(usage_stats.usage_count), 0) as total_usage_count,
                    COALESCE(COUNT(DISTINCT usage_stats.project_id), 0) as unique_projects_using
                FROM plugins p
                LEFT JOIN (
                    SELECT
                        pp.plugin_id,
                        COUNT(pp.project_id) as usage_count,
                        pp.project_id
                    FROM project_plugins pp
                    GROUP BY pp.plugin_id, pp.project_id
                ) usage_stats ON usage_stats.plugin_id = p.id
                GROUP BY p.format
            )
            SELECT
                format,
                plugin_count,
                installed_plugins,
                missing_plugins,
                unknown_plugins,
                total_usage_count,
                unique_projects_using
            FROM format_stats
            ORDER BY {} {} LIMIT ? OFFSET ?
            "#,
            sort_column, sort_order
        );

        let mut stmt = self.conn.prepare(&query)?;
        let rows = stmt.query_map(
            params![limit.unwrap_or(1000), offset.unwrap_or(0)],
            |row| {
                let format: String = row.get(0)?;
                
                // Get plugins by vendor for this format
                let mut plugins_by_vendor = std::collections::HashMap::new();
                let mut vendor_stmt = self.conn.prepare(
                    "SELECT vendor, COUNT(*) FROM plugins WHERE format = ? AND vendor IS NOT NULL GROUP BY vendor"
                )?;
                let vendor_rows = vendor_stmt.query_map(params![&format], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
                })?;

                for vendor_result in vendor_rows {
                    let (vendor, count) = vendor_result?;
                    plugins_by_vendor.insert(vendor, count);
                }

                Ok(FormatInfo {
                    format,
                    plugin_count: row.get(1)?,
                    installed_plugins: row.get(2)?,
                    missing_plugins: row.get(3)?,
                    unknown_plugins: row.get(4)?,
                    total_usage_count: row.get(5)?,
                    unique_projects_using: row.get(6)?,
                    plugins_by_vendor,
                })
            },
        )?;

        let formats: Result<Vec<FormatInfo>, _> = rows.collect();
        Ok((formats?, total_count))
    }

    /// Get a single plugin by ID with usage statistics
    pub fn get_plugin_by_id(&self, plugin_id: &str) -> Result<Option<GrpcPlugin>, DatabaseError> {
        // Parse the plugin ID as UUID
        let uuid = match uuid::Uuid::parse_str(plugin_id) {
            Ok(uuid) => uuid,
            Err(_) => return Ok(None), // Invalid UUID format
        };

        // Build query with usage data
        let query = r#"
            SELECT 
                p.*,
                COALESCE(usage_stats.usage_count, 0) as usage_count,
                COALESCE(usage_stats.project_count, 0) as project_count
            FROM plugins p
            LEFT JOIN (
                SELECT 
                    pp.plugin_id,
                    COUNT(pp.project_id) as usage_count,
                    COUNT(DISTINCT pp.project_id) as project_count
                FROM project_plugins pp
                GROUP BY pp.plugin_id
            ) usage_stats ON usage_stats.plugin_id = p.id
            WHERE p.id = ?
        "#;

        let mut stmt = self.conn.prepare(query)?;
        let result = stmt.query_row(params![uuid.to_string()], |row| {
            let plugin = crate::database::helpers::row_to_plugin(row)?;
            
            Ok(GrpcPlugin {
                plugin,
                usage_count: row.get("usage_count")?,
                project_count: row.get("project_count")?,
            })
        });

        match result {
            Ok(plugin) => Ok(Some(plugin)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(DatabaseError::from(e)),
        }
    }


}

#[derive(serde::Serialize)]
pub struct PluginStats {
    pub total_plugins: i32,
    pub installed_plugins: i32,
    pub missing_plugins: i32,
    /// Plugins no scan has looked for yet. `installed + missing + unknown == total`.
    pub unknown_plugins: i32,
    pub unique_vendors: i32,
    pub plugins_by_format: std::collections::HashMap<String, i32>,
    pub plugins_by_vendor: std::collections::HashMap<String, i32>,
}

#[derive(serde::Serialize)]
pub struct PluginRefreshResult {
    /// Plugin binaries the scan attempted to load.
    pub candidates_scanned: i32,
    /// Rows the scan confirmed are installed.
    pub plugins_installed: i32,
    /// Rows a full scan looked for and did not find.
    pub plugins_missing: i32,
    /// Duplicate rows merged into the bundle that exports their class.
    pub plugins_reconciled: i32,
    /// Binaries that would not load — usually genuinely broken.
    pub scan_failures: i32,
}

#[derive(serde::Serialize)]
pub struct VendorInfo {
    pub vendor: String,
    pub plugin_count: i32,
    pub installed_plugins: i32,
    pub missing_plugins: i32,
    /// This vendor's plugins no scan has looked for yet.
    /// `installed + missing + unknown == plugin_count`.
    pub unknown_plugins: i32,
    pub total_usage_count: i32,
    pub unique_projects_using: i32,
    pub plugins_by_format: std::collections::HashMap<String, i32>,
}

#[derive(serde::Serialize)]
pub struct FormatInfo {
    pub format: String,
    pub plugin_count: i32,
    pub installed_plugins: i32,
    pub missing_plugins: i32,
    /// This format's plugins no scan has looked for yet.
    /// `installed + missing + unknown == plugin_count`.
    pub unknown_plugins: i32,
    pub total_usage_count: i32,
    pub unique_projects_using: i32,
    pub plugins_by_vendor: std::collections::HashMap<String, i32>,
}


