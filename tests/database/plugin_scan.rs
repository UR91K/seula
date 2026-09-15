//! Schema recreation and plugin-scan persistence.
//!
//! These cover the three behaviours that only show up at the boundaries: discarding a
//! database written by an older schema, refusing one written by a newer one, and the
//! difference between a full scan (which may declare plugins missing) and a partial one
//! (which may not).

use std::path::PathBuf;

use rusqlite::Connection;
use seula::database::plugin_scan::PLUGIN_SCAN_COMPLETED_KEY;
use seula::database::SCHEMA_VERSION;
use seula::database::LiveSetDatabase;
use seula::models::PluginKey;
use seula::scan::plugins::{PluginScanResult, ScanReport};
use tempfile::TempDir;
use vst_meta::meta::{FormatExtra, PluginFormat, PluginMeta, Vst3Class};
use vst_meta::protocol::{ErrorType, Outcome};

use crate::common::setup;

// ---------------------------------------------------------------- fixtures

/// A VST3 uid in the form the scanner reports: 32 hex digits, uppercase.
fn vst3_uid(seed: u8) -> String {
    format!("{:02X}", seed).repeat(16)
}

fn scanned_vst3(uid: &str, name: &str, classes: Vec<Vst3Class>) -> PluginMeta {
    PluginMeta {
        path: format!("C:\\Plugins\\{}.vst3", name),
        format: PluginFormat::Vst3,
        name: name.to_string(),
        vendor: "Test Vendor".to_string(),
        version: "1.0.0".to_string(),
        uid: uid.to_string(),
        category: "Fx".to_string(),
        is_instrument: false,
        audio_in_channels: 2,
        audio_out_channels: 2,
        audio_in_buses: Some(1),
        audio_out_buses: Some(1),
        has_midi_input: false,
        has_midi_output: false,
        presets: None,
        parameters: None,
        latency_samples: None,
        has_gui: Some(true),
        vendor_url: None,
        vendor_email: None,
        is_shell: false,
        shell_parent_uid: None,
        extra: FormatExtra::Vst3 {
            factory_flags: 16,
            classes,
            buses: Vec::new(),
        },
    }
}

fn class(name: &str, class_id: &str) -> Vst3Class {
    Vst3Class {
        name: name.to_string(),
        category: "Audio Module Class".to_string(),
        class_id: class_id.to_string(),
        cardinality: 0x7FFF_FFFF,
        version: "1.0.0".to_string(),
    }
}

fn report_of(plugins: Vec<PluginMeta>) -> ScanReport {
    ScanReport {
        results: plugins
            .into_iter()
            .map(|meta| PluginScanResult {
                path: PathBuf::from(&meta.path),
                outcome: Outcome::Success {
                    plugins: vec![meta],
                },
            })
            .collect(),
        restarts: 0,
        budget_exhausted: false,
    }
}

fn temp_db() -> (TempDir, LiveSetDatabase) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seula.db");
    let db = LiveSetDatabase::new(path).expect("fresh database");
    (dir, db)
}

fn installed_state(db: &LiveSetDatabase, uid: &str) -> Option<bool> {
    db.conn
        .query_row(
            "SELECT installed FROM plugins WHERE uid = ?",
            [uid],
            |row| row.get(0),
        )
        .unwrap()
}

// ---------------------------------------------------------------- recreate path

#[test]
fn a_fresh_database_is_stamped_with_the_current_schema() {
    setup("error");
    let (_dir, db) = temp_db();

    let version: i32 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
}

