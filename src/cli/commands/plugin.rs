use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay, SimpleTable};
use crate::cli::{CliError, InstallFilter, OutputFormat, PluginCommands};
use crate::database::plugins::{InstallState, PluginStats, PluginRefreshResult, VendorInfo, FormatInfo};
use crate::models::Plugin;
use crate::services::PluginsService;
use crate::{colored_cell, simple_table_row};
use colored::Colorize;

use crate::config::CONFIG;
use crate::scan::plugins::{ScanReport, scan_system};
use vst_meta::protocol::Outcome;

use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub struct PluginCommand;

/// Map the repeatable `--installed` flag onto the database layer's tri-state set.
fn install_states(filters: &[InstallFilter]) -> Vec<InstallState> {
    filters.iter().copied().map(InstallState::from).collect()
}

#[async_trait::async_trait]
impl CliCommand for PluginCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use PluginCommands for actual functionality.
        println!("Use 'seula plugin list', 'seula plugin search', 'seula plugin show', 'seula plugin stats', 'seula plugin refresh', 'seula plugin vendors', or 'seula plugin formats' for plugin operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for PluginCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            PluginCommands::List { vendor, format, installed, limit, offset, sort_by, sort_desc } => {
                let plugins_list = self.get_plugins_list(&ctx.services.plugins, vendor, format, installed, *limit, *offset, sort_by, *sort_desc).await?;
                formatter.print(&plugins_list)?;
            }
            PluginCommands::Search { query, vendor, format, installed, limit } => {
                let search_results = self.search_plugins(&ctx.services.plugins, query, vendor, format, installed, *limit).await?;
                formatter.print(&search_results)?;
            }
            PluginCommands::Show { id } => {
                let plugin_details = self.get_plugin_details(&ctx.services.plugins, id).await?;
                formatter.print(&plugin_details)?;
            }
            PluginCommands::Stats => {
                let stats = self.get_plugin_stats(&ctx.services.plugins).await?;
                formatter.print(&stats)?;
            }
            PluginCommands::Refresh => {
                let refresh_result = self.refresh_plugin_installation_status(&ctx.services.plugins).await?;
                formatter.print(&refresh_result)?;
            }
            PluginCommands::Vendors => {
                let vendors = self.get_plugin_vendors(&ctx.services.plugins).await?;
                formatter.print(&vendors)?;
            }
            PluginCommands::Formats => {
                let formats = self.get_plugin_formats(&ctx.services.plugins).await?;
                formatter.print(&formats)?;
            }
            PluginCommands::ScanSystem { paths, timeout, failures_only, limit } => {
                let scan = self.scan_system(paths, *timeout, *failures_only, *limit)?;
                formatter.print(&scan)?;
                // The summary would be truncated as a table row, and JSON/CSV already
                // carry these counts as fields.
                if matches!(ctx.output_format, OutputFormat::Table) {
                    if scan.truncated {
                        formatter.print_message(
                            &format!("Showing {} of {} rows -- raise --limit to see the rest.", scan.displayed.len(), scan.total_rows),
                            MessageType::Info,
                        );
                    }
                    formatter.print_message(
                        &scan.summary(),
                        if scan.failed > 0 { MessageType::Warning } else { MessageType::Success },
                    );
                }
            }
        }

        Ok(())
    }
}

impl PluginCommands {
    async fn get_plugins_list(
        &self,
        plugins: &PluginsService,
        vendor: &Option<String>,
        format: &Option<String>,
        installed: &[InstallFilter],
        limit: usize,
        offset: usize,
        sort_by: &Option<String>,
        sort_desc: bool,
    ) -> Result<PluginsList, CliError> {
        let (plugins, total_count) = plugins.get_all_plugins(
            Some(limit as i32),
            Some(offset as i32),
            sort_by.clone(),
            Some(sort_desc),
            vendor.as_ref().map(|s| s.clone()),
            format.as_ref().map(|s| s.clone()),
            &install_states(installed),
            None, // min_usage_count
        ).await?;

        let displayed = plugins
            .into_iter()
            .map(|grpc_plugin| PluginRow {
                id: grpc_plugin.plugin.id.to_string(),
                name: grpc_plugin.plugin.name,
                vendor: grpc_plugin.plugin.vendor.unwrap_or_else(|| "Unknown".to_string()),
                format: grpc_plugin.plugin.plugin_format.to_string(),
                installed: grpc_plugin.plugin.installed,
                usage_count: grpc_plugin.usage_count,
                project_count: grpc_plugin.project_count,
            })
            .collect();

        Ok(PluginsList {
            displayed,
            total_count: total_count as usize,
            limit,
            offset,
        })
    }

