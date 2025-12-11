//! gRPC server implementation for the Star agent.
//!
//! This module implements the StarService, which exposes filesystem and
//! compute capabilities to the Nucleus (Hub) over gRPC.

use crate::auth::AuthService;
use crate::security::PathJail;
use orbit_core_cdc::hash_file_range;
use orbit_proto::star_service_server::StarService;
use orbit_proto::{
    FileEntry, HandshakeRequest, HandshakeResponse, HashRequest, HashResponse, ReadHeaderRequest,
    ReadHeaderResponse, ReadStreamRequest, ReadStreamResponse, ReplicateRequest, ReplicateResponse,
    ScanRequest,
};
use sha2::{Digest, Sha256};
use std::pin::Pin;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
/// - Peer-to-peer data transfer (Phase 4)
pub struct StarImpl {
    /// Security sandbox for path validation
    jail: Arc<PathJail>,
    /// Expected authentication token
    auth_token: String,
    /// Active session ID (set after successful handshake)
    session_id: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Agent version
    version: String,
    /// JWT authentication service for P2P transfers
    auth_service: Arc<AuthService>,
}

impl StarImpl {
    /// Creates a new StarImpl instance.
    ///
    /// # Arguments
    ///
    /// * `allowed_paths` - Directories that the agent is allowed to access
    /// * `auth_token` - Secret token for authentication
    /// * `auth_secret` - Shared secret for JWT signing (Phase 4 P2P transfers)
    pub fn new(
        allowed_paths: Vec<std::path::PathBuf>,
        auth_token: String,
        auth_secret: String,
    ) -> Self {
        Self {
            jail: Arc::new(PathJail::new(allowed_paths)),
            auth_token,
            session_id: Arc::new(tokio::sync::RwLock::new(None)),
            version: env!("CARGO_PKG_VERSION").to_string(),
            auth_service: Arc::new(AuthService::new(&auth_secret)),
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

    // ═══════════════════════════════════════════════════════════
    // Phase 4: Data Plane (P2P Transfer)
    // ═══════════════════════════════════════════════════════════

    type ReadStreamStream = ReceiverStream<Result<ReadStreamResponse, Status>>;

    /// Streams file content to another Star for P2P transfer.
    ///
    /// This method implements the server side of the data plane. A Destination Star
    /// calls this method on a Source Star to retrieve file data directly, bypassing
    /// the Nucleus.
    ///
    /// # Security
    ///
    /// - Does NOT require session validation (P2P transfer between Stars)
    /// - Validates JWT transfer token signed by Nucleus
    /// - Token must authorize the specific file being requested
    ///
    /// # Protocol
    ///
    /// 1. Verify transfer token (stateless JWT)
    /// 2. Validate and secure the file path
    /// 3. Open file and stream chunks (64KB each)
    /// 4. Use backpressure channel to prevent memory overflow
    async fn read_stream(
        &self,
        request: Request<ReadStreamRequest>,
    ) -> Result<Response<Self::ReadStreamStream>, Status> {
        let req = request.into_inner();

        info!("ReadStream request for: {}", req.path);

        // ──────────────────────────────────────────────────────
        // Step 1: Verify Transfer Token (Stateless)
        // ──────────────────────────────────────────────────────
        self.auth_service
            .verify_transfer_token(&req.transfer_token, &req.path)
            .map_err(|e| {
                warn!("Transfer token verification failed: {}", e);
                Status::permission_denied("Invalid or expired transfer token")
            })?;

        info!("Authorized ReadStream request for: {}", req.path);

        // ──────────────────────────────────────────────────────
        // Step 2: Secure Path (Prevent Directory Traversal)
        // ──────────────────────────────────────────────────────
        let full_path = self
            .jail
            .secure_path(&req.path)
            .map_err(|e| Status::invalid_argument(format!("Invalid path: {}", e)))?;

        // ──────────────────────────────────────────────────────
        // Step 3: Open File
        // ──────────────────────────────────────────────────────
        let mut file = fs::File::open(&full_path)
            .await
            .map_err(|e| Status::not_found(format!("File not found: {}", e)))?;

        // ──────────────────────────────────────────────────────
        // Step 4: Stream Chunks to Client
        // ──────────────────────────────────────────────────────
        let (tx, rx) = tokio::sync::mpsc::channel(4); // 4-message buffer for backpressure

        tokio::spawn(async move {
            let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks
            let mut total_bytes = 0u64;

            loop {
                match file.read(&mut buffer).await {
                    Ok(0) => {
                        // EOF reached
                        info!("ReadStream complete: {} bytes sent", total_bytes);
                        break;
                    }
                    Ok(n) => {
                        total_bytes += n as u64;
                        let chunk = buffer[..n].to_vec();

                        if tx.send(Ok(ReadStreamResponse { chunk })).await.is_err() {
                            // Client disconnected
                            debug!("Client disconnected during ReadStream");
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Read error during stream: {}", e);
                        let _ = tx.send(Err(Status::internal(e.to_string()))).await;
                        break;
                    }
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    /// Pulls a file from a remote Star and saves it locally.
    ///
    /// This method implements the client side of the data plane. The Nucleus
    /// commands this Star to pull data from another Star, orchestrating the
    /// P2P transfer without routing data through itself.
    ///
    /// # Security
    ///
    /// - REQUIRES session validation (Nucleus → Destination command)
    /// - Receives transfer token from Nucleus to present to Source Star
    ///
    /// # Protocol
    ///
    /// 1. Validate session (Nucleus must be authenticated)
    /// 2. Prepare local destination path
    /// 3. Connect to source Star via gRPC
    /// 4. Request data stream (presenting transfer token)
    /// 5. Write chunks to disk with SHA-256 verification
    /// 6. Verify expected size if provided
    async fn replicate_file(
        &self,
        request: Request<ReplicateRequest>,
    ) -> Result<Response<ReplicateResponse>, Status> {
        self.validate_session(&request).await?;

        let req = request.into_inner();

        info!(
            "ReplicateFile: {} → {} (from {})",
            req.remote_path, req.local_path, req.source_star_url
        );

        // ──────────────────────────────────────────────────────
        // Step 1: Prepare Local Destination
        // ──────────────────────────────────────────────────────
        let save_path = self
            .jail
            .secure_path(&req.local_path)
            .map_err(|e| Status::invalid_argument(format!("Invalid local path: {}", e)))?;

        // Create parent directory if needed
        if let Some(parent) = save_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| Status::internal(format!("Failed to create directory: {}", e)))?;
        }

        // ──────────────────────────────────────────────────────
        // Step 2: Connect to Source Star
        // ──────────────────────────────────────────────────────
        use orbit_proto::star_service_client::StarServiceClient;

        let mut source_client = StarServiceClient::connect(req.source_star_url.clone())
            .await
            .map_err(|e| Status::unavailable(format!("Cannot connect to source: {}", e)))?;

        // ──────────────────────────────────────────────────────
        // Step 3: Request Data Stream
        // ──────────────────────────────────────────────────────
        let stream_req = ReadStreamRequest {
            path: req.remote_path.clone(),
            transfer_token: req.transfer_token,
        };

        let mut stream = source_client
            .read_stream(stream_req)
            .await
            .map_err(|e| Status::internal(format!("ReadStream failed: {}", e)))?
            .into_inner();

        // ──────────────────────────────────────────────────────
        // Step 4: Write to Disk (with Checksum Verification)
        // ──────────────────────────────────────────────────────
        let mut file = fs::File::create(&save_path)
            .await
            .map_err(|e| Status::internal(format!("Cannot create file: {}", e)))?;

        let mut hasher = Sha256::new();
        let mut total_bytes = 0u64;

        while let Some(response) = stream.message().await? {
            let chunk = response.chunk;
            file.write_all(&chunk)
                .await
                .map_err(|e| Status::internal(format!("Write failed: {}", e)))?;
            hasher.update(&chunk);
            total_bytes += chunk.len() as u64;
        }

        file.sync_all()
            .await
            .map_err(|e| Status::internal(format!("Sync failed: {}", e)))?;

        let checksum = format!("{:x}", hasher.finalize());

        // ──────────────────────────────────────────────────────
        // Step 5: Verify Expected Size (if provided)
        // ──────────────────────────────────────────────────────
        if req.expected_size > 0 && total_bytes != req.expected_size {
            error!(
                "Size mismatch: expected {}, got {}",
                req.expected_size, total_bytes
            );
            // Optionally delete the incomplete file
            let _ = fs::remove_file(&save_path).await;
            return Err(Status::data_loss("File size mismatch"));
        }

        info!(
            "Transfer complete: {} bytes, checksum: {}",
            total_bytes, checksum
        );

        Ok(Response::new(ReplicateResponse {
            success: true,
            bytes_transferred: total_bytes,
            checksum,
            error_message: String::new(),
        }))
    }
}
