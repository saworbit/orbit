//! Integration tests for Backend streaming functionality
//!
//! These tests verify that the Backend trait's streaming write() and list()
//! methods work correctly and handle large datasets without OOM.

#[cfg(feature = "backend-abstraction")]
mod streaming_tests {
    use orbit::backend::{Backend, ListOptions, LocalBackend, WriteOptions};
    use std::path::Path;
    use tempfile::TempDir;
    use tokio::io::AsyncRead;

    /// Helper to create a large data stream
    struct LargeDataReader {
        remaining: u64,
        chunk_size: usize,
    }

    impl LargeDataReader {
        fn new(total_size: u64) -> Self {
            Self {
                remaining: total_size,
                chunk_size: 64 * 1024, // 64KB chunks
            }
        }
    }

    impl AsyncRead for LargeDataReader {
        fn poll_read(
            mut self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            if self.remaining == 0 {
                return std::task::Poll::Ready(Ok(()));
            }

            let to_write = (self.chunk_size.min(self.remaining as usize)).min(buf.remaining());
            let data = vec![0x42u8; to_write]; // Fill with 'B'
            buf.put_slice(&data);
            self.remaining -= to_write as u64;

            std::task::Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_streaming_write_small_file() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();
        let file_path = temp_dir.path().join("small.txt");

        // Create 1MB test data
        let data = vec![0x41u8; 1024 * 1024]; // 1MB of 'A'
        let reader: Box<dyn AsyncRead + Unpin + Send> =
            Box::new(std::io::Cursor::new(data.clone()));

        // Write using streaming API
        let bytes_written = backend
            .write(
                &file_path,
                reader,
                Some(1024 * 1024),
                WriteOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(bytes_written, 1024 * 1024);
        assert!(file_path.exists());

        // Verify file contents
        let contents = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(contents, data);
    }

    #[tokio::test]
    async fn test_streaming_write_large_file() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();
        let file_path = temp_dir.path().join("large.bin");

        // Create 100MB test data stream (would OOM if buffered entirely)
        let size = 100 * 1024 * 1024u64; // 100MB
        let reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(LargeDataReader::new(size));

        // Write using streaming API
        let bytes_written = backend
            .write(&file_path, reader, Some(size), WriteOptions::default())
            .await
            .unwrap();

        assert_eq!(bytes_written, size);
        assert!(file_path.exists());

        // Verify file size
        let metadata = tokio::fs::metadata(&file_path).await.unwrap();
        assert_eq!(metadata.len(), size);
    }

    #[tokio::test]
    async fn test_streaming_list_many_entries() {
        use futures::StreamExt;

        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        // Create 1000 files (would use significant memory if all buffered)
        for i in 0..1000 {
            let file_path = temp_dir.path().join(format!("file_{:04}.txt", i));
            tokio::fs::write(&file_path, format!("File {}", i))
                .await
                .unwrap();
        }

        // List using streaming API
        let mut stream = backend
            .list(temp_dir.path(), ListOptions::shallow())
            .await
            .unwrap();

        let mut count = 0;
        while let Some(entry) = stream.next().await {
            let entry = entry.unwrap();
            assert!(entry.is_file());
            count += 1;
        }

        assert_eq!(count, 1000);
    }

    #[tokio::test]
    async fn test_streaming_list_processes_incrementally() {
        use futures::StreamExt;

        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        // Create 100 files
        for i in 0..100 {
            let file_path = temp_dir.path().join(format!("file_{:03}.txt", i));
            tokio::fs::write(&file_path, format!("File {}", i))
                .await
                .unwrap();
        }

        // List using streaming API - the key is that we can process incrementally
        let mut stream = backend
            .list(temp_dir.path(), ListOptions::shallow())
            .await
            .unwrap();

        // Process first 25 entries and stop (demonstrates streaming benefit)
        let mut count = 0;
        while let Some(entry) = stream.next().await {
            let _entry = entry.unwrap();
            count += 1;
            if count >= 25 {
                break; // Early termination - didn't need to list all 100
            }
        }

        assert_eq!(count, 25);
    }

    #[tokio::test]
    async fn test_write_with_create_parents() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();
        let file_path = temp_dir.path().join("nested/dir/structure/file.txt");

        let data = b"test data";
        let reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(std::io::Cursor::new(data));

        let options = WriteOptions {
            create_parents: true,
            ..Default::default()
        };

        backend
            .write(&file_path, reader, Some(data.len() as u64), options)
            .await
            .unwrap();

        assert!(file_path.exists());
        let contents = tokio::fs::read(&file_path).await.unwrap();
        assert_eq!(contents, data);
    }

    #[tokio::test]
    async fn test_write_no_overwrite() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();
        let file_path = temp_dir.path().join("existing.txt");

        // Create existing file
        tokio::fs::write(&file_path, b"existing").await.unwrap();

        // Try to write without overwrite
        let data = b"new data";
        let reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(std::io::Cursor::new(data));

        let options = WriteOptions {
            overwrite: false,
            ..Default::default()
        };

        let result = backend
            .write(&file_path, reader, Some(data.len() as u64), options)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[tokio::test]
    async fn test_streaming_list_recursive() {
        use futures::StreamExt;

        let temp_dir = TempDir::new().unwrap();
        let backend = LocalBackend::new();

        // Create nested directory structure
        tokio::fs::create_dir_all(temp_dir.path().join("dir1/subdir1"))
            .await
            .unwrap();
        tokio::fs::create_dir_all(temp_dir.path().join("dir2/subdir2"))
            .await
            .unwrap();

        // Create files in various locations
        tokio::fs::write(temp_dir.path().join("root.txt"), b"root")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("dir1/file1.txt"), b"file1")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("dir1/subdir1/file2.txt"), b"file2")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("dir2/file3.txt"), b"file3")
            .await
            .unwrap();

        // List recursively
        let mut stream = backend
            .list(temp_dir.path(), ListOptions::recursive())
            .await
            .unwrap();

        let mut file_count = 0;
        let mut dir_count = 0;

        while let Some(entry) = stream.next().await {
            let entry = entry.unwrap();
            if entry.is_file() {
                file_count += 1;
            } else if entry.is_dir() {
                dir_count += 1;
            }
        }

        assert_eq!(file_count, 4); // root.txt, file1.txt, file2.txt, file3.txt
        assert!(dir_count >= 2); // At least dir1/subdir1 and dir2/subdir2
    }

    #[tokio::test]
    async fn test_write_permissions_unix() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = TempDir::new().unwrap();
            let backend = LocalBackend::new();
            let file_path = temp_dir.path().join("perms.txt");

            let data = b"test data";
            let reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(std::io::Cursor::new(data));

            let options = WriteOptions {
                permissions: Some(0o600), // rw-------
                ..Default::default()
            };

            backend
                .write(&file_path, reader, Some(data.len() as u64), options)
                .await
                .unwrap();

            let metadata = tokio::fs::metadata(&file_path).await.unwrap();
            let perms = metadata.permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }
    }
}