#[test]
fn a_database_from_before_versioning_is_discarded() {
    setup("error");
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seula.db");

    // Stand in for a pre-versioning database: tables present, user_version still 0.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE plugins (id TEXT PRIMARY KEY, dev_identifier TEXT);
             INSERT INTO plugins (id, dev_identifier) VALUES ('old', 'device:vst3:audiofx:x');",
        )
        .unwrap();
    }

    let db = LiveSetDatabase::new(path.clone()).expect("stale database should be rebuilt");

    let survivors: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(survivors, 0, "the old contents should be gone");

    // And the rebuilt file really is the current schema, not the old one.
    let version: i32 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, SCHEMA_VERSION);
    db.conn
        .query_row("SELECT COUNT(*) FROM plugin_refs", [], |row| {
            row.get::<_, i64>(0)
        })
        .expect("plugin_refs should exist after the rebuild");
}

#[test]
fn a_database_from_a_newer_build_is_refused_not_destroyed() {
    setup("error");
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seula.db");

    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("CREATE TABLE keepsake (note TEXT); INSERT INTO keepsake VALUES ('x');")
            .unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
    }

    assert!(
        LiveSetDatabase::new(path.clone()).is_err(),
        "a newer schema must not be opened"
    );

    // The point of refusing is that the data is still there afterwards.
    let conn = Connection::open(&path).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM keepsake", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}

// ---------------------------------------------------------------- persisting

#[test]
fn a_scan_inserts_plugins_and_is_idempotent() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let uid = vst3_uid(0x11);
    let report = report_of(vec![scanned_vst3(&uid, "Test Reverb", vec![])]);

    let first = db.persist_plugin_scan(&report, true).unwrap();
    assert_eq!(first.inserted, 1);
    assert_eq!(first.updated, 0);

    let second = db.persist_plugin_scan(&report, true).unwrap();
    assert_eq!(second.inserted, 0, "rescanning must not duplicate the row");
    assert_eq!(second.updated, 1);

    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);

    let key = PluginKey::from_uid_hex(&uid).unwrap();
    assert_eq!(installed_state(&db, &key.uid_hex()), Some(true));
}

#[test]
fn scanned_uids_are_stored_normalised() {
    setup("error");
    let (_dir, mut db) = temp_db();

    // The scanner reports uppercase; Ableton writes lowercase. If the stored form
    // followed the scanner, every reference lookup would miss.
    let uid = vst3_uid(0xAB);
    db.persist_plugin_scan(&report_of(vec![scanned_vst3(&uid, "Case Test", vec![])]), true)
        .unwrap();

    let stored: String = db
        .conn
        .query_row("SELECT uid FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stored, uid.to_lowercase());
}

#[test]
fn a_full_scan_marks_what_it_did_not_find_as_missing() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let gone = vst3_uid(0x22);
    let kept = vst3_uid(0x33);

    db.persist_plugin_scan(
        &report_of(vec![
            scanned_vst3(&gone, "Uninstalled Later", vec![]),
            scanned_vst3(&kept, "Still Here", vec![]),
        ]),
        true,
    )
    .unwrap();

    // The second scan no longer sees the first plugin.
    let result = db
        .persist_plugin_scan(&report_of(vec![scanned_vst3(&kept, "Still Here", vec![])]), true)
        .unwrap();

    assert_eq!(result.marked_missing, 1);
    assert_eq!(installed_state(&db, &gone.to_lowercase()), Some(false));
    assert_eq!(installed_state(&db, &kept.to_lowercase()), Some(true));
}

#[test]
fn a_partial_scan_leaves_plugins_it_did_not_reach_alone() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let elsewhere = vst3_uid(0x44);
    let scanned = vst3_uid(0x55);

    db.persist_plugin_scan(
        &report_of(vec![scanned_vst3(&elsewhere, "In Another Folder", vec![])]),
        true,
    )
    .unwrap();

    // A scan narrowed to one directory has no grounds to call anything missing.
    let result = db
        .persist_plugin_scan(
            &report_of(vec![scanned_vst3(&scanned, "In This Folder", vec![])]),
            false,
        )
        .unwrap();

    assert_eq!(result.marked_missing, 0);
    assert_eq!(
        installed_state(&db, &elsewhere.to_lowercase()),
        Some(true),
        "a partial scan must not contradict what a full one established"
    );
}

