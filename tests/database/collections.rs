//! Database collections functionality tests

use super::*;
use seula::database::ProjectScope;
use crate::common::{create_test_live_set_from_parse, setup, LiveSetBuilder};

// TODO: Create database-level collection tests

#[test]
fn test_collections() {
    setup("error");
    let mut db =
        ProjectDatabase::new(PathBuf::from(":memory:")).expect("Failed to create database");

    // Create three test projects with different characteristics
    let edm_project = create_test_live_set_from_parse(
        "EDM Project.als",
        LiveSetBuilder::new()
            .with_plugin("Serum")
            .with_tempo(140.0)
            .build(),
    );

    let rock_project = create_test_live_set_from_parse(
        "Rock Band.als",
        LiveSetBuilder::new()
            .with_plugin("Guitar Rig 6")
            .with_tempo(120.0)
            .build(),
    );

    let ambient_project = create_test_live_set_from_parse(
        "Ambient Soundscape.als",
        LiveSetBuilder::new()
            .with_plugin("Omnisphere")
            .with_tempo(80.0)
            .build(),
    );

    // Insert all projects
    db.insert_project(&edm_project)
        .expect("Failed to insert EDM project");
    db.insert_project(&rock_project)
        .expect("Failed to insert rock project");
    db.insert_project(&ambient_project)
        .expect("Failed to insert ambient project");

    // Test creating a collection
    let collection_id = db
        .create_collection(
            "Electronic Music",
            Some("Collection of electronic music projects"),
            None,
        )
        .expect("Failed to create collection");

    // Test listing collections
    let (collections, total_count) = db.list_collections(None, None, None, None, ProjectScope::All).expect("Failed to list collections");
    assert_eq!(collections.len(), 1);
    assert_eq!(total_count, 1);
    let (id, name, description) = &collections[0];
    assert_eq!(id, &collection_id);
    assert_eq!(name, "Electronic Music");
    assert_eq!(
        description.as_ref(),
        Some(&"Collection of electronic music projects".to_string())
    );

    // Test adding projects to collection
    db.add_project_to_collection(&collection_id, &edm_project.id.to_string())
        .expect("Failed to add EDM project");
    db.add_project_to_collection(&collection_id, &ambient_project.id.to_string())
        .expect("Failed to add ambient project");
    db.add_project_to_collection(&collection_id, &rock_project.id.to_string())
        .expect("Failed to add rock project");

    // Test retrieving projects in order
    let projects = db
        .get_collection_projects(&collection_id, ProjectScope::All)
        .expect("Failed to get collection projects");
    assert_eq!(projects.len(), 3);
    assert_eq!(projects[0].name, "EDM Project.als");
    assert_eq!(projects[1].name, "Ambient Soundscape.als");
    assert_eq!(projects[2].name, "Rock Band.als");

    // Test reordering projects
    db.reorder_project_in_collection(&collection_id, &rock_project.id.to_string(), 0)
        .expect("Failed to reorder project");

    let projects = db
        .get_collection_projects(&collection_id, ProjectScope::All)
        .expect("Failed to get collection projects after reorder");
    assert_eq!(projects.len(), 3);
    assert_eq!(projects[0].name, "Rock Band.als");
    assert_eq!(projects[1].name, "EDM Project.als");
    assert_eq!(projects[2].name, "Ambient Soundscape.als");

    // Test removing a project
    db.remove_project_from_collection(&collection_id, &ambient_project.id.to_string())
        .expect("Failed to remove project");

    let projects = db
        .get_collection_projects(&collection_id, ProjectScope::All)
        .expect("Failed to get collection projects after removal");
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].name, "Rock Band.als");
    assert_eq!(projects[1].name, "EDM Project.als");

    // Test deleting collection
    db.delete_collection(&collection_id)
        .expect("Failed to delete collection");
    let (collections, total_count) = db
        .list_collections(None, None, None, None, ProjectScope::All)
        .expect("Failed to list collections after deletion");
    assert_eq!(collections.len(), 0);
    assert_eq!(total_count, 0);
}

