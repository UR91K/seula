//! `ProjectsService` tests. Ported from `tests/grpc/projects.rs` (ADR-0046).

use super::{create_test_project_in_db, list_projects as list, test_env};
use seula::error::DatabaseError;
use seula::project::Project;
use seula::services::DeletionScope;

fn ids(projects: &[Project]) -> Vec<String> {
    projects.iter().map(|p| p.id.to_string()).collect()
}

// Names.

#[tokio::test]
async fn update_project_renames_it() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let original = env
        .services
        .projects
        .get_project(&project_id)
        .await
        .unwrap()
        .unwrap()
        .name;

    env.services
        .projects
        .update_project(&project_id, Some("My Custom Project Alias"), None)
        .await
        .unwrap();

    let updated = env
        .services
        .projects
        .get_project(&project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "My Custom Project Alias");
    assert_ne!(updated.name, original);
}

/// Renaming a project that is not there changes nothing and is not an error. The old
/// gRPC test called it "graceful handling"; the adapters inherit it.
#[tokio::test]
async fn update_project_of_a_missing_project_is_a_no_op() {
    let env = test_env();

    env.services
        .projects
        .update_project("non-existent-project-id", Some("New Name"), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn update_project_accepts_an_empty_name() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    env.services
        .projects
        .update_project(&project_id, Some(""), None)
        .await
        .unwrap();

    let updated = env
        .services
        .projects
        .get_project(&project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, "");
}

#[tokio::test]
async fn update_project_keeps_special_characters_and_unicode() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let name = "🎵 My Project (remix) [2024] - Draft v2.1 🎶";

    env.services
        .projects
        .update_project(&project_id, Some(name), None)
        .await
        .unwrap();

    let updated = env
        .services
        .projects
        .get_project(&project_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated.name, name);
}

#[tokio::test]
async fn a_renamed_project_stays_renamed() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    env.services
        .projects
        .update_project(&project_id, Some("Persistent Test Name"), None)
        .await
        .unwrap();

    for _ in 0..3 {
        let project = env
            .services
            .projects
            .get_project(&project_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.name, "Persistent Test Name");
    }
}

// Deletion.

#[tokio::test]
async fn mark_deleted_moves_a_project_to_the_deleted_list() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    env.services
        .projects
        .mark_deleted(&project_id)
        .await
        .unwrap();

    let deleted = list(&env, DeletionScope::DeletedOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&deleted), vec![project_id]);
}

#[tokio::test]
async fn reactivate_brings_a_deleted_project_back() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    env.services
        .projects
        .mark_deleted(&project_id)
        .await
        .unwrap();

    env.services.projects.reactivate(&project_id).await.unwrap();

    let active = list(&env, DeletionScope::ActiveOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&active), vec![project_id]);
}

#[tokio::test]
async fn deletion_scopes_split_the_projects() {
    let env = test_env();
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;
    let project3 = create_test_project_in_db(&env.db).await;
    env.services.projects.mark_deleted(&project1).await.unwrap();
    env.services.projects.mark_deleted(&project2).await.unwrap();

    let active = list(&env, DeletionScope::ActiveOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&active), vec![project3]);

    let deleted = ids(&list(&env, DeletionScope::DeletedOnly, Some(10), Some(0)).await);
    assert_eq!(deleted.len(), 2);
    assert!(deleted.contains(&project1));
    assert!(deleted.contains(&project2));

    let all = list(&env, DeletionScope::All, None, None).await;
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn permanently_delete_removes_a_deleted_project() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    env.services
        .projects
        .mark_deleted(&project_id)
        .await
        .unwrap();

    env.services
        .projects
        .permanently_delete(&project_id)
        .await
        .unwrap();

    assert!(env
        .services
        .projects
        .get_project_any_status(&project_id)
        .await
        .unwrap()
        .is_none());
}

