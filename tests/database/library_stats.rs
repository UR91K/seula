//! The library statistics behind the stats view (ADR-0045): every figure takes the
//! project scope, and the series and histograms say where there is nothing.

use chrono::NaiveDate;
use seula::database::{ProjectDatabase, ProjectScope};
use tempfile::TempDir;

fn temp_db() -> (TempDir, ProjectDatabase) {
    let dir = TempDir::new().expect("temp dir");
    let db = ProjectDatabase::new(dir.path().join("stats.db")).expect("fresh database");
    (dir, db)
}

fn epoch(y: i32, m: u32, d: u32) -> i64 {
    NaiveDate::from_ymd_opt(y, m, d)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp()
}

fn add_project(db: &ProjectDatabase, id: &str, tempo: f64, created_at: i64) {
    db.conn
        .execute(
            "INSERT INTO projects (id, path, name, hash, created_at, modified_at, last_parsed_at,
                tempo, time_signature_numerator, time_signature_denominator,
                daw_type, daw_version_display)
             VALUES (?1, ?1, ?1, '', ?2, ?2, 0, ?3, 4, 4, 'Ableton Live', '12.0.0')",
            rusqlite::params![id, created_at, tempo],
        )
        .expect("insert project");
}

fn archive(db: &ProjectDatabase, id: &str) {
    db.conn.execute("UPDATE projects SET is_active = 0 WHERE id = ?1", [id]).unwrap();
}

/// Regression: every tempo under 90 went into one bin labelled 80, and empty bins were
/// left out, so a chart closed the gap.
#[test]
fn tempo_bins_are_equal_width_with_empty_bins_as_zero() {
    let (_dir, db) = temp_db();
    for (id, tempo) in [("a", 62.0), ("b", 70.0), ("c", 89.9), ("d", 124.0)] {
        add_project(&db, id, tempo, 0);
    }

    let bins = db.get_tempo_distribution(ProjectScope::Active).unwrap();

    assert_eq!(
        bins,
        vec![(60.0, 1), (70.0, 1), (80.0, 1), (90.0, 0), (100.0, 0), (110.0, 0), (120.0, 1)]
    );
}

/// Regression: "the last 12 months" was the last 12 months with any project, so a
/// quiet month vanished and the window stretched further back.
#[test]
fn projects_per_month_is_twelve_calendar_months_with_quiet_months_as_zero() {
    let (_dir, db) = temp_db();
    let today = NaiveDate::from_ymd_opt(2026, 9, 24).unwrap();
    add_project(&db, "old", 120.0, epoch(2025, 9, 30)); // one month before the window
    add_project(&db, "oct", 120.0, epoch(2025, 10, 1));
    add_project(&db, "jun1", 120.0, epoch(2026, 6, 3));
    add_project(&db, "jun2", 120.0, epoch(2026, 6, 20));

    let months = db.get_projects_per_month(12, today, ProjectScope::Active).unwrap();

    assert_eq!(months.len(), 12);
    assert_eq!(months.first(), Some(&(2025, 10, 1)));
    assert_eq!(months.last(), Some(&(2026, 9, 0)));
    assert_eq!(months.iter().find(|m| (m.0, m.1) == (2026, 5)), Some(&(2026, 5, 0)));
    assert_eq!(months.iter().find(|m| (m.0, m.1) == (2026, 6)), Some(&(2026, 6, 2)));
}

#[test]
fn projects_per_year_fills_the_years_between() {
    let (_dir, db) = temp_db();
    add_project(&db, "a", 120.0, epoch(2021, 5, 1));
    add_project(&db, "b", 120.0, epoch(2024, 5, 1));

    let years = db.get_projects_per_year(ProjectScope::Active).unwrap();

    assert_eq!(years, vec![(2021, 1), (2022, 0), (2023, 0), (2024, 1)]);
}

