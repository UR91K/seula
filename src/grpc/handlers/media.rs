use log::{debug, error, info};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

use super::super::collections::*;
use super::super::common::*;
use super::super::media::*;
use crate::media::MediaFile as DomainMediaFile;
use crate::services::MediaService;

fn to_proto(file: DomainMediaFile) -> MediaFile {
    MediaFile {
        id: file.id,
        original_filename: file.original_filename,
        file_extension: file.file_extension,
        media_type: file.media_type.as_str().to_string(),
        file_size_bytes: file.file_size_bytes as i64,
        mime_type: file.mime_type,
        uploaded_at: file.uploaded_at.timestamp(),
        checksum: file.checksum,
    }
}

#[derive(Clone)]
pub struct MediaHandler {
    pub service: MediaService,
}

impl MediaHandler {
    pub fn new(service: MediaService) -> Self {
        Self { service }
    }

    // Media Management - Streaming implementations
    pub async fn upload_cover_art(
        &self,
        request: Request<tonic::Streaming<UploadCoverArtRequest>>,
    ) -> Result<Response<UploadCoverArtResponse>, Status> {
        debug!("UploadCoverArt streaming request received");

        let mut stream = request.into_inner();
        let mut collection_id: Option<String> = None;
        let mut filename: Option<String> = None;
        let mut data_chunks: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.message().await? {
            if let Some(data) = chunk.data {
                match data {
                    upload_cover_art_request::Data::CollectionId(id) => collection_id = Some(id),
                    upload_cover_art_request::Data::Filename(name) => filename = Some(name),
                    upload_cover_art_request::Data::Chunk(bytes) => data_chunks.extend(bytes),
                }
            }
        }

        let collection_id =
            collection_id.ok_or_else(|| Status::invalid_argument("Collection ID is required"))?;
        let filename = filename.ok_or_else(|| Status::invalid_argument("Filename is required"))?;
        if data_chunks.is_empty() {
            return Err(Status::invalid_argument("No file data received"));
        }

        match self.service.store_cover_art(&data_chunks, &filename, &collection_id).await {
            Ok(media_file) => {
                info!(
                    "Successfully uploaded cover art: {} bytes for collection {}",
                    data_chunks.len(),
                    collection_id
                );
                Ok(Response::new(UploadCoverArtResponse {
                    media_file_id: media_file.id,
                    success: true,
                    error_message: None,
                }))
            }
            Err(e) => {
                error!("Failed to store cover art: {:?}", e);
                Ok(Response::new(UploadCoverArtResponse {
                    media_file_id: String::new(),
                    success: false,
                    error_message: Some(e.to_string()),
                }))
            }
        }
    }

    pub async fn upload_audio_file(
        &self,
        request: Request<tonic::Streaming<UploadAudioFileRequest>>,
    ) -> Result<Response<UploadAudioFileResponse>, Status> {
        debug!("UploadAudioFile streaming request received");

        let mut stream = request.into_inner();
        let mut project_id: Option<String> = None;
        let mut filename: Option<String> = None;
        let mut data_chunks: Vec<u8> = Vec::new();

        while let Some(chunk) = stream.message().await? {
            if let Some(data) = chunk.data {
                match data {
                    upload_audio_file_request::Data::ProjectId(id) => project_id = Some(id),
                    upload_audio_file_request::Data::Filename(name) => filename = Some(name),
                    upload_audio_file_request::Data::Chunk(bytes) => data_chunks.extend(bytes),
                }
            }
        }

        let project_id = project_id.ok_or_else(|| Status::invalid_argument("Project ID is required"))?;
        let filename = filename.ok_or_else(|| Status::invalid_argument("Filename is required"))?;
        if data_chunks.is_empty() {
            return Err(Status::invalid_argument("No file data received"));
        }

        match self.service.store_audio_file(&data_chunks, &filename, &project_id).await {
            Ok(media_file) => {
                info!(
                    "Successfully uploaded audio file: {} bytes for project {}",
                    data_chunks.len(),
                    project_id
                );
                Ok(Response::new(UploadAudioFileResponse {
                    media_file_id: media_file.id,
                    success: true,
                    error_message: None,
                }))
            }
            Err(e) => {
                error!("Failed to store audio file: {:?}", e);
                Ok(Response::new(UploadAudioFileResponse {
                    media_file_id: String::new(),
                    success: false,
                    error_message: Some(e.to_string()),
                }))
            }
        }
    }

