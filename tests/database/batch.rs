//! Database batch insert tests

use super::*;
use crate::common::{generate_test_live_sets_arc, setup};
use std::collections::HashSet;
use seula::database::batch::BatchInsertManager;
use tempfile::tempdir;

#[test]
fn test_batch_insert() {
    setup("error");
    // Create a temporary database
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");

    // Initialize database with schema from ProjectDatabase
    let mut live_set_db = ProjectDatabase::new(db_path.clone()).expect("Failed to create database");

    // Get connection for batch insert
    let mut conn = &mut live_set_db.conn;

    // Generate test data
    let test_sets = generate_test_live_sets_arc(3);
    let expected_projects = test_sets.len();
    let expected_plugins: usize = test_sets
        .iter()
        .flat_map(|ls| &ls.plugins)
        .map(|p| &p.dev_identifier)
        .collect::<HashSet<_>>()
        .len();
    let expected_samples: usize = test_sets
        .iter()
        .flat_map(|ls| &ls.samples)
        .map(|s| s.path.to_string_lossy().to_string())
        .collect::<HashSet<_>>()
        .len();

    // Execute batch insert
    let mut batch_manager = BatchInsertManager::new(&mut conn, test_sets.clone());
    let stats = batch_manager.execute().expect("Batch insert failed");

    // Verify stats
    assert_eq!(
        stats.projects_inserted, expected_projects,
        "Should insert all projects"
    );
    assert_eq!(
        stats.plugins_inserted, expected_plugins,
        "Should insert unique plugins"
    );
    assert_eq!(
        stats.samples_inserted, expected_samples,
        "Should insert unique samples"
    );

    // Verify database contents
    let project_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .expect("Failed to count projects");
    assert_eq!(project_count as usize, expected_projects);

    let plugin_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .expect("Failed to count plugins");
    assert_eq!(plugin_count as usize, expected_plugins);

    let sample_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM samples", [], |row| row.get(0))
        .expect("Failed to count samples");
    assert_eq!(sample_count as usize, expected_samples);

    // Verify relationships
    for live_set in test_sets.iter() {
        let project_id = live_set.id.to_string();

        // Check plugins
        let plugin_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_plugins WHERE project_id = ?",
                [&project_id],
                |row| row.get(0),
            )
            .expect("Failed to count plugin links");
        assert_eq!(plugin_links as usize, live_set.plugins.len());

        // Check samples
        let sample_links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_samples WHERE project_id = ?",
                [&project_id],
                |row| row.get(0),
            )
            .expect("Failed to count sample links");
        assert_eq!(sample_links as usize, live_set.samples.len());
    }
}

/// Presence from a rescan wins. It used to be ORed with what was stored, so once a
/// sample had been seen, no rescan could ever mark it missing.
#[test]
fn a_rescan_that_finds_a_sample_gone_marks_it_missing() {
    use crate::common::{create_test_live_set_from_parse, LiveSetBuilder};

    setup("error");
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let mut db = ProjectDatabase::new(temp_dir.path().join("test.db")).expect("database");

    let scan = |db: &mut ProjectDatabase, project: &str, samples: &[(&str, bool)]| {
        let mut parse = LiveSetBuilder::new().build();
        for (name, present) in samples {
            parse.samples.insert(seula::models::Sample {
                id: uuid::Uuid::new_v4(),
                name: name.to_string(),
                path: PathBuf::from(format!("C:/samples/{}", name)),
                is_present: *present,
            });
        }
        let project = create_test_live_set_from_parse(project, parse);
        BatchInsertManager::new(&mut db.conn, std::sync::Arc::new(vec![project]))
            .execute()
            .expect("batch");
    };
    let present = |db: &ProjectDatabase, name: &str| -> bool {
        db.conn
            .query_row("SELECT is_present FROM samples WHERE name = ?", [name], |r| r.get(0))
            .expect("sample row")
    };

    scan(&mut db, "a.als", &[("kick.wav", true), ("snare.wav", true)]);
    scan(&mut db, "b.als", &[("hat.wav", true)]);
    assert!(present(&db, "kick.wav"));

    // a.als rescanned after kick.wav was deleted from disk
    scan(&mut db, "a.als", &[("kick.wav", false), ("snare.wav", true)]);
    assert!(!present(&db, "kick.wav"), "the rescan's answer replaces the stored one");
    assert!(present(&db, "snare.wav"));
    assert!(present(&db, "hat.wav"), "a sample this scan did not see is left alone");

    // Within one scan, a sample any project finds is present.
    scan(&mut db, "a.als", &[("kick.wav", true)]);
    assert!(present(&db, "kick.wav"));
}
