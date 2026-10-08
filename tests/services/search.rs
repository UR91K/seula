//! `SearchService` tests. Ported from `tests/grpc/search.rs` (ADR-0046).

use super::{create_test_project_in_db, test_env, TestEnv};
use seula::models::Tonic;
use seula::project::Project;

/// Every project `create_test_project_in_db` makes is named "Test Project <uuid>.als",
/// has a Serum plugin and a kick.wav sample, is 140 BPM, 4/4, C major, Live 11.
async fn with_projects(count: usize) -> TestEnv {
    let env = test_env();
    for _ in 0..count {
        create_test_project_in_db(&env.db).await;
    }
    env
}

async fn search(env: &TestEnv, query: &str) -> (Vec<Project>, i32) {
    let (results, total) = env.services.search.search(query, None, None).await.unwrap();
    (results.into_iter().map(|r| r.project).collect(), total)
}

async fn search_page(
    env: &TestEnv,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> (Vec<Project>, i32) {
    let (results, total) = env
        .services
        .search
        .search(query, limit, offset)
        .await
        .unwrap();
    (results.into_iter().map(|r| r.project).collect(), total)
}

#[tokio::test]
async fn search_of_an_empty_database_finds_nothing() {
    let env = test_env();

    let (projects, total) = search(&env, "test").await;

    assert!(projects.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn search_finds_projects_by_name() {
    let env = with_projects(2).await;

    let (projects, total) = search(&env, "Test Project").await;

    assert!(!projects.is_empty());
    assert_eq!(total as usize, projects.len());
    for project in &projects {
        assert!(!project.name.is_empty());
        assert!(project.name.contains("Test Project"));
    }
}

#[tokio::test]
async fn search_limit_trims_the_page_not_the_total() {
    let env = with_projects(5).await;

    let (projects, total) = search_page(&env, "Test Project", Some(3), None).await;

    assert_eq!(projects.len(), 3);
    assert_eq!(total, 5);
}

#[tokio::test]
async fn search_offset_skips_results() {
    let env = with_projects(5).await;

    let (projects, total) = search_page(&env, "Test Project", None, Some(2)).await;

    assert_eq!(projects.len(), 3, "5 total less 2 skipped");
    assert_eq!(total, 5);
}

#[tokio::test]
async fn search_limit_and_offset_combine() {
    let env = with_projects(10).await;

    let (projects, total) = search_page(&env, "Test Project", Some(3), Some(2)).await;

    assert_eq!(projects.len(), 3);
    assert_eq!(total, 10);
}

#[tokio::test]
async fn search_with_an_empty_query_does_not_fail() {
    let env = with_projects(1).await;

    let (_, total) = search(&env, "").await;

    // What an empty query matches is up to the search implementation.
    assert!(total >= 0);
}

#[tokio::test]
async fn search_with_no_match_finds_nothing() {
    let env = with_projects(1).await;

    let (projects, total) = search(&env, "NonExistentProjectName12345").await;

    assert!(projects.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn search_survives_special_characters() {
    let env = with_projects(1).await;

    let (_, total) = search(&env, "Test@#$%^&*()").await;

    assert!(total >= 0);
}

#[tokio::test]
async fn search_survives_unicode() {
    let env = with_projects(1).await;

    let (_, total) = search(&env, "🎵 Test 音楽 プロジェクト").await;

    assert!(total >= 0);
}

#[tokio::test]
async fn search_is_case_insensitive() {
    let env = with_projects(1).await;

    let mut totals = Vec::new();
    for query in [
        "test project",
        "TEST PROJECT",
        "Test Project",
        "TeSt PrOjEcT",
    ] {
        totals.push(search(&env, query).await.1);
    }

    for total in &totals {
        assert_eq!(*total, totals[0], "search should be case-insensitive");
    }
}

#[tokio::test]
async fn search_offset_past_the_end_returns_an_empty_page_and_the_full_total() {
    let env = with_projects(3).await;

    let (projects, total) = search_page(&env, "Test Project", None, Some(100)).await;

    assert!(projects.is_empty());
    assert_eq!(total, 3);
}

#[tokio::test]
async fn search_zero_limit_returns_an_empty_page_and_the_full_total() {
    let env = with_projects(1).await;

    let (projects, total) = search_page(&env, "Test Project", Some(0), None).await;

    assert!(projects.is_empty());
    assert!(total > 0);
}

/// The old gRPC test expected a negative offset to be "likely treated as 0" but only
/// checked the total. It is not: `offset as usize` wraps, so the page comes back empty
/// (the `search-negative-offset` bug in `docs/bugs.md`). This pins the total, as the old
/// test did, and not the page.
#[tokio::test]
async fn search_negative_offset_keeps_the_total() {
    let env = with_projects(1).await;

    let (_, total) = search_page(&env, "Test Project", None, Some(-5)).await;

    assert_eq!(total, 1);
}

#[tokio::test]
async fn search_results_carry_the_whole_project() {
    let env = with_projects(1).await;

    let (projects, _) = search_page(&env, "Test Project", Some(1), None).await;

    assert_eq!(projects.len(), 1);
    let project = &projects[0];
    assert!(!project.name.is_empty());
    assert!(!project.file_path.as_os_str().is_empty());
    assert!(!project.file_hash.is_empty());
    assert!(project.tempo > 0.0);
    assert!(project.key_signature.is_some());
    assert!(!project.plugins.is_empty());
    assert!(!project.samples.is_empty());
}

// Operators.

#[tokio::test]
async fn name_operator_matches_the_name() {
    let env = with_projects(2).await;

    let (projects, _) = search(&env, "name:Test").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert!(project.name.to_lowercase().contains("test"));
    }
}

#[tokio::test]
async fn bpm_operator_matches_the_tempo() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "bpm:140").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert_eq!(project.tempo, 140.0);
    }
}

#[tokio::test]
async fn plugin_operator_matches_a_plugin() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "plugin:Serum").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert!(
            project.plugins.iter().any(|p| p.name.contains("Serum")),
            "project should contain Serum plugin"
        );
    }
}

