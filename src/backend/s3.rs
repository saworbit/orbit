//! S3 backend implementation via `object_store`
//!
//! Provides the unified `Backend` interface for AWS S3 and S3-compatible
//! storage (MinIO, LocalStack, R2, etc.) by delegating to the `object_store`
//! crate. This keeps the cloud Backend abstraction consistent across S3, Azure
//! Blob, and Google Cloud Storage.
//!
//! For features beyond the unified Backend surface (presigned URLs, versioning,
//! per-PUT storage class/SSE selection, batch APIs, etc.), enable the `s3-cli`
//! feature which adds the `orbit s3 ...` subcommand tree backed by aws-sdk-s3.

use super::config::S3BackendConfig;
use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{StreamExt, TryStreamExt};
use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
use object_store::{
    path::Path as ObjectPath, Attribute, AttributeValue, Attributes, ObjectMeta, ObjectStore,
    PutMode, PutMultipartOpts, PutOptions, PutPayload, WriteMultipart,
};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

/// Threshold above which uploads stream via multipart instead of buffering.
/// 8 MiB sits comfortably above S3's 5 MiB minimum part size.
const MULTIPART_THRESHOLD: u64 = 8 * 1024 * 1024;
/// Per-part chunk size for multipart streaming uploads.
const MULTIPART_CHUNK_SIZE: usize = 8 * 1024 * 1024;
/// Read buffer size used to feed the multipart writer.
const READ_BUF_SIZE: usize = 64 * 1024;
/// Maximum in-flight part uploads (backpressure cap).
const MULTIPART_CONCURRENCY: usize = 4;
/// Channel buffer for the lazy list stream.
const LIST_CHANNEL_CAPACITY: usize = 64;

/// S3 backend using the `object_store` crate
pub struct S3Backend {
    store: Arc<dyn ObjectStore>,
    /// Prefix applied to all paths (like a "root" directory)
    prefix: Option<String>,
}

impl S3Backend {
    /// Create a new S3 backend from configuration
    pub async fn new(config: S3BackendConfig) -> BackendResult<Self> {
        let store = build_store(&config)?;
        Ok(Self {
            store: Arc::new(store),
            prefix: None,
        })
    }

    /// Create a new S3 backend with a prefix applied to all operations
    pub async fn with_prefix(
        config: S3BackendConfig,
        prefix: impl Into<String>,
    ) -> BackendResult<Self> {
        let mut backend = Self::new(config).await?;
        backend.prefix = Some(prefix.into());
        Ok(backend)
    }

    /// Convert a Path to an `ObjectPath` (object_store's normalized key).
    ///
    /// The same key is used for both exact-object operations and as a
    /// list-prefix. `ObjectPath` normalizes by stripping leading and trailing
    /// slashes, and `object_store`'s list APIs are documented to match on a
    /// path-segment basis — i.e. a prefix `dir` matches `dir/x` but never
    /// `dir2/x`. The list client also appends the path delimiter to the
    /// prefix before sending, so there is no "sibling prefix" leak risk.
    fn path_to_key(&self, path: &Path) -> ObjectPath {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let key = path_str.trim_start_matches('/');
        let full = match &self.prefix {
            Some(p) => format!("{}/{}", p.trim_end_matches('/'), key),
            None => key.to_string(),
        };
        ObjectPath::from(full)
    }

    #[allow(dead_code)]
    fn key_to_path(&self, object_path: &ObjectPath) -> PathBuf {
        strip_prefix(object_path.as_ref(), self.prefix.as_deref())
    }

    fn convert_meta(&self, meta: &ObjectMeta) -> Metadata {
        let mut metadata = Metadata::file(meta.size as u64);
        metadata.modified = Some(meta.last_modified.into());
        metadata.etag = meta.e_tag.clone();
        metadata
    }
}

