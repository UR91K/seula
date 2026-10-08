//! `CollectionsService` tests. Ported from `tests/grpc/collections.rs` (ADR-0046).

use super::{create_test_project_in_db, test_env, TestEnv};
use seula::database::ProjectScope;
use seula::error::DatabaseError;
use seula::services::CollectionDetail;

const SCOPE: ProjectScope = ProjectScope::All;

async fn create(env: &TestEnv, name: &str) -> CollectionDetail {
    env.services
        .collections
        .create_collection(name, None, None)
        .await
        .unwrap()
}

async fn get(env: &TestEnv, id: &str) -> CollectionDetail {
    env.services
        .collections
        .get_collection(id, SCOPE)
        .await
        .unwrap()
        .expect("the collection exists")
}

async fn add(env: &TestEnv, collection_id: &str, project_id: &str) {
    env.services
        .collections
        .add_project_to_collection(collection_id, project_id)
        .await
        .unwrap();
}

async fn list_all(env: &TestEnv) -> Vec<CollectionDetail> {
    env.services
        .collections
        .list_collections(None, None, None, None, SCOPE)
        .await
        .unwrap()
        .0
}

#[tokio::test]
async fn list_collections_is_empty_to_begin_with() {
    let env = test_env();

    assert!(list_all(&env).await.is_empty());
}

#[tokio::test]
async fn create_collection_returns_the_new_collection() {
    let env = test_env();

    let collection = env
        .services
        .collections
        .create_collection(
            "My Test Collection",
            Some("A collection for testing"),
            Some("Test notes"),
        )
        .await
        .unwrap();

    assert_eq!(collection.name, "My Test Collection");
    assert_eq!(
        collection.description.as_deref(),
        Some("A collection for testing")
    );
    assert_eq!(collection.notes.as_deref(), Some("Test notes"));
    assert!(!collection.id.is_empty());
    assert!(collection.created_at > 0);
    assert!(collection.modified_at > 0);
    assert!(collection.project_ids.is_empty());
}

#[tokio::test]
async fn list_collections_returns_what_was_created() {
    let env = test_env();
    let created = env
        .services
        .collections
        .create_collection("Test Collection", Some("Test Description"), None)
        .await
        .unwrap();

    let collections = list_all(&env).await;

    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].id, created.id);
    assert_eq!(collections[0].name, "Test Collection");
    assert_eq!(
        collections[0].description.as_deref(),
        Some("Test Description")
    );
    assert_eq!(collections[0].notes, None);
}

#[tokio::test]
async fn update_collection_changes_every_field_it_is_given() {
    let env = test_env();
    let created = env
        .services
        .collections
        .create_collection("Original Name", Some("Original Description"), None)
        .await
        .unwrap();

    let updated = env
        .services
        .collections
        .update_collection(
            &created.id,
            Some("Updated Name"),
            Some("Updated Description"),
            Some("Updated Notes"),
        )
        .await
        .unwrap();

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.description.as_deref(), Some("Updated Description"));
    assert_eq!(updated.notes.as_deref(), Some("Updated Notes"));
}

#[tokio::test]
async fn update_collection_leaves_unmentioned_fields_alone() {
    let env = test_env();
    let created = env
        .services
        .collections
        .create_collection(
            "Original Name",
            Some("Original Description"),
            Some("Original Notes"),
        )
        .await
        .unwrap();

    let updated = env
        .services
        .collections
        .update_collection(&created.id, Some("Updated Name Only"), None, None)
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Name Only");
    assert_eq!(updated.description.as_deref(), Some("Original Description"));
    assert_eq!(updated.notes.as_deref(), Some("Original Notes"));
}

#[tokio::test]
async fn update_collection_refuses_a_missing_collection() {
    let env = test_env();

    let result = env
        .services
        .collections
        .update_collection(
            &uuid::Uuid::new_v4().to_string(),
            Some("Should Fail"),
            None,
            None,
        )
        .await;

    assert!(matches!(result, Err(DatabaseError::NotFound(_))));
}

#[tokio::test]
async fn add_project_to_collection_adds_it() {
    let env = test_env();
    let collection = create(&env, "Test Collection").await;
    let project_id = create_test_project_in_db(&env.db).await;

    add(&env, &collection.id, &project_id).await;

    let collections = list_all(&env).await;
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].project_ids, vec![project_id]);
}

#[tokio::test]
async fn projects_are_added_in_order() {
    let env = test_env();
    let collection = create(&env, "Multi-Project Collection").await;
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;

    add(&env, &collection.id, &project1).await;
    add(&env, &collection.id, &project2).await;

    let collections = list_all(&env).await;
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].project_ids, vec![project1, project2]);
}

