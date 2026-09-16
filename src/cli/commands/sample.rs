use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay, SimpleTable};
use crate::cli::{CliError, SampleCommands};
use crate::models::Sample;
use crate::services::SamplesService;
use crate::{colored_cell, simple_table_row};
use colored::Colorize;

use serde::Serialize;

pub struct SampleCommand;

#[async_trait::async_trait]
impl CliCommand for SampleCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use SampleCommands for actual functionality.
        println!("Use 'seula sample list', 'seula sample search', 'seula sample stats', or 'seula sample check-presence' for sample operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for SampleCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            SampleCommands::List { limit, offset } => {
                let samples_list = self.get_samples_list(&ctx.services.samples, *limit, *offset).await?;
                formatter.print(&samples_list)?;
            }
            SampleCommands::Search { query, limit } => {
                let search_results = self.search_samples(&ctx.services.samples, query, *limit).await?;
                formatter.print(&search_results)?;
            }
            SampleCommands::Stats => {
                let stats = self.get_sample_stats(&ctx.services.samples).await?;
                formatter.print(&stats)?;
            }
            SampleCommands::CheckPresence => {
                let refresh_result = self.check_sample_presence(&ctx.services.samples).await?;
                formatter.print(&refresh_result)?;
            }
        }

        Ok(())
    }
}

impl SampleCommands {
    async fn get_samples_list(
        &self,
        samples: &SamplesService,
        limit: usize,
        offset: usize,
    ) -> Result<SamplesList, CliError> {
        let (samples, total_count) = samples.get_all_samples(
            Some(limit as i32),
            Some(offset as i32),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ).await?;

        let displayed = samples
            .into_iter()
            .map(|sample| SampleRow {
                id: sample.id.to_string(),
                name: sample.name,
                path: sample.path.to_string_lossy().to_string(),
                status: if sample.is_present { "Present" } else { "Missing" },
            })
            .collect();

        Ok(SamplesList {
            displayed,
            total_count: total_count as usize,
            limit,
            offset,
        })
    }

    async fn search_samples(
        &self,
        samples: &SamplesService,
        query: &str,
        limit: usize,
    ) -> Result<SamplesSearchResults, CliError> {
        let (samples, total_count) = samples.search_samples(
            query,
            Some(limit as i32),
            Some(0),
            None,
            None,
        ).await?;

        let displayed = samples
            .into_iter()
            .map(|sample| SampleRow {
                id: sample.id.to_string(),
                name: sample.name,
                path: sample.path.to_string_lossy().to_string(),
                status: if sample.is_present { "Present" } else { "Missing" },
            })
            .collect();

        Ok(SamplesSearchResults {
            query: query.to_string(),
            displayed,
            total_count: total_count as usize,
        })
    }

    async fn get_sample_stats(&self, samples: &SamplesService) -> Result<SampleStatsDisplay, CliError> {
        let stats = samples.get_sample_stats().await?;
        let analytics = samples.get_sample_analytics().await?;

        Ok(SampleStatsDisplay {
            stats,
            analytics,
        })
    }

    async fn check_sample_presence(&self, samples: &SamplesService) -> Result<SamplePresenceCheckResult, CliError> {
        let refresh_result = samples.refresh_sample_presence_status().await?;

        Ok(SamplePresenceCheckResult {
            total_checked: refresh_result.total_samples_checked as usize,
            now_present: refresh_result.samples_now_present as usize,
            now_missing: refresh_result.samples_now_missing as usize,
            unchanged: refresh_result.samples_unchanged as usize,
        })
    }
}

#[derive(Serialize)]
pub struct SampleRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct SamplesList {
    pub displayed: Vec<SampleRow>,
    pub total_count: usize,
    pub limit: usize,
    pub offset: usize,
}