#[test]
fn test_duplicate_collection() {
    setup("error");
    let mut db =
        ProjectDatabase::new(PathBuf::from(":memory:")).expect("Failed to create database");

    // Create test projects
    let project1 = create_test_live_set_from_parse(
        "Project 1.als",
        LiveSetBuilder::new()
            .with_plugin("Serum")
            .with_tempo(140.0)
            .build(),
    );

    let project2 = create_test_live_set_from_parse(
        "Project 2.als",
        LiveSetBuilder::new()
            .with_plugin("Guitar Rig 6")
            .with_tempo(120.0)
            .build(),
    );

    let project3 = create_test_live_set_from_parse(
        "Project 3.als",
        LiveSetBuilder::new()
            .with_plugin("Omnisphere")
            .with_tempo(80.0)
            .build(),
    );

    // Insert projects
    db.insert_project(&project1).expect("Failed to insert project 1");
    db.insert_project(&project2).expect("Failed to insert project 2");
    db.insert_project(&project3).expect("Failed to insert project 3");

    // Create original collection
    let original_collection_id = db
        .create_collection(
            "Original Collection",
            Some("Original description"),
            Some("Original notes"),
        )
        .expect("Failed to create original collection");

    // Add projects to original collection
    db.add_project_to_collection(&original_collection_id, &project1.id.to_string())
        .expect("Failed to add project 1");
    db.add_project_to_collection(&original_collection_id, &project2.id.to_string())
        .expect("Failed to add project 2");
    db.add_project_to_collection(&original_collection_id, &project3.id.to_string())
        .expect("Failed to add project 3");

    // Reorder projects in original collection
    db.reorder_project_in_collection(&original_collection_id, &project3.id.to_string(), 0)
        .expect("Failed to reorder project");

    // Duplicate the collection
    let duplicated_collection_id = db
        .duplicate_collection(
            &original_collection_id,
            "Duplicated Collection",
            Some("New description"),
            None, // Use original notes
        )
        .expect("Failed to duplicate collection");

    // Verify the duplicated collection exists
    let (collections, total_count) = db
        .list_collections(None, None, None, None, ProjectScope::All)
        .expect("Failed to list collections");
    assert_eq!(collections.len(), 2);
    assert_eq!(total_count, 2);

    // Get both collections
    let original_collection = db
        .get_collection_by_id(&original_collection_id, ProjectScope::All)
        .expect("Failed to get original collection")
        .expect("Original collection not found");

    let duplicated_collection = db
        .get_collection_by_id(&duplicated_collection_id, ProjectScope::All)
        .expect("Failed to get duplicated collection")
        .expect("Duplicated collection not found");

    // Verify collection metadata
    assert_eq!(original_collection.1, "Original Collection");
    assert_eq!(duplicated_collection.1, "Duplicated Collection");
    assert_eq!(original_collection.2, Some("Original description".to_string()));
    assert_eq!(duplicated_collection.2, Some("New description".to_string()));
    assert_eq!(original_collection.3, Some("Original notes".to_string()));
    assert_eq!(duplicated_collection.3, Some("Original notes".to_string())); // Should inherit original notes

    // Verify both collections have the same projects in the same order
    assert_eq!(original_collection.6.len(), 3);
    assert_eq!(duplicated_collection.6.len(), 3);
    assert_eq!(original_collection.6, duplicated_collection.6);

    // Verify project order is preserved
    let original_projects = db
        .get_collection_projects(&original_collection_id, ProjectScope::All)
        .expect("Failed to get original collection projects");
    let duplicated_projects = db
        .get_collection_projects(&duplicated_collection_id, ProjectScope::All)
        .expect("Failed to get duplicated collection projects");

    assert_eq!(original_projects.len(), 3);
    assert_eq!(duplicated_projects.len(), 3);
    assert_eq!(original_projects[0].name, "Project 3.als");
    assert_eq!(duplicated_projects[0].name, "Project 3.als");
    assert_eq!(original_projects[1].name, "Project 1.als");
    assert_eq!(duplicated_projects[1].name, "Project 1.als");
    assert_eq!(original_projects[2].name, "Project 2.als");
    assert_eq!(duplicated_projects[2].name, "Project 2.als");

    // Verify collection IDs are different
    assert_ne!(original_collection_id, duplicated_collection_id);

    // Test duplicating with all new metadata
    let fully_new_collection_id = db
        .duplicate_collection(
            &original_collection_id,
            "Fully New Collection",
            Some("Completely new description"),
            Some("Completely new notes"),
        )
        .expect("Failed to duplicate collection with new metadata");

    let fully_new_collection = db
        .get_collection_by_id(&fully_new_collection_id, ProjectScope::All)
        .expect("Failed to get fully new collection")
        .expect("Fully new collection not found");

    assert_eq!(fully_new_collection.1, "Fully New Collection");
    assert_eq!(fully_new_collection.2, Some("Completely new description".to_string()));
    assert_eq!(fully_new_collection.3, Some("Completely new notes".to_string()));

    // Test duplicating non-existent collection
    let result = db.duplicate_collection(
        "non-existent-id",
        "Should Fail",
        None,
        None,
    );
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------------
// The project scope (ADR-0043), the counted sorts, and tasks that carry both the
// project's id and its name.

/// Inserts a project with the given length, archived or not, and returns its id.
fn add_project(db: &mut ProjectDatabase, name: &str, seconds: i64, active: bool) -> String {
    let project = create_test_live_set_from_parse(name, LiveSetBuilder::new().build());
    db.insert_project(&project).expect("insert project");
    let id = project.id.to_string();
    db.conn
        .execute(
            "UPDATE projects SET duration_seconds = ?1, is_active = ?2 WHERE id = ?3",
            rusqlite::params![seconds, active, id],
        )
        .expect("set length and state");
    id
}

/// "mixed" holds One (100s), Gone (400s, archived) and Two (200s), in that order,
/// and each has one task. "trio" holds One, Two and Three (50s), all active.
struct Fixture {
    db: ProjectDatabase,
    mixed: String,
    trio: String,
    one: String,
    gone: String,
    two: String,
}

fn fixture() -> Fixture {
    let mut db = ProjectDatabase::new(PathBuf::from(":memory:")).expect("in-memory database");
    let one = add_project(&mut db, "One.als", 100, true);
    let gone = add_project(&mut db, "Gone.als", 400, false);
    let two = add_project(&mut db, "Two.als", 200, true);
    let three = add_project(&mut db, "Three.als", 50, true);

    let mixed = db.create_collection("mixed", None, None).unwrap();
    for id in [&one, &gone, &two] {
        db.add_project_to_collection(&mixed, id).unwrap();
        db.add_task(id, "a task").unwrap();
    }
    let trio = db.create_collection("trio", None, None).unwrap();
    for id in [&one, &two, &three] {
        db.add_project_to_collection(&trio, id).unwrap();
    }
    Fixture { db, mixed, trio, one, gone, two }
}

fn ids(db: &mut ProjectDatabase, collection: &str, scope: ProjectScope) -> Vec<String> {
    db.get_collection_by_id(collection, scope).unwrap().unwrap().6
}

/// Sorting the list by project count used to fail with "no such column:
/// project_count"; the collections table has no such column.
#[test]
fn the_list_sorts_by_project_count_and_length_in_either_scope() {
    let Fixture { mut db, mixed, trio, .. } = fixture();
    let mut order = |sort: &str, scope| -> Vec<String> {
        db.list_collections(None, None, Some(sort.to_string()), Some(true), scope)
            .expect("the sort runs")
            .0
            .into_iter()
            .map(|(id, _, _)| id)
            .collect()
    };

    // Active: trio has 3 projects and 350s, mixed has 2 and 300s.
    assert_eq!(order("project_count", ProjectScope::Active), [trio.clone(), mixed.clone()]);
    assert_eq!(order("total_duration", ProjectScope::Active), [trio.clone(), mixed.clone()]);
    // All: mixed has 3 and 700s. The count ties at 3, so the name decides.
    assert_eq!(order("project_count", ProjectScope::All), [mixed.clone(), trio.clone()]);
    assert_eq!(order("total_duration", ProjectScope::All), [mixed, trio]);
}

#[test]
fn archived_members_are_left_out_of_a_collection_in_the_active_scope() {
    let Fixture { mut db, mixed, one, gone, two, .. } = fixture();

    assert_eq!(ids(&mut db, &mixed, ProjectScope::Active), [one.clone(), two.clone()]);
    assert_eq!(ids(&mut db, &mixed, ProjectScope::All), [one.clone(), gone.clone(), two.clone()]);

    assert_eq!(db.get_collection_statistics(&mixed, ProjectScope::Active).unwrap(), (Some(300.0), 2));
    assert_eq!(db.get_collection_statistics(&mixed, ProjectScope::All).unwrap(), (Some(700.0), 3));

    let detailed = db.get_collection_detailed_statistics(&mixed, ProjectScope::Active).unwrap();
    assert_eq!(detailed.project_count, 2);
    assert_eq!(detailed.total_duration_seconds, Some(300.0));

    let active = db.get_collection_projects(&mixed, ProjectScope::Active).unwrap();
    let names: Vec<_> = active.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["One.als", "Two.als"]);

    let tasks = db.get_collection_tasks(&mixed, ProjectScope::Active).unwrap();
    let task_projects: Vec<_> = tasks.iter().map(|t| t.1.clone()).collect();
    assert_eq!(task_projects, [one, two]);
    assert_eq!(db.get_collection_tasks(&mixed, ProjectScope::All).unwrap().len(), 3);
}