    async fn search_plugins(
        &self,
        plugins: &PluginsService,
        query: &str,
        vendor: &Option<String>,
        format: &Option<String>,
        installed: &[InstallFilter],
        limit: usize,
    ) -> Result<PluginsSearchResults, CliError> {
        let (plugins, total_count) = plugins.search_plugins(
            query,
            Some(limit as i32),
            Some(0),
            &install_states(installed),
            vendor.as_ref().map(|s| s.clone()),
            format.as_ref().map(|s| s.clone()),
        ).await?;

        let displayed = plugins
            .into_iter()
            .map(|plugin| PluginRow {
                id: plugin.id.to_string(),
                name: plugin.name,
                vendor: plugin.vendor.unwrap_or_else(|| "Unknown".to_string()),
                format: plugin.plugin_format.to_string(),
                installed: plugin.installed,
                usage_count: 0, // Search doesn't include usage data
                project_count: 0,
            })
            .collect();

        Ok(PluginsSearchResults {
            query: query.to_string(),
            displayed,
            total_count: total_count as usize,
        })
    }

    async fn get_plugin_details(
        &self,
        plugins: &PluginsService,
        plugin_id: &str,
    ) -> Result<PluginDetails, CliError> {
        let plugin = plugins.get_plugin(plugin_id).await?;

        match plugin {
            Some(grpc_plugin) => Ok(PluginDetails {
                plugin: grpc_plugin.plugin,
                usage_count: grpc_plugin.usage_count,
                project_count: grpc_plugin.project_count,
            }),
            None => Err(format!("Plugin with ID {} not found", plugin_id).into()),
        }
    }

    async fn get_plugin_stats(&self, plugins: &PluginsService) -> Result<PluginStatsDisplay, CliError> {
        let stats = plugins.get_plugin_stats().await?;

        Ok(PluginStatsDisplay { stats })
    }

    async fn refresh_plugin_installation_status(&self, plugins: &PluginsService) -> Result<PluginRefreshDisplay, CliError> {
        let result = plugins.refresh_plugin_installation_status().await?;

        Ok(PluginRefreshDisplay { result })
    }

    async fn get_plugin_vendors(&self, plugins: &PluginsService) -> Result<PluginVendorsDisplay, CliError> {
        let (vendors, total_count) = plugins.get_plugin_vendors(
            Some(50), // limit
            Some(0),  // offset
            Some("vendor".to_string()), // sort_by
            Some(false), // sort_desc
        ).await?;

        Ok(PluginVendorsDisplay {
            vendors,
            total_count: total_count as usize,
        })
    }

    async fn get_plugin_formats(&self, plugins: &PluginsService) -> Result<PluginFormatsDisplay, CliError> {
        let (formats, total_count) = plugins.get_plugin_formats(
            Some(50), // limit
            Some(0),  // offset
            Some("format".to_string()), // sort_by
            Some(false), // sort_desc
        ).await?;

        Ok(PluginFormatsDisplay {
            formats,
            total_count: total_count as usize,
        })
    }
}

impl PluginCommands {
    /// Run the out-of-process scanner over the system's plugin directories.
    fn scan_system(
        &self,
        paths: &[String],
        timeout: Option<u64>,
        failures_only: bool,
        limit: usize,
    ) -> Result<SystemScanDisplay, CliError> {
        let config = CONFIG
            .as_ref()
            .map_err(|e| -> CliError { format!("Failed to load config: {}", e).into() })?;

        let roots: Vec<PathBuf> = if paths.is_empty() {
            config.vst_search_paths.iter().map(PathBuf::from).collect()
        } else {
            paths.iter().map(PathBuf::from).collect()
        };

        let timeout = Duration::from_secs(timeout.unwrap_or(config.vst_scan_timeout_secs));

        let report =
            scan_system(&roots, timeout).map_err(|e| -> CliError { Box::new(e) })?;

        Ok(SystemScanDisplay::new(report, failures_only, limit))
    }
}

#[derive(Serialize)]
pub struct SystemScanRow {
    pub path: String,
    pub name: String,
    pub vendor: String,
    pub format: String,
    /// Failure classification, or `None` when the scan succeeded.
    pub error_type: Option<String>,
    pub detail: String,
}