/// The service refuses with this exact `InvalidOperation` message. Both adapters match
/// on the text to turn it into their own answer (gRPC's `success: false`, HTTP's 400),
/// so a reworded message would silently change what they say.
#[tokio::test]
async fn permanently_delete_refuses_an_active_project() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    let result = env.services.projects.permanently_delete(&project_id).await;

    match result {
        Err(DatabaseError::InvalidOperation(msg)) => {
            assert_eq!(msg, "Cannot permanently delete an active project")
        }
        other => panic!("expected InvalidOperation, got {:?}", other.map(|_| ())),
    }
    assert!(env
        .services
        .projects
        .get_project(&project_id)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn mark_deleted_refuses_a_malformed_id() {
    let env = test_env();

    let result = env
        .services
        .projects
        .mark_deleted("non-existent-project-id")
        .await;

    assert!(matches!(result, Err(DatabaseError::InvalidOperation(_))));
}

#[tokio::test]
async fn reactivate_refuses_a_malformed_id() {
    let env = test_env();

    let result = env
        .services
        .projects
        .reactivate("non-existent-project-id")
        .await;

    assert!(matches!(result, Err(DatabaseError::InvalidOperation(_))));
}

#[tokio::test]
async fn deleted_projects_paginate() {
    let env = test_env();
    for _ in 0..5 {
        let project_id = create_test_project_in_db(&env.db).await;
        env.services
            .projects
            .mark_deleted(&project_id)
            .await
            .unwrap();
    }

    let first = list(&env, DeletionScope::DeletedOnly, Some(3), Some(0)).await;
    let second = list(&env, DeletionScope::DeletedOnly, Some(3), Some(3)).await;
    let beyond = list(&env, DeletionScope::DeletedOnly, Some(3), Some(6)).await;

    assert_eq!(first.len(), 3);
    assert_eq!(second.len(), 2);
    assert_eq!(beyond.len(), 0);
}

#[tokio::test]
async fn a_project_goes_through_the_whole_deletion_cycle() {
    let env = test_env();
    let projects = &env.services.projects;
    let project_id = create_test_project_in_db(&env.db).await;

    let active = list(&env, DeletionScope::ActiveOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&active), vec![project_id.clone()], "initially active");

    projects.mark_deleted(&project_id).await.unwrap();
    let deleted = list(&env, DeletionScope::DeletedOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&deleted), vec![project_id.clone()]);
    let active = list(&env, DeletionScope::ActiveOnly, Some(10), Some(0)).await;
    assert!(active.is_empty());

    projects.reactivate(&project_id).await.unwrap();
    let active = list(&env, DeletionScope::ActiveOnly, Some(10), Some(0)).await;
    assert_eq!(ids(&active), vec![project_id.clone()], "back again");

    projects.mark_deleted(&project_id).await.unwrap();
    projects.permanently_delete(&project_id).await.unwrap();
    assert!(list(&env, DeletionScope::ActiveOnly, Some(10), Some(0))
        .await
        .is_empty());
    assert!(list(&env, DeletionScope::DeletedOnly, Some(10), Some(0))
        .await
        .is_empty());
}

// Filtering and statistics.

#[tokio::test]
async fn list_projects_filters_by_tempo() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    // The test project is 140 BPM.
    let (inside, _) = env
        .services
        .projects
        .list_projects(
            DeletionScope::ActiveOnly,
            Some(10),
            Some(0),
            None,
            None,
            Some(80.0),
            Some(150.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(ids(&inside).contains(&project_id));

    let (outside, _) = env
        .services
        .projects
        .list_projects(
            DeletionScope::ActiveOnly,
            Some(10),
            Some(0),
            None,
            None,
            Some(80.0),
            Some(120.0),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert!(outside.is_empty());
}

#[tokio::test]
async fn project_statistics_describe_the_library() {
    let env = test_env();
    create_test_project_in_db(&env.db).await;

    let stats = env
        .services
        .projects
        .get_statistics(
            None, None, None, None, None, None, None, None, None, None, None, None,
        )
        .await
        .unwrap();

    assert!(stats.total_projects > 0);
    assert!(stats.average_tempo >= 0.0);
    assert!(stats.min_tempo >= 0.0);
    assert!(stats.max_tempo >= 0.0);
    assert!(stats.average_plugins_per_project >= 0.0);
    assert!(stats.average_samples_per_project >= 0.0);
    assert!(stats.average_tags_per_project >= 0.0);
}

// Rescan.

/// The test project's file does not exist, so a rescan reports that, forced or not,
/// instead of failing.
#[tokio::test]
async fn rescan_of_a_project_whose_file_is_gone_reports_failure() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    for force in [false, true] {
        let result = env
            .services
            .projects
            .rescan(&project_id, force)
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result
            .error_message
            .expect("an error message")
            .contains("Project file not found"));
        assert!(!result.was_updated);
        assert!(result
            .scan_summary
            .contains("Project file no longer exists"));
    }
}

#[tokio::test]
async fn rescan_of_an_unknown_project_is_an_error() {
    let env = test_env();

    let error = env
        .services
        .projects
        .rescan("00000000-0000-0000-0000-000000000000", false)
        .await
        .err()
        .expect("an error");

    assert!(error.to_string().contains("Project not found"), "{error}");
}

#[tokio::test]
async fn rescan_of_a_malformed_id_is_an_error() {
    let env = test_env();

    let error = env
        .services
        .projects
        .rescan("invalid-uuid", false)
        .await
        .err()
        .expect("an error");

    assert!(error.to_string().contains("Project not found"), "{error}");
}
