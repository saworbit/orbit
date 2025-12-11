//! RemoteSystem: OrbitSystem implementation that proxies to a remote Star via gRPC

use async_trait::async_trait;
use orbit_core_interface::{FileMetadata, OrbitSystem, OrbitSystemError, Result};
use orbit_proto::{
    star_service_client::StarServiceClient, HashRequest, ReadHeaderRequest, ScanRequest,
};
use std::path::Path;
use tonic::transport::Channel;
use tracing::{debug, warn};

use crate::error::ConnectError;

/// A remote implementation of OrbitSystem that delegates operations to a Star node via gRPC.
///
/// This struct is cheaply cloneable (internally uses Arc via tonic's Channel).
///
/// # Example
///
/// ```rust,no_run
/// use orbit_connect::RemoteSystem;
/// use orbit_core_interface::OrbitSystem;
/// use tonic::transport::Channel;
///
/// # async fn example() -> anyhow::Result<()> {
/// let channel = Channel::from_static("http://10.0.0.5:50051").connect().await?;
/// let system = RemoteSystem::new(channel, "session-abc123".to_string());
///
/// // Use it like any OrbitSystem
/// if system.exists(std::path::Path::new("/data/file.bin")).await {
///     println!("File exists on remote Star");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct RemoteSystem {
    /// The gRPC client (cheap to clone due to Arc internally)
    client: StarServiceClient<Channel>,

    /// Session ID obtained from handshake, attached to every request
    session_id: String,
}

impl RemoteSystem {
    /// Create a new RemoteSystem with an established channel and session ID.
    ///
    /// # Arguments
    ///
    /// * `channel` - A connected gRPC channel to the Star
    /// * `session_id` - The session ID obtained from the handshake response
    ///
    /// # Note
    ///
    /// This constructor assumes the handshake has already been performed.
    /// Use `StarManager` for automatic handshake and connection management.
    pub fn new(channel: Channel, session_id: String) -> Self {
        let client = StarServiceClient::new(channel);
        Self { client, session_id }
    }

    /// Helper to attach session metadata to a request
    fn with_session<T>(&self, request: T) -> tonic::Request<T> {
        let mut req = tonic::Request::new(request);

        // Attach session ID to metadata
        match self.session_id.parse() {
            Ok(value) => {
                req.metadata_mut().insert("x-orbit-session", value);
            }
            Err(e) => {
                warn!("Failed to parse session_id as metadata: {}", e);
            }
        }

        req
    }
}

#[async_trait]
impl OrbitSystem for RemoteSystem {
    // ═══════════════════════════════════════════════════════════════════════
    // 1. Discovery Operations
    // ═══════════════════════════════════════════════════════════════════════

    async fn exists(&self, path: &Path) -> bool {
        // We can implement this by attempting to get metadata
        // A proper implementation might add a dedicated Exists RPC in the future
        self.metadata(path).await.is_ok()
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        // For metadata of a single file, we can use read_dir on the parent
        // and find the specific file. However, this is inefficient.
        // A better approach would be to add a GetMetadata RPC.
        //
        // For now, we'll implement a simple version that reads the directory
        // and filters. This should be optimized in Phase 3.5.

        let parent = path.parent().ok_or_else(|| {
            OrbitSystemError::System("Cannot get metadata of root path".to_string())
        })?;

        let file_name = path
            .file_name()
            .ok_or_else(|| OrbitSystemError::System("Path has no file name".to_string()))?;

        let entries = self.read_dir(parent).await?;

        entries
            .into_iter()
            .find(|e| e.path.file_name() == Some(file_name))
            .ok_or_else(|| OrbitSystemError::NotFound(path.to_path_buf()))
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<FileMetadata>> {
        let request = ScanRequest {
            path: path.to_string_lossy().to_string(),
        };

        debug!("Scanning remote directory: {}", path.display());

        let req = self.with_session(request);

        let response = self
            .client
            .clone()
            .scan_directory(req)
            .await
            .map_err(ConnectError::from)?;

        let mut stream = response.into_inner();
        let mut results = Vec::new();

        while let Some(entry) = stream.message().await.map_err(ConnectError::from)? {
            let entry_path = path.join(&entry.name);

            results.push(FileMetadata {
                path: entry_path,
                len: entry.size,
                is_dir: entry.is_dir,
                modified: std::time::UNIX_EPOCH
                    + std::time::Duration::from_secs(entry.modified_at_ts),
            });
        }

        debug!("Scanned {} entries from {}", results.len(), path.display());

        Ok(results)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 2. Data Access Operations
    // ═══════════════════════════════════════════════════════════════════════

    async fn reader(&self, _path: &Path) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        // Phase 3 Design Decision: We do NOT implement streaming reads through the Nucleus.
        // This forces the use of Star-to-Star transfer (Phase 4).
        //
        // Rationale: Reading through the Nucleus would double network usage and CPU overhead.
        // Instead, Phase 4 will implement direct Star-to-Star transfer.

        Err(OrbitSystemError::System(
            "Direct read from Nucleus not supported in Phase 3. Use Grid Transfer (Phase 4)."
                .to_string(),
        ))
    }

    async fn writer(&self, _path: &Path) -> Result<Box<dyn tokio::io::AsyncWrite + Unpin + Send>> {
        // Same rationale as reader: we want Star-to-Star writes, not through Nucleus

        Err(OrbitSystemError::System(
            "Direct write from Nucleus not supported in Phase 3. Use Grid Transfer (Phase 4)."
                .to_string(),
        ))
    }

    // ═══════════════════════════════════════════════════════════════════════
    // 3. Compute Offloading Operations
    // ═══════════════════════════════════════════════════════════════════════

    async fn read_header(&self, path: &Path, len: usize) -> Result<Vec<u8>> {
        let request = ReadHeaderRequest {
            path: path.to_string_lossy().to_string(),
            length: len as u32,
        };

        debug!(
            "Reading header from remote file: {} ({} bytes)",
            path.display(),
            len
        );

        let req = self.with_session(request);

        let response = self
            .client
            .clone()
            .read_header(req)
            .await
            .map_err(ConnectError::from)?;

        let data = response.into_inner().data;

        debug!("Received {} bytes of header data", data.len());

        Ok(data)
    }

    async fn calculate_hash(&self, path: &Path, offset: u64, len: u64) -> Result<[u8; 32]> {
        let request = HashRequest {
            path: path.to_string_lossy().to_string(),
            offset,
            length: len,
        };

        debug!(
            "Calculating hash on remote Star: {} (offset={}, len={})",
            path.display(),
            offset,
            len
        );

        let req = self.with_session(request);

        let response = self
            .client
            .clone()
            .calculate_hash(req)
            .await
            .map_err(ConnectError::from)?;

        let hash_bytes = response.into_inner().hash;

        // Convert Vec<u8> to [u8; 32]
        let hash_array: [u8; 32] = hash_bytes.try_into().map_err(|v: Vec<u8>| {
            OrbitSystemError::System(format!(
                "Invalid hash length from Star: expected 32 bytes, got {}",
                v.len()
            ))
        })?;

        debug!("Received hash: {}", hex::encode(hash_array));

        Ok(hash_array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_system_is_send_sync() {
        // Compile-time check that RemoteSystem satisfies trait bounds
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RemoteSystem>();
    }
}
