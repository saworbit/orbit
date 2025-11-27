//! Multipart upload and download operations for large files

use super::client::S3Client;
use super::error::{S3Error, S3Result};
use super::types::{ResumeState, UploadPartInfo};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use bytes::Bytes;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

impl S3Client {
    /// Upload a large file using multipart upload
    ///
    /// # Arguments
    ///
    /// * `local_path` - Path to the local file
    /// * `key` - S3 object key
    /// * `resume_state` - Optional resume state for interrupted uploads
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # use std::path::Path;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    ///
    /// client.upload_file_multipart(
    ///     Path::new("large-file.bin"),
    ///     "remote/large-file.bin",
    ///     None
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file_multipart(
        &self,
        local_path: &Path,
        key: &str,
        resume_state: Option<ResumeState>,
    ) -> S3Result<ResumeState> {
        // Open the file
        let mut file = File::open(local_path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();

        // Initialize or resume multipart upload
        let (upload_id, mut completed_parts) = if let Some(state) = resume_state {
            // Resume from existing upload
            if let Some(uid) = state.upload_id {
                (uid, state.completed_parts)
            } else {
                return Err(S3Error::ResumeState(
                    "Missing upload ID in resume state".to_string(),
                ));
            }
        } else {
            // Start new multipart upload
            let upload_id = self.initiate_multipart_upload(key).await?;
            (upload_id, Vec::new())
        };

        // Calculate parts
        let chunk_size = self.config().chunk_size;
        let total_parts = ((file_size as f64) / (chunk_size as f64)).ceil() as usize;

        // Determine which parts still need to be uploaded
        let uploaded_part_numbers: std::collections::HashSet<i32> =
            completed_parts.iter().map(|p| p.part_number).collect();

        let mut parts_to_upload = Vec::new();
        for part_num in 1..=total_parts {
            if !uploaded_part_numbers.contains(&(part_num as i32)) {
                parts_to_upload.push(part_num);
            }
        }

        // Upload remaining parts with parallelism
        let parallel_uploads = self.config().parallel_operations.min(parts_to_upload.len());
        let mut upload_tasks = Vec::new();

        for part_num in parts_to_upload {
            let offset = (part_num - 1) * chunk_size;
            let size = if part_num == total_parts {
                // Last part might be smaller
                (file_size as usize) - offset
            } else {
                chunk_size
            };

            // Read chunk
            file.seek(std::io::SeekFrom::Start(offset as u64)).await?;
            let mut buffer = vec![0u8; size];
            file.read_exact(&mut buffer).await?;

            // Upload part
            let client = self.clone_for_multipart();
            let key = key.to_string();
            let upload_id_clone = upload_id.clone();

            let task = tokio::spawn(async move {
                client
                    .upload_part(&key, &upload_id_clone, part_num as i32, Bytes::from(buffer))
                    .await
            });

            upload_tasks.push(task);

            // Limit concurrent uploads
            if upload_tasks.len() >= parallel_uploads {
                // Wait for one to complete
                if let Some(task) = upload_tasks.pop() {
                    let part_info = task.await.map_err(|e| {
                        S3Error::MultipartUpload(format!("Task join error: {}", e))
                    })??;
                    completed_parts.push(part_info);
                }
            }
        }

        // Wait for remaining uploads
        for task in upload_tasks {
            let part_info = task
                .await
                .map_err(|e| S3Error::MultipartUpload(format!("Task join error: {}", e)))??;
            completed_parts.push(part_info);
        }

        // Sort parts by part number
        completed_parts.sort_by_key(|p| p.part_number);

        // Complete the multipart upload
        self.complete_multipart_upload(key, &upload_id, &completed_parts)
            .await?;

        // Return final resume state
        Ok(ResumeState {
            upload_id: Some(upload_id),
            completed_parts,
            total_size: file_size,
            chunk_size,
            etag: None,
        })
    }

    /// Initiate a multipart upload
    async fn initiate_multipart_upload(&self, key: &str) -> S3Result<String> {
        let mut request = self
            .aws_client()
            .create_multipart_upload()
            .bucket(self.bucket())
            .key(key);

        // Set storage class
        request = request.storage_class(self.config().storage_class.to_aws());

        // Set server-side encryption
        if let Some(sse) = self.config().server_side_encryption.to_aws() {
            request = request.server_side_encryption(sse);

            if let super::types::S3ServerSideEncryption::AwsKms { key_id: Some(kid) } =
                &self.config().server_side_encryption
            {
                request = request.ssekms_key_id(kid);
            }
        }

        let response = request.send().await.map_err(S3Error::from)?;

        response
            .upload_id()
            .ok_or_else(|| S3Error::MultipartUpload("No upload ID returned".to_string()))
            .map(|s| s.to_string())
    }

    /// Upload a single part
    async fn upload_part(
        &self,
        key: &str,
        upload_id: &str,
        part_number: i32,
        data: Bytes,
    ) -> S3Result<UploadPartInfo> {
        let size = data.len();
        let byte_stream = ByteStream::from(data);

        let response = self
            .aws_client()
            .upload_part()
            .bucket(self.bucket())
            .key(key)
            .upload_id(upload_id)
            .part_number(part_number)
            .body(byte_stream)
            .send()
            .await
            .map_err(S3Error::from)?;

        let etag = response
            .e_tag()
            .ok_or_else(|| S3Error::MultipartUpload("No ETag returned for part".to_string()))?
            .to_string();

        Ok(UploadPartInfo::new(part_number, etag, size))
    }

    /// Complete a multipart upload
    async fn complete_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
        parts: &[UploadPartInfo],
    ) -> S3Result<()> {
        let completed_parts: Vec<CompletedPart> = parts
            .iter()
            .map(|p| {
                CompletedPart::builder()
                    .part_number(p.part_number)
                    .e_tag(&p.etag)
                    .build()
            })
            .collect();

        let multipart_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.aws_client()
            .complete_multipart_upload()
            .bucket(self.bucket())
            .key(key)
            .upload_id(upload_id)
            .multipart_upload(multipart_upload)
            .send()
            .await
            .map_err(S3Error::from)?;

        Ok(())
    }

