# Backend Scalability Refactoring Plan

## Executive Summary

This plan addresses critical scalability flaws in the Backend trait abstraction layer that prevent Orbit from handling large datasets. The underlying S3 protocol implementation is excellent, but the Backend trait forces inefficient patterns that cause OOM crashes and poor throughput.

## Critical Issues Identified

### 1. 🚨 Write Method: Forced In-Memory Buffering
**Location**: `src/backend/mod.rs:209-214`

**Current Signature**:
```rust
async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64>;
```

**Problems**:
- Forces entire file to be loaded into RAM before upload
- S3Backend uses `put_object()` which has a 5GB limit
- Beautiful multipart upload implementation in `src/protocol/s3/multipart.rs:40-148` is never used
- Will OOM crash on files larger than available RAM

**Impact**: `orbit cp 10GB.iso s3://bucket/` will crash

**Affected Files**: ALL backends (local.rs, s3.rs, ssh.rs, smb.rs)

---

### 2. 🚨 List Method: Unbounded Memory Accumulation
**Location**: `src/backend/mod.rs:175`

**Current Signature**:
```rust
async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>>;
```

**Problems**:
- Implementation at `src/backend/s3.rs:158-248` accumulates all entries in `Vec`
- Loop structure:
  ```rust
  let mut entries = Vec::new();
  loop {
      // fetch page from S3
      for object in objects {
          entries.push(DirEntry::new(...)); // ACCUMULATES
      }
  }
  Ok(entries) // Returns everything
  ```
- No memory bounds on large buckets

**Impact**: Listing a bucket with 10 million objects will exhaust RAM

**Affected Files**: ALL backends

---

### 3. ⚠️ Download Concurrency: Stop-and-Wait Batching
**Location**: `src/protocol/s3/multipart.rs:352-383`

**Current Logic**:
```rust
while current_offset < total_size {
    let mut download_tasks = Vec::new();

    // Queue up batch of N tasks
    for _ in 0..parallel_downloads {
        download_tasks.push(spawn_download_task());
    }

    // Wait for ALL tasks to complete (BLOCKING)
    for task in download_tasks {
        let data = task.await; // Stalls if one task is slow
        write(data);
    }
}
```

**Problem**:
- If chunks download at: `[100ms, 100ms, 5000ms, 100ms]`, the fast chunks waste 4.7 seconds waiting
- Theoretical throughput on variable-latency networks is severely degraded

**Better Approach**: Sliding window (keep pipeline full)
```rust
while has_work || active_tasks > 0 {
    // Fill pipeline
    while active_tasks < MAX && has_work {
        spawn_task();
    }

    // Wait for NEXT completed task (not all)
    let result = join_next().await;
    write(result);
}
```

---

## Refactoring Plan

### Phase 1: Update Backend Trait (Breaking Changes)

**File**: `src/backend/mod.rs`

#### Change 1: Streaming Write
```rust
use tokio::io::AsyncRead;
use futures::stream::BoxStream;

#[async_trait]
pub trait Backend: Send + Sync {
    // BEFORE:
    // async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64>;

    // AFTER - Option A: AsyncRead (preferred for large files)
    async fn write(
        &self,
        path: &Path,
        reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64>;

    // OR Option B: Bytes stream (more flexible but complex)
    async fn write_stream(
        &self,
        path: &Path,
        stream: BoxStream<'static, std::io::Result<Bytes>>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64>;
}
```

**Decision Required**: AsyncRead vs Stream?
- **AsyncRead**: Simpler, works well with file-to-file copies, supports multipart upload from disk
- **Stream**: More flexible, supports chunked transfers, network-to-storage direct

**Recommendation**: AsyncRead for v0.5, add Stream variant in v0.6 if needed

---

#### Change 2: Streaming List
```rust
#[async_trait]
pub trait Backend: Send + Sync {
    // BEFORE:
    // async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>>;

    // AFTER:
    async fn list(
        &self,
        path: &Path,
        options: ListOptions,
    ) -> BackendResult<BoxStream<'static, BackendResult<DirEntry>>>;
}
```

**Rationale**: Stream allows caller to:
- Process entries incrementally
- Stop early (e.g., "find first match")
- Control memory usage
- Display results as they arrive

---

### Phase 2: Update S3Backend Implementation

**File**: `src/backend/s3.rs`

#### Task 2.1: Implement Streaming Write

**Current** (line 303-340):
```rust
async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64> {
    // ... validation ...

    let mut request = self.client.aws_client()
        .put_object()
        .body(data.clone().into()); // IN-MEMORY!

    request.send().await?;
    Ok(data.len() as u64)
}
```