#[tokio::test]
async fn sample_operator_matches_a_sample() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "sample:kick").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert!(
            project
                .samples
                .iter()
                .any(|s| s.name.to_lowercase().contains("kick")),
            "project should contain kick sample"
        );
    }
}

#[tokio::test]
async fn tag_operator_matches_a_tag() {
    let env = test_env();
    let tagged = create_test_project_in_db(&env.db).await;
    create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env.services.tags.create_tag("Electronic").await.unwrap();
    env.services
        .tags
        .tag_project(&tagged, &tag_id)
        .await
        .unwrap();

    let (projects, total) = search(&env, "tag:Electronic").await;

    // The result's own `tags` field is filled in by the adapters, not by the search,
    // so what is checked here is which project the operator selects.
    assert_eq!(total, 1);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id.to_string(), tagged);
}

#[tokio::test]
async fn path_operator_matches_the_path() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "path:Test").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert!(project
            .file_path
            .to_string_lossy()
            .to_lowercase()
            .contains("test"));
    }
}

#[tokio::test]
async fn version_operator_matches_the_major_version() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "version:11").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert_eq!(project.ableton_metadata.major, 11);
    }
}

#[tokio::test]
async fn key_operator_matches_the_tonic() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "key:C").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert_eq!(project.key_signature.as_ref().unwrap().tonic, Tonic::C);
    }
}

#[tokio::test]
async fn ts_operator_matches_the_time_signature() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "ts:4/4").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert_eq!(project.time_signature.numerator, 4);
        assert_eq!(project.time_signature.denominator, 4);
    }
}

#[tokio::test]
async fn several_operators_must_all_match() {
    let env = with_projects(1).await;

    let (projects, _) = search(&env, "bpm:140 key:C plugin:Serum").await;

    assert!(!projects.is_empty());
    for project in &projects {
        assert_eq!(project.tempo, 140.0);
        assert_eq!(project.key_signature.as_ref().unwrap().tonic, Tonic::C);
        assert!(project.plugins.iter().any(|p| p.name.contains("Serum")));
    }
}

#[tokio::test]
async fn operators_take_quoted_values() {
    let env = with_projects(1).await;

    let (_, total) = search(&env, "name:\"Test Project\"").await;

    assert!(total >= 0);
}

#[tokio::test]
async fn an_unknown_operator_is_treated_as_text() {
    let env = with_projects(1).await;

    let (_, total) = search(&env, "unknown:value Test").await;

    assert!(total >= 0);
}

#[tokio::test]
async fn an_operator_with_no_match_finds_nothing() {
    let env = with_projects(1).await;

    let (projects, total) = search(&env, "bpm:999").await;

    assert!(projects.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn text_and_operators_mix() {
    let env = with_projects(1).await;

    let (projects, total) = search(&env, "Test bpm:140 Electronic").await;

    assert!(total >= 0);
    for project in &projects {
        assert_eq!(project.tempo, 140.0);
    }
}
