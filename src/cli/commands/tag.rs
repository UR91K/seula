use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay};
use crate::cli::{CliError, TagCommands};
use crate::database::tags::{TagStatistics, TagUsageInfo};
use crate::database::LiveSetDatabase;
use crate::live_set::LiveSet;
use crate::{colored_cell, table_row};
use colored::Colorize;
use comfy_table::Table;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct TagCommand;

#[async_trait::async_trait]
impl CliCommand for TagCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use TagCommands for actual functionality.
        println!("Use 'seula tag list', 'seula tag create', 'seula tag assign', 'seula tag remove', or 'seula tag search' for tag operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for TagCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            TagCommands::List => {
                let tags_list = self.get_tags_list(&ctx.db).await?;
                formatter.print(&tags_list)?;
            }
            TagCommands::Create { name, color } => {
                let create_result = self.create_tag(&ctx.db, name, color.as_deref()).await?;
                formatter.print(&create_result)?;
            }
            TagCommands::Assign { project_id, tag_id } => {
                let assign_result = self.assign_tag(&ctx.db, project_id, tag_id).await?;
                formatter.print(&assign_result)?;
            }
            TagCommands::Remove { project_id, tag_id } => {
                let remove_result = self.remove_tag(&ctx.db, project_id, tag_id).await?;
                formatter.print(&remove_result)?;
            }
            TagCommands::Search { tag } => {
                let search_results = self.search_by_tag(&ctx.db, tag).await?;
                formatter.print(&search_results)?;
            }
        }

        Ok(())
    }
}

impl TagCommands {
    async fn get_tags_list(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
    ) -> Result<TagsList, CliError> {
        let mut db_guard = db.lock().await;
        let (tags, total_count) = db_guard.get_all_tags_with_usage(None, None, None, None, None)?;

        let displayed = tags
            .into_iter()
            .map(|tag| TagRow {
                id: tag.tag_id[..8].to_string(), // Show only first 8 chars of UUID
                name: tag.name,
                project_count: tag.project_count,
                usage_percentage: format!("{:.1}%", tag.usage_percentage),
            })
            .collect();

        Ok(TagsList {
            displayed,
            total_count: total_count as usize,
        })
    }

    async fn create_tag(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        name: &str,
        _color: Option<&str>, // Color is not stored in current database schema
    ) -> Result<TagCreateResult, CliError> {
        let mut db_guard = db.lock().await;
        let tag_id = db_guard.add_tag(name)?;

        Ok(TagCreateResult {
            id: tag_id[..8].to_string(),
            name: name.to_string(),
            success: true,
        })
    }

    async fn assign_tag(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        project_id: &str,
        tag_id: &str,
    ) -> Result<TagAssignResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify tag exists
        let tag_data = db_guard.get_tag_by_id(tag_id)?;
        let tag_name = match tag_data {
            Some((_, name, _)) => name,
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Tag {} not found", tag_id)
                )) as CliError);
            }
        };

        db_guard.tag_project(project_id, tag_id)?;

        Ok(TagAssignResult {
            project_id: project_id[..8].to_string(),
            tag_id: tag_id[..8].to_string(),
            tag_name,
            action: "Assigned".to_string(),
            success: true,
        })
    }

    async fn remove_tag(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        project_id: &str,
        tag_id: &str,
    ) -> Result<TagAssignResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify tag exists
        let tag_data = db_guard.get_tag_by_id(tag_id)?;
        let tag_name = match tag_data {
            Some((_, name, _)) => name,
            None => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Tag {} not found", tag_id)
                )) as CliError);
            }
        };

        db_guard.untag_project(project_id, tag_id)?;

        Ok(TagAssignResult {
            project_id: project_id[..8].to_string(),
            tag_id: tag_id[..8].to_string(),
            tag_name,
            action: "Removed".to_string(),
            success: true,
        })
    }

    async fn search_by_tag(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        tag: &str,
    ) -> Result<TagSearchResults, CliError> {
        let mut db_guard = db.lock().await;
        
        // First try to find the tag by name or ID
        let tag_id = if tag.len() == 8 || tag.len() == 36 {
            // Looks like a UUID (shortened or full)
            tag.to_string()
        } else {
            // Search by name
            let (tags, _) = db_guard.search_tags(tag, Some(1), Some(0))?;
            if tags.is_empty() {
                return Ok(TagSearchResults {
                    query: tag.to_string(),
                    tag_name: None,
                    projects: Vec::new(),
                    total_count: 0,
                });
            }
            tags[0].0.clone() // Use the first matching tag's ID
        };

        // Get the tag info for display
        let tag_data = db_guard.get_tag_by_id(&tag_id)?;
        let tag_name = tag_data.as_ref().map(|(_, name, _)| name.clone());

        // Get projects with this tag
        let projects = db_guard.get_projects_by_tag(&tag_id)?;
        let project_rows = projects
            .into_iter()
            .map(|project| ProjectWithTagRow {
                id: project.id.to_string()[..8].to_string(),
                name: project.name,
                path: project.file_path.to_string_lossy().to_string(),
                tempo: project.tempo,
                key: project.key_signature.map(|k| k.to_string()).unwrap_or_else(|| "Unknown".to_string()),
            })
            .collect::<Vec<_>>();

        let total_count = project_rows.len();

        Ok(TagSearchResults {
            query: tag.to_string(),
            tag_name,
            projects: project_rows,
            total_count,
        })
    }
}

