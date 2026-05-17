//! Google Cloud Storage backend implementation via `object_store`
//!
//! Authentication is configured through environment variables:
//! - `GOOGLE_APPLICATION_CREDENTIALS` pointing at a service account JSON file, or
//! - `GOOGLE_SERVICE_ACCOUNT` + `GOOGLE_SERVICE_ACCOUNT_KEY`.

use super::error::{BackendError, BackendResult};
use super::types::{DirEntry, ListOptions, ListStream, Metadata, ReadStream, WriteOptions};
use super::Backend;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::{StreamExt, TryStreamExt};
use object_store::gcp::GoogleCloudStorageBuilder;
use object_store::{
    path::Path as ObjectPath, Attribute, AttributeValue, Attributes, ObjectMeta, ObjectStore,
    PutMode, PutMultipartOpts, PutOptions, PutPayload, WriteMultipart,
};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncRead;

const MULTIPART_THRESHOLD: u64 = 8 * 1024 * 1024;
const MULTIPART_CHUNK_SIZE: usize = 8 * 1024 * 1024;
const READ_BUF_SIZE: usize = 64 * 1024;
const MULTIPART_CONCURRENCY: usize = 4;
const LIST_CHANNEL_CAPACITY: usize = 64;

/// Google Cloud Storage backend
pub struct GcsBackend {
    store: Arc<dyn ObjectStore>,
    /// Prefix applied to all operations (like a "root" directory)
    prefix: Option<String>,
}

impl GcsBackend {
    /// Create a new GCS backend from environment variables
    pub async fn new(bucket_name: &str) -> BackendResult<Self> {
        let store = GoogleCloudStorageBuilder::new()
            .with_bucket_name(bucket_name)
            .build()
            .map_err(|e| BackendError::InvalidConfig {
                backend: "gcs".to_string(),
                message: format!("Failed to create GCS client: {}", e),
            })?;

        Ok(Self {
            store: Arc::new(store),
            prefix: None,
        })
    }

    /// Create a new GCS backend with a prefix
    pub async fn with_prefix(bucket_name: &str, prefix: impl Into<String>) -> BackendResult<Self> {
        let mut backend = Self::new(bucket_name).await?;
        backend.prefix = Some(prefix.into());
        Ok(backend)
    }

    /// Convert a Path to a GCS object name. object_store's list APIs are
    /// segment-aware, so this prefix matches `dir/x` but never `dir2/x`.
    fn path_to_object_name(&self, path: &Path) -> ObjectPath {
        let path_str = path.to_string_lossy().replace('\\', "/");
        let object_name = path_str.trim_start_matches('/');
        let full_path = match &self.prefix {
            Some(p) => format!("{}/{}", p.trim_end_matches('/'), object_name),
            None => object_name.to_string(),
        };
        ObjectPath::from(full_path)
    }

    #[allow(dead_code)]
    fn object_name_to_path(&self, object_path: &ObjectPath) -> PathBuf {
        strip_prefix(object_path.as_ref(), self.prefix.as_deref())
    }

    fn convert_object_meta(&self, meta: &ObjectMeta) -> Metadata {
        let mut metadata = Metadata::file(meta.size as u64);
        metadata.modified = Some(meta.last_modified.into());
        metadata.etag = meta.e_tag.clone();
        metadata
    }
}