**New Implementation**:
```rust
async fn write(
    &self,
    path: &Path,
    reader: Box<dyn AsyncRead + Unpin + Send>,
    size_hint: Option<u64>,
    options: WriteOptions,
) -> BackendResult<u64> {
    let key = self.path_to_key(path);

    // Check overwrite
    if !options.overwrite && self.client.exists(&key).await? {
        return Err(BackendError::AlreadyExists { path: path.to_path_buf() });
    }

    // Determine upload strategy
    let use_multipart = size_hint.map_or(true, |s| s > 5_000_000); // >5MB

    if use_multipart {
        // Use streaming multipart upload
        self.upload_from_reader(&key, reader, size_hint, options).await
    } else {
        // Small file: buffer and use PutObject
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await?;
        self.put_object(&key, Bytes::from(data), options).await
    }
}
```

**New Helper Method**:
```rust
impl S3Backend {
    async fn upload_from_reader(
        &self,
        key: &str,
        mut reader: Box<dyn AsyncRead + Unpin + Send>,
        size_hint: Option<u64>,
        options: WriteOptions,
    ) -> BackendResult<u64> {
        // Initiate multipart upload
        let upload_id = self.client.initiate_multipart_upload_with_options(
            key,
            options.content_type.as_deref(),
            options.metadata.as_ref(),
        ).await?;

        let chunk_size = self.client.config().chunk_size;
        let mut part_number = 1;
        let mut completed_parts = Vec::new();
        let mut total_uploaded = 0u64;

        loop {
            // Read chunk from stream
            let mut buffer = vec![0u8; chunk_size];
            let mut chunk_data = Vec::new();

            loop {
                match reader.read(&mut buffer).await? {
                    0 => break, // EOF
                    n => {
                        chunk_data.extend_from_slice(&buffer[..n]);
                        if chunk_data.len() >= chunk_size {
                            break;
                        }
                    }
                }
            }

            if chunk_data.is_empty() {
                break; // End of stream
            }

            // Upload part
            let part_info = self.client.upload_part(
                key,
                &upload_id,
                part_number,
                Bytes::from(chunk_data),
            ).await?;

            total_uploaded += part_info.size as u64;
            completed_parts.push(part_info);
            part_number += 1;
        }

        // Complete multipart upload
        self.client.complete_multipart_upload(key, &upload_id, &completed_parts).await?;

        Ok(total_uploaded)
    }
}
```

---

#### Task 2.2: Implement Streaming List

**Current** (line 148-251):
```rust
async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>> {
    let mut entries = Vec::new(); // ACCUMULATES ALL
    let mut continuation_token: Option<String> = None;

    loop {
        let response = /* list_objects_v2 */;
        for object in response.contents() {
            entries.push(DirEntry::new(...)); // GROWS UNBOUNDED
        }

        if !response.is_truncated() {
            break;
        }
    }

    Ok(entries)
}
```

**New Implementation**:
```rust
use futures::stream::{self, BoxStream, StreamExt};

async fn list(
    &self,
    path: &Path,
    options: ListOptions,
) -> BackendResult<BoxStream<'static, BackendResult<DirEntry>>> {
    let prefix = self.path_to_key(path);
    let prefix = if prefix.is_empty() {
        "".to_string()
    } else {
        format!("{}/", prefix.trim_end_matches('/'))
    };

    let client = self.client.clone();
    let options_clone = options.clone();
    let self_prefix = self.prefix.clone();

    // Create stream that lazily fetches pages
    let stream = stream::unfold(
        (client, Some(None::<String>), false), // (client, continuation_token, done)
        move |(client, token_opt, done)| {
            let prefix = prefix.clone();
            let options = options_clone.clone();
            let self_prefix = self_prefix.clone();

            async move {
                if done {
                    return None;
                }

                let token = match token_opt {
                    Some(t) => t,
                    None => return None, // Already done
                };

                // Fetch next page
                let mut request = client.aws_client()
                    .list_objects_v2()
                    .bucket(client.bucket())
                    .prefix(&prefix);

                if !options.recursive {
                    request = request.delimiter("/");
                }

                if let Some(ref t) = token {
                    request = request.continuation_token(t);
                }

                let response = match request.send().await {
                    Ok(r) => r,
                    Err(e) => {
                        return Some((
                            Err(BackendError::Other {
                                backend: "s3".to_string(),
                                message: e.to_string(),
                            }),
                            (client, None, true),
                        ));
                    }
                };

                // Convert objects to DirEntry stream
                let mut entries = Vec::new();

                for object in response.contents() {
                    if let Some(key) = object.key() {
                        // ... convert to DirEntry (same logic as before) ...
                        entries.push(DirEntry::new(...));
                    }
                }

                // Handle common prefixes (directories)
                if !options.recursive {
                    for prefix in response.common_prefixes() {
                        // ... add directory entries ...
                    }
                }

                // Determine next state
                let next_token = if response.is_truncated().unwrap_or(false) {
                    Some(response.next_continuation_token().map(|s| s.to_string()))
                } else {
                    None // Done
                };

                let is_done = next_token.is_none();

                // Emit this page's entries as a stream
                Some((
                    Ok(stream::iter(entries.into_iter().map(Ok))),
                    (client, next_token, is_done),
                ))
            }
        },
    )
    .flat_map(|result| match result {
        Ok(page_stream) => page_stream.boxed(),
        Err(e) => stream::once(async move { Err(e) }).boxed(),
    })
    .boxed();

    Ok(stream)
}
```