/// Result of `plugin scan-system`.
#[derive(Serialize)]
pub struct SystemScanDisplay {
    pub displayed: Vec<SystemScanRow>,
    /// Paths attempted.
    pub scanned: usize,
    /// Plugin records extracted. Exceeds `succeeded` when VST2 shells expand.
    pub plugin_count: usize,
    pub succeeded: usize,
    pub failed: usize,
    /// Worker restarts. Non-zero means plugins here crashed or hung the scanner --
    /// which is the whole reason it runs out of process.
    pub restarts: usize,
    pub budget_exhausted: bool,
    /// Failure counts by kind, most common first.
    pub failures_by_type: Vec<(String, usize)>,
    /// Rows before `--limit` was applied.
    pub total_rows: usize,
    pub truncated: bool,
}

impl SystemScanDisplay {
    fn new(report: ScanReport, failures_only: bool, limit: usize) -> Self {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for result in &report.results {
            if let Some(error_type) = result.error_type() {
                *counts.entry(error_type.to_string()).or_insert(0) += 1;
            }
        }
        let mut failures_by_type: Vec<(String, usize)> = counts.into_iter().collect();
        failures_by_type.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let mut rows = Vec::new();
        for result in &report.results {
            let path = result.path.display().to_string();
            match &result.outcome {
                Outcome::Success { plugins } => {
                    if failures_only {
                        continue;
                    }
                    // A shell binary yields several records; show each on its own row.
                    for plugin in plugins {
                        rows.push(SystemScanRow {
                            path: path.clone(),
                            name: plugin.name.clone(),
                            vendor: plugin.vendor.clone(),
                            format: plugin.format.as_str().to_string(),
                            error_type: None,
                            detail: plugin.uid.clone(),
                        });
                    }
                }
                Outcome::Error { error_type, error } => {
                    rows.push(SystemScanRow {
                        path: path.clone(),
                        name: file_label(&result.path),
                        vendor: String::new(),
                        format: String::new(),
                        error_type: Some(error_type.to_string()),
                        detail: error.clone(),
                    });
                }
            }
        }

        let total_rows = rows.len();
        let truncated = total_rows > limit;
        rows.truncate(limit);

        SystemScanDisplay {
            displayed: rows,
            scanned: report.results.len(),
            plugin_count: report.plugin_count(),
            succeeded: report.succeeded(),
            failed: report.failed(),
            restarts: report.restarts,
            budget_exhausted: report.budget_exhausted,
            failures_by_type,
            total_rows,
            truncated,
        }
    }

    /// One-line summary printed under the table.
    pub fn summary(&self) -> String {
        let mut summary = format!(
            "{} scanned, {} ok, {} failed, {} plugin records, {} restart(s)",
            self.scanned, self.succeeded, self.failed, self.plugin_count, self.restarts
        );
        if !self.failures_by_type.is_empty() {
            let breakdown: Vec<String> = self
                .failures_by_type
                .iter()
                .map(|(kind, count)| format!("{} {}", count, kind))
                .collect();
            summary.push_str(&format!(" [{}]", breakdown.join(", ")));
        }
        if self.budget_exhausted {
            summary.push_str(" -- restart budget exhausted, scan incomplete");
        }
        summary
    }
}

/// Best-effort display name for a path that failed before yielding any metadata.
fn file_label(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

impl TableDisplay for SystemScanDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Name".to_string(),
            "Vendor".to_string(),
            "Format".to_string(),
            "Status".to_string(),
            "Detail".to_string(),
        ]);

        for row in &self.displayed {
            let status_cell = match &row.error_type {
                None => colored_cell!("ok", green),
                Some(kind) => colored_cell!(kind, red),
            };

            simple_table_row!(
                table,
                &row.name,
                &row.vendor,
                &row.format,
                &status_cell,
                &row.detail
            );
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer
            .write_record(["path", "name", "vendor", "format", "error_type", "detail"])
            .map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.path.as_str(),
                    row.name.as_str(),
                    row.vendor.as_str(),
                    row.format.as_str(),
                    row.error_type.as_deref().unwrap_or(""),
                    row.detail.as_str(),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginRow {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub format: String,
    /// `None` when no plugin scan has looked yet.
    pub installed: Option<bool>,
    pub usage_count: i32,
    pub project_count: i32,
}

/// Render the tri-state install flag.
///
/// "Unknown" is a real answer, not a fallback: it means no plugin scan has run, which
/// a user fixes with `seula plugin refresh` rather than by installing anything.
fn installed_cell(installed: Option<bool>) -> String {
    match installed {
        Some(true) => colored_cell!("Installed", green),
        Some(false) => colored_cell!("Missing", red),
        None => colored_cell!("Unknown", yellow),
    }
}

/// The same three states, unstyled, for CSV and JSON.
fn installed_label(installed: Option<bool>) -> &'static str {
    match installed {
        Some(true) => "installed",
        Some(false) => "missing",
        None => "unknown",
    }
}