fn map_store_err(e: object_store::Error, path: &Path) -> BackendError {
    use object_store::Error as OsErr;
    match &e {
        OsErr::NotFound { .. } => BackendError::NotFound {
            path: path.to_path_buf(),
            backend: "gcs".to_string(),
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
            backend: "gcs".to_string(),
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
impl Backend for GcsBackend {
    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "gcs", path = %path.display())
    )]
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        let object_path = self.path_to_object_name(path);
        let meta = self
            .store
            .head(&object_path)
            .await
            .map_err(|e| map_store_err(e, path))?;
        Ok(self.convert_object_meta(&meta))
    }

    #[tracing::instrument(
        skip(self, options),
        fields(otel.kind = "client", backend = "gcs", path = %path.display(), recursive = options.recursive)
    )]
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<ListStream> {
        use futures::stream;

        let prefix = self.path_to_object_name(path);
        let self_prefix = self.prefix.clone();
        let max = options.max_entries.unwrap_or(usize::MAX);

        if options.recursive {
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
                                    backend: "gcs".to_string(),
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
            let result = self
                .store
                .list_with_delimiter(Some(&prefix))
                .await
                .map_err(|e| BackendError::Other {
                    backend: "gcs".to_string(),
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
        fields(otel.kind = "client", backend = "gcs", path = %path.display())
    )]
    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        let object_path = self.path_to_object_name(path);
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
        fields(otel.kind = "client", backend = "gcs", path = %path.display(), overwrite = options.overwrite, size_hint = ?size_hint)
    )]
    async fn write(
        &self,
        path: &Path,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        use tokio::io::AsyncReadExt;

        let object_path = self.path_to_object_name(path);
        let attributes = build_attributes(&options);
        let use_multipart = size_hint.is_none_or(|s| s >= MULTIPART_THRESHOLD);

        if !use_multipart {
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
                        backend: "gcs".to_string(),
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
            if let Err(e) = writer.wait_for_capacity(MULTIPART_CONCURRENCY).await {
                let _ = writer.abort().await;
                return Err(BackendError::Other {
                    backend: "gcs".to_string(),
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
            backend: "gcs".to_string(),
            message: format!("Multipart upload failed: {}", e),
        })?;
        Ok(total)
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "gcs", path = %path.display(), recursive)
    )]
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        let object_path = self.path_to_object_name(path);

        if recursive {
            let listing = self.store.list(Some(&object_path));
            futures::pin_mut!(listing);
            let mut deleted_any = false;
            while let Some(meta) = listing.next().await {
                let meta = meta.map_err(|e| BackendError::Other {
                    backend: "gcs".to_string(),
                    message: format!("Failed to list for deletion: {}", e),
                })?;
                self.store
                    .delete(&meta.location)
                    .await
                    .map_err(|e| map_store_err(e, path))?;
                deleted_any = true;
            }

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
        fields(otel.kind = "client", backend = "gcs", path = %path.display(), recursive = _recursive)
    )]
    async fn mkdir(&self, path: &Path, _recursive: bool) -> BackendResult<()> {
        let object_name = format!("{}/", path.to_string_lossy().replace('\\', "/"));
        let object_path = match &self.prefix {
            Some(p) => ObjectPath::from(format!(
                "{}/{}",
                p.trim_end_matches('/'),
                object_name.trim_start_matches('/')
            )),
            None => ObjectPath::from(object_name),
        };

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
        fields(otel.kind = "client", backend = "gcs", src = %src.display(), dest = %dest.display())
    )]
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        let src_path = self.path_to_object_name(src);
        let dest_path = self.path_to_object_name(dest);

        self.store
            .copy(&src_path, &dest_path)
            .await
            .map_err(|e| map_store_err(e, src))?;

        self.store
            .delete(&src_path)
            .await
            .map_err(|e| BackendError::Other {
                backend: "gcs".to_string(),
                message: format!("Failed to delete source after rename: {}", e),
            })?;
        Ok(())
    }

    #[tracing::instrument(
        skip(self),
        fields(otel.kind = "client", backend = "gcs", path = %path.display())
    )]
    async fn exists(&self, path: &Path) -> BackendResult<bool> {
        let object_path = self.path_to_object_name(path);
        match self.store.head(&object_path).await {
            Ok(_) => Ok(true),
            Err(e) if is_not_found(&e) => Ok(false),
            Err(e) => Err(BackendError::Other {
                backend: "gcs".to_string(),
                message: format!("Failed to check existence: {}", e),
            }),
        }
    }

    fn backend_name(&self) -> &str {
        "gcs"
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

    #[test]
    fn test_path_to_object_name() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.path_to_object_name(Path::new("file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend.path_to_object_name(Path::new("/file.txt")).as_ref(),
            "prefix/file.txt"
        );
        assert_eq!(
            backend
                .path_to_object_name(Path::new("dir/file.txt"))
                .as_ref(),
            "prefix/dir/file.txt"
        );
    }

    #[test]
    fn test_object_name_to_path() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: Some("prefix".to_string()),
        };

        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("prefix/file.txt")),
            PathBuf::from("file.txt")
        );
        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("prefix/dir/file.txt")),
            PathBuf::from("dir/file.txt")
        );
    }

    #[test]
    fn test_path_to_object_name_no_prefix() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.path_to_object_name(Path::new("file.txt")).as_ref(),
            "file.txt"
        );
        assert_eq!(
            backend.path_to_object_name(Path::new("/file.txt")).as_ref(),
            "file.txt"
        );
    }

    #[test]
    fn test_object_name_to_path_no_prefix() {
        let backend = GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        };

        assert_eq!(
            backend.object_name_to_path(&ObjectPath::from("file.txt")),
            PathBuf::from("file.txt")
        );
    }

    // === Integration tests against the in-memory ObjectStore ===

    fn in_memory_backend() -> GcsBackend {
        GcsBackend {
            store: Arc::new(object_store::memory::InMemory::new()),
            prefix: None,
        }
    }

    async fn put_bytes(backend: &GcsBackend, path: &str, data: &[u8]) {
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
        let backend = in_memory_backend();
        put_bytes(&backend, "dir/a.txt", b"a").await;
        put_bytes(&backend, "dir2/keep.txt", b"keep").await;

        backend.delete(Path::new("dir"), true).await.unwrap();

        assert!(!backend.exists(Path::new("dir/a.txt")).await.unwrap());
        assert!(backend.exists(Path::new("dir2/keep.txt")).await.unwrap());
    }

    #[tokio::test]
    async fn streaming_write_above_threshold_round_trips_via_multipart() {
        let size: usize = 16 * 1024 * 1024;
        let payload: Vec<u8> = (0..size).map(|i| (i % 251) as u8).collect();

        let backend = in_memory_backend();
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

        let mut read_stream = backend.read(Path::new("big.bin")).await.unwrap();
        let mut buf = Vec::with_capacity(size);
        while let Some(chunk) = read_stream.next().await {
            buf.extend_from_slice(&chunk.unwrap());
        }
        assert_eq!(buf, payload);
    }

    #[tokio::test]
    async fn recursive_list_respects_max_entries() {
        let backend = in_memory_backend();
        for i in 0..30 {
            put_bytes(&backend, &format!("many/{:03}.txt", i), b"x").await;
        }

        let opts = ListOptions {
            recursive: true,
            max_entries: Some(5),
            ..Default::default()
        };
        let stream = backend.list(Path::new("many"), opts).await.unwrap();
        let entries: Vec<_> = stream.collect().await;
        assert_eq!(entries.len(), 5);
    }
}
