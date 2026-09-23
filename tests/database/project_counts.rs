//! Per-row project counts for plugins and samples (ADR-0034), and the scope they
//! count in (ADR-0040).

use seula::database::{ProjectDatabase, ProjectScope};
use tempfile::TempDir;

fn temp_db() -> (TempDir, ProjectDatabase) {
    let dir = TempDir::new().expect("temp dir");
    let db = ProjectDatabase::new(dir.path().join("counts.db")).expect("fresh database");
    (dir, db)
}

fn add_project(db: &ProjectDatabase, id: &str) {
    db.conn
        .execute(
            "INSERT INTO projects (id, path, name, hash, created_at, modified_at, last_parsed_at,
                tempo, time_signature_numerator, time_signature_denominator,
                daw_type, daw_version_display)
             VALUES (?1, ?1, ?1, '', 0, 0, 0, 120, 4, 4, 'Ableton Live', '12.0.0')",
            [id],
        )
        .expect("insert project");
}

#[test]
fn sample_counts_are_projects_using_each_sample_with_unused_as_zero() {
    let (_dir, db) = temp_db();
    for p in ["p1", "p2", "p3"] {
        add_project(&db, p);
    }
    for s in ["kick", "snare", "unused"] {
        db.conn
            .execute("INSERT INTO samples (id, name, path, is_present) VALUES (?1, ?1, ?1, 1)", [s])
            .unwrap();
    }
    for (p, s) in [("p1", "kick"), ("p2", "kick"), ("p3", "kick"), ("p1", "snare")] {
        db.conn
            .execute("INSERT INTO project_samples (project_id, sample_id) VALUES (?1, ?2)", [p, s])
            .unwrap();
    }

    let ids: Vec<String> = ["kick", "snare", "unused"].iter().map(|s| s.to_string()).collect();
    let counts = db.sample_project_counts(&ids, ProjectScope::Active).unwrap();

    assert_eq!(counts["kick"], 3);
    assert_eq!(counts["snare"], 1);
    assert_eq!(counts["unused"], 0, "an unused sample is reported, as zero");
}

/// Archived projects count only under `All`, and the counts agree with the used-in
/// lists and the list's own counting in either scope (ADR-0040).
#[test]
fn archived_projects_count_only_in_the_all_scope() {
    let (_dir, db) = temp_db();
    // Project reads parse ids as UUIDs, so these are real ones.
    let kick = "00000000-0000-4000-8000-00000000000a".to_string();
    let (live1, live2, gone) = (
        "00000000-0000-4000-8000-000000000001",
        "00000000-0000-4000-8000-000000000002",
        "00000000-0000-4000-8000-000000000003",
    );
    for p in [live1, live2, gone] {
        add_project(&db, p);
        db.conn
            .execute("INSERT INTO project_ableton_metadata VALUES (?1, 12, 0, 0, 0)", [p])
            .unwrap();
    }
    db.conn
        .execute("INSERT INTO samples VALUES (?1, 'kick', 'kick.wav', 1)", [kick.as_str()])
        .unwrap();
    for p in [live1, live2, gone] {
        db.conn
            .execute("INSERT INTO project_samples VALUES (?1, ?2)", [p, kick.as_str()])
            .unwrap();
    }
    db.conn.execute("UPDATE projects SET is_active = 0 WHERE id = ?1", [gone]).unwrap();

    for (scope, expected) in [(ProjectScope::Active, 2), (ProjectScope::All, 3)] {
        let counts = db.sample_project_counts(&[kick.clone()], scope).unwrap();
        assert_eq!(counts[&kick], expected, "{:?}", scope);

        let (projects, total) = db.get_projects_by_sample_id(&kick, None, None, scope).unwrap();
        assert_eq!((projects.len(), total), (expected as usize, expected), "{:?}", scope);

        // The list filters on the same count: at least 3 projects matches only under All.
        let (listed, _) = db
            .get_all_samples(None, None, None, None, None, None, None, Some(3), None, scope)
            .unwrap();
        assert_eq!(listed.len(), usize::from(expected == 3), "{:?}", scope);
    }
}

#[test]
fn counts_span_more_ids_than_one_query_chunk() {
    let (_dir, db) = temp_db();
    add_project(&db, "p1");
    let ids: Vec<String> = (0..1200).map(|i| format!("s{i}")).collect();
    for id in &ids {
        db.conn
            .execute("INSERT INTO samples (id, name, path, is_present) VALUES (?1, ?1, ?1, 1)", [id])
            .unwrap();
        db.conn
            .execute("INSERT INTO project_samples (project_id, sample_id) VALUES ('p1', ?1)", [id])
            .unwrap();
    }

    let counts = db.sample_project_counts(&ids, ProjectScope::Active).unwrap();
    assert_eq!(counts.len(), 1200);
    assert!(counts.values().all(|&c| c == 1));
}
