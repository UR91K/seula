use crate::cli::commands::CliContext;
use crate::cli::output::{OutputFormatter, MessageType};
use crate::cli::CliError;
use crate::database::LiveSetDatabase;
use crate::error::LiveSetError;
use crate::live_set::LiveSet;
use crate::process_projects_with_progress;
use crate::scan::parallel::ParallelParser;
use crate::scan::project_scanner::ProjectPathScanner;
use comfy_table::Table;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as TokioMutex;

pub struct ScanCommand {
    pub paths: Vec<PathBuf>,
    pub force: bool,
}

#[async_trait::async_trait]
impl crate::cli::commands::CliCommand for ScanCommand {
    async fn execute(&self, ctx: &CliContext) -> Result<(), CliError> {
        let formatter = OutputFormatter::new(ctx.output_format.clone(), ctx.no_color);
        // If no explicit paths provided, use the shared scanning logic (same as gRPC)
        if self.paths.is_empty() {
            formatter.print_message(
                "Starting scan using configured paths",
                MessageType::Info,
            );

            let progress_callback = move |completed: u32, total: u32, progress: f32, message: String, phase: &str| {
                let phase_label = match phase {
                    "starting" => "Starting",
                    "discovering" => "Discovering",
                    "preprocessing" => "Preprocessing",
                    "parsing" => "Parsing",
                    "inserting" => "Saving",
                    "completed" => "Completed",
                    _ => phase,
                };

                // Simple progress output suitable for CLI
                if total > 0 {
                    println!(
                        "[{}] {:.1}% - {} ({}/{})",
                        phase_label,
                        progress * 100.0,
                        message,
                        completed,
                        total
                    );
                } else {
                    println!("[{}] {}", phase_label, message);
                }
            };

            process_projects_with_progress(Some(progress_callback))
                .map_err(|e| -> CliError { Box::new(e) })?;

            formatter.print_message("\nScan Complete", MessageType::Success);
            return Ok(());
        }

        // Legacy path-specific scanning flow (when explicit paths are provided)
        formatter.print_message(
            &format!("Starting scan of {} path(s)", self.paths.len()),
            MessageType::Info,
        );

        // Discover project files
        let project_paths = self.discover_project_files().await?;
        if project_paths.is_empty() {
            formatter.print_message("No .als files found in specified paths", MessageType::Warning);
            return Ok(());
        }

        formatter.print_message(
            &format!("Found {} project file(s) to process", project_paths.len()),
            MessageType::Info,
        );

        // Filter out existing projects if not forcing
        let paths_to_process = if self.force {
            project_paths
        } else {
            self.filter_existing_projects(&ctx.db, project_paths).await?
        };

        if paths_to_process.is_empty() {
            formatter.print_message(
                "All projects are already scanned (use --force to rescan)",
                MessageType::Info,
            );
            return Ok(());
        }

        formatter.print_message(
            &format!("Processing {} project(s)...", paths_to_process.len()),
            MessageType::Info,
        );

        // Process projects in parallel
        let results = self.process_projects_parallel(paths_to_process).await?;

        // Store results in database
        let (success_count, error_count) = self.store_results(&ctx.db, results).await?;

        // Display results
        self.display_scan_results(&formatter, success_count, error_count);

        Ok(())
    }
}

impl ScanCommand {
    async fn discover_project_files(&self) -> Result<Vec<PathBuf>, CliError> {
        let scanner = ProjectPathScanner::new().map_err(|e| CliError::from(e))?;
        let mut all_paths = HashSet::new();

        for path in &self.paths {
            if path.is_file() {
                // If it's a file, check if it's a .als file
                if let Some(ext) = path.extension() {
                    if ext == "als" {
                        all_paths.insert(path.clone());
                    }
                }
            } else if path.is_dir() {
                // If it's a directory, scan for .als files
                let paths = scanner.scan_directory(path)?;
                all_paths.extend(paths);
            }
        }

        Ok(all_paths.into_iter().collect())
    }

    async fn filter_existing_projects(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        project_paths: Vec<PathBuf>,
    ) -> Result<Vec<PathBuf>, CliError> {
        let mut db_guard = db.lock().await;
        let mut filtered_paths = Vec::new();

        for path in project_paths {
            let path_str = path.to_string_lossy().to_string();
            if let Ok(existing_project) = db_guard.get_project_by_path(&path_str) {
                if existing_project.is_none() {
                    filtered_paths.push(path);
                }
            } else {
                // If we can't check, include it in the scan
                filtered_paths.push(path);
            }
        }

        Ok(filtered_paths)
    }

    async fn process_projects_parallel(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Vec<Result<(PathBuf, LiveSet), (PathBuf, LiveSetError)>>, CliError> {
        let total = paths.len();
        let num_threads = (total / 2).max(1).min(4);
        let parser = ParallelParser::new(num_threads);

        // Submit all paths for processing before receiving
        parser.submit_paths(paths)?;

        // Collect exactly `total` results to avoid blocking forever
        let results_rx = parser.get_results_receiver();
        let mut results = Vec::with_capacity(total);
        while results.len() < total {
            let result = results_rx
                .recv()
                .map_err(|e| -> CliError { Box::new(e) })?;
            results.push(result);
            if results.len() % 10 == 0 || results.len() == total {
                println!("Processed {} / {} projects...", results.len(), total);
            }
        }

        Ok(results)
    }

    async fn store_results(
        &self,
        db: &Arc<TokioMutex<LiveSetDatabase>>,
        results: Vec<Result<(PathBuf, LiveSet), (PathBuf, LiveSetError)>>,
    ) -> Result<(usize, usize), CliError> {
        let mut db_guard = db.lock().await;
        let mut success_count = 0;
        let mut error_count = 0;

        for result in results {
            match result {
                Ok((path, live_set)) => {
                    match db_guard.insert_project(&live_set) {
                        Ok(_) => {
                            success_count += 1;
                            println!("✓ Stored: {}", path.display());
                        }
                        Err(e) => {
                            error_count += 1;
                            eprintln!("✗ Failed to store {}: {}", path.display(), e);
                        }
                    }
                }
                Err((path, err)) => {
                    error_count += 1;
                    eprintln!("✗ Failed to parse {}: {}", path.display(), err);
                }
            }
        }

        Ok((success_count, error_count))
    }

    fn display_scan_results(&self, formatter: &OutputFormatter, success_count: usize, error_count: usize) {
        let total_processed = success_count + error_count;

        let mut table = Table::new();
        table
            .set_header(vec!["Scan Results", "Count"])
            .load_preset(comfy_table::presets::UTF8_FULL);

        table.add_row(vec![
            "Projects Processed".to_string(),
            total_processed.to_string(),
        ]);

        table.add_row(vec![
            "Successfully Stored".to_string(),
            success_count.to_string(),
        ]);

        if error_count > 0 {
            table.add_row(vec![
                "Errors".to_string(),
                error_count.to_string(),
            ]);
        }

        formatter.print_message("\nScan Complete", MessageType::Success);
        println!("{}", table);

        if success_count > 0 {
            formatter.print_message(
                &format!("Successfully scanned and stored {} project(s)", success_count),
                MessageType::Success,
            );
        }

        if error_count > 0 {
            formatter.print_message(
                &format!("Encountered {} error(s) during scanning", error_count),
                MessageType::Warning,
            );
        }
    }
}
