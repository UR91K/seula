use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::database::ProjectDatabase;
use crate::media::{MediaError, MediaFile, MediaStorageManager, MediaType};

#[derive(Clone)]
pub struct MediaService {
    db: Arc<Mutex<ProjectDatabase>>,
    storage: Arc<MediaStorageManager>,
}

impl MediaService {
    pub fn new(db: Arc<Mutex<ProjectDatabase>>, storage: Arc<MediaStorageManager>) -> Self {
        Self { db, storage }
    }

    pub fn storage(&self) -> &Arc<MediaStorageManager> {
        &self.storage
    }

    /// Stores a file and records its metadata, optionally attaching it as a
    /// collection's cover art. Cleans up the stored file if the metadata insert
    /// fails; a failure to attach it to the collection is logged and swallowed
    /// (matches the prior handler behavior -- the upload itself still succeeds).
    pub async fn store_cover_art(
        &self,
        data: &[u8],
        filename: &str,
        collection_id: &str,
    ) -> Result<MediaFile, MediaError> {
        let media_file = self.storage.store_file(data, filename, MediaType::CoverArt)?;

        let mut db = self.db.lock().await;
        if let Err(e) = db.insert_media_file(&media_file) {
            let _ = self
                .storage
                .delete_file(&media_file.id, &media_file.file_extension, &media_file.media_type);
            return Err(e.into());
        }

        if let Err(e) = db.update_collection_cover_art(collection_id, Some(&media_file.id)) {
            tracing::warn!("Failed to set collection cover art: {:?}", e);
        }

        Ok(media_file)
    }

    pub async fn store_audio_file(
        &self,
        data: &[u8],
        filename: &str,
        project_id: &str,
    ) -> Result<MediaFile, MediaError> {
        let media_file = self.storage.store_file(data, filename, MediaType::AudioFile)?;

        let mut db = self.db.lock().await;
        if let Err(e) = db.insert_media_file(&media_file) {
            let _ = self
                .storage
                .delete_file(&media_file.id, &media_file.file_extension, &media_file.media_type);
            return Err(e.into());
        }

        // Added to the list; primary only if the project had none. Uploading used to
        // replace the audio outright (ADR-0037).
        let has_primary = db.get_project_audio_file(project_id).ok().flatten().is_some();
        let attached = if has_primary {
            db.add_project_audio_file(project_id, &media_file.id)
        } else {
            db.update_project_audio_file(project_id, Some(&media_file.id))
        };
        if let Err(e) = attached {
            tracing::warn!("Failed to attach project audio file: {:?}", e);
        }

        Ok(media_file)
    }

    pub async fn get_media_file(&self, media_file_id: &str) -> Result<Option<MediaFile>, MediaError> {
        let db = self.db.lock().await;
        Ok(db.get_media_file(media_file_id)?)
    }

    pub fn file_path(&self, media_file: &MediaFile) -> Result<PathBuf, MediaError> {
        self.storage
            .get_file_path(&media_file.id, &media_file.file_extension, &media_file.media_type)
    }

    /// Deletes a media file's metadata and its physical file. A failure to delete
    /// the physical file is logged and swallowed -- matches prior handler behavior.
    pub async fn delete_media(&self, media_file_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        let media_file = db
            .get_media_file(media_file_id)?
            .ok_or_else(|| MediaError::FileNotFound(media_file_id.to_string()))?;

        db.delete_media_file(media_file_id)?;

        if let Err(e) = self.storage.delete_file(
            &media_file.id,
            &media_file.file_extension,
            &media_file.media_type,
        ) {
            tracing::warn!("Failed to delete physical file from storage: {:?}", e);
        }

        Ok(())
    }

    pub async fn set_collection_cover_art(&self, collection_id: &str, media_file_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        Ok(db.update_collection_cover_art(collection_id, Some(media_file_id))?)
    }

    pub async fn remove_collection_cover_art(&self, collection_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        Ok(db.update_collection_cover_art(collection_id, None)?)
    }

    pub async fn set_project_audio_file(&self, project_id: &str, media_file_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        Ok(db.update_project_audio_file(project_id, Some(media_file_id))?)
    }

    pub async fn remove_project_audio_file(&self, project_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        Ok(db.update_project_audio_file(project_id, None)?)
    }

    /// A project's audition audios, each with whether it is the primary (ADR-0037).
    pub async fn project_audio_files(&self, project_id: &str) -> Result<Vec<(MediaFile, bool)>, MediaError> {
        let db = self.db.lock().await;
        Ok(db.get_project_audio_files(project_id)?)
    }

    /// Attach an already-stored audio to a project's list, without making it primary.
    pub async fn add_project_audio_file(&self, project_id: &str, media_file_id: &str) -> Result<(), MediaError> {
        let mut db = self.db.lock().await;
        let media = db
            .get_media_file(media_file_id)?
            .ok_or_else(|| MediaError::FileNotFound(media_file_id.to_string()))?;
        if media.media_type != MediaType::AudioFile {
            return Err(MediaError::InvalidMediaType(format!("{} is not an audio file", media_file_id)));
        }
        Ok(db.add_project_audio_file(project_id, media_file_id)?)
    }

    /// Take an audio off a project's list; returns false if it was not listed.
    pub async fn remove_project_audio_file_from_list(&self, project_id: &str, media_file_id: &str) -> Result<bool, MediaError> {
        let mut db = self.db.lock().await;
        Ok(db.remove_project_audio_file_from_list(project_id, media_file_id)?)
    }

    pub async fn list_media_files(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<MediaFile>, i32), MediaError> {
        let db = self.db.lock().await;
        let files = db.list_media_files(limit, offset)?;
        let total_count = db.get_media_files_count()?;
        Ok((files, total_count))
    }

    pub async fn get_media_files_by_type(
        &self,
        media_type: &str,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<MediaFile>, i32), MediaError> {
        let db = self.db.lock().await;
        let files = db.get_media_files_by_type(media_type, limit, offset)?;
        let total_count = db.get_media_files_count_by_type(media_type)?;
        Ok((files, total_count))
    }

    pub async fn get_orphaned_media_files(
        &self,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<(Vec<MediaFile>, i32), MediaError> {
        let db = self.db.lock().await;
        let files = db.get_orphaned_media_files(limit, offset)?;
        let total_count = db.get_orphaned_media_files_count()?;
        Ok((files, total_count))
    }

    pub async fn get_media_statistics(&self) -> Result<(i32, i64, i32, i32, i32, i64), MediaError> {
        let db = self.db.lock().await;
        Ok(db.get_media_statistics()?)
    }

    /// Deletes every orphaned media file (or, if `dry_run`, just reports what would
    /// be deleted) and returns the ids removed plus bytes freed.
    pub async fn cleanup_orphaned_media(&self, dry_run: bool) -> Result<(Vec<String>, i64), MediaError> {
        let mut db = self.db.lock().await;
        let orphaned_files = db.get_orphaned_media_files(None, None)?;

        let mut deleted_file_ids = Vec::new();
        let mut bytes_freed = 0i64;

        for file in &orphaned_files {
            if !dry_run {
                if let Err(e) = self
                    .storage
                    .delete_file(&file.id, &file.file_extension, &file.media_type)
                {
                    tracing::warn!("Failed to delete physical file from storage: {:?}", e);
                }
                if let Err(e) = db.delete_media_file(&file.id) {
                    tracing::error!("Failed to delete media file from database: {:?}", e);
                    continue;
                }
            }

            deleted_file_ids.push(file.id.clone());
            bytes_freed += file.file_size_bytes as i64;
        }

        Ok((deleted_file_ids, bytes_freed))
    }
}