#[test]
fn recent_activity_has_every_day_oldest_first() {
    let (_dir, db) = temp_db();
    let today = NaiveDate::from_ymd_opt(2026, 9, 24).unwrap();
    add_project(&db, "a", 120.0, epoch(2026, 9, 22));

    let days = db.get_recent_activity(7, today, ProjectScope::Active).unwrap();

    assert_eq!(days.len(), 7);
    assert_eq!(days[0], (2026, 9, 18, 0, 0));
    assert_eq!(days[4], (2026, 9, 22, 1, 1));
    assert_eq!(days[6], (2026, 9, 24, 0, 0));
}

/// Regression: a project with no key was counted under a key named "Unknown".
#[test]
fn projects_with_no_key_have_no_key() {
    let (_dir, db) = temp_db();
    add_project(&db, "keyed", 120.0, 0);
    add_project(&db, "unkeyed", 120.0, 0);
    db.conn
        .execute(
            "UPDATE projects SET key_signature_tonic = 'DSharp', key_signature_scale = 'Minor' WHERE id = 'keyed'",
            [],
        )
        .unwrap();

    let mut keys = db.get_key_distribution(ProjectScope::Active).unwrap();
    keys.sort();

    assert_eq!(keys, vec![(None, 1), (Some("DSharp Minor".to_string()), 1)]);
}

/// A library of three projects, one archived, where each figure differs by scope.
fn scoped_library(db: &ProjectDatabase) {
    for p in ["live", "bare", "gone"] {
        add_project(db, p, 120.0, 0);
    }
    archive(db, "gone");
    for (id, installed) in [("shared", "1"), ("absent", "0"), ("unscanned", "NULL"), ("archived_only", "1")] {
        db.conn
            .execute(
                &format!(
                    "INSERT INTO plugins (id, plugin_kind, uid, name, format, installed)
                     VALUES (?1, 'VST3', ?1, ?1, 'Vst3Instrument', {installed})"
                ),
                [id],
            )
            .unwrap();
    }
    for (p, plugin) in [
        ("live", "shared"),
        ("live", "absent"),
        ("live", "unscanned"),
        ("gone", "shared"),
        ("gone", "archived_only"),
    ] {
        db.conn
            .execute("INSERT INTO project_plugins (project_id, plugin_id) VALUES (?1, ?2)", [p, plugin])
            .unwrap();
    }
    for (id, present) in [("kick", 1), ("lost", 0)] {
        db.conn
            .execute("INSERT INTO samples (id, name, path, is_present) VALUES (?1, ?1, ?1, ?2)", rusqlite::params![id, present])
            .unwrap();
    }
    db.conn.execute("INSERT INTO project_samples VALUES ('live', 'kick')", []).unwrap();
    db.conn.execute("INSERT INTO project_samples VALUES ('gone', 'lost')", []).unwrap();
    for (id, name) in [("t1", "techno"), ("t2", "old"), ("t3", "never")] {
        db.conn
            .execute("INSERT INTO tags (id, name, created_at) VALUES (?1, ?2, 0)", [id, name])
            .unwrap();
    }
    db.conn.execute("INSERT INTO project_tags (project_id, tag_id, created_at) VALUES ('live', 't1', 0)", []).unwrap();
    db.conn.execute("INSERT INTO project_tags (project_id, tag_id, created_at) VALUES ('gone', 't2', 0)", []).unwrap();
    for (id, name) in [("c1", "live one"), ("c2", "archive one"), ("c3", "empty")] {
        db.conn
            .execute(
                "INSERT INTO collections (id, name, created_at, modified_at) VALUES (?1, ?2, 0, 0)",
                [id, name],
            )
            .unwrap();
    }
    db.conn.execute("INSERT INTO collection_projects (collection_id, project_id, position, added_at) VALUES ('c1', 'live', 0, 0)", []).unwrap();
    db.conn.execute("INSERT INTO collection_projects (collection_id, project_id, position, added_at) VALUES ('c2', 'gone', 0, 0)", []).unwrap();
    for (id, project, done) in [("k1", "live", 1), ("k2", "live", 0), ("k3", "gone", 1)] {
        db.conn
            .execute(
                "INSERT INTO project_tasks (id, project_id, description, completed, created_at) VALUES (?1, ?2, ?1, ?3, 0)",
                rusqlite::params![id, project, done],
            )
            .unwrap();
    }
}