#[derive(Serialize)]
pub struct TagRow {
    pub id: String,
    pub name: String,
    pub project_count: i32,
    pub usage_percentage: String,
}

#[derive(Serialize)]
pub struct TagsList {
    pub displayed: Vec<TagRow>,
    pub total_count: usize,
}

impl TableDisplay for TagsList {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["ID", "Name", "Projects", "Usage %"]);

        for row in &self.displayed {
            let usage_cell = if row.project_count > 0 {
                colored_cell!(row.usage_percentage, green)
            } else {
                colored_cell!("0.0%", red)
            };

            table.add_row(vec![
                &row.id,
                &row.name,
                &row.project_count.to_string(),
                &usage_cell,
            ]);
        }

        // Add summary row
        table.add_row(vec![
            "",
            &format!("Total: {} tags", self.total_count),
            "",
            "",
        ]);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["id", "name", "project_count", "usage_percentage"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.name.as_str(),
                    &row.project_count.to_string(),
                    row.usage_percentage.as_str(),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct TagCreateResult {
    pub id: String,
    pub name: String,
    pub success: bool,
}

impl TableDisplay for TagCreateResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        let result_cell = if self.success {
            colored_cell!("Tag Created", green)
        } else {
            colored_cell!("Failed", red)
        };

        table_row!(table, "Result", result_cell);
        table_row!(table, "ID", self.id);
        table_row!(table, "Name", self.name);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["result", if self.success { "Tag Created" } else { "Failed" }]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["id", &self.id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["name", &self.name]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct TagAssignResult {
    pub project_id: String,
    pub tag_id: String,
    pub tag_name: String,
    pub action: String,
    pub success: bool,
}

impl TableDisplay for TagAssignResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        let result_text = if self.success {
            format!("{} Successfully", self.action)
        } else {
            format!("{} Failed", self.action)
        };
        
        let result_cell = if self.success {
            colored_cell!(result_text, green)
        } else {
            colored_cell!(result_text, red)
        };

        table_row!(table, "Result", result_cell);
        table_row!(table, "Project ID", self.project_id);
        table_row!(table, "Tag ID", self.tag_id);
        table_row!(table, "Tag Name", self.tag_name);
        table_row!(table, "Action", self.action);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_id", &self.project_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["tag_id", &self.tag_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["tag_name", &self.tag_name]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["action", &self.action]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["success", &self.success.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct ProjectWithTagRow {
    pub id: String,
    pub name: String,
    pub path: String,
    pub tempo: f64,
    pub key: String,
}

#[derive(Serialize)]
pub struct TagSearchResults {
    pub query: String,
    pub tag_name: Option<String>,
    pub projects: Vec<ProjectWithTagRow>,
    pub total_count: usize,
}

impl TableDisplay for TagSearchResults {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        
        if let Some(ref tag_name) = self.tag_name {
            table.set_header(vec!["Project ID", "Name", "Tempo", "Key", "Path"]);
            
            // Add tag info header
            table.add_row(vec![
                &format!("Tag: {}", tag_name),
                &format!("{} projects found", self.total_count),
                "",
                "",
                "",
            ]);
            table.add_row(vec!["", "", "", "", ""]); // Separator

            for project in &self.projects {
                table.add_row(vec![
                    &project.id,
                    &project.name,
                    &project.tempo.to_string(),
                    &project.key,
                    &project.path,
                ]);
            }
        } else {
            table.set_header(vec!["Search Result", "Details"]);
            table_row!(table, "Query", self.query);
            table_row!(table, colored_cell!("Result", red), "No tag found matching query");
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        if let Some(ref tag_name) = self.tag_name {
            writer.write_record(["query", "tag_name", "project_id", "project_name", "tempo", "key", "path"]).map_err(|e| -> CliError { e.into() })?;
            for project in &self.projects {
                writer.write_record([
                    &self.query,
                    tag_name,
                    &project.id,
                    &project.name,
                    &project.tempo.to_string(),
                    &project.key,
                    &project.path,
                ]).map_err(|e| -> CliError { e.into() })?;
            }
        } else {
            writer.write_record(["query", "result"]).map_err(|e| -> CliError { e.into() })?;
            writer.write_record([&self.query, "No tag found"]).map_err(|e| -> CliError { e.into() })?;
        }
        
        Ok(())
    }
}