**Alternative: Async Iterator** (if available in Rust version):
```rust
async fn list(...) -> BackendResult<impl Stream<Item = BackendResult<DirEntry>>> {
    // Similar but cleaner with async generators
}
```

---

#### Task 2.3: Optimize Download Concurrency

**File**: `src/protocol/s3/multipart.rs`
**Location**: Lines 311-387

**Current** (Stop-and-Wait):
```rust
while current_offset < total_size {
    let mut download_tasks = Vec::new();

    // Queue batch
    for _ in 0..parallel_downloads {
        download_tasks.push(spawn(download_chunk()));
    }

    // Wait for ALL (BLOCKING)
    for task in download_tasks {
        write(task.await?);
    }
}
```

**New Implementation** (Sliding Window):
```rust
use tokio::task::JoinSet;

pub async fn download_file_resumable(
    &self,
    key: &str,
    local_path: &Path,
    resume_offset: u64,
) -> S3Result<()> {
    let metadata = self.get_metadata(key).await?;
    let total_size = metadata.size;
    let chunk_size = self.config().chunk_size as u64;

    // ... file opening logic (same as before) ...

    let mut join_set = JoinSet::new();
    let mut next_offset = resume_offset;
    let mut next_write_offset = resume_offset;
    let parallel_downloads = self.config().parallel_operations;

    // Track in-flight chunks by offset
    let mut pending_chunks: std::collections::BTreeMap<u64, tokio::task::JoinHandle<S3Result<Bytes>>>
        = std::collections::BTreeMap::new();

    loop {
        // Fill the pipeline
        while pending_chunks.len() < parallel_downloads && next_offset < total_size {
            let end_offset = (next_offset + chunk_size - 1).min(total_size - 1);
            let client = self.clone_for_multipart();
            let key_clone = key.to_string();
            let start = next_offset;

            let handle = tokio::spawn(async move {
                client.download_range(&key_clone, start, end_offset).await
            });

            pending_chunks.insert(next_offset, handle);
            next_offset = end_offset + 1;
        }

        if pending_chunks.is_empty() {
            break; // All done
        }

        // Wait for the NEXT sequential chunk (not any chunk)
        if let Some((offset, handle)) = pending_chunks.remove(&next_write_offset) {
            let data = handle.await
                .map_err(|e| S3Error::Network(format!("Task join error: {}", e)))??;

            file.write_all(&data).await?;
            next_write_offset += data.len() as u64;
        } else {
            // Next sequential chunk not ready yet, wait for any task
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    file.flush().await?;
    Ok(())
}
```

**Benefits**:
- Pipeline stays full even with variable latency
- Writes happen in order (important for file integrity)
- Faster completion on real-world networks

---

### Phase 3: Update Other Backends

#### Task 3.1: LocalBackend
**File**: `src/backend/local.rs`

**Write Implementation**:
```rust
async fn write(
    &self,
    path: &Path,
    mut reader: Box<dyn AsyncRead + Unpin + Send>,
    _size_hint: Option<u64>,
    options: WriteOptions,
) -> BackendResult<u64> {
    let resolved = self.resolve_path(path);

    if options.create_parents {
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await?;
        }
    }

    if !options.overwrite && resolved.exists() {
        return Err(BackendError::AlreadyExists { path: path.to_path_buf() });
    }

    let mut file = fs::File::create(&resolved).await?;

    // Stream copy
    let bytes_written = tokio::io::copy(&mut reader, &mut file).await?;
    file.flush().await?;

    // Set permissions if specified
    #[cfg(unix)]
    if let Some(perms) = options.permissions {
        // ... same as before ...
    }

    Ok(bytes_written)
}
```

**List Implementation**: Already accumulates in Vec, could be optimized similarly but lower priority (local FS less likely to have millions of entries).

---

#### Task 3.2: SshBackend
**File**: `src/backend/ssh.rs`

Similar pattern to LocalBackend - use SFTP streaming APIs if available.

---