    /// Abort a multipart upload
    pub async fn abort_multipart_upload(&self, key: &str, upload_id: &str) -> S3Result<()> {
        self.aws_client()
            .abort_multipart_upload()
            .bucket(self.bucket())
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(S3Error::from)?;

        Ok(())
    }

    /// List parts of a multipart upload (for resume)
    pub async fn list_parts(&self, key: &str, upload_id: &str) -> S3Result<Vec<UploadPartInfo>> {
        let response = self
            .aws_client()
            .list_parts()
            .bucket(self.bucket())
            .key(key)
            .upload_id(upload_id)
            .send()
            .await
            .map_err(S3Error::from)?;

        let parts = response
            .parts()
            .iter()
            .filter_map(|p| {
                let part_number = p.part_number()?;
                let etag = p.e_tag()?.to_string();
                let size = p.size()? as usize;
                Some(UploadPartInfo::new(part_number, etag, size))
            })
            .collect();

        Ok(parts)
    }

    /// Download a large file with range requests and resume support
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `local_path` - Path where to save the file
    /// * `resume_offset` - Byte offset to resume from (0 for new download)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit::protocol::s3::{S3Client, S3Config};
    /// # use std::path::Path;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3Config::new("my-bucket".to_string());
    /// let client = S3Client::new(config).await?;
    ///
    /// client.download_file_resumable(
    ///     "remote/large-file.bin",
    ///     Path::new("large-file.bin"),
    ///     0
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_file_resumable(
        &self,
        key: &str,
        local_path: &Path,
        resume_offset: u64,
    ) -> S3Result<()> {
        use std::collections::BTreeMap;

        // Get object metadata to determine size
        let metadata = self.get_metadata(key).await?;
        let total_size = metadata.size;

        if resume_offset >= total_size {
            return Err(S3Error::InvalidRange(format!(
                "Resume offset {} exceeds file size {}",
                resume_offset, total_size
            )));
        }

        // Create parent directories
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open or create file
        let mut file = if resume_offset > 0 {
            let mut f = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false) // Keep existing content when resuming
                .open(local_path)
                .await?;
            f.seek(std::io::SeekFrom::Start(resume_offset)).await?;
            f
        } else {
            File::create(local_path).await?
        };

        // Download with sliding window concurrency
        let chunk_size = self.config().chunk_size as u64;
        let parallel_downloads = self.config().parallel_operations;

        // Track pending downloads by their start offset
        let mut pending_chunks: BTreeMap<u64, tokio::task::JoinHandle<S3Result<Bytes>>> =
            BTreeMap::new();