#[test]
fn a_failed_scan_result_does_not_become_a_plugin() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let report = ScanReport {
        results: vec![PluginScanResult {
            path: PathBuf::from("C:\\Plugins\\Broken.vst3"),
            outcome: Outcome::Error {
                error_type: ErrorType::LoadFailed,
                error: "nope".to_string(),
            },
        }],
        restarts: 0,
        budget_exhausted: false,
    };

    let result = db.persist_plugin_scan(&report, true).unwrap();

    assert_eq!(result.inserted, 0);
    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn vst3_classes_are_stored_for_the_matching_fallback() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let bundle_uid = vst3_uid(0x66);
    let other_class = vst3_uid(0x77);

    db.persist_plugin_scan(
        &report_of(vec![scanned_vst3(
            &bundle_uid,
            "Multi Class Bundle",
            vec![
                class("Processor", &bundle_uid),
                class("Controller", &other_class),
            ],
        )]),
        true,
    )
    .unwrap();

    let stored: Vec<String> = {
        let mut stmt = db
            .conn
            .prepare("SELECT class_id FROM plugin_classes ORDER BY class_id")
            .unwrap();
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    };

    let mut expected = vec![bundle_uid.to_lowercase(), other_class.to_lowercase()];
    expected.sort();
    assert_eq!(stored, expected, "class ids must be normalised like uids");
}