/// Regression: the overview mixed scopes. Projects left archived ones out, while
/// plugins, samples and tasks counted every row, whoever used it.
#[test]
fn every_count_takes_the_scope_and_splits_into_its_states() {
    let (_dir, db) = temp_db();
    scoped_library(&db);

    let active = db.get_library_counts(ProjectScope::Active).unwrap();
    assert_eq!((active.projects_active, active.projects_archived), (2, 1));
    assert_eq!(
        (active.plugins_installed, active.plugins_missing, active.plugins_not_scanned),
        (1, 1, 1),
        "archived_only is used by no active project"
    );
    assert_eq!((active.samples_present, active.samples_missing), (1, 0));
    assert_eq!((active.tags_in_use, active.tags_unused), (1, 2));
    assert_eq!((active.collections_with_projects, active.collections_empty), (1, 2));
    assert_eq!((active.tasks_completed, active.tasks_pending), (1, 1));

    let all = db.get_library_counts(ProjectScope::All).unwrap();
    assert_eq!((all.projects_active, all.projects_archived), (2, 1), "the project split ignores scope");
    assert_eq!((all.plugins_installed, all.plugins_missing, all.plugins_not_scanned), (2, 1, 1));
    assert_eq!((all.samples_present, all.samples_missing), (1, 1));
    assert_eq!((all.tags_in_use, all.tags_unused), (2, 1));
    assert_eq!((all.collections_with_projects, all.collections_empty), (2, 1));
    assert_eq!((all.tasks_completed, all.tasks_pending), (2, 1));
}

/// Regression: the averages left out projects with no plugins or samples, and the
/// collection average counted archived projects under any scope.
#[test]
fn averages_divide_by_everything_in_scope_including_zeros() {
    let (_dir, db) = temp_db();
    scoped_library(&db);

    // Active: live has 3 plugins and 1 sample, bare has none.
    let (plugins, samples) = db.get_complexity_metrics(ProjectScope::Active).unwrap();
    assert_eq!((plugins, samples), (1.5, 0.5));

    // All: 5 plugin uses and 2 sample uses over 3 projects.
    let (plugins, samples) = db.get_complexity_metrics(ProjectScope::All).unwrap();
    assert_eq!((plugins, samples), (5.0 / 3.0, 2.0 / 3.0));

    // One active membership over three collections, the empty one included.
    let (per_collection, largest) = db.get_collection_analytics(ProjectScope::Active).unwrap();
    assert_eq!(per_collection, 1.0 / 3.0);
    assert_eq!(largest.as_deref(), Some("c1"));
    let (per_collection, _) = db.get_collection_analytics(ProjectScope::All).unwrap();
    assert_eq!(per_collection, 2.0 / 3.0);
}

/// Regression: the overall rate was a percentage while the monthly rates were
/// fractions, and the CSV export multiplied the percentage by 100 again.
#[test]
fn completion_rates_are_fractions() {
    let (_dir, mut db) = temp_db();
    scoped_library(&db);

    let (_, _, rate) = db.get_task_statistics(ProjectScope::Active).unwrap();
    assert_eq!(rate, 0.5);
    let (_, _, rate) = db.get_task_statistics(ProjectScope::All).unwrap();
    assert_eq!(rate, 2.0 / 3.0);

    // Moved into the window: the active tasks are one done and one pending.
    let today = NaiveDate::from_ymd_opt(2026, 9, 24).unwrap();
    db.conn
        .execute("UPDATE project_tasks SET created_at = ?1", [epoch(2026, 9, 1)])
        .unwrap();
    let trends = db.get_task_completion_trends(12, today, ProjectScope::Active).unwrap();
    assert_eq!(trends.len(), 12);
    assert_eq!(trends[0], (2025, 10, 0, 0, 0.0));
    assert_eq!(trends[11], (2026, 9, 1, 2, 0.5));
}
