use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

use crate::config::CONFIG;
use crate::database::batch::BatchInsertManager;
use crate::database::plugins::PluginRefreshResult;
use crate::database::samples::SampleRefreshResult;
use crate::database::ProjectDatabase;
use crate::scan::sample_check::{check_sample_files, default_threads};
use crate::error::DatabaseError;
use crate::grpc::common::{ScanStatus, WatcherEventType};
use crate::grpc::scanning::ScanProgressResponse;
use crate::grpc::system::GetStatisticsResponse;
use crate::grpc::watcher::WatcherEventResponse;
use crate::grpc::handlers::utils::convert_live_set_to_proto;
use crate::process_projects_with_progress;
use crate::project::Project;
use crate::watcher::file_watcher::{FileEvent, FileWatcher};

/// Owns the state shared across the scanning/watcher/statistics RPCs: scan
/// status/progress, the active file watcher (if any), and process start time.
/// Reuses the gRPC-generated `ScanStatus`/`ScanProgressResponse`/`WatcherEventResponse`
/// types directly rather than introducing domain equivalents -- CLI has no
/// consumer of this state today (`system watch`/`scan-status` are stubs), so
/// there's nothing yet motivating that split; revisit if axum needs one.
#[derive(Clone)]
pub struct SystemService {
    db: Arc<Mutex<ProjectDatabase>>,
    scan_status: Arc<Mutex<ScanStatus>>,
    scan_progress: Arc<Mutex<Option<ScanProgressResponse>>>,
    watcher: Arc<Mutex<Option<FileWatcher>>>,
    watcher_events: Arc<Mutex<Option<std::sync::mpsc::Receiver<FileEvent>>>>,
    start_time: Instant,
}

