use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::{SystemCommands, WatchAction};
use crate::cli::CliError;
use crate::cli::output::{SimpleTable, TableDisplay, OutputFormatter};
use colored::Colorize;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
struct SystemInfo {
    table: SimpleTable,
}

impl TableDisplay for SystemInfo {
    fn to_simple_table(&self) -> SimpleTable {
        self.table.clone()
    }
    
    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.table.rows {
            if row.len() >= 2 {
                writer.write_record([&row[0], &row[1]]).map_err(|e| -> CliError { e.into() })?;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for SystemCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        match self {
            SystemCommands::Info => self.show_info(ctx).await,
            SystemCommands::Stats => self.show_stats(ctx).await,
            SystemCommands::Export { format, output } => self.export_data(ctx, format, output).await,
            SystemCommands::Watch { action } => self.handle_watch(ctx, action).await,
            SystemCommands::ScanStatus => self.show_scan_status(ctx).await,
        }
    }
}

impl SystemCommands {
    async fn show_info(&self, ctx: &CliContext) -> Result<(), CliError> {
        println!("{}", "System Information".bold().underline());

        let mut table = SimpleTable::new(vec!["Property".to_string(), "Value".to_string()]);

        // Add configuration info
        table.add_row(vec![
            "Database Path".to_string(),
            ctx.config.database_path.clone().unwrap_or_else(|| "<default>".to_string()),
        ]);

        table.add_row(vec![
            "Project Paths".to_string(),
            ctx.config.paths.len().to_string(),
        ]);

        table.add_row(vec![
            "gRPC Port".to_string(),
            ctx.config.grpc_port.to_string(),
        ]);

        table.add_row(vec![
            "Log Level".to_string(),
            ctx.config.log_level.clone(),
        ]);

        let info = SystemInfo { table };
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);
        formatter.print(&info)?;
        Ok(())
    }

    async fn show_stats(&self, ctx: &CliContext) -> Result<(), CliError> {
        println!("{}", "System Statistics".bold().underline());

        let db = ctx.db.lock().await;

        let mut table = SimpleTable::new(vec!["Statistic".to_string(), "Value".to_string()]);

        let (projects, _plugins, samples, collections, tags, tasks) =
            db.get_basic_counts().unwrap_or((0, 0, 0, 0, 0, 0));
        table.add_row(vec!["Total Projects".to_string(), projects.to_string()]);
        table.add_row(vec!["Total Samples".to_string(), samples.to_string()]);
        table.add_row(vec!["Total Collections".to_string(), collections.to_string()]);
        table.add_row(vec!["Total Tags".to_string(), tags.to_string()]);
        table.add_row(vec!["Total Tasks".to_string(), tasks.to_string()]);

        // Add more statistics as needed
        table.add_row(vec![
            "Database Path".to_string(),
            ctx.config.database_path.clone().unwrap_or_else(|| "<default>".to_string()),
        ]);

        let stats = SystemInfo { table };
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);
        formatter.print(&stats)?;
        Ok(())
    }

    async fn export_data(&self, ctx: &CliContext, format: &str, output: &PathBuf) -> Result<(), CliError> {
        println!("{}", format!("Exporting system data to {} format...", format).bold());

        // For now, just export basic statistics
        let db = ctx.db.lock().await;
        let (projects, _plugins, samples, _collections, _tags, _tasks) =
            db.get_basic_counts().unwrap_or((0, 0, 0, 0, 0, 0));

        let data = format!("Projects: {}, Samples: {}", projects, samples);

        std::fs::write(output, &data)?;

        println!("{}", format!("Data exported to: {}", output.display()).green());
        Ok(())
    }

    async fn handle_watch(&self, _ctx: &CliContext, action: &WatchAction) -> Result<(), CliError> {
        match action {
            WatchAction::Start => {
                println!("{}", "Starting file watcher...".bold());
                println!("{}", "File watcher functionality not yet implemented in CLI mode".yellow());
            }
            WatchAction::Stop => {
                println!("{}", "Stopping file watcher...".bold());
                println!("{}", "File watcher functionality not yet implemented in CLI mode".yellow());
            }
        }
        Ok(())
    }

    async fn show_scan_status(&self, _ctx: &CliContext) -> Result<(), CliError> {
        println!("{}", "Scan Status".bold().underline());
        println!("{}", "Scanning functionality not yet implemented in CLI mode".yellow());
        Ok(())
    }
}