#### Task 3.3: SmbBackend
**File**: `src/backend/smb.rs`

Similar pattern to LocalBackend.

---

## Migration Strategy

### Backward Compatibility Approach

**Option A: Clean Break** (Recommended)
- Update trait immediately
- Update all backends
- Breaking change in v0.5.0
- Document migration path

**Option B: Dual API** (Not Recommended)
- Keep `write()` with `Bytes`
- Add `write_stream()` with `AsyncRead`
- Deprecate old method
- Remove in v0.6.0

**Recommendation**: Option A - there's minimal usage of Backend trait in the wild yet (no cmd/ or transfer/ modules found).

---

## Implementation Checklist

### Phase 1: Trait Updates
- [ ] Update `Backend::write()` signature in `src/backend/mod.rs`
- [ ] Update `Backend::list()` signature in `src/backend/mod.rs`
- [ ] Update trait documentation with examples
- [ ] Update `src/backend/types.rs` if new types needed

### Phase 2: S3Backend
- [ ] Implement streaming `write()` in `src/backend/s3.rs`
- [ ] Add `upload_from_reader()` helper method
- [ ] Implement streaming `list()` in `src/backend/s3.rs`
- [ ] Optimize download concurrency in `src/protocol/s3/multipart.rs`
- [ ] Add integration tests for large file uploads
- [ ] Add tests for streaming list with pagination

### Phase 3: Other Backends
- [ ] Update `LocalBackend::write()` in `src/backend/local.rs`
- [ ] Update `LocalBackend::list()` in `src/backend/local.rs`
- [ ] Update `SshBackend::write()` in `src/backend/ssh.rs`
- [ ] Update `SshBackend::list()` in `src/backend/ssh.rs`
- [ ] Update `SmbBackend::write()` in `src/backend/smb.rs`
- [ ] Update `SmbBackend::list()` in `src/backend/smb.rs`

### Phase 4: Testing & Documentation
- [ ] Update unit tests for all backends
- [ ] Add integration test: upload 1GB file to S3
- [ ] Add integration test: list bucket with 10K+ objects
- [ ] Add integration test: resume interrupted multipart upload
- [ ] Update BACKEND_GUIDE.md with new examples
- [ ] Update README.md examples
- [ ] Add migration guide for v0.5.0

### Phase 5: Performance Validation
- [ ] Benchmark: Local file → S3 (10GB)
- [ ] Benchmark: List 100K objects
- [ ] Benchmark: Download with variable latency
- [ ] Profile memory usage during large operations
- [ ] Verify no regression on small files

---

## Expected Outcomes

### Before Refactoring:
- ❌ 10GB file upload: OOM crash
- ❌ Files >5GB: Not supported (PutObject limit)
- ❌ Listing 1M objects: OOM crash
- ❌ Download throughput: Sub-optimal on variable networks

### After Refactoring:
- ✅ 10GB file upload: Streaming multipart, ~200MB RAM
- ✅ Files up to 5TB: Supported (S3 limit)
- ✅ Listing 1M objects: Constant memory (~10MB)
- ✅ Download throughput: Near-optimal even with jitter

---

## Risk Assessment

### High Risk
- **Breaking API change**: Affects any external code using Backend trait
  - **Mitigation**: Version as v0.5.0, provide migration guide

### Medium Risk
- **Stream complexity**: Harder to debug than Vec
  - **Mitigation**: Comprehensive testing, good error messages

### Low Risk
- **Performance regression**: Streaming might be slower for small files
  - **Mitigation**: Keep PutObject path for files <5MB, benchmark suite

---

## Questions for Review

1. **AsyncRead vs Stream for write()**?
   - AsyncRead is simpler for file-to-file copies
   - Stream is more flexible for network-to-storage
   - Recommend: AsyncRead for v0.5, add Stream variant if needed

2. **Streaming list() return type**?
   - `BoxStream<'static, BackendResult<DirEntry>>` - clean but requires boxing
   - `impl Stream<...>` - zero-cost but complicates trait
   - Recommend: BoxStream for flexibility

3. **Backward compatibility**?
   - Clean break in v0.5.0
   - Or keep dual API until v0.6.0?
   - Recommend: Clean break (minimal usage detected)

4. **Testing strategy**?
   - Unit tests for each backend
   - Integration tests against real S3/MinIO
   - Benchmarks for memory/throughput
   - All of the above

---

## Next Steps

1. **Review & Approve Plan**: Get feedback on approach
2. **Prototype**: Implement streaming write() for S3Backend only
3. **Validate**: Test with 10GB file upload
4. **Iterate**: Refine based on prototype learnings
5. **Full Implementation**: Roll out to all backends
6. **Release**: v0.5.0 with migration guide