fn build_store(config: &S3BackendConfig) -> BackendResult<object_store::aws::AmazonS3> {
    let mut builder = AmazonS3Builder::from_env().with_bucket_name(&config.bucket);

    if let Some(region) = &config.region {
        builder = builder.with_region(region);
    }
    if let Some(endpoint) = &config.endpoint {
        builder = builder.with_endpoint(endpoint);
        if endpoint.starts_with("http://") {
            builder = builder.with_allow_http(true);
        }
    }
    if let Some(access_key) = &config.access_key {
        builder = builder.with_access_key_id(access_key);
    }
    if let Some(secret_key) = &config.secret_key {
        builder = builder.with_secret_access_key(secret_key);
    }
    if let Some(token) = &config.session_token {
        builder = builder.with_token(token);
    }
    if config.force_path_style {
        builder = builder.with_virtual_hosted_style_request(false);
    }
    if config.skip_signature {
        builder = builder.with_config(AmazonS3ConfigKey::SkipSignature, "true");
    }

    builder.build().map_err(|e| BackendError::InvalidConfig {
        backend: "s3".to_string(),
        message: format!("Failed to build S3 client: {}", e),
    })
}

fn map_store_err(e: object_store::Error, path: &Path) -> BackendError {
    use object_store::Error as OsErr;
    match &e {
        OsErr::NotFound { .. } => BackendError::NotFound {
            path: path.to_path_buf(),
            backend: "s3".to_string(),
        },
        OsErr::PermissionDenied { .. } | OsErr::Unauthenticated { .. } => {
            BackendError::PermissionDenied {
                path: path.to_path_buf(),
                message: e.to_string(),
            }
        }
        OsErr::AlreadyExists { .. } => BackendError::AlreadyExists {
            path: path.to_path_buf(),
        },
        _ => BackendError::Other {
            backend: "s3".to_string(),
            message: e.to_string(),
        },
    }
}

fn is_not_found(e: &object_store::Error) -> bool {
    matches!(e, object_store::Error::NotFound { .. })
}

fn build_attributes(options: &WriteOptions) -> Attributes {
    let mut attrs = Attributes::new();
    if let Some(ct) = &options.content_type {
        attrs.insert(Attribute::ContentType, AttributeValue::from(ct.clone()));
    }
    if let Some(meta) = &options.metadata {
        for (k, v) in meta {
            attrs.insert(
                Attribute::Metadata(Cow::Owned(k.clone())),
                AttributeValue::from(v.clone()),
            );
        }
    }
    attrs
}

fn dir_entry_from_meta(meta: &ObjectMeta, self_prefix: Option<&str>) -> DirEntry {
    let full_path = PathBuf::from(meta.location.as_ref());
    let relative_path = strip_prefix(meta.location.as_ref(), self_prefix);
    let mut metadata = Metadata::file(meta.size as u64);
    metadata.modified = Some(meta.last_modified.into());
    metadata.etag = meta.e_tag.clone();
    DirEntry::new(relative_path, full_path, metadata)
}

fn strip_prefix(key: &str, prefix: Option<&str>) -> PathBuf {
    match prefix {
        Some(p) => {
            let p = p.trim_end_matches('/');
            if let Some(stripped) = key.strip_prefix(p) {
                PathBuf::from(stripped.trim_start_matches('/'))
            } else {
                PathBuf::from(key)
            }
        }
        None => PathBuf::from(key),
    }
}

