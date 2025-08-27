use crate::cli::commands::{CliCommand, CliContext};
use crate::cli::output::{MessageType, OutputFormatter, TableDisplay};
use crate::cli::{CliError, TaskCommands};
use crate::database::tasks::TaskAnalytics;
use crate::database::LiveSetDatabase;
use crate::{colored_cell, table_row};
use colored::Colorize;
use comfy_table::Table;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

pub struct TaskCommand;

#[async_trait::async_trait]
impl CliCommand for TaskCommand {
    async fn execute(&self, _ctx: &CliContext) -> Result<(), CliError> {
        // This is a placeholder command. Use TaskCommands for actual functionality.
        println!("Use 'seula task list', 'seula task create', 'seula task complete', or 'seula task delete' for task operations");
        Ok(())
    }
}

#[async_trait::async_trait]
impl CliCommand for TaskCommands {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);

        match self {
            TaskCommands::List { project_id, completed } => {
                let tasks_list = self.get_tasks_list(&ctx.db, project_id.as_deref(), *completed).await?;
                formatter.print(&tasks_list)?;
            }
            TaskCommands::Create { project_id, description, priority } => {
                let create_result = self.create_task(&ctx.db, project_id, description, *priority).await?;
                formatter.print(&create_result)?;
            }
            TaskCommands::Complete { id } => {
                let complete_result = self.complete_task(&ctx.db, id).await?;
                formatter.print(&complete_result)?;
            }
            TaskCommands::Delete { id } => {
                let delete_result = self.delete_task(&ctx.db, id).await?;
                formatter.print(&delete_result)?;
            }
        }

        Ok(())
    }
}

impl TaskCommands {
    async fn get_tasks_list(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        project_id: Option<&str>,
        show_completed: bool,
    ) -> Result<TasksList, CliError> {
        let mut db_guard = db.lock().await;
        
        let tasks = if let Some(pid) = project_id {
            // Get tasks for specific project
            let all_tasks = db_guard.get_project_tasks(pid)?;
            if show_completed {
                all_tasks
            } else {
                all_tasks.into_iter().filter(|(_, _, completed, _)| !completed).collect()
            }
        } else {
            // Get all tasks from all projects (this would require a new database method)
            // For now, we'll return an error suggesting to specify a project
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Please specify a project ID to list tasks. Use 'seula project list' to see available projects."
            )) as CliError);
        };

        let displayed = tasks
            .into_iter()
            .map(|(id, description, completed, created_at)| TaskRow {
                id: id[..8].to_string(), // Show only first 8 chars of UUID
                description,
                status: if completed { "Completed" } else { "Pending" },
                created_at,
            })
            .collect::<Vec<_>>();

        let total_count = displayed.len();

        Ok(TasksList {
            displayed,
            total_count,
            project_id: project_id.map(|s| s.to_string()),
            show_completed,
        })
    }

    async fn create_task(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        project_id: &str,
        description: &str,
        _priority: u8, // Priority is not stored in current database schema
    ) -> Result<TaskCreateResult, CliError> {
        let mut db_guard = db.lock().await;
        let task_id = db_guard.add_task(project_id, description)?;

        Ok(TaskCreateResult {
            id: task_id[..8].to_string(),
            project_id: project_id[..8].to_string(),
            description: description.to_string(),
            success: true,
        })
    }

    async fn complete_task(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        task_id: &str,
    ) -> Result<TaskActionResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify task exists and get its details
        let task_data = db_guard.get_task(task_id)?;
        match task_data {
            Some((_, project_id, description, already_completed, _)) => {
                if already_completed {
                    Ok(TaskActionResult {
                        id: task_id[..8].to_string(),
                        project_id: project_id[..8].to_string(),
                        description,
                        action: "Complete".to_string(),
                        success: false,
                        message: "Task is already completed".to_string(),
                    })
                } else {
                    db_guard.complete_task(task_id, true)?;
                    Ok(TaskActionResult {
                        id: task_id[..8].to_string(),
                        project_id: project_id[..8].to_string(),
                        description,
                        action: "Complete".to_string(),
                        success: true,
                        message: "Task marked as completed".to_string(),
                    })
                }
            }
            None => {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Task {} not found", task_id)
                )) as CliError)
            }
        }
    }

    async fn delete_task(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        task_id: &str,
    ) -> Result<TaskActionResult, CliError> {
        let mut db_guard = db.lock().await;
        
        // Verify task exists and get its details
        let task_data = db_guard.get_task(task_id)?;
        match task_data {
            Some((_, project_id, description, _, _)) => {
                db_guard.remove_task(task_id)?;
                Ok(TaskActionResult {
                    id: task_id[..8].to_string(),
                    project_id: project_id[..8].to_string(),
                    description,
                    action: "Delete".to_string(),
                    success: true,
                    message: "Task deleted successfully".to_string(),
                })
            }
            None => {
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Task {} not found", task_id)
                )) as CliError)
            }
        }
    }
}

