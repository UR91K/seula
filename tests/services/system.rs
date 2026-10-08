//! `SystemService` tests: importing projects, statistics and the scan state. Ported from
//! `tests/grpc/scanning.rs`, `stats.rs` and `server_setup.rs` (ADR-0046).
//!

use super::{create_test_project_in_db, test_env};
use seula::database::ProjectScope;
use seula::services::{ScanProgress, ScanStatus};

// Importing projects.

#[tokio::test]
async fn adding_no_projects_adds_none() {
    let env = test_env();

    let (added, failed) = env.system.add_multiple_projects(vec![]).await;

    assert!(added.is_empty());
    assert!(failed.is_empty());
}

#[tokio::test]
async fn adding_invalid_paths_reports_why_each_failed() {
    let env = test_env();
    let wrong_extension =
        std::env::temp_dir().join(format!("seula_test_{}.txt", uuid::Uuid::new_v4()));
    std::fs::write(&wrong_extension, b"dummy content").expect("Failed to create temp file");
    let wrong_extension_path = wrong_extension.to_string_lossy().to_string();

    let (added, failed) = env
        .system
        .add_multiple_projects(vec![
            "nonexistent_file.als".to_string(),
            wrong_extension_path.clone(),
        ])
        .await;
    let _ = std::fs::remove_file(&wrong_extension);

    assert!(added.is_empty());
    assert_eq!(failed.len(), 2);
    let reason = |path: &str| {
        failed
            .iter()
            .find(|(p, _)| p == path)
            .unwrap_or_else(|| panic!("{path} should have failed"))
            .1
            .as_str()
    };
    assert_eq!(reason("nonexistent_file.als"), "File does not exist");
    assert_eq!(
        reason(&wrong_extension_path),
        "File must have .als extension"
    );
}

/// Both the missing files fail; none succeeds, so nothing is inserted.
#[tokio::test]
async fn adding_only_missing_files_inserts_nothing() {
    let env = test_env();

    let (added, failed) = env
        .system
        .add_multiple_projects(vec![
            "nonexistent_file.als".to_string(),
            "another_nonexistent_file.als".to_string(),
        ])
        .await;

    assert!(added.is_empty());
    assert_eq!(failed.len(), 2);
    assert!(failed.iter().all(|(_, why)| why == "File does not exist"));
    assert!(env
        .services
        .projects
        .list_projects(
            Default::default(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap()
        .0
        .is_empty());
}

// Statistics.

#[tokio::test]
async fn statistics_count_what_is_in_the_library() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;

    let stats = env
        .system
        .get_statistics(ProjectScope::Active)
        .await
        .expect("statistics should not fail");

    assert!(stats.total_projects >= 1);
}

#[tokio::test]
async fn statistics_of_an_empty_library_are_zero() {
    let env = test_env();

    let stats = env
        .system
        .get_statistics(ProjectScope::Active)
        .await
        .expect("statistics should not fail on an empty database");

    assert_eq!(stats.total_projects, 0);
    assert_eq!(stats.total_plugins, 0);
    assert_eq!(stats.total_samples, 0);
    assert_eq!(stats.total_collections, 0);
    assert_eq!(stats.total_tags, 0);
    assert_eq!(stats.total_tasks, 0);
}

/// The pieces `get_statistics` assembles, each on its own, so a failure names the
/// query that broke.
#[tokio::test]
async fn each_statistic_query_succeeds() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;
    let today = chrono::Utc::now().date_naive();
    let scope = ProjectScope::Active;

    {
        let db = env.db.lock().await;
        db.get_projects_per_year(scope)
            .expect("get_projects_per_year should not fail");
        db.get_projects_per_month(12, today, scope)
            .expect("get_projects_per_month should not fail");
        db.get_recent_activity(30, today, scope)
            .expect("get_recent_activity should not fail");
    }
    env.db
        .lock()
        .await
        .get_task_completion_trends(12, today, scope)
        .expect("get_task_completion_trends should not fail");
}

// Scan state.

#[tokio::test]
async fn scan_status_starts_unknown_and_follows_the_shared_state() {
    let env = test_env();

    let (status, progress) = env.system.get_scan_status().await;
    assert_eq!(status, ScanStatus::Unknown);
    assert!(progress.is_none());

    *env.system.scan_progress_handle().lock().await = Some(ScanProgress {
        completed: 50,
        total: 100,
        progress: 0.5,
        message: "Test progress".to_string(),
        status: ScanStatus::Parsing,
    });
    *env.system.scan_status_handle().lock().await = ScanStatus::Parsing;

    let (status, progress) = env.system.get_scan_status().await;
    assert_eq!(status, ScanStatus::Parsing);
    let progress = progress.expect("progress is reported");
    assert_eq!(progress.completed, 50);
    assert_eq!(progress.total, 100);
    assert_eq!(progress.progress, 0.5);
    assert_eq!(progress.message, "Test progress");
}

/// The background sample check finishes, releasing the database while it looks at
/// files. It once held the lock through the whole check and then deadlocked taking
/// it again to write the result (ADR-0041).
///
/// A hand-built runtime, shut down with a timeout: a deadlocked blocking thread
/// would otherwise hang the test forever instead of failing it.
#[test]
fn the_background_sample_check_finishes() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let outcome = rt.block_on(async {
        let env = test_env();
        let system = env.system.clone();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        assert!(
            system
                .start_sample_check(move |event, result| {
                    let _ = tx.send((event, result));
                })
                .await
        );

        let last = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let mut last = None;
            while let Some(event) = rx.recv().await {
                last = Some(event);
            }
            last
        })
        .await;
        (last, system)
    });
    // Before any assertion: a failing one would drop the runtime, which waits.
    rt.shutdown_timeout(std::time::Duration::from_secs(1));
    let (last, system) = outcome;
    let last = last.expect("the check finishes").expect("it reports");
    assert_eq!(last.0.status, ScanStatus::Completed, "{}", last.0.message);
    assert!(last.1.is_some(), "the final event carries the result");
    assert_eq!(
        *system.scan_status_handle().blocking_lock(),
        ScanStatus::Completed
    );
}

/// A project scan, a plugin scan and a sample check share the status; none starts
/// while another runs (ADR-0038).
#[tokio::test]
async fn no_scan_starts_while_one_is_running() {
    let env = test_env();
    let system = &env.system;
    *system.scan_status_handle().lock().await = ScanStatus::Parsing;

    assert!(
        !system
            .start_plugin_scan(seula::scan::plugins::ScanMode::Changes, |_, _| {})
            .await
    );
    assert!(!system.start_sample_check(|_, _| {}).await);
    assert!(!system.start_scan(|_| {}).await);
    assert_eq!(
        *system.scan_status_handle().lock().await,
        ScanStatus::Parsing,
        "a refused start leaves the status alone"
    );

    *system.scan_status_handle().lock().await = ScanStatus::ScanningPlugins;
    assert!(!system.start_scan(|_| {}).await);

    *system.scan_status_handle().lock().await = ScanStatus::CheckingSamples;
    assert!(
        !system
            .start_plugin_scan(seula::scan::plugins::ScanMode::Changes, |_, _| {})
            .await
    );
}
