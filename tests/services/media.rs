//! `MediaService` tests. Ported from `tests/grpc/media.rs` (ADR-0046).
//!
//! The gRPC download streamed a metadata message and then chunks. Nothing in that
//! survives: the HTTP route serves the file straight from disk. What is left to test
//! at this layer is that a stored file is found and read back intact
//! (`a_stored_file_reads_back_intact`); the route itself is covered with the HTTP tests.

use super::{create_test_project_in_db, test_env, TestEnv};
use seula::media::{MediaError, MediaFile, MediaType};

fn media_file(filename: &str, media_type: MediaType, size: u64, mime: &str) -> MediaFile {
    MediaFile {
        id: uuid::Uuid::new_v4().to_string(),
        original_filename: filename.to_string(),
        file_extension: filename.rsplit('.').next().unwrap().to_string(),
        media_type,
        file_size_bytes: size,
        mime_type: mime.to_string(),
        uploaded_at: chrono::Utc::now(),
        checksum: "test-checksum".to_string(),
    }
}

fn cover_art() -> MediaFile {
    media_file("cover.jpg", MediaType::CoverArt, 1024, "image/jpeg")
}

fn audio() -> MediaFile {
    media_file("demo.mp3", MediaType::AudioFile, 2048, "audio/mpeg")
}

async fn insert(env: &TestEnv, file: &MediaFile) {
    env.db.lock().await.insert_media_file(file).unwrap();
}

async fn collection(env: &TestEnv) -> String {
    env.services
        .collections
        .create_collection("Test Collection", None, None)
        .await
        .unwrap()
        .id
}

#[tokio::test]
async fn set_collection_cover_art_associates_the_file() {
    let env = test_env();
    let collection_id = collection(&env).await;
    let file = cover_art();
    insert(&env, &file).await;

    env.services
        .media
        .set_collection_cover_art(&collection_id, &file.id)
        .await
        .unwrap();

    let cover = env
        .db
        .lock()
        .await
        .get_collection_cover_art(&collection_id)
        .unwrap();
    assert_eq!(cover.expect("cover art is set").id, file.id);
}

#[tokio::test]
async fn set_collection_cover_art_refuses_a_missing_collection() {
    let env = test_env();

    let result = env
        .services
        .media
        .set_collection_cover_art("nonexistent-collection", "some-media-id")
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn remove_collection_cover_art_clears_the_association() {
    let env = test_env();
    let collection_id = collection(&env).await;
    let file = cover_art();
    insert(&env, &file).await;
    env.db
        .lock()
        .await
        .update_collection_cover_art(&collection_id, Some(&file.id))
        .unwrap();

    env.services
        .media
        .remove_collection_cover_art(&collection_id)
        .await
        .unwrap();

    let cover = env
        .db
        .lock()
        .await
        .get_collection_cover_art(&collection_id)
        .unwrap();
    assert!(cover.is_none());
}

#[tokio::test]
async fn set_project_audio_file_associates_the_file() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let file = audio();
    insert(&env, &file).await;

    env.services
        .media
        .set_project_audio_file(&project_id, &file.id)
        .await
        .unwrap();

    let audio = env
        .db
        .lock()
        .await
        .get_project_audio_file(&project_id)
        .unwrap();
    assert_eq!(audio.expect("audio is set").id, file.id);
}

#[tokio::test]
async fn remove_project_audio_file_clears_the_association() {
    let env = test_env();
    let project_id = create_test_project_in_db(&env.db).await;
    let file = audio();
    insert(&env, &file).await;
    env.db
        .lock()
        .await
        .update_project_audio_file(&project_id, Some(&file.id))
        .unwrap();

    env.services
        .media
        .remove_project_audio_file(&project_id)
        .await
        .unwrap();

    let audio = env
        .db
        .lock()
        .await
        .get_project_audio_file(&project_id)
        .unwrap();
    assert!(audio.is_none());
}

#[tokio::test]
async fn delete_media_removes_the_file_record() {
    let env = test_env();
    let file = media_file("test.jpg", MediaType::CoverArt, 1024, "image/jpeg");
    insert(&env, &file).await;

    env.services.media.delete_media(&file.id).await.unwrap();

    assert!(env
        .db
        .lock()
        .await
        .get_media_file(&file.id)
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn delete_media_of_an_unknown_file_is_not_found() {
    let env = test_env();

    let result = env
        .services
        .media
        .delete_media("nonexistent-media-id")
        .await;

    match result {
        Err(MediaError::FileNotFound(_)) => {}
        other => panic!("expected FileNotFound, got {:?}", other),
    }
    let message = MediaError::FileNotFound("x".to_string()).to_string();
    assert!(message.contains("not found"), "{message}");
}

#[tokio::test]
async fn a_stored_file_reads_back_intact() {
    let env = test_env();
    let data = b"test file content for streaming";
    let stored = env
        .media_storage
        .store_file(data, "test.jpg", MediaType::CoverArt)
        .unwrap();
    insert(&env, &stored).await;

    let found = env
        .services
        .media
        .get_media_file(&stored.id)
        .await
        .unwrap()
        .expect("the record exists");
    assert_eq!(found.id, stored.id);
    assert_eq!(found.original_filename, "test.jpg");
    assert_eq!(found.media_type.as_str(), "cover_art");

    let path = env.services.media.file_path(&found).unwrap();
    assert_eq!(std::fs::read(path).unwrap(), data);
}

#[tokio::test]
async fn get_media_file_finds_nothing_for_an_unknown_id() {
    let env = test_env();

    let found = env
        .services
        .media
        .get_media_file("nonexistent-media-id")
        .await
        .unwrap();

    assert!(found.is_none());
}