impl SystemService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<Mutex<ProjectDatabase>>,
        scan_status: Arc<Mutex<ScanStatus>>,
        scan_progress: Arc<Mutex<Option<ScanProgressResponse>>>,
        watcher: Arc<Mutex<Option<FileWatcher>>>,
        watcher_events: Arc<Mutex<Option<std::sync::mpsc::Receiver<FileEvent>>>>,
        start_time: Instant,
    ) -> Self {
        Self {
            db,
            scan_status,
            scan_progress,
            watcher,
            watcher_events,
            start_time,
        }
    }

    pub fn db_handle(&self) -> Arc<Mutex<ProjectDatabase>> {
        Arc::clone(&self.db)
    }

    /// Escape hatch for tests that need to simulate scan progress without
    /// running an actual scan.
    pub fn scan_status_handle(&self) -> Arc<Mutex<ScanStatus>> {
        Arc::clone(&self.scan_status)
    }

    pub fn scan_progress_handle(&self) -> Arc<Mutex<Option<ScanProgressResponse>>> {
        Arc::clone(&self.scan_progress)
    }

    pub async fn get_system_info(&self) -> Result<(String, Vec<String>, bool, i64), DatabaseError> {
        let config = CONFIG
            .as_ref()
            .map_err(|e| DatabaseError::ConfigError(e.clone()))?;
        let watcher_active = self.watcher.lock().await.is_some();
        let uptime_seconds = self.start_time.elapsed().as_secs() as i64;

        Ok((
            env!("CARGO_PKG_VERSION").to_string(),
            config.paths.clone(),
            watcher_active,
            uptime_seconds,
        ))
    }

    pub async fn get_scan_status(&self) -> (ScanStatus, Option<ScanProgressResponse>) {
        let status = *self.scan_status.lock().await;
        let progress = self.scan_progress.lock().await.clone();
        (status, progress)
    }

    /// Claims the scan status for a new scan, or returns false when one is already
    /// running. Project scans, plugin scans and sample checks share the status, so one
    /// runs at a time (ADR-0038, ADR-0041).
    async fn begin_scan(&self, status: ScanStatus) -> bool {
        let mut current = self.scan_status.lock().await;
        let running = matches!(
            *current,
            ScanStatus::ScanStarting
                | ScanStatus::ScanScanningPlugins
                | ScanStatus::ScanCheckingSamples
                | ScanStatus::ScanDiscovering
                | ScanStatus::ScanParsing
                | ScanStatus::ScanInserting
        );
        if running {
            return false;
        }
        *current = status;
        *self.scan_progress.lock().await = None;
        true
    }

    /// Resets scan status/progress and starts the scan in the background,
    /// invoking `on_progress` with every update (including the final
    /// success/error one) so the caller can forward it to its own transport.
    /// Returns false, and starts nothing, when a scan is already running.
    pub async fn start_scan<F>(&self, on_progress: F) -> bool
    where
        F: Fn(ScanProgressResponse) + Send + Sync + 'static,
    {
        if !self.begin_scan(ScanStatus::ScanStarting).await {
            return false;
        }

        let scan_status = Arc::clone(&self.scan_status);
        let scan_progress = Arc::clone(&self.scan_progress);
        let scan_status_for_callback = Arc::clone(&scan_status);
        let scan_progress_for_callback = Arc::clone(&scan_progress);
        let on_progress = Arc::new(on_progress);
        let on_progress_for_callback = Arc::clone(&on_progress);

        tokio::spawn(async move {
            let progress_callback =
                move |completed: u32, total: u32, progress: f32, message: String, phase: &str| {
                    let status = match phase {
                        "starting" => ScanStatus::ScanStarting,
                        "scanning_plugins" => ScanStatus::ScanScanningPlugins,
                        "discovering" => ScanStatus::ScanDiscovering,
                        "preprocessing" | "parsing" => ScanStatus::ScanParsing,
                        "inserting" => ScanStatus::ScanInserting,
                        "completed" => ScanStatus::ScanCompleted,
                        _ => ScanStatus::ScanStarting,
                    };

                    let response = ScanProgressResponse {
                        completed,
                        total,
                        progress,
                        message,
                        status: status as i32,
                    };

                    let scan_status_clone = Arc::clone(&scan_status_for_callback);
                    let scan_progress_clone = Arc::clone(&scan_progress_for_callback);
                    let response_clone = response.clone();
                    tokio::spawn(async move {
                        *scan_status_clone.lock().await = status;
                        *scan_progress_clone.lock().await = Some(response_clone);
                    });

                    on_progress_for_callback(response);
                };

            match process_projects_with_progress(Some(progress_callback)) {
                Ok(()) => {
                    let final_status = ScanStatus::ScanCompleted;
                    let final_progress = ScanProgressResponse {
                        completed: 100,
                        total: 100,
                        progress: 1.0,
                        message: "Scan completed successfully".to_string(),
                        status: final_status as i32,
                    };
                    *scan_status.lock().await = final_status;
                    *scan_progress.lock().await = Some(final_progress.clone());
                    on_progress(final_progress);
                }
                Err(e) => {
                    let error_status = ScanStatus::ScanError;
                    let error_progress = ScanProgressResponse {
                        completed: 0,
                        total: 1,
                        progress: 0.0,
                        message: format!("Scan failed: {}", e),
                        status: error_status as i32,
                    };
                    *scan_status.lock().await = error_status;
                    *scan_progress.lock().await = Some(error_progress.clone());
                    on_progress(error_progress);
                }
            }
        });
        true
    }

    /// Rescans the system's plugins in the background (ADR-0038), reporting through
    /// the same status and progress as a project scan: `scanning_plugins` with one
    /// update per plugin binary, then `completed` or `error`. The final update also
    /// carries what the scan changed.
    ///
    /// The scan runs on a blocking thread without the database, which is locked only to
    /// write the result. Returns false, and starts nothing, when a scan is already
    /// running.
    pub async fn start_plugin_scan<F>(&self, on_progress: F) -> bool
    where
        F: Fn(ScanProgressResponse, Option<PluginRefreshResult>) + Send + Sync + 'static,
    {
        if !self.begin_scan(ScanStatus::ScanScanningPlugins).await {
            return false;
        }

        let db = Arc::clone(&self.db);
        let scan_status = Arc::clone(&self.scan_status);
        let scan_progress = Arc::clone(&self.scan_progress);

        tokio::task::spawn_blocking(move || {
            // blocking_lock is right here: this is a blocking thread, not a task.
            let report = |completed: u32, total: u32, message: String, status: ScanStatus| {
                let response = ScanProgressResponse {
                    completed,
                    total,
                    progress: if total == 0 { 0.0 } else { completed as f32 / total as f32 },
                    message,
                    status: status as i32,
                };
                *scan_status.blocking_lock() = status;
                *scan_progress.blocking_lock() = Some(response.clone());
                response
            };

            on_progress(
                report(0, 0, "Finding plugins...".to_string(), ScanStatus::ScanScanningPlugins),
                None,
            );

            let mut on_plugin = |index: usize, total: usize, path: &Path| {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("plugin");
                on_progress(
                    report(
                        index as u32,
                        total as u32,
                        // Counted like the project scan's messages; this one is
                        // being loaded, not yet done.
                        format!("Scanning {} ({}/{})", name, index + 1, total),
                        ScanStatus::ScanScanningPlugins,
                    ),
                    None,
                );
            };
            let outcome = crate::services::plugins::scan_configured_plugins(&mut on_plugin)
                .and_then(|scan| db.blocking_lock().record_plugin_refresh(&scan));

            match outcome {
                Ok(result) => {
                    let message = format!(
                        "Plugin scan completed: {} installed, {} missing, {} failed to load",
                        result.plugins_installed, result.plugins_missing, result.scan_failures
                    );
                    let total = result.candidates_scanned.max(0) as u32;
                    on_progress(report(total, total, message, ScanStatus::ScanCompleted), Some(result));
                }
                Err(e) => {
                    let message = format!("Plugin scan failed: {}", e);
                    on_progress(report(0, 1, message, ScanStatus::ScanError), None);
                }
            }
        });
        true
    }

    /// Checks every sample file in the background (ADR-0041): whether it is still
    /// there, and its size. Reports like the other scans, as `checking_samples` with
    /// progress by folder, then `completed` (carrying what changed) or `error`.
    ///
    /// The database is locked only to read the paths and to write the result. Returns
    /// false, and starts nothing, when a scan is already running.
    pub async fn start_sample_check<F>(&self, on_progress: F) -> bool
    where
        F: Fn(ScanProgressResponse, Option<SampleRefreshResult>) + Send + Sync + 'static,
    {
        if !self.begin_scan(ScanStatus::ScanCheckingSamples).await {
            return false;
        }

        let db = Arc::clone(&self.db);
        let scan_status = Arc::clone(&self.scan_status);
        let scan_progress = Arc::clone(&self.scan_progress);

        tokio::task::spawn_blocking(move || {
            // blocking_lock is right here: this is a blocking thread, not a task.
            let report = |completed: u32, total: u32, message: String, status: ScanStatus| {
                let response = ScanProgressResponse {
                    completed,
                    total,
                    progress: if total == 0 { 0.0 } else { completed as f32 / total as f32 },
                    message,
                    status: status as i32,
                };
                *scan_status.blocking_lock() = status;
                *scan_progress.blocking_lock() = Some(response.clone());
                response
            };

            // Its own statement: a guard taken inside the `and_then` chain would live to
            // the end of it, holding the database through the whole check, and then
            // deadlock on the lock below (the trap ADR-0002 records).
            let paths = db.blocking_lock().sample_paths();
            let outcome = paths.and_then(|paths| {
                on_progress(
                    report(0, 0, format!("Checking {} samples...", paths.len()), ScanStatus::ScanCheckingSamples),
                    None,
                );
                // One event per folder would be thousands on a big library; about 200
                // is plenty for a progress bar.
                let mut on_folder = |done: usize, total: usize, folder: &Path| {
                    if done != total && done % (total / 200).max(1) != 0 {
                        return;
                    }
                    let name = folder.file_name().and_then(|n| n.to_str()).unwrap_or("folder");
                    on_progress(
                        report(
                            done as u32,
                            total as u32,
                            format!("Checked {} ({}/{})", name, done, total),
                            ScanStatus::ScanCheckingSamples,
                        ),
                        None,
                    );
                };
                let found = check_sample_files(&paths, default_threads(), &mut on_folder);
                db.blocking_lock().record_sample_check(&found)
            });

            match outcome {
                Ok(result) => {
                    let message = format!(
                        "Sample check completed: {} checked, {} now missing, {} found again",
                        result.total_samples_checked, result.samples_now_missing, result.samples_now_present
                    );
                    let total = result.total_samples_checked.max(0) as u32;
                    on_progress(report(total, total, message, ScanStatus::ScanCompleted), Some(result));
                }
                Err(e) => {
                    let message = format!("Sample check failed: {}", e);
                    on_progress(report(0, 1, message, ScanStatus::ScanError), None);
                }
            }
        });
        true
    }

    /// Validates, parses, and inserts a single project via BatchInsertManager,
    /// returning the freshly-inserted domain Project. Error strings match the
    /// exact wording the gRPC response contract already commits to.
    pub async fn add_single_project(&self, file_path: &Path) -> Result<Project, String> {
        if !file_path.exists() {
            return Err("File does not exist".to_string());
        }
        if !file_path.extension().map_or(false, |ext| ext == "als") {
            return Err("File must have .als extension".to_string());
        }

        let live_set =
            Project::new(file_path.to_path_buf()).map_err(|e| format!("Failed to parse project: {}", e))?;
        let project_id = live_set.id.to_string();

        let mut db = self.db.lock().await;
        let mut batch_manager = BatchInsertManager::new(&mut db.conn, Arc::new(vec![live_set]));
        batch_manager
            .execute()
            .map_err(|e| format!("Database error: {}", e))?;

        db.get_project_by_id(&project_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| "Project inserted but not found".to_string())
    }

    /// Same shape as `add_single_project`, batched: every path is validated and
    /// parsed independently (a bad file doesn't block the others), then every
    /// successfully-parsed project is inserted in one BatchInsertManager
    /// transaction. Returns (path, project) pairs for what succeeded end to end,
    /// and (path, error message) pairs for everything that didn't.
    pub async fn add_multiple_projects(
        &self,
        file_paths: Vec<String>,
    ) -> (Vec<(String, Project)>, Vec<(String, String)>) {
        let mut failures = Vec::new();
        let mut parsed: Vec<(String, Project)> = Vec::new();

        for file_path_str in file_paths {
            let file_path = PathBuf::from(&file_path_str);

            if !file_path.exists() {
                failures.push((file_path_str, "File does not exist".to_string()));
                continue;
            }
            if !file_path.extension().map_or(false, |ext| ext == "als") {
                failures.push((file_path_str, "File must have .als extension".to_string()));
                continue;
            }

            match Project::new(file_path.clone()) {
                Ok(live_set) => parsed.push((file_path_str, live_set)),
                Err(e) => failures.push((file_path_str, format!("Failed to parse project: {}", e))),
            }
        }

        let mut successes = Vec::new();
        if !parsed.is_empty() {
            let path_ids: Vec<(String, String)> = parsed
                .iter()
                .map(|(path, p)| (path.clone(), p.id.to_string()))
                .collect();
            let projects_to_insert: Vec<Project> = parsed.into_iter().map(|(_, p)| p).collect();

            let mut db = self.db.lock().await;
            let mut batch_manager = BatchInsertManager::new(&mut db.conn, Arc::new(projects_to_insert));
            match batch_manager.execute() {
                Ok(_) => {
                    for (path, project_id) in path_ids {
                        match db.get_project_by_id(&project_id) {
                            Ok(Some(inserted)) => successes.push((path, inserted)),
                            Ok(None) => failures.push((path, "Project inserted but not found".to_string())),
                            Err(e) => failures.push((path, format!("Database error: {}", e))),
                        }
                    }
                }
                Err(e) => {
                    for (path, _) in path_ids {
                        failures.push((path, format!("Database error: {}", e)));
                    }
                }
            }
        }

        (successes, failures)
    }

    /// Starts the file watcher if one isn't already active. Returns `Ok(true)`
    /// if a watcher is active on return (whether newly started or already was).
    pub async fn start_watcher(&self) -> Result<bool, String> {
        let mut watcher_guard = self.watcher.lock().await;
        let mut events_guard = self.watcher_events.lock().await;

        if watcher_guard.is_some() {
            return Ok(true);
        }

        let (mut watcher, event_receiver) =
            FileWatcher::new(Arc::clone(&self.db)).map_err(|e| e.to_string())?;

        let config = CONFIG.as_ref().map_err(|e| format!("Config error: {}", e))?;
        for path in &config.paths {
            if let Err(e) = watcher.add_watch_path(PathBuf::from(path)) {
                tracing::warn!("Failed to add watch path {}: {}", path, e);
            }
        }

        *watcher_guard = Some(watcher);
        *events_guard = Some(event_receiver);
        Ok(true)
    }

    pub async fn stop_watcher(&self) {
        let mut watcher_guard = self.watcher.lock().await;
        let mut events_guard = self.watcher_events.lock().await;
        *watcher_guard = None;
        *events_guard = None;
    }

    /// Takes ownership of the active watcher's event receiver (if any) and
    /// starts forwarding events in the background: `FileEvent::Deleted` marks
    /// the matching project deleted in the database, then every event (after
    /// its DB side effect, if any) is handed to `on_event`. Returns `false` if
    /// no watcher is active -- there is nothing to stream.
    pub async fn start_watcher_event_stream<F>(&self, on_event: F) -> bool
    where
        F: Fn(WatcherEventResponse) + Send + Sync + 'static,
    {
        let mut events_guard = self.watcher_events.lock().await;
        let Some(event_receiver) = events_guard.take() else {
            return false;
        };
        drop(events_guard);

        let db_clone = Arc::clone(&self.db);
        tokio::spawn(async move {
            while let Ok(file_event) = event_receiver.recv() {
                if let FileEvent::Deleted(path) = &file_event {
                    let mut db = db_clone.lock().await;
                    match db.get_project_by_path(&path.to_string_lossy()) {
                        Ok(Some(project)) => {
                            if let Err(e) = db.mark_project_deleted(&project.id) {
                                tracing::warn!("Failed to mark project as deleted: {}", e);
                            }
                        }
                        Ok(None) => {}
                        Err(e) => tracing::warn!("Error looking up project for deleted file: {}", e),
                    }
                }

                let watcher_event = match file_event {
                    FileEvent::Created(path) => WatcherEventResponse {
                        event_type: WatcherEventType::WatcherCreated as i32,
                        path: path.to_string_lossy().to_string(),
                        new_path: None,
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                    FileEvent::Modified(path) => WatcherEventResponse {
                        event_type: WatcherEventType::WatcherModified as i32,
                        path: path.to_string_lossy().to_string(),
                        new_path: None,
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                    FileEvent::Deleted(path) => WatcherEventResponse {
                        event_type: WatcherEventType::WatcherDeleted as i32,
                        path: path.to_string_lossy().to_string(),
                        new_path: None,
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                    FileEvent::Renamed { from, to } => WatcherEventResponse {
                        event_type: WatcherEventType::WatcherRenamed as i32,
                        path: from.to_string_lossy().to_string(),
                        new_path: Some(to.to_string_lossy().to_string()),
                        timestamp: chrono::Utc::now().timestamp(),
                    },
                };

                on_event(watcher_event);
            }
        });

        true
    }

    /// Gathers every statistic and returns the assembled proto response
    /// directly -- this data is proto-shaped by nature (it exists to answer
    /// `GetStatistics`) and has no other consumer today.
    pub async fn get_statistics(&self) -> Result<GetStatisticsResponse, DatabaseError> {
        use crate::grpc::system::*;

        let mut db = self.db.lock().await;

        let (total_projects, total_plugins, total_samples, total_collections, total_tags, total_tasks) =
            db.get_basic_counts()?;

        let top_plugins = db
            .get_top_plugins(10)?
            .into_iter()
            .map(|(name, vendor, count)| PluginStatistic {
                name,
                vendor,
                usage_count: count,
            })
            .collect();

        let top_vendors = db
            .get_top_vendors(10)?
            .into_iter()
            .map(|(vendor, plugin_count, usage_count)| VendorStatistic {
                vendor,
                plugin_count,
                usage_count,
            })
            .collect();

        let tempo_distribution = db
            .get_tempo_distribution()?
            .into_iter()
            .map(|(tempo, count)| TempoStatistic { tempo, count })
            .collect();

        let key_distribution = db
            .get_key_distribution()?
            .into_iter()
            .map(|(key, count)| KeyStatistic { key, count })
            .collect();

        let time_signature_distribution = db
            .get_time_signature_distribution()?
            .into_iter()
            .map(|(numerator, denominator, count)| TimeSignatureStatistic {
                numerator,
                denominator,
                count,
            })
            .collect();

        let projects_per_year = db
            .get_projects_per_year()?
            .into_iter()
            .map(|(year, count)| YearStatistic { year, count })
            .collect();

        let projects_per_month: Vec<MonthStatistic> = db
            .get_projects_per_month(12)?
            .into_iter()
            .map(|(year, month, count)| MonthStatistic { year, month, count })
            .collect();

        let total_months = projects_per_month.len() as f64;
        let average_monthly_projects = if total_months > 0.0 {
            projects_per_month.iter().map(|m| m.count as f64).sum::<f64>() / total_months
        } else {
            0.0
        };

        let (average_project_duration_seconds, projects_under_40_seconds, longest_project_id) =
            db.get_duration_analytics()?;

        let longest_project = if let Some(project_id) = longest_project_id {
            match db.get_project_by_id(&project_id) {
                Ok(Some(project)) => convert_live_set_to_proto(project, &mut db).ok(),
                _ => None,
            }
        } else {
            None
        };

        let (average_plugins_per_project, average_samples_per_project) = db.get_complexity_metrics()?;

        let most_complex_projects_raw = db.get_most_complex_projects(5)?;
        let mut most_complex_projects = Vec::new();
        for (project_id, plugin_count, sample_count, complexity_score) in most_complex_projects_raw {
            if let Ok(Some(project)) = db.get_project_by_id(&project_id) {
                if let Ok(proto_project) = convert_live_set_to_proto(project, &mut db) {
                    most_complex_projects.push(ProjectComplexityStatistic {
                        project: Some(proto_project),
                        plugin_count,
                        sample_count,
                        complexity_score,
                    });
                }
            }
        }

        let top_samples = db
            .get_top_samples(10)?
            .into_iter()
            .map(|(name, path, usage_count)| SampleStatistic {
                name,
                path,
                usage_count,
            })
            .collect();

        let top_tags = db
            .get_top_tags(10)?
            .into_iter()
            .map(|(name, usage_count)| TagStatistic { name, usage_count })
            .collect();

        let (completed_tasks, pending_tasks, task_completion_rate) = db.get_task_statistics()?;

        let recent_activity = db
            .get_recent_activity(30)?
            .into_iter()
            .map(|(year, month, day, projects_created, projects_modified)| ActivityTrendStatistic {
                year,
                month,
                day,
                projects_created,
                projects_modified,
            })
            .collect();

        let ableton_versions = db
            .get_ableton_version_stats()?
            .into_iter()
            .map(|(version, count)| VersionStatistic { version, count })
            .collect();

        let (average_projects_per_collection, largest_collection_id) = db.get_collection_analytics()?;

        let largest_collection = if let Some(collection_id) = largest_collection_id {
            match db.get_collection_by_id(&collection_id, crate::database::ProjectScope::All) {
                Ok(Some((id, name, description, notes, created_at, modified_at, project_ids, cover_art_id))) => {
                    let (total_duration_seconds, project_count) =
                        db.get_collection_statistics(&id, crate::database::ProjectScope::All).unwrap_or((None, 0));
                    Some(crate::grpc::common::Collection {
                        id,
                        name,
                        description,
                        notes,
                        created_at,
                        modified_at,
                        project_ids,
                        cover_art_id,
                        total_duration_seconds,
                        project_count,
                    })
                }
                _ => None,
            }
        } else {
            None
        };

        let task_completion_trends = db
            .get_task_completion_trends(12)?
            .into_iter()
            .map(
                |(year, month, completed_tasks, total_tasks, completion_rate)| TaskCompletionTrendStatistic {
                    year,
                    month,
                    completed_tasks,
                    total_tasks,
                    completion_rate,
                },
            )
            .collect();

        Ok(GetStatisticsResponse {
            total_projects,
            total_plugins,
            total_samples,
            total_collections,
            total_tags,
            total_tasks,
            top_plugins,
            top_vendors,
            tempo_distribution,
            key_distribution,
            time_signature_distribution,
            projects_per_year,
            projects_per_month,
            average_monthly_projects,
            average_project_duration_seconds,
            projects_under_40_seconds,
            longest_project,
            most_complex_projects,
            average_plugins_per_project,
            average_samples_per_project,
            top_samples,
            top_tags,
            completed_tasks,
            pending_tasks,
            task_completion_rate,
            recent_activity,
            ableton_versions,
            average_projects_per_collection,
            largest_collection,
            task_completion_trends,
        })
    }
}
