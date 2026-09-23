//! A project's list of audition audios, and its primary (ADR-0037).

use seula::database::ProjectDatabase;
use seula::media::{MediaFile, MediaType};
use tempfile::TempDir;
use uuid::Uuid;

fn temp_db() -> (TempDir, ProjectDatabase) {
    let dir = TempDir::new().expect("temp dir");
    let db = ProjectDatabase::new(dir.path().join("audio.db")).expect("fresh database");
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

fn add_audio(db: &mut ProjectDatabase, name: &str) -> String {
    let media = MediaFile {
        id: Uuid::new_v4().to_string(),
        original_filename: name.to_string(),
        file_extension: "wav".to_string(),
        media_type: MediaType::AudioFile,
        file_size_bytes: 1000,
        mime_type: "audio/wav".to_string(),
        uploaded_at: chrono::Utc::now(),
        checksum: "x".to_string(),
    };
    db.insert_media_file(&media).expect("insert media");
    media.id
}

/// (filename, primary) in list order.
fn listed(db: &ProjectDatabase, project: &str) -> Vec<(String, bool)> {
    db.get_project_audio_files(project)
        .unwrap()
        .into_iter()
        .map(|(m, primary)| (m.original_filename, primary))
        .collect()
}

#[test]
fn setting_a_primary_lists_it_and_adding_does_not_change_the_primary() {
    let (_dir, mut db) = temp_db();
    add_project(&db, "p");
    let rough = add_audio(&mut db, "rough.wav");
    let master = add_audio(&mut db, "master.wav");

    db.update_project_audio_file("p", Some(&rough)).unwrap();
    db.add_project_audio_file("p", &master).unwrap();
    db.add_project_audio_file("p", &master).unwrap(); // already listed: no-op

    assert_eq!(listed(&db, "p"), vec![("rough.wav".into(), true), ("master.wav".into(), false)]);
}

#[test]
fn removing_the_primary_promotes_the_next_and_the_last_leaves_none() {
    let (_dir, mut db) = temp_db();
    add_project(&db, "p");
    let a = add_audio(&mut db, "a.wav");
    let b = add_audio(&mut db, "b.wav");
    db.update_project_audio_file("p", Some(&a)).unwrap();
    db.add_project_audio_file("p", &b).unwrap();

    assert!(db.remove_project_audio_file_from_list("p", &a).unwrap());
    assert_eq!(listed(&db, "p"), vec![("b.wav".into(), true)]);
    assert_eq!(db.get_project_audio_file("p").unwrap().map(|m| m.id), Some(b.clone()));

    assert!(db.remove_project_audio_file_from_list("p", &b).unwrap());
    assert!(listed(&db, "p").is_empty());
    assert!(db.get_project_audio_file("p").unwrap().is_none());

    assert!(!db.remove_project_audio_file_from_list("p", &b).unwrap(), "not listed any more");
}

#[test]
fn removing_a_non_primary_leaves_the_primary_alone() {
    let (_dir, mut db) = temp_db();
    add_project(&db, "p");
    let a = add_audio(&mut db, "a.wav");
    let b = add_audio(&mut db, "b.wav");
    db.update_project_audio_file("p", Some(&a)).unwrap();
    db.add_project_audio_file("p", &b).unwrap();

    db.remove_project_audio_file_from_list("p", &b).unwrap();
    assert_eq!(listed(&db, "p"), vec![("a.wav".into(), true)]);
}

#[test]
fn a_listed_audio_that_is_not_primary_is_not_an_orphan() {
    let (_dir, mut db) = temp_db();
    add_project(&db, "p");
    let a = add_audio(&mut db, "a.wav");
    let b = add_audio(&mut db, "b.wav");
    let loose = add_audio(&mut db, "loose.wav");
    db.update_project_audio_file("p", Some(&a)).unwrap();
    db.add_project_audio_file("p", &b).unwrap();

    let orphans: Vec<String> = db.get_orphaned_media_files(None, None).unwrap().into_iter().map(|m| m.id).collect();
    assert_eq!(orphans, vec![loose]);
}

/// A database written before the list existed has its one audio in
/// projects.audio_file_id only. Opening it lists that audio as the primary.
#[test]
fn an_existing_single_audio_is_listed_as_primary_on_open() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.db");
    let a;
    {
        let mut db = ProjectDatabase::new(path.clone()).unwrap();
        add_project(&db, "p");
        a = add_audio(&mut db, "old.wav");
        // Simulate the old write path: the column only, no list row.
        db.conn.execute("UPDATE projects SET audio_file_id = ? WHERE id = 'p'", [&a]).unwrap();
        db.conn.execute("DELETE FROM project_audio_files", []).unwrap();
        assert!(listed(&db, "p").is_empty());
    }
    let db = ProjectDatabase::new(path).unwrap();
    assert_eq!(listed(&db, "p"), vec![("old.wav".into(), true)]);
}
