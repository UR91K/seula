//! A fresh in-memory database with the services built over it, and the helpers that
//! put rows in it. Shared by the service tests and the HTTP tests.

use seula::database::ProjectDatabase;
use seula::media::{MediaConfig, MediaStorageManager};
use seula::services::{ScanStatus, Services, SystemService};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{setup, LiveSetBuilder};

/// A fresh in-memory database with the services built over it.
pub struct TestEnv {
    pub db: Arc<Mutex<ProjectDatabase>>,
    pub media_storage: Arc<MediaStorageManager>,
    pub services: Services,
    /// Built apart from `services`, as `SystemService` is not yet a member of it
    /// (ADR-0046 folds it in).
    pub system: SystemService,
}

pub fn test_env() -> TestEnv {
    setup("error");

    let db =
        ProjectDatabase::new(PathBuf::from(":memory:")).expect("Failed to create test database");
    let db = Arc::new(Mutex::new(db));

    let media_dir = std::env::temp_dir().join(format!("seula_test_media_{}", uuid::Uuid::new_v4()));
    let media_storage = MediaStorageManager::new(media_dir, MediaConfig::default())
        .expect("Failed to create test media storage");

    let media_storage = Arc::new(media_storage);
    let services = Services::new(Arc::clone(&db), Arc::clone(&media_storage));
    let system = SystemService::new(
        Arc::clone(&db),
        Arc::new(Mutex::new(ScanStatus::Unknown)),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
        Arc::new(Mutex::new(None)),
        std::time::Instant::now(),
    );
    TestEnv {
        db,
        media_storage,
        services,
        system,
    }
}

/// Inserts a bare project row (no plugins, samples or tags) with the given name and
/// path; returns its id.
pub async fn create_test_project(env: &TestEnv, name: &str, path: &str) -> String {
    let project_id = uuid::Uuid::new_v4().to_string();

    let db = env.db.lock().await;
    db.conn
        .execute(
            "INSERT INTO projects (
            id, name, path, hash, created_at, modified_at, last_parsed_at,
            tempo, time_signature_numerator, time_signature_denominator,
            daw_type, daw_version_display
        ) VALUES (?, ?, ?, ?, datetime('now'), datetime('now'), datetime('now'), ?, ?, ?, ?, ?)",
            rusqlite::params![
                project_id,
                name,
                path,
                format!("test_hash_{}", project_id),
                120.0,
                4,
                4,
                "Ableton Live",
                "11.0.0"
            ],
        )
        .expect("Failed to insert test project");
    db.conn.execute(
        "INSERT INTO project_ableton_metadata (project_id, version_major, version_minor, version_patch, version_beta) VALUES (?, ?, ?, ?, ?)",
        rusqlite::params![project_id, 11, 0, 0, false],
    ).expect("Failed to insert test project ableton metadata");

    project_id
}

/// Inserts a project with a plugin, a sample and a key signature; returns its id.
pub async fn create_test_project_in_db(db: &Arc<Mutex<ProjectDatabase>>) -> String {
    let test_project = LiveSetBuilder::new()
        .with_plugin("Serum")
        .with_sample("kick.wav")
        .with_tempo(140.0)
        .with_time_signature(4, 4)
        .with_key_signature(seula::models::KeySignature {
            tonic: seula::models::Tonic::C,
            scale: seula::models::Scale::Major,
        })
        .with_version(11, 0, 0, false)
        .build();

    let unique_id = uuid::Uuid::new_v4();
    let unique_name = format!("Test Project {}.als", unique_id);

    let test_live_set = seula::project::Project {
        is_active: true,
        id: unique_id,
        file_path: PathBuf::from(&unique_name),
        name: unique_name.clone(),
        file_hash: format!("test_hash_{}", unique_id),
        created_time: chrono::Local::now(),
        modified_time: chrono::Local::now(),
        last_parsed_timestamp: chrono::Local::now(),
        tempo: test_project.tempo,
        time_signature: test_project.time_signature,
        key_signature: test_project.key_signature,
        furthest_bar: test_project.furthest_bar,
        estimated_duration: None,
        daw_type: "Ableton Live".to_string(),
        daw_version_display: test_project.version.to_string(),
        ableton_metadata: test_project.version,
        plugins: test_project.plugins,
        samples: test_project.samples,
        tags: std::collections::HashSet::new(),
    };

    let project_id = test_live_set.id.to_string();
    let mut db_guard = db.lock().await;
    db_guard
        .insert_project(&test_live_set)
        .expect("Failed to insert test project");

    project_id
}

/// Inserts a sample row; returns its id.
pub async fn create_test_sample(env: &TestEnv, name: &str, path: &str, is_present: bool) -> String {
    let sample_id = uuid::Uuid::new_v4().to_string();
    env.db
        .lock()
        .await
        .conn
        .execute(
            "INSERT INTO samples (id, name, path, is_present) VALUES (?, ?, ?, ?)",
            rusqlite::params![sample_id, name, path, is_present],
        )
        .expect("Failed to insert test sample");
    sample_id
}

/// Links a sample to a project (a usage).
pub async fn add_sample_to_project(env: &TestEnv, project_id: &str, sample_id: &str) {
    env.db
        .lock()
        .await
        .conn
        .execute(
            "INSERT INTO project_samples (project_id, sample_id) VALUES (?, ?)",
            rusqlite::params![project_id, sample_id],
        )
        .expect("Failed to link sample to project");
}

/// Lists projects in a deletion scope, with no other filter set.
pub async fn list_projects(
    env: &TestEnv,
    scope: seula::services::DeletionScope,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Vec<seula::project::Project> {
    env.services
        .projects
        .list_projects(
            scope, limit, offset, None, None, None, None, None, None, None, None, None, None, None,
            None, None, None, None, None,
        )
        .await
        .unwrap()
        .0
}
