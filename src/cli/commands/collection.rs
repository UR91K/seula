use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay};
use crate::cli::{CliError, CollectionCommands};
use crate::database::LiveSetDatabase;
use crate::live_set::LiveSet;
use crate::models::CollectionStatistics;
use crate::{colored_cell, table_row};
use colored::Colorize;
use comfy_table::Table;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct CollectionCommand;

#[async_trait::async_trait]
impl CliCommand for CollectionCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use CollectionCommands for actual functionality.
        println!("Use 'seula collection list', 'seula collection show', 'seula collection create', 'seula collection add', or 'seula collection remove' for collection operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for CollectionCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            CollectionCommands::List => {
                let collections_list = self.get_collections_list(&ctx.db).await?;
                formatter.print(&collections_list)?;
            }
            CollectionCommands::Show { id } => {
                let collection_details = self.get_collection_details(&ctx.db, id).await?;
                formatter.print(&collection_details)?;
            }
            CollectionCommands::Create { name, description } => {
                let create_result = self.create_collection(&ctx.db, name, description.as_deref()).await?;
                formatter.print(&create_result)?;
            }
            CollectionCommands::Add { collection_id, project_id } => {
                let add_result = self.add_project_to_collection(&ctx.db, collection_id, project_id).await?;
                formatter.print(&add_result)?;
            }
            CollectionCommands::Remove { collection_id, project_id } => {
                let remove_result = self.remove_project_from_collection(&ctx.db, collection_id, project_id).await?;
                formatter.print(&remove_result)?;
            }
        }

        Ok(())
    }
}

impl CollectionCommands {
    async fn get_collections_list(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
    ) -> Result<CollectionsList, CliError> {
        let mut db_guard = db.lock().await;
        let (collections, total_count) = db_guard.list_collections(None, None, None, None)?;

        let displayed = collections
            .into_iter()
            .map(|(id, name, description)| CollectionRow {
                id: id[..8].to_string(), // Show only first 8 chars of UUID
                name,
                description: description.unwrap_or_else(|| "No description".to_string()),
            })
            .collect();

        Ok(CollectionsList {
            displayed,
            total_count: total_count as usize,
        })
    }

    async fn get_collection_details(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        collection_id: &str,
    ) -> Result<CollectionDetails, CliError> {
        let mut db_guard = db.lock().await;
        
        // Get collection basic info
        let collection_data = db_guard.get_collection_by_id(collection_id)?;
        let collection_data = collection_data.ok_or_else(|| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Collection {} not found", collection_id)
            )) as CliError
        })?;

        let (id, name, description, notes, created_at, modified_at, _project_ids, _cover_art_id) = collection_data;
        
        // Get collection statistics
        let stats = db_guard.get_collection_detailed_statistics(collection_id)?;
        
        // Get projects in collection
        let projects = db_guard.get_collection_projects(collection_id)?;
        let project_rows = projects
            .into_iter()
            .map(|project| ProjectInCollectionRow {
                id: project.id.to_string()[..8].to_string(),
                name: project.name,
                tempo: project.tempo,
                duration: project.estimated_duration
                    .map(|d| format!("{:.1}s", d.num_seconds() as f64))
                    .unwrap_or_else(|| "Unknown".to_string()),
            })
            .collect();

        Ok(CollectionDetails {
            id: id[..8].to_string(),
            name,
            description: description.unwrap_or_else(|| "No description".to_string()),
            notes: notes.unwrap_or_else(|| "No notes".to_string()),
            created_at,
            modified_at,
            stats,
            projects: project_rows,
        })
    }

    async fn create_collection(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        name: &str,
        description: Option<&str>,
    ) -> Result<CollectionCreateResult, CliError> {
        let mut db_guard = db.lock().await;
        let collection_id = db_guard.create_collection(name, description, None)?;

        Ok(CollectionCreateResult {
            id: collection_id[..8].to_string(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()).unwrap_or_else(|| "No description".to_string()),
        })
    }

    async fn add_project_to_collection(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        collection_id: &str,
        project_id: &str,
    ) -> Result<CollectionProjectResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify collection exists
        let collection_data = db_guard.get_collection_by_id(collection_id)?;
        if collection_data.is_none() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Collection {} not found", collection_id)
            )) as CliError);
        }

        db_guard.add_project_to_collection(collection_id, project_id)?;

        Ok(CollectionProjectResult {
            collection_id: collection_id[..8].to_string(),
            project_id: project_id[..8].to_string(),
            action: "Added".to_string(),
            success: true,
        })
    }

    async fn remove_project_from_collection(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        collection_id: &str,
        project_id: &str,
    ) -> Result<CollectionProjectResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify collection exists
        let collection_data = db_guard.get_collection_by_id(collection_id)?;
        if collection_data.is_none() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Collection {} not found", collection_id)
            )) as CliError);
        }

        db_guard.remove_project_from_collection(collection_id, project_id)?;

        Ok(CollectionProjectResult {
            collection_id: collection_id[..8].to_string(),
            project_id: project_id[..8].to_string(),
            action: "Removed".to_string(),
            success: true,
        })
    }
}