/// A collection's projects used to all come back `is_active: true`, archived or not.
#[test]
fn a_collections_projects_say_whether_they_are_archived() {
    let Fixture { mut db, mixed, gone, .. } = fixture();
    let all = db.get_collection_projects(&mixed, ProjectScope::All).unwrap();
    for project in &all {
        assert_eq!(project.is_active, project.id.to_string() != gone, "{}", project.name);
    }
}

/// Tasks carry the project's id and its name; the name used to stand in for the id.
#[test]
fn collection_tasks_carry_the_project_id_and_name() {
    let Fixture { mut db, mixed, one, .. } = fixture();
    let tasks = db.get_collection_tasks(&mixed, ProjectScope::Active).unwrap();
    let (_, project_id, project_name, description, completed, _) = &tasks[0];
    assert_eq!(project_id, &one);
    assert_eq!(project_name, "One.als");
    assert_eq!(description, "a task");
    assert!(!completed);
}

/// Under the active scope the caller reorders what it was shown; the archived
/// member keeps its slot, so unarchiving it puts it back where it was.
#[tokio::test]
async fn reordering_the_active_members_keeps_the_archived_ones_in_place() {
    let Fixture { db, mixed, one, gone, two, .. } = fixture();
    let db = std::sync::Arc::new(tokio::sync::Mutex::new(db));
    let service = seula::services::CollectionsService::new(db.clone());

    service
        .reorder_collection(&mixed, &[two.clone(), one.clone()], ProjectScope::Active)
        .await
        .expect("the active set reorders");
    let mut guard = db.lock().await;
    assert_eq!(ids(&mut guard, &mixed, ProjectScope::All), [two.clone(), gone.clone(), one.clone()]);
    drop(guard);

    let with_archived = [one.clone(), gone.clone(), two.clone()];
    assert!(
        service.reorder_collection(&mixed, &with_archived, ProjectScope::Active).await.is_err(),
        "the active scope takes the active set, not every member"
    );
    service
        .reorder_collection(&mixed, &with_archived, ProjectScope::All)
        .await
        .expect("the all scope takes every member");
    let mut guard = db.lock().await;
    assert_eq!(ids(&mut guard, &mixed, ProjectScope::All), with_archived);
}
