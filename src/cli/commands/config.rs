use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay};
use crate::cli::{CliError, ConfigCommands};
use crate::config::{Config, CONFIG};
use crate::{colored_cell, table_row};
use colored::Colorize;
use comfy_table::Table;
use serde::Serialize;
use std::process::Command;

pub struct ConfigCommand;

#[async_trait::async_trait]
impl CliCommand for ConfigCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use ConfigCommands for actual functionality.
        println!("Use 'seula config show', 'seula config validate', or 'seula config edit' for configuration operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for ConfigCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            ConfigCommands::Show => {
                let config_display = self.show_config().await?;
                formatter.print(&config_display)?;
            }
            ConfigCommands::Validate => {
                let validation_result = self.validate_config().await?;
                formatter.print(&validation_result)?;
            }
            ConfigCommands::Edit => {
                let edit_result = self.edit_config().await?;
                formatter.print(&edit_result)?;
            }
        }

        Ok(())
    }
}

impl ConfigCommands {
    async fn show_config(&self) -> Result<ConfigDisplay, CliError> {
        let config = CONFIG.as_ref().map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to load configuration: {}", e)
            )) as CliError
        })?;

        Ok(ConfigDisplay {
            paths: config.paths.clone(),
            database_path: config.database_path().unwrap_or_else(|| "Default".to_string()),
            live_database_dir: config.live_database_dir.clone(),
            grpc_port: config.grpc_port(),
            log_level: config.log_level(),
            media_storage_dir: config.media_storage_dir.clone(),
            max_cover_art_size_mb: config.max_cover_art_size_mb,
            max_audio_file_size_mb: config.max_audio_file_size_mb,
            status_message: config.get_status_message(),
            is_ready: config.is_ready_for_operation(),
        })
    }

    async fn validate_config(&self) -> Result<ConfigValidationResult, CliError> {
        let config = CONFIG.as_ref().map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to load configuration: {}", e)
            )) as CliError
        })?;

        match config.validate() {
            Ok(warnings) => Ok(ConfigValidationResult {
                is_valid: true,
                warnings,
                errors: Vec::new(),
            }),
            Err(e) => Ok(ConfigValidationResult {
                is_valid: false,
                warnings: Vec::new(),
                errors: vec![e.to_string()],
            }),
        }
    }

    async fn edit_config(&self) -> Result<ConfigEditResult, CliError> {
        // Find the config file path
        let config_path = crate::config::loader::find_config_file().map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Failed to find config file: {}", e)
            )) as CliError
        })?;

        // Try to open with system default editor
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| {
                // Default editors by platform
                if cfg!(windows) {
                    "notepad".to_string()
                } else if cfg!(target_os = "macos") {
                    "open".to_string()
                } else {
                    "nano".to_string()
                }
            });

        let mut cmd = Command::new(&editor);
        
        // Special handling for macOS 'open' command
        if cfg!(target_os = "macos") && editor == "open" {
            cmd.arg("-t"); // Open in text editor
        }
        
        cmd.arg(&config_path);

        match cmd.status() {
            Ok(status) => {
                if status.success() {
                    Ok(ConfigEditResult {
                        success: true,
                        message: format!("Config file opened with {}", editor),
                        config_path: config_path.to_string_lossy().to_string(),
                    })
                } else {
                    Ok(ConfigEditResult {
                        success: false,
                        message: format!("Editor {} exited with error", editor),
                        config_path: config_path.to_string_lossy().to_string(),
                    })
                }
            }
            Err(e) => Ok(ConfigEditResult {
                success: false,
                message: format!("Failed to open editor {}: {}", editor, e),
                config_path: config_path.to_string_lossy().to_string(),
            }),
        }
    }
}

#[derive(Serialize)]
pub struct ConfigDisplay {
    pub paths: Vec<String>,
    pub database_path: String,
    pub live_database_dir: String,
    pub grpc_port: u16,
    pub log_level: String,
    pub media_storage_dir: String,
    pub max_cover_art_size_mb: Option<u32>,
    pub max_audio_file_size_mb: Option<u32>,
    pub status_message: String,
    pub is_ready: bool,
}

impl TableDisplay for ConfigDisplay {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Setting", "Value"]);

        // Status
        let status_cell = if self.is_ready {
            colored_cell!("Ready", green)
        } else {
            colored_cell!("Setup Required", yellow)
        };
        table_row!(table, "Status", status_cell);
        table_row!(table, "Status Message", self.status_message);

        // Basic settings
        table_row!(table, "gRPC Port", self.grpc_port);
        table_row!(table, "Log Level", self.log_level);

