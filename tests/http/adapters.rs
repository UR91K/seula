//! What the HTTP handlers add to the services' answers. See `tests/http_tests.rs`.

use axum::http::{Method, StatusCode};
use serde_json::json;

use super::{app, send, send_json};
use crate::common::fixtures::create_test_project_in_db;
use seula::media::MediaType;

// Status codes. The services answer with `DatabaseError`s; the router decides what
// they mean to a client (`NotFound` is 404, `InvalidOperation` is 400).

#[tokio::test]
async fn an_unknown_project_is_a_404() {
    let (app, _env) = app();

    let reply = send(
        &app,
        Method::GET,
        "/api/v1/projects/00000000-0000-0000-0000-000000000000",
    )
    .await;

    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_malformed_project_id_is_a_400_to_delete() {
    let (app, _env) = app();

    let reply = send(
        &app,
        Method::DELETE,
        "/api/v1/projects/non-existent-project-id",
    )
    .await;

    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}

/// The old gRPC answer was `success: false`; here it is a refusal, and the project
/// stays.
#[tokio::test]
async fn an_active_project_cannot_be_permanently_deleted() {
    let (app, env) = app();
    let project_id = create_test_project_in_db(&env.db).await;

    let refused = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/projects/{project_id}/permanent"),
    )
    .await;
    assert_eq!(refused.status, StatusCode::BAD_REQUEST);
    let still_there = send(&app, Method::GET, &format!("/api/v1/projects/{project_id}")).await;
    assert_eq!(still_there.status, StatusCode::OK);

    send(
        &app,
        Method::DELETE,
        &format!("/api/v1/projects/{project_id}"),
    )
    .await;
    let deleted = send(
        &app,
        Method::DELETE,
        &format!("/api/v1/projects/{project_id}/permanent"),
    )
    .await;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    let gone = send(&app, Method::GET, &format!("/api/v1/projects/{project_id}")).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn renaming_a_missing_project_changes_nothing_and_succeeds() {
    let (app, _env) = app();

    let reply = send_json(
        &app,
        Method::PUT,
        "/api/v1/projects/non-existent-project-id/name",
        json!({ "name": "New Name" }),
    )
    .await;

    assert_eq!(reply.status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn collection_failures_map_to_404_and_400() {
    let (app, env) = app();
    let missing = uuid::Uuid::new_v4();

    let get = send(&app, Method::GET, &format!("/api/v1/collections/{missing}")).await;
    assert_eq!(get.status, StatusCode::NOT_FOUND);

    let update = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/collections/{missing}"),
        json!({ "name": "Should Fail" }),
    )
    .await;
    assert_eq!(update.status, StatusCode::NOT_FOUND);

    let reorder_missing = send_json(
        &app,
        Method::PUT,
        "/api/v1/collections/nonexistent-collection-id/reorder",
        json!({ "project_ids": ["project1", "project2"] }),
    )
    .await;
    assert_eq!(reorder_missing.status, StatusCode::NOT_FOUND);

    let collection = env
        .services
        .collections
        .create_collection("Invalid Reorder", None, None)
        .await
        .unwrap();
    let inside = create_test_project_in_db(&env.db).await;
    let outside = create_test_project_in_db(&env.db).await;
    env.services
        .collections
        .add_project_to_collection(&collection.id, &inside)
        .await
        .unwrap();
    let reorder_wrong_set = send_json(
        &app,
        Method::PUT,
        &format!("/api/v1/collections/{}/reorder", collection.id),
        json!({ "project_ids": [inside, outside] }),
    )
    .await;
    assert_eq!(reorder_wrong_set.status, StatusCode::BAD_REQUEST);
}

// What a response carries beyond the service's rows.

/// A service returns a project's tags as names; the response carries them as `{id,
/// name, created_at}`, which only the adapter can fill in.
#[tokio::test]
async fn a_project_comes_back_with_its_tags_plugins_samples_and_version() {
    let (app, env) = app();
    let project_id = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env.services.tags.create_tag("Electronic").await.unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag_id)
        .await
        .unwrap();

    let reply = send(&app, Method::GET, &format!("/api/v1/projects/{project_id}")).await;

    assert_eq!(reply.status, StatusCode::OK);
    let project = reply.json();
    assert_eq!(project["id"], project_id.as_str());
    assert!(!project["name"].as_str().unwrap().is_empty());
    assert!(!project["path"].as_str().unwrap().is_empty());
    assert!(!project["hash"].as_str().unwrap().is_empty());
    assert!(project["created_at"].as_i64().unwrap() > 0);
    assert!(project["modified_at"].as_i64().unwrap() > 0);
    assert_eq!(project["tempo"], 140.0);
    assert_eq!(project["time_signature"]["numerator"], 4);
    assert_eq!(project["time_signature"]["denominator"], 4);
    assert!(project["key_signature"].is_object());
    assert_eq!(project["ableton_version"]["major"], 11);
    assert_eq!(project["tags"].as_array().unwrap().len(), 1);
    assert_eq!(project["tags"][0]["id"], tag_id.as_str());
    assert_eq!(project["tags"][0]["name"], "Electronic");
    assert!(project["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["name"].as_str().unwrap().contains("Serum")));
    assert!(project["samples"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["name"].as_str().unwrap().contains("kick")));
}

#[tokio::test]
async fn search_results_carry_their_tags() {
    let (app, env) = app();
    let tagged = create_test_project_in_db(&env.db).await;
    create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env.services.tags.create_tag("Electronic").await.unwrap();
    env.services
        .tags
        .tag_project(&tagged, &tag_id)
        .await
        .unwrap();

    let reply = send(&app, Method::GET, "/api/v1/search?query=tag:Electronic").await;

    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.json();
    assert_eq!(body["total_count"], 1);
    let projects = body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], tagged.as_str());
    assert_eq!(projects[0]["tags"][0]["name"], "Electronic");
}

#[tokio::test]
async fn batch_responses_count_successes_and_failures() {
    let (app, env) = app();
    let valid = create_test_project_in_db(&env.db).await;
    let (tag_id, _, _) = env.services.tags.create_tag("Test Tag").await.unwrap();

    let reply = send_json(
        &app,
        Method::POST,
        "/api/v1/tags/batch-tag",
        json!({
            "project_ids": [valid, "non-existent-project-1", "non-existent-project-2"],
            "tag_ids": [tag_id],
        }),
    )
    .await;

    assert_eq!(reply.status, StatusCode::OK);
    let body = reply.json();
    assert_eq!(body["successful_count"], 1);
    assert_eq!(body["failed_count"], 2);
    assert_eq!(body["results"].as_array().unwrap().len(), 3);
    let failed: Vec<_> = body["results"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["success"] == false)
        .collect();
    assert_eq!(failed.len(), 2);
    assert!(failed.iter().all(|r| r["error_message"].is_string()));
}

#[tokio::test]
async fn importing_projects_reports_each_failure_and_the_totals() {
    let (app, _env) = app();

    let none = send_json(
        &app,
        Method::POST,
        "/api/v1/projects/add-multiple",
        json!({ "file_paths": [] }),
    )
    .await
    .json();
    assert_eq!(none["success"], true, "nothing to import is not a failure");
    assert_eq!(none["total_requested"], 0);
    assert_eq!(none["successful_imports"], 0);
    assert_eq!(none["failed_imports"], 0);

    let bad = send_json(
        &app,
        Method::POST,
        "/api/v1/projects/add-multiple",
        json!({ "file_paths": ["nonexistent_file.als", "another_nonexistent_file.als"] }),
    )
    .await
    .json();
    assert_eq!(bad["success"], false);
    assert_eq!(bad["total_requested"], 2);
    assert_eq!(bad["successful_imports"], 0);
    assert_eq!(bad["failed_imports"], 2);
    assert_eq!(bad["projects"].as_array().unwrap().len(), 0);
    assert_eq!(bad["failed_paths"].as_array().unwrap().len(), 2);
    assert_eq!(
        bad["error_messages"],
        json!(["File does not exist", "File does not exist"])
    );
}

#[tokio::test]
async fn rescanning_a_project_whose_file_is_gone_reports_it_in_the_body() {
    let (app, env) = app();
    let project_id = create_test_project_in_db(&env.db).await;

    let reply = send_json(
        &app,
        Method::POST,
        &format!("/api/v1/projects/{project_id}/rescan"),
        json!({ "force_rescan": true }),
    )
    .await;

    assert_eq!(
        reply.status,
        StatusCode::OK,
        "a missing file is a result, not an error"
    );
    let body = reply.json();
    assert_eq!(body["success"], false);
    assert_eq!(body["was_updated"], false);
    assert!(body["error_message"]
        .as_str()
        .unwrap()
        .contains("Project file not found"));
    assert!(body["scan_summary"]
        .as_str()
        .unwrap()
        .contains("Project file no longer exists"));
}

// Media.

#[tokio::test]
async fn a_stored_file_downloads_whole_under_its_own_name() {
    let (app, env) = app();
    let data = b"test file content for streaming";
    let stored = env
        .media_storage
        .store_file(data, "test.jpg", MediaType::CoverArt)
        .unwrap();
    env.db.lock().await.insert_media_file(&stored).unwrap();

    let reply = send(&app, Method::GET, &format!("/api/v1/media/{}", stored.id)).await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body, data);
    assert_eq!(reply.headers["content-type"], "image/jpeg");
    let disposition = reply.headers["content-disposition"].to_str().unwrap();
    assert!(disposition.contains("test.jpg"), "{disposition}");
}

