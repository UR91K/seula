use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay};
use crate::cli::{CliError, PluginCommands};
use crate::database::LiveSetDatabase;
use crate::database::plugins::{PluginStats, PluginRefreshResult, VendorInfo, FormatInfo};
use crate::models::{Plugin, GrpcPlugin};
use crate::{colored_cell, table_row};
use colored::Colorize;
use comfy_table::Table;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct PluginCommand;

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
                let plugins_list = self.get_plugins_list(&ctx.db, vendor, format, installed, *limit, *offset, sort_by, *sort_desc).await?;
                formatter.print(&plugins_list)?;
            }
            PluginCommands::Search { query, vendor, format, installed, limit } => {
                let search_results = self.search_plugins(&ctx.db, query, vendor, format, installed, *limit).await?;
                formatter.print(&search_results)?;
            }
            PluginCommands::Show { id } => {
                let plugin_details = self.get_plugin_details(&ctx.db, id).await?;
                formatter.print(&plugin_details)?;
            }
            PluginCommands::Stats => {
                let stats = self.get_plugin_stats(&ctx.db).await?;
                formatter.print(&stats)?;
            }
            PluginCommands::Refresh => {
                let refresh_result = self.refresh_plugin_installation_status(&ctx.db).await?;
                formatter.print(&refresh_result)?;
            }
            PluginCommands::Vendors => {
                let vendors = self.get_plugin_vendors(&ctx.db).await?;
                formatter.print(&vendors)?;
            }
            PluginCommands::Formats => {
                let formats = self.get_plugin_formats(&ctx.db).await?;
                formatter.print(&formats)?;
            }
        }

        Ok(())
    }
}

impl PluginCommands {
    async fn get_plugins_list(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        vendor: &Option<String>,
        format: &Option<String>,
        installed: &Option<bool>,
        limit: usize,
        offset: usize,
        sort_by: &Option<String>,
        sort_desc: bool,
    ) -> Result<PluginsList, CliError> {
        let db_guard = db.lock().await;
        let (plugins, total_count) = db_guard.get_all_plugins(
            Some(limit as i32),
            Some(offset as i32),
            sort_by.clone(),
            Some(sort_desc),
            vendor.as_ref().map(|s| s.clone()),
            format.as_ref().map(|s| s.clone()),
            *installed,
            None, // min_usage_count
        )?;

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
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        query: &str,
        vendor: &Option<String>,
        format: &Option<String>,
        installed: &Option<bool>,
        limit: usize,
    ) -> Result<PluginsSearchResults, CliError> {
        let db_guard = db.lock().await;
        let (plugins, total_count) = db_guard.search_plugins(
            query,
            Some(limit as i32),
            Some(0),
            *installed,
            vendor.as_ref().map(|s| s.clone()),
            format.as_ref().map(|s| s.clone()),
        )?;

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
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        plugin_id: &str,
    ) -> Result<PluginDetails, CliError> {
        let db_guard = db.lock().await;
        let plugin = db_guard.get_plugin_by_id(plugin_id)?;

        match plugin {
            Some(grpc_plugin) => Ok(PluginDetails {
                plugin: grpc_plugin.plugin,
                usage_count: grpc_plugin.usage_count,
                project_count: grpc_plugin.project_count,
            }),
            None => Err(format!("Plugin with ID {} not found", plugin_id).into()),
        }
    }

    async fn get_plugin_stats(&self, db: &Arc<TokioMutex<LiveSetDatabase>>) -> Result<PluginStatsDisplay, CliError> {
        let db_guard = db.lock().await;
        let stats = db_guard.get_plugin_stats()?;

        Ok(PluginStatsDisplay { stats })
    }

    async fn refresh_plugin_installation_status(&self, db: &Arc<TokioMutex<LiveSetDatabase>>) -> Result<PluginRefreshDisplay, CliError> {
        let mut db_guard = db.lock().await;
        let result = db_guard.refresh_plugin_installation_status()?;

        Ok(PluginRefreshDisplay { result })
    }

    async fn get_plugin_vendors(&self, db: &Arc<TokioMutex<LiveSetDatabase>>) -> Result<PluginVendorsDisplay, CliError> {
        let db_guard = db.lock().await;
        let (vendors, total_count) = db_guard.get_plugin_vendors(
            Some(50), // limit
            Some(0),  // offset
            Some("vendor".to_string()), // sort_by
            Some(false), // sort_desc
        )?;

        Ok(PluginVendorsDisplay {
            vendors,
            total_count: total_count as usize,
        })
    }

    async fn get_plugin_formats(&self, db: &Arc<TokioMutex<LiveSetDatabase>>) -> Result<PluginFormatsDisplay, CliError> {
        let db_guard = db.lock().await;
        let (formats, total_count) = db_guard.get_plugin_formats(
            Some(50), // limit
            Some(0),  // offset
            Some("format".to_string()), // sort_by
            Some(false), // sort_desc
        )?;

        Ok(PluginFormatsDisplay {
            formats,
            total_count: total_count as usize,
        })
    }
}

#[derive(Serialize)]
pub struct PluginRow {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub format: String,
    pub installed: bool,
    pub usage_count: i32,
    pub project_count: i32,
}

#[derive(Serialize)]
pub struct PluginsList {
    pub displayed: Vec<PluginRow>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

impl TableDisplay for PluginsList {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["ID", "Name", "Vendor", "Format", "Status", "Usage", "Projects"]);

        for row in &self.displayed {
            let status_cell = if row.installed {
                colored_cell!("Installed", green)
            } else {
                colored_cell!("Missing", red)
            };

            table.add_row(vec![
                &row.id[..8], // Show only first 8 chars of UUID
                &row.name,
                &row.vendor,
                &row.format,
                &status_cell,
                &row.usage_count.to_string(),
                &row.project_count.to_string(),
            ]);
        }