#[async_trait]
impl Backend for S3Backend {
    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", path = %path.display())
    )]
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let object_path = self.path_to_key(path);
        let meta = self
            .store
            .head(&object_path)
            .await
            .map_err(|e| map_store_err(e, path))?;
        Ok(self.convert_meta(&meta))
    }

    #[tracing::instrument(
        skip(self, options),
        fields(otel.kind = "client", backend = "s3", path = %path.display(), recursive = options.recursive)
    )]
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream;

        // object_store's list APIs match on a path-segment basis, so
        // `dir` as a prefix never accidentally captures `dir2/...`.
        let prefix = self.path_to_key(path);
        let self_prefix = self.prefix.clone();
        let max = options.max_entries.unwrap_or(usize::MAX);

        if options.recursive {
            // Drive the underlying list stream on a spawned task and forward
            // entries through a bounded channel. This keeps the unified
            // `ListStream` type `'static` (object_store's BoxStream borrows
            // from &self.store) and provides natural backpressure so we
            // never materialise more than `LIST_CHANNEL_CAPACITY` entries.
            let store = self.store.clone();
            let (tx, rx) =
                tokio::sync::mpsc::channel::<BackendResult<DirEntry>>(LIST_CHANNEL_CAPACITY);

            tokio::spawn(async move {
                let mut listing = store.list(Some(&prefix));
                let mut sent = 0usize;
                while sent < max {
                    match listing.next().await {
                        Some(Ok(meta)) => {
                            let entry = dir_entry_from_meta(&meta, self_prefix.as_deref());
                            if tx.send(Ok(entry)).await.is_err() {
                                return;
                            }
                            sent += 1;
                        }
                        Some(Err(e)) => {
                            let _ = tx
                                .send(Err(BackendError::Other {
                                    backend: "s3".to_string(),
                                    message: format!("List error: {}", e),
                                }))
                                .await;
                            return;
                        }
                        None => return,
                    }
                }
            });

            let stream = stream::unfold(rx, |mut rx| async move {
                rx.recv().await.map(|item| (item, rx))
            });
            Ok(stream.boxed())
        } else {
            // Non-recursive listing is bounded by S3 page semantics; collect
            // the single page synchronously.
            let result = self
                .store
                .list_with_delimiter(Some(&prefix))
                .await
                .map_err(|e| BackendError::Other {
                    backend: "s3".to_string(),
                    message: format!("List error: {}", e),
                })?;

            let mut entries = Vec::new();
            for meta in result.objects {
                entries.push(Ok(dir_entry_from_meta(&meta, self_prefix.as_deref())));
            }
            for prefix_path in result.common_prefixes {
                let full_path = PathBuf::from(prefix_path.as_ref());
                let relative_path = strip_prefix(prefix_path.as_ref(), self_prefix.as_deref());
                entries.push(Ok(DirEntry::new(
                    relative_path,
                    full_path,
                    Metadata::directory(),
                )));
            }
            Ok(stream::iter(entries).take(max).boxed())
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", path = %path.display())
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let object_path = self.path_to_key(path);
        let stream = self
            .store
            .get(&object_path)
            .await
            .map_err(|e| map_store_err(e, path))?
            .into_stream()
            .map_err(|e| std::io::Error::other(e.to_string()))
            .boxed();
        Ok(stream)
    }

    #[tracing::instrument(
        skip(self, reader, options),
        fields(otel.kind = "client", backend = "s3", path = %path.display(), overwrite = options.overwrite, size_hint = ?size_hint)
    )]
    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        let object_path = self.path_to_key(path);
        let attributes = build_attributes(&options);
        let use_multipart = size_hint.is_none_or(|s| s >= MULTIPART_THRESHOLD);

        if !use_multipart {
            // Small object: single PUT preserves atomic Create semantics
            // for the `!overwrite` case via PutMode::Create.
            let cap = size_hint.unwrap_or(0).min(MULTIPART_THRESHOLD) as usize;
            let mut buffer = Vec::with_capacity(cap);
            let bytes_read = reader
                .read_to_end(&mut buffer)
                .await
                .map_err(BackendError::from)?;

            let put_opts = PutOptions {
                mode: if options.overwrite {
                    PutMode::Overwrite
                } else {
                    PutMode::Create
                },
                attributes,
                ..Default::default()
            };

            self.store
                .put_opts(
                    &object_path,
                    PutPayload::from_bytes(Bytes::from(buffer)),
                    put_opts,
                )
                .await
                .map_err(|e| map_store_err(e, path))?;
            return Ok(bytes_read as u64);
        }

        // Streaming multipart path. object_store's multipart API has no
        // conditional-create mode, so emulate it with a best-effort head()
        // probe before starting. A concurrent writer could still race, but
        // that matches the prior aws-sdk-s3 behaviour for the same flow.
        if !options.overwrite {
            match self.store.head(&object_path).await {
                Ok(_) => {
                    return Err(BackendError::AlreadyExists {
                        path: path.to_path_buf(),
                    });
                }
                Err(e) if is_not_found(&e) => {}
                Err(e) => {
                    return Err(BackendError::Other {
                        backend: "s3".to_string(),
                        message: format!("Failed to check existence: {}", e),
                    });
                }
            }
        }

        let mp_opts = PutMultipartOpts {
            attributes,
            ..Default::default()
        };
        let upload = self
            .store
            .put_multipart_opts(&object_path, mp_opts)
            .await
            .map_err(|e| map_store_err(e, path))?;
        let mut writer = WriteMultipart::new_with_chunk_size(upload, MULTIPART_CHUNK_SIZE);

        let mut buf = vec![0u8; READ_BUF_SIZE];
        let mut total: u64 = 0;
        loop {
            // Apply backpressure before reading the next chunk so we never
            // run with more than MULTIPART_CONCURRENCY parts in flight.
            if let Err(e) = writer.wait_for_capacity(MULTIPART_CONCURRENCY).await {
                let _ = writer.abort().await;
                return Err(BackendError::Other {
                    backend: "s3".to_string(),
                    message: format!("Multipart upload failed: {}", e),
                });
            }

            let n = match reader.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) => {
                    let _ = writer.abort().await;
                    return Err(BackendError::from(e));
                }
            };
            writer.write(&buf[..n]);
            total += n as u64;
        }

        writer.finish().await.map_err(|e| BackendError::Other {
            backend: "s3".to_string(),
            message: format!("Multipart upload failed: {}", e),
        })?;
        Ok(total)
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", path = %path.display(), recursive)
    )]
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let object_path = self.path_to_key(path);

        if recursive {
            // object_store matches the prefix segment-wise, so listing
            // under `dir` will never traverse a sibling prefix like `dir2/`.
            let listing = self.store.list(Some(&object_path));
            futures::pin_mut!(listing);
            let mut deleted_any = false;
            while let Some(meta) = listing.next().await {
                let meta = meta.map_err(|e| BackendError::Other {
                    backend: "s3".to_string(),
                    message: format!("Failed to list for deletion: {}", e),
                })?;
                self.store
                    .delete(&meta.location)
                    .await
                    .map_err(|e| map_store_err(e, path))?;
                deleted_any = true;
            }

            // Then delete the exact key — covers `delete("file.txt", true)`
            // and any object that happens to sit at `dir` (without trailing
            // slash). If only the children existed we already removed them,
            // so a NotFound here is fine.
            match self.store.delete(&object_path).await {
                Ok(()) => Ok(()),
                Err(e) if is_not_found(&e) && deleted_any => Ok(()),
                Err(e) => Err(map_store_err(e, path)),
            }
        } else {
            self.store
                .delete(&object_path)
                .await
                .map_err(|e| map_store_err(e, path))
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", path = %path.display(), recursive = _recursive)
    )]
    async fn mkdir(&self, path: &Path, _recursive: bool) -> BackendResult<()> {
        // S3 has no real directories; emulate with a 0-byte marker object.
        let key = format!("{}/", self.path_to_key(path).as_ref().trim_end_matches('/'));
        let object_path = ObjectPath::from(key);

        let put_opts = PutOptions {
            mode: PutMode::Create,
            ..Default::default()
        };
        self.store
            .put_opts(&object_path, PutPayload::from_static(b""), put_opts)
            .await
            .map_err(|e| map_store_err(e, path))?;
        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", src = %src.display(), dest = %dest.display())
    )]
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_path = self.path_to_key(src);
        let dest_path = self.path_to_key(dest);

        self.store
            .copy(&src_path, &dest_path)
            .await
            .map_err(|e| map_store_err(e, src))?;

        self.store
            .delete(&src_path)
            .await
            .map_err(|e| BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to delete source after rename: {}", e),
            })?;
        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "s3", path = %path.display())
    )]
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let object_path = self.path_to_key(path);
        match self.store.head(&object_path).await {
            Ok(_) => Ok(true),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(BackendError::Other {
                backend: "s3".to_string(),
                message: format!("Failed to check existence: {}", e),
            }),
        }
    }

    fn backend_name(&self) -> &str {
        "s3"
    }

    fn supports(&self, operation: &str) -> bool {
        matches!(
            operation,
            "stat" | "list" | "read" | "write" | "delete" | "mkdir" | "rename" | "exists"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_backend(prefix: Option<&str>) -> S3Backend {
        S3Backend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: prefix.map(|s| s.to_string()),
        }
    }

    #[test]
    fn test_path_to_key_with_prefix() {
        let backend = make_backend(Some("prefix"));
        assert_eq!(
            backend.path_to_key(Path::new("file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend.path_to_key(Path::new("/file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend.path_to_key(Path::new("dir/file.txt")).as_ref(),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_path_to_key_no_prefix() {
        let backend = make_backend(None);
        assert_eq!(
            backend.path_to_key(Path::new("file.txt")).as_ref(),
            "file.txt"
        );
        assert_eq!(
            backend.path_to_key(Path::new("/file.txt")).as_ref(),
            "file.txt"
        );
    }

    #[test]
    fn test_key_to_path_strips_prefix() {
        let backend = make_backend(Some("prefix"));
        assert_eq!(
            backend.key_to_path(&ObjectPath::from("prefix/file.txt")),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            backend.key_to_path(&ObjectPath::from("prefix/dir/file.txt")),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_key_to_path_no_prefix() {
        let backend = make_backend(None);
        assert_eq!(
            backend.key_to_path(&ObjectPath::from("file.txt")),
            PathBuf::from("file.txt")
        );
    }

    #[test]
    fn test_build_attributes_content_type_and_metadata() {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("owner".to_string(), "alice".to_string());
        metadata.insert("source".to_string(), "ci".to_string());
        let options = WriteOptions::new()
            .with_content_type("application/json".to_string())
            .with_metadata(metadata);

        let attrs = build_attributes(&options);
        assert_eq!(
            attrs.get(&Attribute::ContentType).map(|v| v.as_ref()),
            Some("application/json")
        );
        assert_eq!(
            attrs
                .get(&Attribute::Metadata(Cow::Borrowed("owner")))
                .map(|v| v.as_ref()),
            Some("alice")
        );
        assert_eq!(
            attrs
                .get(&Attribute::Metadata(Cow::Borrowed("source")))
                .map(|v| v.as_ref()),
            Some("ci")
        );
    }

    #[test]
    fn test_build_attributes_empty_when_unset() {
        let attrs = build_attributes(&WriteOptions::new());
        assert!(attrs.is_empty());
    }

    // === Integration tests against an in-memory ObjectStore ===
    // These exercise the prefix-boundary fix and streaming write logic
    // end-to-end without touching the network.

    fn in_memory_backend(prefix: Option<&str>) -> S3Backend {
        S3Backend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: prefix.map(|s| s.to_string()),
        }
    }

    async fn put_bytes(backend: &S3Backend, path: &str, data: &[u8]) {
        let reader: Box<dyn AsyncRead + Unpin + Send> =
            Box::new(std::io::Cursor::new(data.to_vec()));
        backend
            .write(
                Path::new(path),
                reader,
                Some(data.len() as u64),
                WriteOptions::new(),
            )
            .await
            .expect("write should succeed");
    }

    #[tokio::test]
    async fn recursive_delete_does_not_touch_sibling_prefixes() {
        let backend = in_memory_backend(None);

        put_bytes(&backend, "dir/a.txt", b"a").await;
        put_bytes(&backend, "dir/b.txt", b"b").await;
        // `dir2/` is a sibling prefix that shares the literal stem "dir".
        // The pre-fix code would have included these objects in a recursive
        // delete of "dir".
        put_bytes(&backend, "dir2/keep.txt", b"keep").await;

        backend
            .delete(Path::new("dir"), true)
            .await
            .expect("recursive delete should succeed");

        assert!(!backend.exists(Path::new("dir/a.txt")).await.unwrap());
        assert!(!backend.exists(Path::new("dir/b.txt")).await.unwrap());
        // Sibling untouched.
        assert!(backend.exists(Path::new("dir2/keep.txt")).await.unwrap());
    }

    #[tokio::test]
    async fn non_recursive_list_does_not_match_sibling_prefixes() {
        let backend = in_memory_backend(None);

        put_bytes(&backend, "dir/a.txt", b"a").await;
        put_bytes(&backend, "dir2/b.txt", b"b").await;

        let mut stream = backend
            .list(Path::new("dir"), ListOptions::shallow())
            .await
            .expect("list should succeed");

        let mut paths = Vec::new();
        while let Some(entry) = stream.next().await {
            let entry = entry.unwrap();
            paths.push(entry.path.to_string_lossy().into_owned());
        }

        // Only `dir/a.txt` should appear, never `dir2/b.txt`.
        assert!(paths.iter().any(|p| p.ends_with("a.txt")));
        assert!(!paths.iter().any(|p| p.contains("dir2")));
    }

    #[tokio::test]
    async fn recursive_list_streams_lazily_and_respects_max_entries() {
        let backend = in_memory_backend(None);
        for i in 0..50 {
            put_bytes(&backend, &format!("many/{:03}.txt", i), b"x").await;
        }

        let opts = ListOptions {
            recursive: true,
            max_entries: Some(5),
            ..Default::default()
        };
        let stream = backend
            .list(Path::new("many"), opts)
            .await
            .expect("list should succeed");
        let entries: Vec<_> = stream.collect().await;
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn streaming_write_above_threshold_round_trips_via_multipart() {
        // 16 MiB > MULTIPART_THRESHOLD; exercises the put_multipart path.
        let size: usize = 16 * 1024 * 1024;
        let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();

        let backend = in_memory_backend(None);
        let reader: Box<dyn AsyncRead + Unpin + Send> =
            Box::new(std::io::Cursor::new(payload.clone()));
        let written = backend
            .write(
                Path::new("big.bin"),
                reader,
                Some(size as u64),
                WriteOptions::new(),
            )
            .await
            .expect("multipart write should succeed");
        assert_eq!(written, size as u64);

        let mut read_stream = backend
            .read(Path::new("big.bin"))
            .await
            .expect("read should succeed");
        let mut buf = Vec::with_capacity(size);
        while let Some(chunk) = read_stream.next().await {
            buf.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(buf, payload);
    }

    #[tokio::test]
    async fn write_no_overwrite_returns_already_exists() {
        let backend = in_memory_backend(None);
        put_bytes(&backend, "existing.txt", b"first").await;

        let reader: Box<dyn AsyncRead + Unpin + Send> =
            Box::new(std::io::Cursor::new(b"second".to_vec()));
        let err = backend
            .write(
                Path::new("existing.txt"),
                reader,
                Some(6),
                WriteOptions::new().no_overwrite(),
            )
            .await
            .expect_err("should reject overwrite");
        assert!(matches!(err, BackendError::AlreadyExists { .. }));
    }
}