        // Buffer for completed chunks that arrived out of order
        let mut completed_buffer: BTreeMap<u64, Bytes> = BTreeMap::new();

        let mut next_download_offset = resume_offset;
        let mut next_write_offset = resume_offset;

        loop {
            // Fill the pipeline: spawn tasks up to parallel_downloads limit
            while pending_chunks.len() < parallel_downloads && next_download_offset < total_size {
                let end_offset = (next_download_offset + chunk_size - 1).min(total_size - 1);
                let client = self.clone_for_multipart();
                let key_clone = key.to_string();
                let start = next_download_offset;

                let handle = tokio::spawn(async move {
                    client.download_range(&key_clone, start, end_offset).await
                });

                pending_chunks.insert(next_download_offset, handle);
                next_download_offset = end_offset + 1;
            }

            // Check if we're done
            if pending_chunks.is_empty() && completed_buffer.is_empty() {
                break;
            }

            // First, try to write any buffered chunks that are now sequential
            while let Some(data) = completed_buffer.remove(&next_write_offset) {
                file.write_all(&data).await?;
                next_write_offset += data.len() as u64;
            }

            // Wait for the next sequential chunk if it's in flight
            if let Some(handle) = pending_chunks.remove(&next_write_offset) {
                // This is the chunk we need to write next
                let data = handle
                    .await
                    .map_err(|e| S3Error::Network(format!("Task join error: {}", e)))??;

                file.write_all(&data).await?;
                next_write_offset += data.len() as u64;
            } else if !pending_chunks.is_empty() {
                // Next sequential chunk is not ready yet, but we have other pending downloads
                // Poll for ANY completed task to keep pipeline full

                // Find any completed task
                let mut completed_offset = None;
                for (&offset, handle) in pending_chunks.iter_mut() {
                    if handle.is_finished() {
                        completed_offset = Some(offset);
                        break;
                    }
                }

                if let Some(offset) = completed_offset {
                    // Remove and buffer the completed task
                    if let Some(handle) = pending_chunks.remove(&offset) {
                        let data = handle
                            .await
                            .map_err(|e| S3Error::Network(format!("Task join error: {}", e)))??;
                        // Buffer it for later sequential write
                        completed_buffer.insert(offset, data);
                        // Pipeline slot freed - loop will spawn more downloads
                    }
                } else {
                    // No tasks completed yet, yield briefly
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            }
        }

        file.flush().await?;
        Ok(())
    }

    /// Download a byte range from S3
    async fn download_range(&self, key: &str, start: u64, end: u64) -> S3Result<Bytes> {
        let range = format!("bytes={}-{}", start, end);

        let response = self
            .aws_client()
            .get_object()
            .bucket(self.bucket())
            .key(key)
            .range(range)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("404") || e.to_string().contains("NoSuchKey") {
                    S3Error::NotFound {
                        bucket: self.bucket().to_string(),
                        key: key.to_string(),
                    }
                } else {
                    S3Error::from(e)
                }
            })?;

        let body = response
            .body
            .collect()
            .await
            .map_err(|e| S3Error::Network(format!("Failed to collect response body: {}", e)))?;

        Ok(body.into_bytes())
    }

    /// Clone client for parallel operations
    /// This creates a lightweight clone that shares the underlying HTTP client
    fn clone_for_multipart(&self) -> Self {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::super::config::S3Config;
    use super::*;

    #[tokio::test]
    async fn test_multipart_methods_exist() {
        let config = S3Config::new("test-bucket".to_string());
        let client = S3Client::new(config).await.unwrap();

        // Just verify the methods exist and compile
        // Actual functionality requires integration tests with real S3
        assert_eq!(client.bucket(), "test-bucket");
    }

    #[test]
    fn test_resume_state_creation() {
        let state = ResumeState::new("upload123".to_string(), 1000000, 5242880);
        assert_eq!(state.upload_id, Some("upload123".to_string()));
        assert_eq!(state.total_size, 1000000);
        assert_eq!(state.chunk_size, 5242880);
        assert!(!state.has_progress());
    }

    #[test]
    fn test_upload_part_info() {
        let part = UploadPartInfo::new(1, "etag123".to_string(), 5242880);
        assert_eq!(part.part_number, 1);
        assert_eq!(part.etag, "etag123");
        assert_eq!(part.size, 5242880);
    }
}