#[tokio::test]
async fn an_unknown_media_file_is_a_404_to_download() {
    let (app, _env) = app();

    let download = send(&app, Method::GET, "/api/v1/media/nonexistent-media-id").await;

    assert_eq!(download.status, StatusCode::NOT_FOUND);
}

/// The media mutations report failure in a 200 body (`success: false` and a message),
/// as the gRPC calls did, where every other route answers with a status code. Pinned as
/// it is (the `media-mutations-200-on-failure` bug in `docs/bugs.md`).
#[tokio::test]
async fn media_mutations_report_failure_in_the_body() {
    let (app, _env) = app();

    let delete = send(&app, Method::DELETE, "/api/v1/media/nonexistent-media-id").await;
    assert_eq!(delete.status, StatusCode::OK);
    let body = delete.json();
    assert_eq!(body["success"], false);
    assert!(body["error_message"]
        .as_str()
        .unwrap()
        .contains("not found"));

    let set_cover = send_json(
        &app,
        Method::PUT,
        "/api/v1/collections/nonexistent-collection/cover-art",
        json!({ "media_file_id": "some-media-id" }),
    )
    .await;
    assert_eq!(set_cover.status, StatusCode::OK);
    let body = set_cover.json();
    assert_eq!(body["success"], false);
    assert!(body["error_message"].is_string());
}