#[derive(Serialize)]
pub struct TaskRow {
    pub id: String,
    pub description: String,
    pub status: &'static str,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct TasksList {
    pub displayed: Vec<TaskRow>,
    pub total_count: usize,
    pub project_id: Option<String>,
    pub show_completed: bool,
}

impl TableDisplay for TasksList {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["ID", "Description", "Status", "Created"]);

        for row in &self.displayed {
            let status_cell = match row.status {
                "Completed" => colored_cell!("Completed", green),
                "Pending" => colored_cell!("Pending", yellow),
                _ => row.status.to_string(),
            };

            // Format timestamp as readable date
            let created_date = chrono::DateTime::from_timestamp(row.created_at, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            table.add_row(vec![
                &row.id,
                &row.description,
                &status_cell,
                &created_date,
            ]);
        }

        // Add summary row
        let project_info = if let Some(ref pid) = self.project_id {
            format!("Project: {}", &pid[..8])
        } else {
            "All Projects".to_string()
        };
        
        let status_info = if self.show_completed {
            "All Tasks".to_string()
        } else {
            "Pending Tasks".to_string()
        };

        table.add_row(vec![
            "",
            &format!("Total: {} tasks", self.total_count),
            &format!("{} | {}", project_info, status_info),
            "",
        ]);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["id", "description", "status", "created_at", "project_id"]).map_err(|e| -> CliError { e.into() })?;
        for row in &self.displayed {
            writer
                .write_record([
                    row.id.as_str(),
                    row.description.as_str(),
                    row.status,
                    &row.created_at.to_string(),
                    self.project_id.as_deref().unwrap_or(""),
                ])
                .map_err(|e| -> CliError { e.into() })?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub struct TaskCreateResult {
    pub id: String,
    pub project_id: String,
    pub description: String,
    pub success: bool,
}

impl TableDisplay for TaskCreateResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        let result_cell = if self.success {
            colored_cell!("Task Created", green)
        } else {
            colored_cell!("Failed", red)
        };

        table_row!(table, "Result", result_cell);
        table_row!(table, "Task ID", self.id);
        table_row!(table, "Project ID", self.project_id);
        table_row!(table, "Description", self.description);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["result", if self.success { "Task Created" } else { "Failed" }]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["task_id", &self.id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_id", &self.project_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["description", &self.description]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}

#[derive(Serialize)]
pub struct TaskActionResult {
    pub id: String,
    pub project_id: String,
    pub description: String,
    pub action: String,
    pub success: bool,
    pub message: String,
}

impl TableDisplay for TaskActionResult {
    fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);

        let result_text = if self.success {
            format!("{} Successful", self.action)
        } else {
            format!("{} Failed", self.action)
        };
        
        let result_cell = if self.success {
            colored_cell!(result_text, green)
        } else {
            colored_cell!(result_text, red)
        };

        table_row!(table, "Result", result_cell);
        table_row!(table, "Task ID", self.id);
        table_row!(table, "Project ID", self.project_id);
        table_row!(table, "Description", self.description);
        table_row!(table, "Action", self.action);
        table_row!(table, "Message", self.message);

        table
    }

    fn to_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> Result<(), CliError> {
        writer.write_record(["property", "value"]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["task_id", &self.id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["project_id", &self.project_id]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["description", &self.description]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["action", &self.action]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["success", &self.success.to_string()]).map_err(|e| -> CliError { e.into() })?;
        writer.write_record(["message", &self.message]).map_err(|e| -> CliError { e.into() })?;
        Ok(())
    }
}