        // Add summary row
        table.add_row(vec![
            "",
            &format!("Total: {} plugins", self.total_count),
            &format!("Showing {}-{} of {}", self.offset + 1, self.offset + self.displayed.len(), self.total_count),
            "",
            "",
            "",
            "",
        ]);

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
                    &row.installed.to_string(),
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
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["ID", "Name", "Vendor", "Format", "Status"]);

        for row in &self.displayed {
            let status_cell = if row.installed {
                colored_cell!("Installed", green)
            } else {
                colored_cell!("Missing", red)
            };

            table.add_row(vec![
                &row.id[..8],
                &row.name,
                &row.vendor,
                &row.format,
                &status_cell,
            ]);
        }

        // Add search summary
        table.add_row(vec![
            "",
            &format!("Search: '{}' - {} results", self.query, self.total_count),
            "",
            "",
            "",
        ]);

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
                    &row.installed.to_string(),
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
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        table_row!(table, "ID", self.plugin.id);
        table_row!(table, "Name", self.plugin.name);
        table_row!(table, "Vendor", self.plugin.vendor.as_deref().unwrap_or("Unknown"));
        table_row!(table, "Format", self.plugin.plugin_format.to_string());
        table_row!(table, "Dev Identifier", self.plugin.dev_identifier);
        
        let status = if self.plugin.installed {
            colored_cell!("Installed", green)
        } else {
            colored_cell!("Missing", red)
        };
        table_row!(table, "Status", status);
        
        table_row!(table, "Version", self.plugin.version.as_deref().unwrap_or("Unknown"));
        table_row!(table, "SDK Version", self.plugin.sdk_version.as_deref().unwrap_or("Unknown"));
        table_row!(table, "Usage Count", self.usage_count);
        table_row!(table, "Used in Projects", self.project_count);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["id", &self.plugin.id.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["name", &self.plugin.name]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["vendor", &self.plugin.vendor.as_deref().unwrap_or("")]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["format", &self.plugin.plugin_format.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["dev_identifier", &self.plugin.dev_identifier]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["installed", &self.plugin.installed.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["version", &self.plugin.version.as_deref().unwrap_or("")]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["sdk_version", &self.plugin.sdk_version.as_deref().unwrap_or("")]).map_err(|e| -> CliError { e.into() })?;
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
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Category", "Metric", "Value"]);

        // Basic stats
        table_row!(table, "Overview", "Total Plugins", self.stats.total_plugins);
        table_row!(table, "Overview", "Installed Plugins", self.stats.installed_plugins);
        table_row!(table, "Overview", "Missing Plugins", self.stats.missing_plugins);
        table_row!(table, "Overview", "Unique Vendors", self.stats.unique_vendors);

        // Format breakdown
        for (format, count) in &self.stats.plugins_by_format {
            table_row!(table, "Formats", format, count);
        }

        // Top vendors
        for (vendor, count) in &self.stats.plugins_by_vendor {
            table_row!(table, "Top Vendors", vendor, count);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["category", "metric", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Total Plugins", &self.stats.total_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Installed Plugins", &self.stats.installed_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Missing Plugins", &self.stats.missing_plugins.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Unique Vendors", &self.stats.unique_vendors.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginRefreshDisplay {
    pub result: PluginRefreshResult,
}

impl TableDisplay for PluginRefreshDisplay {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Refresh Result", "Count"]);

        table_row!(table, "Total Checked", self.result.total_plugins_checked);
        table_row!(table, colored_cell!("Now Installed", green), self.result.plugins_now_installed);
        table_row!(table, colored_cell!("Now Missing", red), self.result.plugins_now_missing);
        table_row!(table, "Unchanged", self.result.plugins_unchanged);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["result", "count"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["total_checked", &self.result.total_plugins_checked.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["now_installed", &self.result.plugins_now_installed.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["now_missing", &self.result.plugins_now_missing.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["unchanged", &self.result.plugins_unchanged.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct PluginVendorsDisplay {
    pub vendors: Vec<VendorInfo>,
    pub total_count: usize,
}

impl TableDisplay for PluginVendorsDisplay {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Vendor", "Total", "Installed", "Missing", "Usage", "Projects"]);

        for vendor in &self.vendors {
            table.add_row(vec![
                vendor.vendor.as_str(),
                &vendor.plugin_count.to_string(),
                &vendor.installed_plugins.to_string(),
                &vendor.missing_plugins.to_string(),
                &vendor.total_usage_count.to_string(),
                &vendor.unique_projects_using.to_string(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["vendor", "total_plugins", "installed", "missing", "usage_count", "projects_using"]).map_err(|e| -> CliError { e.into() })?;
        for vendor in &self.vendors {
            writer.write_record([
                vendor.vendor.as_str(),
                &vendor.plugin_count.to_string(),
                &vendor.installed_plugins.to_string(),
                &vendor.missing_plugins.to_string(),
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
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Format", "Total", "Installed", "Missing", "Usage", "Projects"]);

        for format in &self.formats {
            table.add_row(vec![
                format.format.as_str(),
                &format.plugin_count.to_string(),
                &format.installed_plugins.to_string(),
                &format.missing_plugins.to_string(),
                &format.total_usage_count.to_string(),
                &format.unique_projects_using.to_string(),
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["format", "total_plugins", "installed", "missing", "usage_count", "projects_using"]).map_err(|e| -> CliError { e.into() })?;
        for format in &self.formats {
            writer.write_record([
                format.format.as_str(),
                &format.plugin_count.to_string(),
                &format.installed_plugins.to_string(),
                &format.missing_plugins.to_string(),
                &format.total_usage_count.to_string(),
                &format.unique_projects_using.to_string(),
            ]).map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}