#[tokio::test]
async fn remove_project_from_collection_removes_it() {
    let env = test_env();
    let collection = create(&env, "Test Collection").await;
    let project_id = create_test_project_in_db(&env.db).await;
    add(&env, &collection.id, &project_id).await;

    env.services
        .collections
        .remove_project_from_collection(&collection.id, &project_id)
        .await
        .unwrap();

    let collections = list_all(&env).await;
    assert_eq!(collections.len(), 1);
    assert!(collections[0].project_ids.is_empty());
}

#[tokio::test]
async fn removing_a_project_keeps_the_others_in_order() {
    let env = test_env();
    let collection = create(&env, "Order Test Collection").await;
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;
    let project3 = create_test_project_in_db(&env.db).await;
    for project in [&project1, &project2, &project3] {
        add(&env, &collection.id, project).await;
    }

    env.services
        .collections
        .remove_project_from_collection(&collection.id, &project2)
        .await
        .unwrap();

    let collections = list_all(&env).await;
    assert_eq!(collections[0].project_ids, vec![project1, project3]);
}

/// The service checks the collection exists before touching it. Without that, this
/// fell through to a raw foreign-key failure that surfaced as an internal error.
#[tokio::test]
async fn add_project_to_collection_refuses_a_missing_collection() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    let result = env
        .services
        .collections
        .add_project_to_collection(&uuid::Uuid::new_v4().to_string(), &project_id)
        .await;

    assert!(matches!(result, Err(DatabaseError::NotFound(_))));
}

#[tokio::test]
async fn remove_project_from_collection_refuses_a_missing_collection() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    let result = env
        .services
        .collections
        .remove_project_from_collection(&uuid::Uuid::new_v4().to_string(), &project_id)
        .await;

    assert!(matches!(result, Err(DatabaseError::NotFound(_))));
}

#[tokio::test]
async fn collection_timestamps_track_creation_and_update() {
    let env = test_env();
    let before_create = chrono::Utc::now().timestamp();

    let collection = create(&env, "Timestamp Test").await;
    let after_create = chrono::Utc::now().timestamp();

    assert!(collection.created_at >= before_create);
    assert!(collection.created_at <= after_create);
    assert!(collection.modified_at >= before_create);
    assert!(collection.modified_at <= after_create);

    // Timestamps are whole seconds, so wait for one to pass.
    tokio::time::sleep(tokio::time::Duration::from_millis(1100)).await;
    let before_update = chrono::Utc::now().timestamp();

    let updated = env
        .services
        .collections
        .update_collection(&collection.id, Some("Updated Name"), None, None)
        .await
        .unwrap();
    let after_update = chrono::Utc::now().timestamp();

    assert_eq!(
        updated.created_at, collection.created_at,
        "creation is fixed"
    );
    assert!(updated.modified_at >= before_update);
    assert!(updated.modified_at <= after_update);
    assert!(updated.modified_at > collection.modified_at);
}

#[tokio::test]
async fn get_collection_returns_its_details() {
    let env = test_env();
    let created = env
        .services
        .collections
        .create_collection(
            "Test Collection for Get",
            Some("Test Description"),
            Some("Test Notes"),
        )
        .await
        .unwrap();

    let collection = get(&env, &created.id).await;

    assert_eq!(collection.id, created.id);
    assert_eq!(collection.name, "Test Collection for Get");
    assert_eq!(collection.description.as_deref(), Some("Test Description"));
    assert_eq!(collection.notes.as_deref(), Some("Test Notes"));
    assert!(collection.project_ids.is_empty());
    assert!(collection.created_at > 0);
    assert!(collection.modified_at > 0);
}

#[tokio::test]
async fn get_collection_finds_nothing_for_an_unknown_id() {
    let env = test_env();

    let found = env
        .services
        .collections
        .get_collection(&uuid::Uuid::new_v4().to_string(), SCOPE)
        .await
        .unwrap();

    assert!(found.is_none());
}

#[tokio::test]
async fn get_collection_lists_its_projects_and_counts_them() {
    let env = test_env();
    let collection = create(&env, "Collection with Projects").await;
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;
    add(&env, &collection.id, &project1).await;
    add(&env, &collection.id, &project2).await;

    let detail = get(&env, &collection.id).await;

    assert_eq!(detail.name, "Collection with Projects");
    assert_eq!(detail.project_ids.len(), 2);
    assert!(detail.project_ids.contains(&project1));
    assert!(detail.project_ids.contains(&project2));
    assert_eq!(detail.project_count, 2);
}