impl TableDisplay for SamplesList {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Path".to_string(),
            "Status".to_string(),
        ]);

        for row in &self.displayed {
            let status_cell = match row.status {
                "Present" => colored_cell!("Present", green),
                "Missing" => colored_cell!("Missing", red),
                _ => row.status.to_string(),
            };

            simple_table_row!(table,
                &row.id[..8], // Show only first 8 chars of UUID
                &row.name,
                &row.path,
                &status_cell
            );
        }

        // Add summary row
        simple_table_row!(table,
            "",
            &format!("Total: {} samples", self.total_count),
            &format!("Showing {}-{} of {}", self.offset + 1, self.offset + self.displayed.len(), self.total_count),
            ""
        );

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["id", "name", "path", "status"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.name.as_str(),
                    row.path.as_str(),
                    row.status,
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct SamplesSearchResults {
    pub query: String,
    pub displayed: Vec<SampleRow>,
    pub total_count: usize,
}

impl TableDisplay for SamplesSearchResults {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec![
            "ID".to_string(),
            "Name".to_string(),
            "Path".to_string(),
            "Status".to_string(),
        ]);

        for row in &self.displayed {
            let status_cell = match row.status {
                "Present" => colored_cell!("Present", green),
                "Missing" => colored_cell!("Missing", red),
                _ => row.status.to_string(),
            };

            simple_table_row!(table,
                &row.id[..8],
                &row.name,
                &row.path,
                &status_cell
            );
        }

        // Add search summary
        simple_table_row!(table,
            "",
            &format!("Search: '{}' - {} results", self.query, self.total_count),
            "",
            ""
        );

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["query", "id", "name", "path", "status"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    self.query.as_str(),
                    row.id.as_str(),
                    row.name.as_str(),
                    row.path.as_str(),
                    row.status,
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct SampleStatsDisplay {
    pub stats: crate::database::samples::SampleStats,
    pub analytics: crate::database::samples::SampleAnalytics,
}

impl TableDisplay for SampleStatsDisplay {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec!["Category".to_string(), "Metric".to_string(), "Value".to_string()]);

        // Basic stats
        simple_table_row!(table, "Overview", "Total Samples", self.stats.total_samples);
        simple_table_row!(table, "Overview", "Present Samples", self.stats.present_samples);
        simple_table_row!(table, "Overview", "Missing Samples", self.stats.missing_samples);
        simple_table_row!(table, "Overview", "Unique Paths", self.stats.unique_paths);

        // Storage info
        let total_gb = self.stats.total_estimated_size_bytes as f64 / 1_000_000_000.0;
        simple_table_row!(table, "Storage", "Estimated Total Size", format!("{:.2} GB", total_gb));

        // Usage distribution
        simple_table_row!(table, "Usage", "Most Used (≥5)", self.analytics.most_used_samples_count);
        simple_table_row!(table, "Usage", "Moderately Used (2-4)", self.analytics.moderately_used_samples_count);
        simple_table_row!(table, "Usage", "Rarely Used (=1)", self.analytics.rarely_used_samples_count);
        simple_table_row!(table, "Usage", "Unused (=0)", self.analytics.unused_samples_count);

        // Presence percentages
        simple_table_row!(table, "Presence", "Present %", format!("{}%", self.analytics.present_samples_percentage));
        simple_table_row!(table, "Presence", "Missing %", format!("{}%", self.analytics.missing_samples_percentage));

        // Extension breakdown
        for (ext, analytics) in &self.analytics.extensions {
            simple_table_row!(table, "Extensions", format!("{} files", ext), analytics.count);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["category", "metric", "value"]).map_err(|e| -> CliError { e.into() })?;

        // Basic stats
        writer.write_record(["Overview", "Total Samples", &self.stats.total_samples.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Present Samples", &self.stats.present_samples.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Missing Samples", &self.stats.missing_samples.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Overview", "Unique Paths", &self.stats.unique_paths.to_string()]).map_err(|e| -> CliError { e.into() })?;

        // Usage distribution
        writer.write_record(["Usage", "Most Used (≥5)", &self.analytics.most_used_samples_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Usage", "Moderately Used (2-4)", &self.analytics.moderately_used_samples_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Usage", "Rarely Used (=1)", &self.analytics.rarely_used_samples_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["Usage", "Unused (=0)", &self.analytics.unused_samples_count.to_string()]).map_err(|e| -> CliError { e.into() })?;

        Ok(())
    }
}

#[derive(Serialize)]
pub struct SamplePresenceCheckResult {
    pub total_checked: usize,
    pub now_present: usize,
    pub now_missing: usize,
    pub unchanged: usize,
}

impl TableDisplay for SamplePresenceCheckResult {
    fn to_simple_table(&self) -> SimpleTable {
        let mut table = SimpleTable::new(vec!["Check Result".to_string(), "Count".to_string()]);

        simple_table_row!(table, "Total Checked", self.total_checked);
        simple_table_row!(table, colored_cell!("Now Present", green), self.now_present);
        simple_table_row!(table, colored_cell!("Now Missing", red), self.now_missing);
        simple_table_row!(table, "Unchanged", self.unchanged);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["result", "count"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["total_checked", &self.total_checked.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["now_present", &self.now_present.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["now_missing", &self.now_missing.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["unchanged", &self.unchanged.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}
