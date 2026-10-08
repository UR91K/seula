//! `TagsService` tests. Ported from `tests/grpc/tags.rs` (ADR-0046).

use super::{create_test_project_in_db, test_env};
use seula::error::DatabaseError;

#[tokio::test]
async fn list_tags_is_empty_to_begin_with() {
    let env = test_env();

    assert!(env.services.tags.list_tags().await.unwrap().is_empty());
}

#[tokio::test]
async fn create_tag_returns_the_new_row() {
    let env = test_env();

    let (id, name, created_at) = env.services.tags.create_tag("Electronic").await.unwrap();

    assert_eq!(name, "Electronic");
    assert!(!id.is_empty());
    let now = chrono::Utc::now().timestamp();
    assert!(
        (created_at - now).abs() < 5,
        "the timestamp should be recent"
    );
}

#[tokio::test]
async fn list_tags_is_sorted_by_name() {
    let env = test_env();
    for name in ["Rock", "Electronic", "Ambient"] {
        env.services.tags.create_tag(name).await.unwrap();
    }

    let tags = env.services.tags.list_tags().await.unwrap();

    let names: Vec<&str> = tags.iter().map(|t| t.1.as_str()).collect();
    assert_eq!(names, vec!["Ambient", "Electronic", "Rock"]);
    for (id, name, created_at) in &tags {
        assert!(!id.is_empty());
        assert!(!name.is_empty());
        assert!(*created_at > 0);
    }
}

#[tokio::test]
async fn tag_project_applies_the_tag() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env
        .services
        .tags
        .create_tag("Work In Progress")
        .await
        .unwrap();

    let tag = env
        .services
        .tags
        .tag_project(&project_id, &tag_id)
        .await
        .unwrap();

    assert_eq!(tag.1, "Work In Progress", "it returns the tag it applied");
    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert_eq!(project_tags.len(), 1);
    assert!(project_tags.contains("Work In Progress"));
}

#[tokio::test]
async fn untag_project_removes_only_that_tag() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag1, _, _) = env.services.tags.create_tag("Tag 1").await.unwrap();
    let (tag2, _, _) = env.services.tags.create_tag("Tag 2").await.unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag1)
        .await
        .unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag2)
        .await
        .unwrap();
    {
        let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
        assert_eq!(project_tags.len(), 2);
    }

    let removed = env
        .services
        .tags
        .untag_project(&project_id, &tag1)
        .await
        .unwrap();

    assert_eq!(removed.1, "Tag 1");
    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert_eq!(project_tags.len(), 1);
    assert!(project_tags.contains("Tag 2"));
    assert!(!project_tags.contains("Tag 1"));
}

#[tokio::test]
async fn tag_project_refuses_a_missing_project() {
    let env = test_env();
    let (tag_id, _, _) = env.services.tags.create_tag("Test Tag").await.unwrap();

    let result = env
        .services
        .tags
        .tag_project("non-existent-project-id", &tag_id)
        .await;

    assert!(matches!(result, Err(DatabaseError::NotFound(_))));
}

#[tokio::test]
async fn tag_project_refuses_a_missing_tag() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;

    let result = env
        .services
        .tags
        .tag_project(&project_id, "non-existent-tag-id")
        .await;

    assert!(matches!(result, Err(DatabaseError::NotFound(_))));
    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert!(project_tags.is_empty());
}

#[tokio::test]
async fn create_tag_refuses_a_duplicate_name() {
    let env = test_env();
    env.services.tags.create_tag("Duplicate Tag").await.unwrap();

    let result = env.services.tags.create_tag("Duplicate Tag").await;

    assert!(result.is_err());
    assert_eq!(env.services.tags.list_tags().await.unwrap().len(), 1);
}

#[tokio::test]
async fn tag_project_is_idempotent() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env
        .services
        .tags
        .create_tag("Idempotent Tag")
        .await
        .unwrap();

    for _ in 0..3 {
        env.services
            .tags
            .tag_project(&project_id, &tag_id)
            .await
            .unwrap();
    }

    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert_eq!(project_tags.len(), 1);
    assert!(project_tags.contains("Idempotent Tag"));
}

#[tokio::test]
async fn update_tag_renames_it() {
    let env = test_env();
    let (tag_id, _, _) = env.services.tags.create_tag("Original Tag").await.unwrap();

    let updated = env
        .services
        .tags
        .update_tag(&tag_id, "Updated Tag")
        .await
        .unwrap();

    assert_eq!(updated.1, "Updated Tag");
    assert_eq!(updated.0, tag_id);
    let tags = env.services.tags.list_tags().await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].1, "Updated Tag");
    assert_eq!(tags[0].0, tag_id);
}

#[tokio::test]
async fn update_tag_refuses_a_missing_tag() {
    let env = test_env();

    let result = env
        .services
        .tags
        .update_tag("non-existent-tag-id", "New Name")
        .await;

    assert!(result.is_err());
}

/// The old gRPC test said "should be allowed", so the service does not forbid it. If
/// that is ever wrong, this is the test to change, deliberately.
#[tokio::test]
async fn update_tag_accepts_an_empty_name() {
    let env = test_env();
    let (tag_id, _, _) = env.services.tags.create_tag("Test Tag").await.unwrap();

    let updated = env.services.tags.update_tag(&tag_id, "").await.unwrap();

    assert_eq!(updated.1, "");
    assert_eq!(updated.0, tag_id);
    let tags = env.services.tags.list_tags().await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].1, "");
}

#[tokio::test]
async fn delete_tag_removes_only_that_tag() {
    let env = test_env();
    let (tag1, _, _) = env.services.tags.create_tag("Tag 1").await.unwrap();
    let (tag2, _, _) = env.services.tags.create_tag("Tag 2").await.unwrap();
    assert_eq!(env.services.tags.list_tags().await.unwrap().len(), 2);

    env.services.tags.delete_tag(&tag1).await.unwrap();

    let tags = env.services.tags.list_tags().await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].1, "Tag 2");
    assert_eq!(tags[0].0, tag2);
}

#[tokio::test]
async fn delete_tag_of_a_missing_tag_is_a_no_op() {
    let env = test_env();

    env.services
        .tags
        .delete_tag("non-existent-tag-id")
        .await
        .unwrap();
}

#[tokio::test]
async fn delete_tag_removes_its_project_associations() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env
        .services
        .tags
        .create_tag("Associated Tag")
        .await
        .unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag_id)
        .await
        .unwrap();
    assert_eq!(
        env.db
            .lock()
            .await
            .get_project_tags(&project_id)
            .unwrap()
            .len(),
        1
    );

    env.services.tags.delete_tag(&tag_id).await.unwrap();

    assert!(env.services.tags.list_tags().await.unwrap().is_empty());
    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert!(project_tags.is_empty(), "the association cascades away");
}

#[tokio::test]
async fn update_tag_keeps_its_project_associations() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env.services.tags.create_tag("Original Tag").await.unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag_id)
        .await
        .unwrap();

    let updated = env
        .services
        .tags
        .update_tag(&tag_id, "Updated Tag")
        .await
        .unwrap();

    assert_eq!(updated.1, "Updated Tag");
    assert_eq!(updated.0, tag_id);
    let project_tags = env.db.lock().await.get_project_tags(&project_id).unwrap();
    assert_eq!(project_tags.len(), 1);
    assert!(project_tags.contains("Updated Tag"));
    assert!(!project_tags.contains("Original Tag"));
}
