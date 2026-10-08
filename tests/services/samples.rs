//! `SamplesService` tests. Ported from `tests/grpc/samples.rs` (ADR-0046).

use super::{add_sample_to_project, create_test_project, create_test_sample, test_env};
use seula::database::ProjectScope;

#[tokio::test]
async fn get_sample_returns_the_row() {
    let env = test_env();
    let sample_id = create_test_sample(&env, "Test Sample", "/path/to/test/sample.wav", true).await;

    let sample = env
        .services
        .samples
        .get_sample(&sample_id)
        .await
        .unwrap()
        .expect("the sample exists");

    assert_eq!(sample.id.to_string(), sample_id);
    assert_eq!(sample.name, "Test Sample");
    assert_eq!(sample.path.to_str().unwrap(), "/path/to/test/sample.wav");
    assert!(sample.is_present);
}

/// "Not found" is `None` here; the adapters turn it into their own 404.
#[tokio::test]
async fn get_sample_finds_nothing_for_an_unknown_id() {
    let env = test_env();

    let found = env
        .services
        .samples
        .get_sample("non-existent-sample")
        .await
        .unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn refresh_sample_presence_status_accounts_for_every_sample() {
    let env = test_env();

    let result = env
        .services
        .samples
        .refresh_sample_presence_status()
        .await
        .unwrap();

    assert!(result.total_samples_checked >= 0);
    assert!(result.samples_now_present >= 0);
    assert!(result.samples_now_missing >= 0);
    assert!(result.samples_unchanged >= 0);
    assert_eq!(
        result.total_samples_checked,
        result.samples_now_present + result.samples_now_missing + result.samples_unchanged
    );
}

#[tokio::test]
async fn sample_analytics_describe_usage_extensions_and_presence() {
    let env = test_env();

    let sample1 = create_test_sample(&env, "kick.wav", "/samples/kick.wav", true).await;
    create_test_sample(&env, "snare.mp3", "/samples/snare.mp3", false).await;
    let sample3 = create_test_sample(&env, "hihat.aiff", "/samples/hihat.aiff", true).await;

    // Five projects use sample1, one uses sample3, none uses sample2.
    for i in 0..5 {
        let project = create_test_project(
            &env,
            &format!("Test Project {}", i),
            &format!("/path/to/test/project{}.als", i),
        )
        .await;
        add_sample_to_project(&env, &project, &sample1).await;
    }
    let project = create_test_project(
        &env,
        "Test Project for Sample3",
        "/path/to/test/project_sample3.als",
    )
    .await;
    add_sample_to_project(&env, &project, &sample3).await;

    let analytics = env.services.samples.get_sample_analytics().await.unwrap();

    assert_eq!(analytics.most_used_samples_count, 1, "sample1, 5+ usages");
    assert_eq!(analytics.moderately_used_samples_count, 0, "none with 2-4");
    assert_eq!(analytics.rarely_used_samples_count, 1, "sample3, 1 usage");
    assert_eq!(analytics.unused_samples_count, 1, "sample2, 0 usages");

    for extension in ["wav", "mp3", "aiff"] {
        assert!(analytics.extensions.contains_key(extension), "{extension}");
    }

    // Two of three present, rounded down.
    assert_eq!(analytics.present_samples_percentage, 66);
    assert_eq!(analytics.missing_samples_percentage, 33);

    // Storage is measured by a sample check (ADR-0041), and none has run here. It used
    // to be a guessed size per extension, which is why these were once non-zero.
    assert_eq!(analytics.total_storage_bytes, 0);
    assert_eq!(analytics.present_storage_bytes, 0);
    assert_eq!(analytics.missing_storage_bytes, 0);

    assert_eq!(analytics.top_used_samples.len(), 3);
    assert_eq!(analytics.top_used_samples[0].usage_count, 5);
    assert_eq!(analytics.top_used_samples[0].sample_id, sample1);

    // Creation dates are not tracked.
    assert_eq!(analytics.recently_added_samples, 0);
}

#[tokio::test]
async fn get_all_samples_applies_each_filter() {
    let env = test_env();
    let project = create_test_project(&env, "Test Project", "/path/to/test/project.als").await;
    let kick = create_test_sample(&env, "kick.wav", "/samples/kick.wav", true).await;
    create_test_sample(&env, "snare.mp3", "/samples/snare.mp3", false).await;
    create_test_sample(&env, "hihat.aiff", "/samples/hihat.aiff", true).await;
    add_sample_to_project(&env, &project, &kick).await;

    #[allow(clippy::too_many_arguments)]
    async fn list(
        env: &super::TestEnv,
        present_only: Option<bool>,
        missing_only: Option<bool>,
        format: Option<&str>,
        min_usage: Option<i32>,
        max_usage: Option<i32>,
    ) -> Vec<seula::models::Sample> {
        env.services
            .samples
            .get_all_samples(
                None,
                None,
                None,
                None,
                present_only,
                missing_only,
                format.map(String::from),
                min_usage,
                max_usage,
                ProjectScope::Active,
            )
            .await
            .unwrap()
            .0
    }

    let present = list(&env, Some(true), None, None, None, None).await;
    assert_eq!(present.len(), 2, "kick.wav and hihat.aiff");
    assert!(present.iter().all(|s| s.is_present));

    let missing = list(&env, None, Some(true), None, None, None).await;
    assert_eq!(missing.len(), 1, "snare.mp3");
    assert!(missing.iter().all(|s| !s.is_present));

    let wav = list(&env, None, None, Some("wav"), None, None).await;
    assert_eq!(wav.len(), 1);
    assert!(wav[0].path.to_str().unwrap().ends_with(".wav"));

    let used = list(&env, None, None, None, Some(1), None).await;
    assert_eq!(used.len(), 1, "only kick.wav has a usage");
    assert_eq!(used[0].id.to_string(), kick);

    let unused = list(&env, None, None, None, None, Some(0)).await;
    assert_eq!(unused.len(), 2, "snare.mp3 and hihat.aiff");
    assert!(!unused.iter().any(|s| s.id.to_string() == kick));

    let combined = list(&env, Some(true), None, Some("aiff"), None, None).await;
    assert_eq!(combined.len(), 1, "only hihat.aiff matches both");
    assert!(combined[0].path.to_str().unwrap().ends_with(".aiff"));
    assert!(combined[0].is_present);
}

#[tokio::test]
async fn get_all_samples_sorts_by_name_and_usage() {
    let env = test_env();
    let zebra = create_test_sample(&env, "zebra.wav", "/samples/zebra.wav", true).await;
    let alpha = create_test_sample(&env, "alpha.wav", "/samples/alpha.wav", true).await;
    let beta = create_test_sample(&env, "beta.wav", "/samples/beta.wav", true).await;

    // zebra: 1 usage, alpha: 2, beta: 0.
    let project1 = create_test_project(&env, "Test Project", "/path/to/test/project.als").await;
    let project2 = create_test_project(&env, "Test Project 2", "/path/to/test/project2.als").await;
    add_sample_to_project(&env, &project1, &zebra).await;
    add_sample_to_project(&env, &project1, &alpha).await;
    add_sample_to_project(&env, &project2, &alpha).await;

    async fn names_and_ids(
        env: &super::TestEnv,
        sort_by: &str,
        sort_desc: bool,
    ) -> Vec<(String, String)> {
        env.services
            .samples
            .get_all_samples(
                None,
                None,
                Some(sort_by.to_string()),
                Some(sort_desc),
                None,
                None,
                None,
                None,
                None,
                ProjectScope::Active,
            )
            .await
            .unwrap()
            .0
            .into_iter()
            .map(|s| (s.name, s.id.to_string()))
            .collect()
    }

    let ascending = names_and_ids(&env, "name", false).await;
    let names: Vec<&str> = ascending.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(names, vec!["alpha.wav", "beta.wav", "zebra.wav"]);

    let descending = names_and_ids(&env, "name", true).await;
    let names: Vec<&str> = descending.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(names, vec!["zebra.wav", "beta.wav", "alpha.wav"]);

    let by_usage = names_and_ids(&env, "usage_count", true).await;
    let ids: Vec<&str> = by_usage.iter().map(|s| s.1.as_str()).collect();
    assert_eq!(ids, vec![alpha.as_str(), zebra.as_str(), beta.as_str()]);
}

#[tokio::test]
async fn sample_extensions_count_presence_and_average_usage() {
    let env = test_env();
    let project1 = create_test_project(&env, "Test Project 1", "/path/to/project1.als").await;
    let project2 = create_test_project(&env, "Test Project 2", "/path/to/project2.als").await;
    let wav = create_test_sample(&env, "test_wav.wav", "/path/to/test_wav.wav", true).await;
    let aiff = create_test_sample(&env, "test_aiff.aiff", "/path/to/test_aiff.aiff", true).await;
    let mp3 = create_test_sample(&env, "test_mp3.mp3", "/path/to/test_mp3.mp3", false).await;
    let flac = create_test_sample(&env, "test_flac.flac", "/path/to/test_flac.flac", true).await;
    add_sample_to_project(&env, &project1, &wav).await;
    add_sample_to_project(&env, &project1, &aiff).await;
    add_sample_to_project(&env, &project2, &wav).await; // the wav is used in two projects
    add_sample_to_project(&env, &project2, &mp3).await;
    add_sample_to_project(&env, &project2, &flac).await;

    let extensions = env.services.samples.get_sample_extensions().await.unwrap();

    for extension in ["wav", "aiff", "mp3", "flac"] {
        assert!(extensions.contains_key(extension), "{extension}");
    }
    let wav_stats = &extensions["wav"];
    assert_eq!(wav_stats.count, 1);
    assert_eq!(wav_stats.present_count, 1);
    assert_eq!(wav_stats.missing_count, 0);
    assert_eq!(wav_stats.average_usage_count, 2.0);
    let mp3_stats = &extensions["mp3"];
    assert_eq!(mp3_stats.count, 1);
    assert_eq!(mp3_stats.present_count, 0);
    assert_eq!(mp3_stats.missing_count, 1);
    assert_eq!(mp3_stats.average_usage_count, 1.0);

    // Sizes come from a sample check (ADR-0041); none has run here.
    assert_eq!(wav_stats.total_size_bytes, 0);
    assert_eq!(mp3_stats.total_size_bytes, 0);
}

/// Not a service test: the model's own `add_sample`, which sat in the gRPC tests by
/// accident. A project's samples are a set, so adding one twice keeps one.
#[test]
fn project_add_sample_does_not_duplicate() {
    use chrono::Local;
    use seula::models::{AbletonVersion, Sample, TimeSignature};
    use seula::project::Project;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use uuid::Uuid;

    let version = AbletonVersion {
        major: 11,
        minor: 0,
        patch: 0,
        beta: false,
    };
    let mut project = Project {
        is_active: true,
        id: Uuid::new_v4(),
        file_path: PathBuf::from("/test/path.als"),
        name: "Test Project".to_string(),
        file_hash: "test_hash".to_string(),
        created_time: Local::now(),
        modified_time: Local::now(),
        last_parsed_timestamp: Local::now(),
        daw_type: "Ableton Live".to_string(),
        daw_version_display: version.to_string(),
        ableton_metadata: version,
        key_signature: None,
        tempo: 120.0,
        time_signature: TimeSignature {
            numerator: 4,
            denominator: 4,
        },
        furthest_bar: None,
        plugins: HashSet::new(),
        samples: HashSet::new(),
        tags: HashSet::new(),
        estimated_duration: None,
    };
    let sample = |name: &str, path: &str, is_present| Sample {
        id: Uuid::new_v4(),
        name: name.to_string(),
        path: PathBuf::from(path),
        is_present,
    };
    let kick = sample("kick.wav", "/samples/kick.wav", true);
    let snare = sample("snare.wav", "/samples/snare.wav", false);
    assert_eq!(project.samples.len(), 0);

    project.add_sample(kick.clone());
    project.add_sample(snare.clone());

    assert_eq!(project.samples.len(), 2);
    assert!(project.samples.contains(&kick));
    assert!(project.samples.contains(&snare));

    project.add_sample(kick);
    assert_eq!(project.samples.len(), 2);
}