#[derive(Serialize)]
pub struct PluginsList {
    pub displayed: Vec<PluginRow>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

impl TableDisplay for PluginsList {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Vendor".to_string(),
            "Format".to_string(),
            "Status".to_string(),
            "Usage".to_string(),
            "Projects".to_string(),
        ]);

        for row in &self.displayed {
            let status_cell = installed_cell(row.installed);

            simple_table_row!(table,
                &row.id[..8], // Show only first 8 chars of UUID
                &row.name,
                &row.vendor,
                &row.format,
                &status_cell,
                &row.usage_count.to_string(),
                &row.project_count.to_string()
            );
        }

        // Add summary row
        simple_table_row!(table,
            "",
            &format!("Total: {} plugins", self.total_count),
            &format!("Showing {}-{} of {}", self.offset + 1, self.offset + self.displayed.len(), self.total_count),
            "",
            "",
            "",
            ""
        );

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["id", "name", "vendor", "format", "installed", "usage_count", "project_count"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.name.as_str(),
                    row.vendor.as_str(),
                    row.format.as_str(),
                    installed_label(row.installed),
                    &row.usage_count.to_string(),
                    &row.project_count.to_string(),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginsSearchResults {
    pub query: String,
    pub displayed: Vec<PluginRow>,
    pub total_count: usize,
}

impl TableDisplay for PluginsSearchResults {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Vendor".to_string(),
            "Format".to_string(),
            "Status".to_string(),
        ]);

        for row in &self.displayed {
            let status_cell = installed_cell(row.installed);

            simple_table_row!(table,
                &row.id[..8],
                &row.name,
                &row.vendor,
                &row.format,
                &status_cell
            );
        }

        // Add search summary
        simple_table_row!(table,
            "",
            &format!("Search: '{}' - {} results", self.query, self.total_count),
            "",
            "",
            ""
        );

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["query", "id", "name", "vendor", "format", "installed"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    self.query.as_str(),
                    row.id.as_str(),
                    row.name.as_str(),
                    row.vendor.as_str(),
                    row.format.as_str(),
                    installed_label(row.installed),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginDetails {
    pub plugin: Plugin,
    pub usage_count: i32,
    pub project_count: i32,
}

impl TableDisplay for PluginDetails {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec!["Property".to_string(), "Value".to_string()]);

        simple_table_row!(table, "ID", self.plugin.id);
        simple_table_row!(table, "Name", self.plugin.name);
        simple_table_row!(table, "Vendor", self.plugin.vendor.as_deref().unwrap_or("Unknown"));
        simple_table_row!(table, "Format", self.plugin.plugin_format.to_string());
        simple_table_row!(table, "Dev Identifier", self.plugin.dev_identifier);
        
        let status = installed_cell(self.plugin.installed);
        simple_table_row!(table, "Status", status);
        
        simple_table_row!(table, "Version", self.plugin.version.as_deref().unwrap_or("Unknown"));
        simple_table_row!(table, "Usage Count", self.usage_count);
        simple_table_row!(table, "Used in Projects", self.project_count);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["id", &self.plugin.id.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["name", &self.plugin.name]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["vendor", &self.plugin.vendor.as_deref().unwrap_or("")]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["format", &self.plugin.plugin_format.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["dev_identifier", &self.plugin.dev_identifier]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["installed", installed_label(self.plugin.installed)]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["version", &self.plugin.version.as_deref().unwrap_or("")]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["usage_count", &self.usage_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_count", &self.project_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginStatsDisplay {
    pub stats: PluginStats,
}

impl TableDisplay for PluginStatsDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec!["Category".to_string(), "Metric".to_string(), "Value".to_string()]);

        // Basic stats
        simple_table_row!(table, "Overview", "Total Plugins", self.stats.total_plugins);
        simple_table_row!(table, "Overview", "Installed Plugins", self.stats.installed_plugins);
        simple_table_row!(table, "Overview", "Missing Plugins", self.stats.missing_plugins);
        simple_table_row!(table, "Overview", "Not Yet Scanned", self.stats.unknown_plugins);
        simple_table_row!(table, "Overview", "Unique Vendors", self.stats.unique_vendors);

        // Format breakdown
        for (format, count) in &self.stats.plugins_by_format {
            simple_table_row!(table, "Formats", format, count);
        }

        // Top vendors
        for (vendor, count) in &self.stats.plugins_by_vendor {
            simple_table_row!(table, "Top Vendors", vendor, count);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["category", "metric", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Total Plugins", &self.stats.total_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Installed Plugins", &self.stats.installed_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Missing Plugins", &self.stats.missing_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Not Yet Scanned", &self.stats.unknown_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Unique Vendors", &self.stats.unique_vendors.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginRefreshDisplay {
    pub result: PluginRefreshResult,
}

impl TableDisplay for PluginRefreshDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec!["Refresh Result".to_string(), "Count".to_string()]);

        simple_table_row!(table, "Candidates Scanned", self.result.candidates_scanned);
        simple_table_row!(table, colored_cell!("Installed", green), self.result.plugins_installed);
        simple_table_row!(table, colored_cell!("Missing", red), self.result.plugins_missing);
        simple_table_row!(table, "Reconciled", self.result.plugins_reconciled);
        simple_table_row!(table, colored_cell!("Scan Failures", yellow), self.result.scan_failures);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["result", "count"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["candidates_scanned", &self.result.candidates_scanned.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["plugins_installed", &self.result.plugins_installed.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["plugins_missing", &self.result.plugins_missing.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["plugins_reconciled", &self.result.plugins_reconciled.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["scan_failures", &self.result.scan_failures.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginVendorsDisplay {
    pub vendors: Vec<VendorInfo>,
    pub total_count: usize,
}

impl TableDisplay for PluginVendorsDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Vendor".to_string(),
            "Total".to_string(),
            "Installed".to_string(),
            "Missing".to_string(),
            "Unknown".to_string(),
            "Usage".to_string(),
            "Projects".to_string(),
        ]);

        for vendor in &self.vendors {
            simple_table_row!(table,
                vendor.vendor.as_str(),
                &vendor.plugin_count.to_string(),
                &vendor.installed_plugins.to_string(),
                &vendor.missing_plugins.to_string(),
                &vendor.unknown_plugins.to_string(),
                &vendor.total_usage_count.to_string(),
                &vendor.unique_projects_using.to_string()
            );
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["vendor", "total_plugins", "installed", "missing", "unknown", "usage_count", "projects_using"]).map_err(|e| -> CliError { e.into() })?;
        for vendor in &self.vendors {
            writer.write_record([
                vendor.vendor.as_str(),
                &vendor.plugin_count.to_string(),
                &vendor.installed_plugins.to_string(),
                &vendor.missing_plugins.to_string(),
                &vendor.unknown_plugins.to_string(),
                &vendor.total_usage_count.to_string(),
                &vendor.unique_projects_using.to_string(),
            ]).map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginFormatsDisplay {
    pub formats: Vec<FormatInfo>,
    pub total_count: usize,
}

impl TableDisplay for PluginFormatsDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "Format".to_string(),
            "Total".to_string(),
            "Installed".to_string(),
            "Missing".to_string(),
            "Unknown".to_string(),
            "Usage".to_string(),
            "Projects".to_string(),
        ]);

        for format in &self.formats {
            simple_table_row!(table,
                format.format.as_str(),
                &format.plugin_count.to_string(),
                &format.installed_plugins.to_string(),
                &format.missing_plugins.to_string(),
                &format.unknown_plugins.to_string(),
                &format.total_usage_count.to_string(),
                &format.unique_projects_using.to_string()
            );
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["format", "total_plugins", "installed", "missing", "unknown", "usage_count", "projects_using"]).map_err(|e| -> CliError { e.into() })?;
        for format in &self.formats {
            writer.write_record([
                format.format.as_str(),
                &format.plugin_count.to_string(),
                &format.installed_plugins.to_string(),
                &format.missing_plugins.to_string(),
                &format.unknown_plugins.to_string(),
                &format.total_usage_count.to_string(),
                &format.unique_projects_using.to_string(),
            ]).map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}
