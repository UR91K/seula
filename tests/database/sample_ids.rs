//! A project's samples carry their stored ids. They used to be given a fresh random id
//! on every read, so nothing could link a project's sample to the samples list.

use seula::database::ProjectDatabase;
use tempfile::TempDir;

#[test]
fn a_projects_samples_keep_their_stored_ids_across_reads() {
    let dir = TempDir::new().expect("temp dir");
    let mut db = ProjectDatabase::new(dir.path().join("samples.db")).expect("fresh database");
    let project = "11111111-1111-4111-8111-111111111111";
    let sample = "22222222-2222-4222-8222-222222222222";
    db.conn
        .execute_batch(&format!(
            "INSERT INTO projects (id, path, name, hash, created_at, modified_at, last_parsed_at,
                tempo, time_signature_numerator, time_signature_denominator,
                daw_type, daw_version_display)
             VALUES ('{project}', 'p.als', 'p', '', 0, 0, 0, 120, 4, 4, 'Ableton Live', '12.0.0');
             INSERT INTO project_ableton_metadata VALUES ('{project}', 12, 0, 0, 0);
             INSERT INTO samples VALUES ('{sample}', 'kick.wav', 'C:\\kick.wav', 1);
             INSERT INTO project_samples VALUES ('{project}', '{sample}');"
        ))
        .unwrap();

    for _ in 0..2 {
        let read = db.get_project_by_id(project).unwrap().expect("project exists");
        let ids: Vec<String> = read.samples.iter().map(|s| s.id.to_string()).collect();
        assert_eq!(ids, vec![sample.to_string()]);
    }
}