        // Paths
        table_row!(table, "Database Path", self.database_path);
        table_row!(table, "Live Database Dir", self.live_database_dir);
        table_row!(table, "Media Storage Dir", self.media_storage_dir);

        // Project paths
        if self.paths.is_empty() {
            table_row!(table, "Project Paths", colored_cell!("None configured", red));
        } else {
            table_row!(table, "Project Paths", format!("{} configured", self.paths.len()));
            for (i, path) in self.paths.iter().enumerate() {
                table_row!(table, format!("  Path {}", i + 1), path);
            }
        }

        // Media limits
        let cover_art_limit = self.max_cover_art_size_mb
            .map(|size| if size == 0 { "No limit".to_string() } else { format!("{} MB", size) })
            .unwrap_or_else(|| "Default".to_string());
        table_row!(table, "Max Cover Art Size", cover_art_limit);

        let audio_limit = self.max_audio_file_size_mb
            .map(|size| if size == 0 { "No limit".to_string() } else { format!("{} MB", size) })
            .unwrap_or_else(|| "Default".to_string());
        table_row!(table, "Max Audio File Size", audio_limit);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["setting", "value"]).map_err(|e| -> CliError { e.into() })?;
        
        writer.write_record(["status", if self.is_ready { "Ready" } else { "Setup Required" }]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["status_message", &self.status_message]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["grpc_port", &self.grpc_port.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["log_level", &self.log_level]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["database_path", &self.database_path]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["live_database_dir", &self.live_database_dir]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["media_storage_dir", &self.media_storage_dir]).map_err(|e| -> CliError { e.into() })?;
        
        // Project paths
        for (i, path) in self.paths.iter().enumerate() {
            writer.write_record([&format!("project_path_{}", i + 1), path]).map_err(|e| -> CliError { e.into() })?;
        }

        // Media limits
        let cover_art_limit = self.max_cover_art_size_mb
            .map(|size| if size == 0 { "No limit".to_string() } else { format!("{} MB", size) })
            .unwrap_or_else(|| "Default".to_string());
        writer.write_record(["max_cover_art_size", &cover_art_limit]).map_err(|e| -> CliError { e.into() })?;

        let audio_limit = self.max_audio_file_size_mb
            .map(|size| if size == 0 { "No limit".to_string() } else { format!("{} MB", size) })
            .unwrap_or_else(|| "Default".to_string());
        writer.write_record(["max_audio_file_size", &audio_limit]).map_err(|e| -> CliError { e.into() })?;

        Ok(())
    }
}

#[derive(Serialize)]
pub struct ConfigValidationResult {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl TableDisplay for ConfigValidationResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Validation Result", "Details"]);

        let status_cell = if self.is_valid {
            colored_cell!("Valid", green)
        } else {
            colored_cell!("Invalid", red)
        };
        table_row!(table, "Status", status_cell);

        // Errors
        if !self.errors.is_empty() {
            table.add_row(vec!["", ""]);
            table_row!(table, colored_cell!("Errors", red), "");
            for error in &self.errors {
                table_row!(table, "  •", error);
            }
        }

        // Warnings
        if !self.warnings.is_empty() {
            table.add_row(vec!["", ""]);
            table_row!(table, colored_cell!("Warnings", yellow), "");
            for warning in &self.warnings {
                table_row!(table, "  •", warning);
            }
        }

        if self.errors.is_empty() && self.warnings.is_empty() {
            table_row!(table, "Result", "Configuration is valid with no issues");
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["type", "message"]).map_err(|e| -> CliError { e.into() })?;
        
        writer.write_record(["status", if self.is_valid { "Valid" } else { "Invalid" }]).map_err(|e| -> CliError { e.into() })?;
        
        for error in &self.errors {
            writer.write_record(["error", error]).map_err(|e| -> CliError { e.into() })?;
        }
        
        for warning in &self.warnings {
            writer.write_record(["warning", warning]).map_err(|e| -> CliError { e.into() })?;
        }

        Ok(())
    }
}

#[derive(Serialize)]
pub struct ConfigEditResult {
    pub success: bool,
    pub message: String,
    pub config_path: String,
}

impl TableDisplay for ConfigEditResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        let status_cell = if self.success {
            colored_cell!("Success", green)
        } else {
            colored_cell!("Failed", red)
        };
        table_row!(table, "Result", status_cell);
        table_row!(table, "Message", self.message);
        table_row!(table, "Config Path", self.config_path);

        if self.success {
            table.add_row(vec!["", ""]);
            table_row!(table, "Note", "After editing, run 'seula config validate' to check your changes");
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["success", &self.success.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["message", &self.message]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["config_path", &self.config_path]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}