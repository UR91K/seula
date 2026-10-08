//! Batch operations across the services. Ported from `tests/grpc/batch.rs` (ADR-0046).
//!
//! The services return one result per item. Turning those into the
//! `successful_count` / `failed_count` the APIs report is the adapters' job, so
//! `counts` stands in for it here.

use super::{create_test_project_in_db, list_projects, test_env, TestEnv};
use seula::database::ProjectScope;
use seula::error::DatabaseError;
use seula::services::DeletionScope;

type Results = Vec<(String, Result<(), DatabaseError>)>;

/// `(succeeded, failed)`.
fn counts(results: &Results) -> (usize, usize) {
    let ok = results.iter().filter(|(_, r)| r.is_ok()).count();
    (ok, results.len() - ok)
}

fn all_ok(results: &Results) -> bool {
    results.iter().all(|(_, r)| r.is_ok())
}

async fn projects(env: &TestEnv, count: usize) -> Vec<String> {
    let mut ids = Vec::new();
    for _ in 0..count {
        ids.push(create_test_project_in_db(&env.db).await);
    }
    ids
}

async fn tag(env: &TestEnv, name: &str) -> String {
    env.services.tags.create_tag(name).await.unwrap().0
}

async fn tag_names(env: &TestEnv, project_id: &str) -> std::collections::HashSet<String> {
    env.db.lock().await.get_project_tags(project_id).unwrap()
}

async fn active(env: &TestEnv) -> usize {
    list_projects(env, DeletionScope::ActiveOnly, Some(10), Some(0))
        .await
        .len()
}

async fn archived(env: &TestEnv) -> Vec<String> {
    list_projects(env, DeletionScope::DeletedOnly, Some(10), Some(0))
        .await
        .iter()
        .map(|p| p.id.to_string())
        .collect()
}

async fn collection_project_ids(env: &TestEnv, collection_id: &str) -> (Vec<String>, i32) {
    let detail = env
        .services
        .collections
        .get_collection(collection_id, ProjectScope::All)
        .await
        .unwrap()
        .expect("the collection exists");
    (detail.project_ids, detail.project_count)
}

#[tokio::test]
async fn batch_mark_archived_archives_every_project() {
    let env = test_env();
    let ids = projects(&env, 3).await;
    assert_eq!(active(&env).await, 3);

    let results = env
        .services
        .projects
        .batch_mark_archived(&ids, true)
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (3, 0));
    assert_eq!(active(&env).await, 0);
    let archived = archived(&env).await;
    assert_eq!(archived.len(), 3);
    for id in &ids {
        assert!(archived.contains(id));
    }
}

#[tokio::test]
async fn batch_delete_removes_archived_projects_for_good() {
    let env = test_env();
    let ids = projects(&env, 3).await;
    env.services
        .projects
        .batch_mark_archived(&ids, true)
        .await
        .unwrap();
    assert_eq!(archived(&env).await.len(), 3);

    let results = env.services.projects.batch_delete(&ids).await.unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (3, 0));
    assert!(archived(&env).await.is_empty());
    assert_eq!(active(&env).await, 0);
}

#[tokio::test]
async fn batch_tag_applies_every_tag_to_every_project() {
    let env = test_env();
    let ids = projects(&env, 2).await;
    let tag1 = tag(&env, "Test Tag 1").await;
    let tag2 = tag(&env, "Test Tag 2").await;

    let results = env
        .services
        .tags
        .batch_tag_projects(&ids, &[tag1, tag2])
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (4, 0), "2 projects x 2 tags");
    for id in &ids {
        let names = tag_names(&env, id).await;
        assert_eq!(names.len(), 2);
        assert!(names.contains("Test Tag 1"));
        assert!(names.contains("Test Tag 2"));
    }
}

#[tokio::test]
async fn batch_untag_removes_only_the_given_tags() {
    let env = test_env();
    let ids = projects(&env, 2).await;
    let tag1 = tag(&env, "Test Tag 1").await;
    let tag2 = tag(&env, "Test Tag 2").await;
    env.services
        .tags
        .batch_tag_projects(&ids, &[tag1.clone(), tag2])
        .await
        .unwrap();
    for id in &ids {
        assert_eq!(tag_names(&env, id).await.len(), 2);
    }

    let results = env
        .services
        .tags
        .batch_untag_projects(&ids, &[tag1])
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (2, 0));
    for id in &ids {
        let names = tag_names(&env, id).await;
        assert_eq!(names.len(), 1);
        assert!(names.contains("Test Tag 2"));
    }
}

#[tokio::test]
async fn batch_add_to_collection_adds_every_project() {
    let env = test_env();
    let ids = projects(&env, 3).await;
    let collection = env
        .services
        .collections
        .create_collection(
            "Test Collection",
            Some("Test collection for batch operations"),
            Some("Test notes"),
        )
        .await
        .unwrap();

    let results = env
        .services
        .collections
        .batch_add_to_collection(&ids, &collection.id)
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (3, 0));
    let (members, count) = collection_project_ids(&env, &collection.id).await;
    assert_eq!(count, 3);
    for id in &ids {
        assert!(members.contains(id));
    }
}

#[tokio::test]
async fn batch_remove_from_collection_removes_only_those_projects() {
    let env = test_env();
    let ids = projects(&env, 3).await;
    let collection = env
        .services
        .collections
        .create_collection("Test Collection", None, None)
        .await
        .unwrap();
    env.services
        .collections
        .batch_add_to_collection(&ids, &collection.id)
        .await
        .unwrap();
    assert_eq!(collection_project_ids(&env, &collection.id).await.1, 3);

    let results = env
        .services
        .collections
        .batch_remove_from_collection(&ids[..2], &collection.id)
        .await
        .unwrap();

    assert!(all_ok(&results));
    let (members, count) = collection_project_ids(&env, &collection.id).await;
    assert_eq!(count, 1);
    assert_eq!(members[0], ids[2]);
}