#[tokio::test]
async fn reorder_collection_applies_the_new_order() {
    let env = test_env();
    let collection = env
        .services
        .collections
        .create_collection("Reorder Test Collection", Some("Testing reordering"), None)
        .await
        .unwrap();
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;
    let project3 = create_test_project_in_db(&env.db).await;
    for project in [&project1, &project2, &project3] {
        add(&env, &collection.id, project).await;
    }
    assert_eq!(
        get(&env, &collection.id).await.project_ids,
        vec![project1.clone(), project2.clone(), project3.clone()]
    );

    env.services
        .collections
        .reorder_collection(
            &collection.id,
            &[project3.clone(), project2.clone(), project1.clone()],
            SCOPE,
        )
        .await
        .unwrap();

    assert_eq!(
        get(&env, &collection.id).await.project_ids,
        vec![project3, project2, project1]
    );
}

#[tokio::test]
async fn reorder_collection_refuses_a_different_set_of_projects() {
    let env = test_env();
    let collection = create(&env, "Invalid Reorder Test Collection").await;
    let in_collection = create_test_project_in_db(&env.db).await;
    let outside = create_test_project_in_db(&env.db).await;
    add(&env, &collection.id, &in_collection).await;

    let result = env
        .services
        .collections
        .reorder_collection(&collection.id, &[in_collection.clone(), outside], SCOPE)
        .await;

    match result {
        Err(DatabaseError::InvalidOperation(msg)) => {
            assert!(msg.contains("Project IDs must match exactly"), "{msg}")
        }
        other => panic!("expected InvalidOperation, got {:?}", other),
    }
    assert_eq!(
        get(&env, &collection.id).await.project_ids,
        vec![in_collection],
        "a refused reorder changes nothing"
    );
}

#[tokio::test]
async fn reorder_collection_refuses_a_missing_collection() {
    let env = test_env();

    let result = env
        .services
        .collections
        .reorder_collection(
            "nonexistent-collection-id",
            &["project1".to_string(), "project2".to_string()],
            SCOPE,
        )
        .await;

    match result {
        Err(DatabaseError::NotFound(msg)) => assert!(msg.contains("Collection not found"), "{msg}"),
        other => panic!("expected NotFound, got {:?}", other),
    }
}

#[tokio::test]
async fn list_collections_paginates_and_sorts() {
    let env = test_env();
    for i in 0..5 {
        env.services
            .collections
            .create_collection(
                &format!("Collection {}", i),
                Some(&format!("Description {}", i)),
                None,
            )
            .await
            .unwrap();
    }
    let collections = &env.services.collections;

    let (page, total) = collections
        .list_collections(Some(3), None, None, None, SCOPE)
        .await
        .unwrap();
    assert_eq!(page.len(), 3);
    assert_eq!(total, 5);

    let (page, total) = collections
        .list_collections(Some(2), Some(2), None, None, SCOPE)
        .await
        .unwrap();
    assert_eq!(page.len(), 2);
    assert_eq!(total, 5);

    let (page, total) = collections
        .list_collections(None, None, Some("name".to_string()), Some(true), SCOPE)
        .await
        .unwrap();
    assert_eq!(total, 5);
    let names: Vec<&str> = page.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Collection 4",
            "Collection 3",
            "Collection 2",
            "Collection 1",
            "Collection 0"
        ]
    );
}

#[tokio::test]
async fn search_collections_matches_name_description_and_notes() {
    let env = test_env();
    let collections = &env.services.collections;
    for (name, description, notes) in [
        (
            "Electronic Music",
            "Collection of electronic music projects",
            Some("EDM, techno, house"),
        ),
        ("Rock Band Projects", "Rock and alternative music", None),
        (
            "Jazz Standards",
            "Jazz music collection",
            Some("Traditional jazz standards"),
        ),
        (
            "Film Scoring",
            "Film and video game music",
            Some("Orchestral and cinematic"),
        ),
    ] {
        collections
            .create_collection(name, Some(description), notes)
            .await
            .unwrap();
    }

    for (query, expected) in [
        ("Electronic", "Electronic Music"),
        ("jazz", "Jazz Standards"),
        ("orchestral", "Film Scoring"),
    ] {
        let (found, total) = collections
            .search_collections(query, None, None, SCOPE)
            .await
            .unwrap();
        assert_eq!(found.len(), 1, "{query}");
        assert_eq!(total, 1, "{query}");
        assert_eq!(found[0].name, expected, "{query}");
    }

    // "music" is in all four descriptions; the total ignores the page.
    let (found, total) = collections
        .search_collections("music", Some(2), None, SCOPE)
        .await
        .unwrap();
    assert_eq!(found.len(), 2);
    assert_eq!(total, 4);

    let (found, total) = collections
        .search_collections("music", Some(1), Some(1), SCOPE)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(total, 4);

    let (found, total) = collections
        .search_collections("nonexistent", None, None, SCOPE)
        .await
        .unwrap();
    assert!(found.is_empty());
    assert_eq!(total, 0);
}

