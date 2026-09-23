//! Samples are filtered and counted by format, not extension: `.aif`, `.aiff` and
//! `.aifc` are all AIFF.

use seula::database::{ProjectDatabase, ProjectScope};
use seula::models::SampleFormat;
use tempfile::TempDir;

fn db_with(paths: &[&str]) -> (TempDir, ProjectDatabase) {
    let dir = TempDir::new().expect("temp dir");
    let db = ProjectDatabase::new(dir.path().join("formats.db")).expect("fresh database");
    for (i, path) in paths.iter().enumerate() {
        db.conn
            .execute(
                "INSERT INTO samples VALUES (?, ?, ?, 1)",
                rusqlite::params![format!("00000000-0000-4000-8000-{:012}", i), path, path],
            )
            .unwrap();
    }
    (dir, db)
}

fn names(db: &ProjectDatabase, format: &str) -> Vec<String> {
    let (samples, total) = db
        .get_all_samples(None, None, Some("name".into()), None, None, None, Some(format.into()), None, None, ProjectScope::Active)
        .unwrap();
    assert_eq!(total as usize, samples.len());
    samples.into_iter().map(|s| s.name).collect()
}

#[test]
fn every_aiff_extension_is_one_format() {
    let (_dir, db) = db_with(&["a.aif", "b.AIFF", "c.aifc", "d.wav", "e.rex"]);
    assert_eq!(names(&db, "aiff"), ["a.aif", "b.AIFF", "c.aifc"]);
    assert_eq!(names(&db, "aif"), ["a.aif", "b.AIFF", "c.aifc"], "an extension finds its format");
    assert_eq!(names(&db, "other"), ["e.rex"]);
    assert!(names(&db, "nonsense").is_empty(), "an unknown value matches nothing");

    let buckets = db.get_sample_extensions().unwrap();
    assert_eq!(buckets["aiff"].count, 3);
    assert_eq!(buckets["wav"].count, 1);
    assert_eq!(buckets["other"].count, 1);
}

#[test]
fn search_filters_by_format_too() {
    let (_dir, db) = db_with(&["kick.aif", "kick.wav"]);
    let (hits, _) = db.search_samples("kick", None, None, None, Some("aiff".into())).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].name, "kick.aif");
    assert_eq!(SampleFormat::find(".AIFC").map(|f| f.id), Some("aiff"));
}
