//! The sample check (ADR-0041): what it finds on disk, and what it records.

use std::fs;

use seula::database::samples::SampleFilter;
use seula::database::ProjectDatabase;
use seula::scan::sample_check::check_sample_files;
use tempfile::TempDir;

#[test]
fn the_check_finds_files_by_listing_a_folder_or_one_by_one() {
    let dir = TempDir::new().unwrap();
    let pack = dir.path().join("pack");
    let lone = dir.path().join("lone");
    fs::create_dir_all(&pack).unwrap();
    fs::create_dir_all(&lone).unwrap();
    for (name, bytes) in [("kick.wav", 10), ("snare.wav", 20), ("hat.wav", 30)] {
        fs::write(pack.join(name), vec![0u8; bytes]).unwrap();
    }
    fs::write(lone.join("pad.aif"), vec![0u8; 40]).unwrap();

    let p = |path: std::path::PathBuf| path.to_string_lossy().to_string();
    let paths = vec![
        p(pack.join("kick.wav")),
        p(pack.join("snare.wav")),
        p(pack.join("hat.wav")),
        p(pack.join("gone.wav")),           // listed folder, file missing
        p(lone.join("pad.aif")),            // one sample in its folder: stat'ed
        p(dir.path().join("nowhere").join("a.wav")), // folder missing
    ];
    let mut progress = Vec::new();
    let found = check_sample_files(&paths, 4, &mut |done, total, _| progress.push((done, total)));

    let size = |path: &str| found[path].map(|f| f.size_bytes);
    assert_eq!(size(&paths[0]), Some(10));
    assert_eq!(size(&paths[2]), Some(30));
    assert_eq!(size(&paths[3]), None);
    assert_eq!(size(&paths[4]), Some(40));
    assert_eq!(size(&paths[5]), None);
    assert!(found[&paths[0]].unwrap().modified_at.is_some());
    assert_eq!(progress.last(), Some(&(3, 3)), "one report per folder, ending at the total");

    if cfg!(windows) {
        // Names compare without case, as the filesystem does.
        let upper = p(pack.join("KICK.WAV"));
        let found = check_sample_files(&[upper.clone(), paths[1].clone(), paths[2].clone()], 1, &mut |_, _, _| {});
        assert_eq!(found[&upper].map(|f| f.size_bytes), Some(10));
    }
}

#[test]
fn recording_a_check_sets_presence_and_keeps_a_missing_samples_last_size() {
    let dir = TempDir::new().unwrap();
    let mut db = ProjectDatabase::new(dir.path().join("check.db")).unwrap();
    let file = dir.path().join("kick.wav");
    fs::write(&file, vec![0u8; 1234]).unwrap();
    let path = file.to_string_lossy().to_string();
    db.conn
        .execute(
            "INSERT INTO samples VALUES ('00000000-0000-4000-8000-000000000001', 'kick.wav', ?1, 0)",
            [&path],
        )
        .unwrap();

    let check = |db: &mut ProjectDatabase| {
        let paths = db.sample_paths().unwrap();
        let found = check_sample_files(&paths, 2, &mut |_, _, _| {});
        db.record_sample_check(&found).unwrap()
    };

    let first = check(&mut db);
    assert_eq!((first.total_samples_checked, first.samples_now_present), (1, 1));
    let stats = db.get_sample_stats_filtered(&SampleFilter::default()).unwrap();
    assert_eq!((stats.present_samples, stats.sized_samples, stats.total_size_bytes), (1, 1, 1234));

    fs::remove_file(&file).unwrap();
    let second = check(&mut db);
    assert_eq!(second.samples_now_missing, 1);
    let stats = db.get_sample_stats_filtered(&SampleFilter::default()).unwrap();
    assert_eq!((stats.present_samples, stats.total_size_bytes), (0, 0), "sizes count present samples");
    let kept = db.sample_sizes(&["00000000-0000-4000-8000-000000000001".into()]).unwrap();
    assert_eq!(kept.values().copied().collect::<Vec<_>>(), [1234], "the last size is kept");
}