#[tokio::test]
async fn batch_create_collection_from_projects_holds_them_all() {
    let env = test_env();
    let ids = projects(&env, 2).await;

    let (collection, results) = env
        .services
        .collections
        .batch_create_collection_from(
            "New Collection from Batch",
            &ids,
            None,
            Some("Collection created from batch operation"),
        )
        .await
        .unwrap();

    assert!(all_ok(&results));
    let collection = collection.expect("the collection is created");
    assert_eq!(collection.name, "New Collection from Batch");
    assert_eq!(
        collection.notes.as_deref(),
        Some("Collection created from batch operation")
    );
    assert_eq!(collection.project_count, 2);
    for id in &ids {
        assert!(collection.project_ids.contains(id));
    }
}

#[tokio::test]
async fn batch_update_task_status_completes_every_task() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let mut task_ids = Vec::new();
    for description in ["First task", "Second task", "Third task"] {
        task_ids.push(
            env.services
                .tasks
                .create_task(&project_id, description)
                .await
                .unwrap()
                .0,
        );
    }
    assert_eq!(
        env.services
            .tasks
            .get_project_tasks(&project_id)
            .await
            .unwrap()
            .len(),
        3
    );

    let results = env
        .services
        .tasks
        .batch_update_task_status(&task_ids, true)
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (3, 0));
    for task in env
        .services
        .tasks
        .get_project_tasks(&project_id)
        .await
        .unwrap()
    {
        assert!(task.2, "task {} should be completed", task.0);
    }
}

#[tokio::test]
async fn batch_delete_tasks_removes_only_those_tasks() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let mut task_ids = Vec::new();
    for description in ["First task", "Second task", "Third task"] {
        task_ids.push(
            env.services
                .tasks
                .create_task(&project_id, description)
                .await
                .unwrap()
                .0,
        );
    }

    let results = env
        .services
        .tasks
        .batch_delete_tasks(&task_ids[..2])
        .await
        .unwrap();

    assert!(all_ok(&results));
    assert_eq!(counts(&results), (2, 0));
    let remaining = env
        .services
        .tasks
        .get_project_tasks(&project_id)
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].0, task_ids[2]);
}

/// One bad id does not stop the batch: each item reports for itself.
#[tokio::test]
async fn batch_tag_reports_each_project_separately() {
    let env = test_env();
    let valid = create_test_project_in_db(&env.db).await;
    let mixed = vec![
        valid.clone(),
        "non-existent-project-1".to_string(),
        "non-existent-project-2".to_string(),
    ];
    let tag_id = tag(&env, "Test Tag").await;

    let results = env
        .services
        .tags
        .batch_tag_projects(&mixed, &[tag_id])
        .await
        .unwrap();

    assert_eq!(counts(&results), (1, 2), "only the valid project succeeds");
    assert_eq!(tag_names(&env, &valid).await.len(), 1);
    assert!(tag_names(&env, &valid).await.contains("Test Tag"));
}

#[tokio::test]
async fn batch_operations_compose_into_a_workflow() {
    let env = test_env();
    let ids = projects(&env, 3).await;
    let tag1 = tag(&env, "Workflow Tag 1").await;
    let tag2 = tag(&env, "Workflow Tag 2").await;

    // 1. Tag every project with both tags.
    let results = env
        .services
        .tags
        .batch_tag_projects(&ids, &[tag1.clone(), tag2])
        .await
        .unwrap();
    assert!(all_ok(&results));
    assert_eq!(counts(&results), (6, 0), "3 projects x 2 tags");

    // 2. Make a collection from them.
    let (collection, results) = env
        .services
        .collections
        .batch_create_collection_from(
            "Workflow Collection",
            &ids,
            None,
            Some("Collection created during workflow test"),
        )
        .await
        .unwrap();
    assert!(all_ok(&results));
    let collection_id = collection.unwrap().id;

    // 3. One task per project.
    let mut task_ids = Vec::new();
    for id in &ids {
        task_ids.push(
            env.services
                .tasks
                .create_task(id, &format!("Task for project {}", id))
                .await
                .unwrap()
                .0,
        );
    }

    // 4. Set every task's status.
    let results = env
        .services
        .tasks
        .batch_update_task_status(&task_ids, false)
        .await
        .unwrap();
    assert!(all_ok(&results));
    assert_eq!(counts(&results), (3, 0));

    // 5. Archive two projects.
    let results = env
        .services
        .projects
        .batch_mark_archived(&ids[..2], true)
        .await
        .unwrap();
    assert!(all_ok(&results));
    assert_eq!(counts(&results), (2, 0));

    // 6. Take one out of the collection.
    let results = env
        .services
        .collections
        .batch_remove_from_collection(&ids[2..3], &collection_id)
        .await
        .unwrap();
    assert!(all_ok(&results));
    assert_eq!(counts(&results), (1, 0));

    // 7. Untag one.
    let results = env
        .services
        .tags
        .batch_untag_projects(&ids[..1], &[tag1])
        .await
        .unwrap();
    assert!(all_ok(&results));
    assert_eq!(counts(&results), (1, 0));

    // The end state.
    assert_eq!(archived(&env).await.len(), 2);
    assert_eq!(
        collection_project_ids(&env, &collection_id).await.1,
        2,
        "3 added, 1 removed"
    );
    for id in &ids {
        let tasks = env.services.tasks.get_project_tasks(id).await.unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(!tasks[0].2, "the tasks are not completed");
    }
}