#[tokio::test]
async fn collection_statistics_summarise_its_projects() {
    let env = test_env();
    let collection = env
        .services
        .collections
        .create_collection("Test Collection", Some("Test Description"), None)
        .await
        .unwrap();
    for _ in 0..3 {
        let project_id = create_test_project_in_db(&env.db).await;
        add(&env, &collection.id, &project_id).await;
    }

    let stats = env
        .services
        .collections
        .get_collection_statistics(&collection.id, SCOPE)
        .await
        .unwrap();

    assert_eq!(stats.project_count, 3);
    assert!(stats.total_plugins >= 0);
    assert!(stats.total_samples >= 0);
    assert!(stats.total_tags >= 0);
}

#[tokio::test]
async fn statistics_of_an_empty_collection_are_empty() {
    let env = test_env();
    let empty = create(&env, "Empty Collection").await;

    let stats = env
        .services
        .collections
        .get_collection_statistics(&empty.id, SCOPE)
        .await
        .unwrap();

    assert_eq!(stats.project_count, 0);
    assert_eq!(stats.total_plugins, 0);
    assert_eq!(stats.total_samples, 0);
    assert_eq!(stats.total_tags, 0);
    assert!(stats.total_duration_seconds.is_none());
    assert!(stats.average_tempo.is_none());
    assert!(stats.most_common_key.is_none());
    assert!(stats.most_common_time_signature.is_none());
}

#[tokio::test]
async fn duplicate_collection_copies_the_projects_in_order() {
    let env = test_env();
    let original = env
        .services
        .collections
        .create_collection(
            "Original Collection",
            Some("Original description"),
            Some("Original notes"),
        )
        .await
        .unwrap();
    let project1 = create_test_project_in_db(&env.db).await;
    let project2 = create_test_project_in_db(&env.db).await;
    let project3 = create_test_project_in_db(&env.db).await;
    for project in [&project1, &project2, &project3] {
        add(&env, &original.id, project).await;
    }
    env.services
        .collections
        .reorder_collection(
            &original.id,
            &[project3.clone(), project1.clone(), project2.clone()],
            SCOPE,
        )
        .await
        .unwrap();

    let duplicate = env
        .services
        .collections
        .duplicate_collection(
            &original.id,
            "Duplicated Collection",
            Some("New description"),
            None, // keep the original notes
        )
        .await
        .unwrap();

    assert_eq!(duplicate.name, "Duplicated Collection");
    assert_eq!(duplicate.description.as_deref(), Some("New description"));
    assert_eq!(
        duplicate.notes.as_deref(),
        Some("Original notes"),
        "notes not given are inherited"
    );
    assert_ne!(duplicate.id, original.id);
    assert_eq!(duplicate.project_ids.len(), 3);
    assert_eq!(duplicate.project_count, 3);

    let refreshed = get(&env, &original.id).await;
    assert_eq!(refreshed.project_ids, vec![project3, project1, project2]);
    assert_eq!(
        refreshed.project_ids, duplicate.project_ids,
        "same projects, same order"
    );
    assert_eq!(
        get(&env, &duplicate.id).await.project_ids,
        refreshed.project_ids
    );
}

#[tokio::test]
async fn duplicate_collection_can_replace_all_the_metadata() {
    let env = test_env();
    let original = env
        .services
        .collections
        .create_collection(
            "Original Collection",
            Some("Original description"),
            Some("Original notes"),
        )
        .await
        .unwrap();
    let project_id = create_test_project_in_db(&env.db).await;
    add(&env, &original.id, &project_id).await;

    let duplicate = env
        .services
        .collections
        .duplicate_collection(
            &original.id,
            "Fully New Collection",
            Some("Completely new description"),
            Some("Completely new notes"),
        )
        .await
        .unwrap();

    assert_eq!(duplicate.name, "Fully New Collection");
    assert_eq!(
        duplicate.description.as_deref(),
        Some("Completely new description")
    );
    assert_eq!(duplicate.notes.as_deref(), Some("Completely new notes"));
    assert_ne!(duplicate.id, original.id);
    assert_eq!(duplicate.project_ids, vec![project_id]);
}

#[tokio::test]
async fn duplicate_collection_refuses_a_missing_collection() {
    let env = test_env();

    let result = env
        .services
        .collections
        .duplicate_collection("non-existent-id", "Should Fail", None, None)
        .await;

    assert!(result.is_err());
}
