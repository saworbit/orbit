//! gRPC server implementation for the Star agent.
//!
//! This module implements the StarService, which exposes filesystem and
//! compute capabilities to the Nucleus (Hub) over gRPC.

use crate::security::PathJail;
use orbit_core_cdc::hash_file_range;
use orbit_proto::star_service_server::StarService;
use orbit_proto::{
    FileEntry, HandshakeRequest, HandshakeResponse, HashRequest, HashResponse, ReadHeaderRequest,
    ReadHeaderResponse, ScanRequest,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncReadExt;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Implementation of the StarService gRPC interface.
///
/// This struct handles all remote requests from the Nucleus, including:
/// - Handshake/authentication
/// - Directory scanning
/// - File header reading
/// - Content-defined chunking and hashing
pub struct StarImpl {
    /// Security sandbox for path validation
    jail: Arc<PathJail>,
    /// Expected authentication token
    auth_token: String,
    /// Active session ID (set after successful handshake)
    session_id: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Agent version
    version: String,
}

impl StarImpl {
    /// Creates a new StarImpl instance.
    ///
    /// # Arguments
    ///
    /// * `allowed_paths` - Directories that the agent is allowed to access
    /// * `auth_token` - Secret token for authentication
    pub fn new(allowed_paths: Vec<std::path::PathBuf>, auth_token: String) -> Self {
        Self {
            jail: Arc::new(PathJail::new(allowed_paths)),
            auth_token,
            session_id: Arc::new(tokio::sync::RwLock::new(None)),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Validates the session ID from request metadata.
    ///
    /// This is called by all methods except Handshake to ensure the client
    /// has successfully authenticated.
    async fn validate_session<T>(&self, request: &Request<T>) -> Result<(), Status> {
        let session = self.session_id.read().await;

        if session.is_none() {
            return Err(Status::unauthenticated(
                "No active session - call Handshake first",
            ));
        }

        // Check for session_id in metadata
        let metadata = request.metadata();
        if let Some(session_id) = metadata.get("session-id") {
            let provided_session = session_id
                .to_str()
                .map_err(|_| Status::invalid_argument("Invalid session-id format"))?;

            if Some(provided_session.to_string()) == *session {
                return Ok(());
            }
        }

        Err(Status::unauthenticated("Invalid or missing session-id"))
    }
}

#[tonic::async_trait]
impl StarService for StarImpl {
    /// Establishes a session with the Nucleus.
    ///
    /// The client must provide the correct authentication token to receive
    /// a session ID for subsequent requests.
    async fn handshake(
        &self,
        request: Request<HandshakeRequest>,
    ) -> Result<Response<HandshakeResponse>, Status> {
        let req = request.into_inner();

        info!(
            "Handshake request from client version {} with capabilities: {:?}",
            req.version, req.capabilities
        );

        // Validate token
        if req.star_token != self.auth_token {
            warn!("Handshake failed: invalid token");
            return Ok(Response::new(HandshakeResponse {
                accepted: false,
                session_id: String::new(),
                nucleus_version: String::new(),
            }));
        }

        // Generate new session ID
        let new_session_id = Uuid::new_v4().to_string();
        *self.session_id.write().await = Some(new_session_id.clone());

        info!("Handshake successful - session: {}", new_session_id);

        Ok(Response::new(HandshakeResponse {
            accepted: true,
            session_id: new_session_id,
            nucleus_version: self.version.clone(),
        }))
    }

    type ScanDirectoryStream = Pin<Box<dyn Stream<Item = Result<FileEntry, Status>> + Send>>;

    /// Scans a directory and streams back file entries.
    ///
    /// This uses streaming to handle directories with millions of files
    /// without overwhelming memory or network buffers.
    async fn scan_directory(
        &self,
        request: Request<ScanRequest>,
    ) -> Result<Response<Self::ScanDirectoryStream>, Status> {
        self.validate_session(&request).await?;

        let req = request.into_inner();
        let jail = self.jail.clone();

        // Validate and secure the path
        let secure_path = jail
            .secure_path(&req.path)
            .map_err(|e| Status::permission_denied(e.to_string()))?;

        debug!("Scanning directory: {}", secure_path.display());

        // Create a channel for streaming results
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawn a task to walk the directory
        tokio::spawn(async move {
            let mut read_dir = match fs::read_dir(&secure_path).await {
                Ok(rd) => rd,
                Err(e) => {
                    error!("Failed to read directory: {}", e);
                    let _ = tx
                        .send(Err(Status::internal(format!(
                            "Failed to read directory: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            };

            let mut count = 0;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let name = entry.file_name().to_string_lossy().to_string();

                let metadata = match entry.metadata().await {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Failed to get metadata for {}: {}", name, e);
                        continue;
                    }
                };

                let modified_at_ts = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                let file_entry = FileEntry {
                    name,
                    size: metadata.len(),
                    is_dir: metadata.is_dir(),
                    modified_at_ts,
                };

                if tx.send(Ok(file_entry)).await.is_err() {
                    debug!("Client disconnected during scan");
                    break;
                }

                count += 1;
            }

            info!("Scan complete: {} entries", count);
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream) as Self::ScanDirectoryStream))
    }

    /// Reads the first N bytes of a file for magic number detection.
    ///
    /// This is used for semantic analysis without transferring entire files.
    async fn read_header(
        &self,
        request: Request<ReadHeaderRequest>,
    ) -> Result<Response<ReadHeaderResponse>, Status> {
        self.validate_session(&request).await?;

        let req = request.into_inner();

        // Validate path
        let secure_path = self
            .jail
            .secure_path(&req.path)
            .map_err(|e| Status::permission_denied(e.to_string()))?;

        debug!(
            "Reading {} bytes from: {}",
            req.length,
            secure_path.display()
        );

        // Open file and read header
        let mut file = fs::File::open(&secure_path)
            .await
            .map_err(|e| Status::internal(format!("Failed to open file: {}", e)))?;

        let mut buffer = vec![0u8; req.length as usize];
        let bytes_read = file
            .read(&mut buffer)
            .await
            .map_err(|e| Status::internal(format!("Failed to read file: {}", e)))?;

        buffer.truncate(bytes_read);

        Ok(Response::new(ReadHeaderResponse { data: buffer }))
    }

    /// Calculates the BLAKE3 hash of a file range.
    ///
    /// This is the compute-intensive operation that leverages the Star's CPU
    /// for content-defined chunking and deduplication.
    async fn calculate_hash(
        &self,
        request: Request<HashRequest>,
    ) -> Result<Response<HashResponse>, Status> {
        self.validate_session(&request).await?;

        let req = request.into_inner();

        // Validate path
        let secure_path = self
            .jail
            .secure_path(&req.path)
            .map_err(|e| Status::permission_denied(e.to_string()))?;

        debug!(
            "Hashing {} bytes at offset {} from: {}",
            req.length,
            req.offset,
            secure_path.display()
        );

        // Execute hashing (CPU intensive - already async in orbit-core-cdc)
        let hash = hash_file_range(&secure_path, req.offset, req.length)
            .await
            .map_err(|e| Status::internal(format!("Hash calculation failed: {}", e)))?;

        Ok(Response::new(HashResponse {
            hash: hash.to_vec(),
        }))
    }
}