    pub async fn download_media(
        &self,
        request: Request<DownloadMediaRequest>,
    ) -> Result<Response<ReceiverStream<Result<DownloadMediaResponse, Status>>>, Status> {
        debug!("DownloadMedia request: {:?}", request);
        let req = request.into_inner();

        let media_file = self
            .service
            .get_media_file(&req.media_file_id)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found("Media file not found"))?;

        let (tx, rx) = mpsc::channel(100);

        let file_path = self
            .service
            .file_path(&media_file)
            .map_err(|e| Status::internal(format!("Failed to get file path: {}", e)))?;

        let metadata_response = DownloadMediaResponse {
            data: Some(download_media_response::Data::Metadata(to_proto(media_file))),
        };
        if tx.send(Ok(metadata_response)).await.is_err() {
            return Err(Status::internal("Failed to send metadata"));
        }

        match tokio::fs::read(&file_path).await {
            Ok(file_data) => {
                const CHUNK_SIZE: usize = 64 * 1024;
                for chunk in file_data.chunks(CHUNK_SIZE) {
                    let chunk_response = DownloadMediaResponse {
                        data: Some(download_media_response::Data::Chunk(chunk.to_vec())),
                    };
                    if tx.send(Ok(chunk_response)).await.is_err() {
                        return Err(Status::internal("Failed to send file chunk"));
                    }
                }
            }
            Err(e) => {
                error!("Failed to read file: {:?}", e);
                return Err(Status::internal(format!("Failed to read file: {}", e)));
            }
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    pub async fn delete_media(
        &self,
        request: Request<DeleteMediaRequest>,
    ) -> Result<Response<DeleteMediaResponse>, Status> {
        debug!("DeleteMedia request: {:?}", request);
        let req = request.into_inner();

        match self.service.delete_media(&req.media_file_id).await {
            Ok(()) => {
                info!("Successfully deleted media file: {}", req.media_file_id);
                Ok(Response::new(DeleteMediaResponse {
                    success: true,
                    error_message: None,
                }))
            }
            Err(e) => Ok(Response::new(DeleteMediaResponse {
                success: false,
                error_message: Some(e.to_string()),
            })),
        }
    }

    pub async fn set_collection_cover_art(
        &self,
        request: Request<SetCollectionCoverArtRequest>,
    ) -> Result<Response<SetCollectionCoverArtResponse>, Status> {
        debug!("SetCollectionCoverArt request: {:?}", request);
        let req = request.into_inner();

        match self.service.set_collection_cover_art(&req.collection_id, &req.media_file_id).await {
            Ok(()) => Ok(Response::new(SetCollectionCoverArtResponse {
                success: true,
                error_message: None,
            })),
            Err(e) => {
                error!("Failed to set collection cover art: {:?}", e);
                Ok(Response::new(SetCollectionCoverArtResponse {
                    success: false,
                    error_message: Some(format!("Database error: {}", e)),
                }))
            }
        }
    }

    pub async fn remove_collection_cover_art(
        &self,
        request: Request<RemoveCollectionCoverArtRequest>,
    ) -> Result<Response<RemoveCollectionCoverArtResponse>, Status> {
        debug!("RemoveCollectionCoverArt request: {:?}", request);
        let req = request.into_inner();

        match self.service.remove_collection_cover_art(&req.collection_id).await {
            Ok(()) => Ok(Response::new(RemoveCollectionCoverArtResponse {
                success: true,
                error_message: None,
            })),
            Err(e) => {
                error!("Failed to remove collection cover art: {:?}", e);
                Ok(Response::new(RemoveCollectionCoverArtResponse {
                    success: false,
                    error_message: Some(format!("Database error: {}", e)),
                }))
            }
        }
    }

    pub async fn set_project_audio_file(
        &self,
        request: Request<SetProjectAudioFileRequest>,
    ) -> Result<Response<SetProjectAudioFileResponse>, Status> {
        debug!("SetProjectAudioFile request: {:?}", request);
        let req = request.into_inner();

        match self.service.set_project_audio_file(&req.project_id, &req.media_file_id).await {
            Ok(()) => Ok(Response::new(SetProjectAudioFileResponse {
                success: true,
                error_message: None,
            })),
            Err(e) => {
                error!("Failed to set project audio file: {:?}", e);
                Ok(Response::new(SetProjectAudioFileResponse {
                    success: false,
                    error_message: Some(format!("Database error: {}", e)),
                }))
            }
        }
    }

    pub async fn remove_project_audio_file(
        &self,
        request: Request<RemoveProjectAudioFileRequest>,
    ) -> Result<Response<RemoveProjectAudioFileResponse>, Status> {
        debug!("RemoveProjectAudioFile request: {:?}", request);
        let req = request.into_inner();

        match self.service.remove_project_audio_file(&req.project_id).await {
            Ok(()) => Ok(Response::new(RemoveProjectAudioFileResponse {
                success: true,
                error_message: None,
            })),
            Err(e) => {
                error!("Failed to remove project audio file: {:?}", e);
                Ok(Response::new(RemoveProjectAudioFileResponse {
                    success: false,
                    error_message: Some(format!("Database error: {}", e)),
                }))
            }
        }
    }

    /// List all media files with optional pagination
    pub async fn list_media_files(
        &self,
        request: Request<ListMediaFilesRequest>,
    ) -> Result<Response<ListMediaFilesResponse>, Status> {
        let req = request.into_inner();

        let (media_files, total_count) = self
            .service
            .list_media_files(req.limit, req.offset)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(ListMediaFilesResponse {
            media_files: media_files.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    /// Get media files by type
    pub async fn get_media_files_by_type(
        &self,
        request: Request<GetMediaFilesByTypeRequest>,
    ) -> Result<Response<GetMediaFilesByTypeResponse>, Status> {
        let req = request.into_inner();

        let (media_files, total_count) = self
            .service
            .get_media_files_by_type(&req.media_type, req.limit, req.offset)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(GetMediaFilesByTypeResponse {
            media_files: media_files.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    /// Get orphaned media files
    pub async fn get_orphaned_media_files(
        &self,
        request: Request<GetOrphanedMediaFilesRequest>,
    ) -> Result<Response<GetOrphanedMediaFilesResponse>, Status> {
        let req = request.into_inner();

        let (orphaned_files, total_count) = self
            .service
            .get_orphaned_media_files(req.limit, req.offset)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(GetOrphanedMediaFilesResponse {
            orphaned_files: orphaned_files.into_iter().map(to_proto).collect(),
            total_count,
        }))
    }

    /// Get media statistics
    pub async fn get_media_statistics(
        &self,
        _request: Request<GetMediaStatisticsRequest>,
    ) -> Result<Response<GetMediaStatisticsResponse>, Status> {
        let (total_files, total_size, cover_art_count, audio_file_count, orphaned_count, orphaned_size) = self
            .service
            .get_media_statistics()
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        let mut files_by_type = std::collections::HashMap::new();
        files_by_type.insert("cover_art".to_string(), cover_art_count);
        files_by_type.insert("audio_file".to_string(), audio_file_count);

        Ok(Response::new(GetMediaStatisticsResponse {
            total_files,
            total_size_bytes: total_size,
            cover_art_count,
            audio_file_count,
            orphaned_files_count: orphaned_count,
            orphaned_files_size_bytes: orphaned_size,
            files_by_type,
        }))
    }

    /// Cleanup orphaned media files
    pub async fn cleanup_orphaned_media(
        &self,
        request: Request<CleanupOrphanedMediaRequest>,
    ) -> Result<Response<CleanupOrphanedMediaResponse>, Status> {
        let req = request.into_inner();

        let (deleted_file_ids, bytes_freed) = self
            .service
            .cleanup_orphaned_media(req.dry_run)
            .await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?;

        Ok(Response::new(CleanupOrphanedMediaResponse {
            files_cleaned: deleted_file_ids.len() as i32,
            bytes_freed,
            deleted_file_ids,
            success: true,
            error_message: None,
        }))
    }
}