// System: the routes whose answers are built from the scan state and the statistics.

#[tokio::test]
async fn scan_status_reports_the_shared_state_by_name() {
    let (app, env) = app();

    let idle = send(&app, Method::GET, "/api/v1/system/scan-status")
        .await
        .json();
    assert_eq!(idle["status"], "unknown");
    assert!(idle["current_progress"].is_null());

    *env.system.scan_progress_handle().lock().await = Some(seula::services::ScanProgress {
        completed: 50,
        total: 100,
        progress: 0.5,
        message: "Test progress".to_string(),
        status: seula::services::ScanStatus::ScanningPlugins,
    });
    *env.system.scan_status_handle().lock().await = seula::services::ScanStatus::ScanningPlugins;

    let running = send(&app, Method::GET, "/api/v1/system/scan-status")
        .await
        .json();
    assert_eq!(running["status"], "scanning_plugins");
    assert_eq!(running["current_progress"]["completed"], 50);
    assert_eq!(running["current_progress"]["status"], "scanning_plugins");
    assert_eq!(running["current_progress"]["message"], "Test progress");
}

/// The projects the statistics embed are converted like any other project in a
/// response, with their tags and audio.
#[tokio::test]
async fn statistics_embed_whole_projects() {
    let (app, env) = app();
    let project_id = create_test_project_in_db(&env.db).await;
    // Only a project with a duration can be the longest.
    env.db
        .lock()
        .await
        .conn
        .execute(
            "UPDATE projects SET duration_seconds = 120 WHERE id = ?",
            [&project_id],
        )
        .unwrap();
    let (tag_id, _, _) = env.services.tags.create_tag("Electronic").await.unwrap();
    env.services
        .tags
        .tag_project(&project_id, &tag_id)
        .await
        .unwrap();

    let reply = send(&app, Method::GET, "/api/v1/system/statistics").await;

    assert_eq!(reply.status, StatusCode::OK);
    let stats = reply.json();
    assert_eq!(stats["projects"]["active"], 1);
    assert_eq!(stats["plugins"]["total"], 1);
    assert_eq!(stats["samples"]["total"], 1);
    let complex = stats["most_complex_projects"].as_array().unwrap();
    assert_eq!(complex.len(), 1);
    assert_eq!(complex[0]["project"]["id"], project_id.as_str());
    assert_eq!(complex[0]["project"]["tags"][0]["name"], "Electronic");
    assert_eq!(stats["longest_project"]["id"], project_id.as_str());
}

#[tokio::test]
async fn statistics_of_an_empty_library_embed_no_projects() {
    let (app, _env) = app();

    let stats = send(&app, Method::GET, "/api/v1/system/statistics")
        .await
        .json();

    assert_eq!(stats["projects"]["total"], 0);
    assert!(stats["longest_project"].is_null());
    assert_eq!(stats["most_complex_projects"], json!([]));
    assert!(stats["largest_collection"].is_null());
}

#[tokio::test]
async fn statistics_export_as_csv() {
    let (app, env) = app();
    create_test_project_in_db(&env.db).await;

    let reply = send(&app, Method::GET, "/api/v1/system/statistics/export").await;

    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["content-type"], "text/csv");
    let csv = String::from_utf8(reply.body).unwrap();
    assert!(
        csv.starts_with("Category,Value\nTotal Projects,1\n"),
        "{csv}"
    );
    assert!(csv.contains("Top Plugins\n"));
}