#[test]
fn a_phantom_row_is_merged_into_the_bundle_that_owns_its_class() {
    setup("error");
    let (_dir, mut db) = temp_db();

    let bundle_uid = vst3_uid(0x88);
    let class_uid = vst3_uid(0x99);

    // A project referenced the bundle's non-primary class before anything was
    // scanned, so that reference became a row of its own.
    let phantom_id = uuid::Uuid::new_v4().to_string();
    let project_id = uuid::Uuid::new_v4().to_string();
    db.conn
        .execute_batch(&format!(
            "INSERT INTO plugins (id, plugin_kind, uid, name, format)
             VALUES ('{phantom}', 'VST3', '{class_uid}', 'Phantom', 'VST3 Effect');
             INSERT INTO plugin_refs (dev_identifier, plugin_id, resolved_via, first_seen_at)
             VALUES ('device:vst3:audiofx:{class_uid}', '{phantom}', 'created', 0);
             INSERT INTO projects (
                 id, path, name, hash, created_at, modified_at, last_parsed_at,
                 tempo, time_signature_numerator, time_signature_denominator,
                 ableton_version_major, ableton_version_minor, ableton_version_patch,
                 ableton_version_beta
             ) VALUES ('{project}', 'C:\\\\p.als', 'p', 'h', 0, 0, 0, 120.0, 4, 4, 11, 0, 0, 0);
             INSERT INTO project_plugins (project_id, plugin_id)
             VALUES ('{project}', '{phantom}');",
            phantom = phantom_id,
            class_uid = class_uid.to_lowercase(),
            project = project_id,
        ))
        .unwrap();

    // Now the bundle itself is scanned, and it exports that class.
    let result = db
        .persist_plugin_scan(
            &report_of(vec![scanned_vst3(
                &bundle_uid,
                "Real Bundle",
                vec![class("Processor", &bundle_uid), class("Extra", &class_uid)],
            )]),
            true,
        )
        .unwrap();

    assert_eq!(result.reconciled, 1);

    // The phantom is gone...
    let phantoms: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM plugins WHERE id = ?",
            [&phantom_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(phantoms, 0);

    // ...and the project still points at a plugin — the real one.
    let linked_name: String = db
        .conn
        .query_row(
            "SELECT p.name FROM plugins p
             JOIN project_plugins pp ON pp.plugin_id = p.id
             WHERE pp.project_id = ?",
            [&project_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(linked_name, "Real Bundle");

    // The reference survives too, now resolved by class.
    let via: String = db
        .conn
        .query_row(
            "SELECT resolved_via FROM plugin_refs WHERE dev_identifier = ?",
            [format!("device:vst3:audiofx:{}", class_uid.to_lowercase())],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(via, "class_id");
}

#[test]
fn scanning_fills_in_a_plugin_a_project_referenced_but_did_not_have() {
    setup("error");
    let (_dir, mut db) = temp_db();

    // The gap-filling claim, end to end: a reference exists with nothing behind it,
    // then the plugin turns up in a scan.
    let uid = vst3_uid(0xAA);
    let plugin_id = uuid::Uuid::new_v4().to_string();
    db.conn
        .execute(
            "INSERT INTO plugins (id, plugin_kind, uid, name, format)
             VALUES (?, 'VST3', ?, 'Wanted', 'VST3 Effect')",
            rusqlite::params![plugin_id, uid.to_lowercase()],
        )
        .unwrap();

    assert_eq!(
        installed_state(&db, &uid.to_lowercase()),
        None,
        "nothing has looked for it yet"
    );

    db.persist_plugin_scan(&report_of(vec![scanned_vst3(&uid, "Wanted", vec![])]), true)
        .unwrap();

    assert_eq!(installed_state(&db, &uid.to_lowercase()), Some(true));

    // Same row, not a second one.
    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);

    let id: String = db
        .conn
        .query_row("SELECT id FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(id, plugin_id, "the existing row must keep its identity");
}

// ---------------------------------------------------------------- first-run trigger

#[test]
fn a_new_database_has_never_scanned_plugins() {
    setup("error");
    let (_dir, db) = temp_db();

    assert!(!db.has_scanned_plugins().unwrap());
}

#[test]
fn a_full_scan_records_that_we_have_looked() {
    setup("error");
    let (_dir, mut db) = temp_db();

    db.persist_plugin_scan(
        &report_of(vec![scanned_vst3(&vst3_uid(0xBB), "Anything", vec![])]),
        true,
    )
    .unwrap();

    assert!(db.has_scanned_plugins().unwrap());
    assert!(db.get_app_state(PLUGIN_SCAN_COMPLETED_KEY).unwrap().is_some());
}

#[test]
fn a_scan_that_finds_nothing_still_counts_as_having_looked() {
    setup("error");
    let (_dir, mut db) = temp_db();

    // The case an "is the plugins table empty" trigger gets wrong: a machine whose
    // plugins all fail to load leaves the table empty, and would be rescanned on every
    // single project scan, forever.
    db.persist_plugin_scan(&report_of(vec![]), true).unwrap();

    let rows: i64 = db
        .conn
        .query_row("SELECT COUNT(*) FROM plugins", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0, "nothing was found");
    assert!(
        db.has_scanned_plugins().unwrap(),
        "but we have still looked, so the first-run scan must not repeat"
    );
}

#[test]
fn a_partial_scan_does_not_count_as_having_looked() {
    setup("error");
    let (_dir, mut db) = temp_db();

    // A scan narrowed with --paths, or cut short by the restart budget, leaves the
    // question open.
    db.persist_plugin_scan(
        &report_of(vec![scanned_vst3(&vst3_uid(0xCC), "Partial", vec![])]),
        false,
    )
    .unwrap();

    assert!(!db.has_scanned_plugins().unwrap());
}

#[test]
fn app_state_round_trips_and_overwrites() {
    setup("error");
    let (_dir, db) = temp_db();

    assert_eq!(db.get_app_state("some_key").unwrap(), None);

    db.set_app_state("some_key", "first").unwrap();
    assert_eq!(db.get_app_state("some_key").unwrap().as_deref(), Some("first"));

    db.set_app_state("some_key", "second").unwrap();
    assert_eq!(db.get_app_state("some_key").unwrap().as_deref(), Some("second"));
}
