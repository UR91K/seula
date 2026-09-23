//! The plugins view's data: one plugin's scan record, and counts under a filter.

use seula::database::plugins::InstallState;
use seula::database::{PluginFilter, ProjectDatabase};
use tempfile::TempDir;

fn temp_db() -> (TempDir, ProjectDatabase) {
    let dir = TempDir::new().expect("temp dir");
    let db = ProjectDatabase::new(dir.path().join("plugins.db")).expect("fresh database");
    (dir, db)
}

/// `installed`: Some(true) found, Some(false) looked and absent, None never scanned.
fn add_plugin(db: &ProjectDatabase, id: &str, name: &str, vendor: &str, format: &str, installed: Option<bool>) {
    let kind = if format.starts_with("VST3") { "VST3" } else { "VST2" };
    let found = installed == Some(true);
    db.conn
        .execute(
            "INSERT INTO plugins (id, plugin_kind, uid, name, format, vendor, installed,
                last_scanned_at, path, presets)
             VALUES (?1, ?2, ?1, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                id,
                kind,
                name,
                format,
                vendor,
                installed,
                installed.map(|_| 1_700_000_000_i64),
                found.then(|| format!("C:\\VST3\\{}.vst3", name)),
                found.then_some(12),
            ],
        )
        .expect("insert plugin");
}

#[test]
fn details_carry_the_scan_record_classes_buses_and_references() {
    let (_dir, db) = temp_db();
    add_plugin(&db, "a", "Pro-Q 3", "FabFilter", "VST3 Effect", Some(true));
    db.conn
        .execute_batch(
            "INSERT INTO plugin_classes VALUES ('a', 'Pro-Q 3', 'Audio Module Class', 'c1', 1, '3.21');
             INSERT INTO plugin_buses VALUES ('a', 'output', 'audio', 'Main Out', 2, 0, 1);
             INSERT INTO plugin_buses VALUES ('a', 'input', 'audio', 'Main In', 2, 0, 1);
             INSERT INTO plugin_refs VALUES ('device:vst3:audiofx:x', 'a', 'Pro-Q 3', 'audiofx', 'uid', 5);",
        )
        .unwrap();

    let details = db.get_plugin_details("a").unwrap().expect("plugin exists");
    assert_eq!(details.plugin_kind, "VST3");
    assert_eq!(details.path.as_deref(), Some("C:\\VST3\\Pro-Q 3.vst3"));
    assert_eq!(details.presets, Some(12));
    assert_eq!(details.last_scanned_at, Some(1_700_000_000));
    assert_eq!(details.classes.len(), 1);
    let directions: Vec<&str> = details.buses.iter().map(|b| b.direction.as_str()).collect();
    assert_eq!(directions, ["input", "output"], "inputs first");
    assert_eq!(details.references[0].ableton_name.as_deref(), Some("Pro-Q 3"));
}

#[test]
fn a_never_scanned_plugin_has_no_scanner_data_and_an_unknown_one_is_none() {
    let (_dir, db) = temp_db();
    add_plugin(&db, "u", "Arcade", "Output", "VST3 Instrument", None);

    let details = db.get_plugin_details("u").unwrap().expect("plugin exists");
    assert!(details.path.is_none());
    assert!(details.last_scanned_at.is_none());
    assert!(details.classes.is_empty() && details.buses.is_empty());

    assert!(db.get_plugin_details("nope").unwrap().is_none());
}

#[test]
fn stats_count_only_what_the_filter_selects() {
    let (_dir, db) = temp_db();
    add_plugin(&db, "1", "Pro-Q 3", "FabFilter", "VST3 Effect", Some(true));
    add_plugin(&db, "2", "Pro-L 2", "FabFilter", "VST3 Effect", Some(false));
    add_plugin(&db, "3", "Serum", "Xfer Records", "VST2 Instrument", Some(true));
    add_plugin(&db, "4", "Arcade", "Output", "VST3 Instrument", None);

    let all = db.get_plugin_stats().unwrap();
    assert_eq!((all.total_plugins, all.installed_plugins, all.missing_plugins, all.unknown_plugins), (4, 2, 1, 1));
    assert_eq!(all.unique_vendors, 3);

    let fabfilter = db
        .get_plugin_stats_filtered(&PluginFilter { vendor: Some("FabFilter".into()), ..Default::default() })
        .unwrap();
    assert_eq!((fabfilter.total_plugins, fabfilter.installed_plugins, fabfilter.missing_plugins), (2, 1, 1));
    assert_eq!(fabfilter.unique_vendors, 1);

    let not_loadable = db
        .get_plugin_stats_filtered(&PluginFilter {
            install_states: vec![InstallState::Absent, InstallState::Unscanned],
            ..Default::default()
        })
        .unwrap();
    assert_eq!((not_loadable.total_plugins, not_loadable.installed_plugins), (2, 0));

    let searched = db
        .get_plugin_stats_filtered(&PluginFilter {
            query: Some("pro".into()),
            format: Some("VST3 Effect".into()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(searched.total_plugins, 2);
    assert_eq!(searched.plugins_by_format.get("VST3 Effect"), Some(&2));
}