#[derive(Serialize)]
pub struct CollectionRow {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct CollectionsList {
    pub displayed: Vec<CollectionRow>,
    pub total_count: usize,
}

impl TableDisplay for CollectionsList {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["ID", "Name", "Description"]);

        for row in &self.displayed {
            table.add_row(vec![
                &row.id,
                &row.name,
                &row.description,
            ]);
        }

        // Add summary row
        table.add_row(vec![
            "",
            &format!("Total: {} collections", self.total_count),
            "",
        ]);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["id", "name", "description"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.name.as_str(),
                    row.description.as_str(),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct ProjectInCollectionRow {
    pub id: String,
    pub name: String,
    pub tempo: f64,
    pub duration: String,
}

#[derive(Serialize)]
pub struct CollectionDetails {
    pub id: String,
    pub name: String,
    pub description: String,
    pub notes: String,
    pub created_at: i64,
    pub modified_at: i64,
    pub stats: CollectionStatistics,
    pub projects: Vec<ProjectInCollectionRow>,
}

impl TableDisplay for CollectionDetails {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        // Basic info
        table_row!(table, "ID", self.id);
        table_row!(table, "Name", self.name);
        table_row!(table, "Description", self.description);
        table_row!(table, "Notes", self.notes);
        
        // Statistics
        table_row!(table, "Project Count", self.stats.project_count);
        if let Some(duration) = self.stats.total_duration_seconds {
            table_row!(table, "Total Duration", format!("{:.1} seconds", duration));
        }
        if let Some(tempo) = self.stats.average_tempo {
            table_row!(table, "Average Tempo", format!("{:.1} BPM", tempo));
        }
        table_row!(table, "Total Plugins", self.stats.total_plugins);
        table_row!(table, "Total Samples", self.stats.total_samples);
        table_row!(table, "Total Tags", self.stats.total_tags);
        
        if let Some(key) = &self.stats.most_common_key {
            table_row!(table, "Most Common Key", key);
        }
        if let Some(time_sig) = &self.stats.most_common_time_signature {
            table_row!(table, "Most Common Time Signature", time_sig);
        }

        // Add separator
        table.add_row(vec!["", ""]);
        table.add_row(vec!["Projects in Collection", ""]);
        table.add_row(vec!["", ""]);
        
        // Projects header
        table.add_row(vec!["Project ID", "Name", "Tempo", "Duration"]);
        
        for project in &self.projects {
            table.add_row(vec![
                &project.id,
                &project.name,
                &project.tempo.to_string(),
                &project.duration,
            ]);
        }

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        // Collection info
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["id", &self.id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["name", &self.name]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["description", &self.description]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_count", &self.stats.project_count.to_string()]).map_err(|e| -> CliError { e.into() })?;
        
        // Projects
        writer.write_record(["", ""]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_id", "project_name", "tempo", "duration"]).map_err(|e| -> CliError { e.into() })?;
        for project in &self.projects {
            writer.write_record([
                &project.id,
                &project.name,
                &project.tempo.to_string(),
                &project.duration,
            ]).map_err(|e| -> CliError { e.into() })?;
        }
        
        Ok(())
    }
}

#[derive(Serialize)]
pub struct CollectionCreateResult {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl TableDisplay for CollectionCreateResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        table_row!(table, "Result", colored_cell!("Collection Created", green));
        table_row!(table, "ID", self.id);
        table_row!(table, "Name", self.name);
        table_row!(table, "Description", self.description);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["result", "Collection Created"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["id", &self.id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["name", &self.name]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["description", &self.description]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct CollectionProjectResult {
    pub collection_id: String,
    pub project_id: String,
    pub action: String,
    pub success: bool,
}

impl TableDisplay for CollectionProjectResult {
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
        table_row!(table, "Collection ID", self.collection_id);
        table_row!(table, "Project ID", self.project_id);
        table_row!(table, "Action", self.action);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["collection_id", &self.collection_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_id", &self.project_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["action", &self.action]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["success", &self.success.to_string()]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}