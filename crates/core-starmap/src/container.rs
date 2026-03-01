//! Container Packing: Chunk packing into container files to reduce inode pressure
//!
//! Instead of storing each CDC chunk as a separate file (which causes inode/handle
//! pressure at scale), chunks are appended to "container files". The Universe index
//! stores `(container_id, offset, length)` tuples for each chunk.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │              Container File (.orbitpak)           │
//! ├──────────────────────────────────────────────────┤
//! │ [Header: magic + version + created_at]           │
//! │ [Chunk 0: raw bytes]                             │
//! │ [Chunk 1: raw bytes]                             │
//! │ [Chunk 2: raw bytes]                             │
//! │ ...                                              │
//! └──────────────────────────────────────────────────┘
//!
//! ┌──────────────────────────────────────────────────┐
//! │              Container Index (in Universe)        │
//! ├──────────────────────────────────────────────────┤
//! │ BLAKE3 hash -> (container_id, offset, length)    │
//! └──────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```no_run
//! use orbit_core_starmap::container::{ContainerWriter, ContainerReader, PackedChunkRef};
//! use std::path::Path;
//!
//! // Write chunks into a container
//! let mut writer = ContainerWriter::create(Path::new("/data/chunks/container_001.orbitpak")).unwrap();
//! let ref1 = writer.append_chunk(&[0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
//! let ref2 = writer.append_chunk(&[0xCA, 0xFE]).unwrap();
//! writer.flush().unwrap();
//!
//! // Read chunks back
//! let reader = ContainerReader::open(Path::new("/data/chunks/container_001.orbitpak")).unwrap();
//! let data = reader.read_chunk(&ref1).unwrap();
//! assert_eq!(data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
//! ```

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Magic bytes for container files
pub const CONTAINER_MAGIC: &[u8; 8] = b"ORBITPAK";

/// Current container format version
pub const CONTAINER_VERSION: u16 = 1;

/// Header size in bytes (magic:8 + version:2 + reserved:6 = 16)
pub const HEADER_SIZE: u64 = 16;

/// Default maximum container file size (4 GiB)
pub const DEFAULT_MAX_CONTAINER_SIZE: u64 = 4 * 1024 * 1024 * 1024;

/// A reference to a chunk packed inside a container file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedChunkRef {
    /// ID of the container file (typically the filename without extension)
    pub container_id: String,

    /// Byte offset within the container file (after header)
    pub offset: u64,

    /// Length of the chunk data in bytes
    pub length: u32,
}

impl PackedChunkRef {
    /// Create a new packed chunk reference
    pub fn new(container_id: String, offset: u64, length: u32) -> Self {
        Self {
            container_id,
            offset,
            length,
        }
    }
}

/// Writes chunks into a container file by appending.
///
/// Each chunk is written sequentially. The writer tracks the current
/// write position and returns `PackedChunkRef` for each appended chunk.
pub struct ContainerWriter {
    container_id: String,
    writer: BufWriter<File>,
    current_offset: u64,
    chunks_written: u64,
    bytes_written: u64,
    max_size: u64,
    path: PathBuf,
}

impl ContainerWriter {
    /// Create a new container file
    pub fn create(path: &Path) -> io::Result<Self> {
        Self::create_with_max_size(path, DEFAULT_MAX_CONTAINER_SIZE)
    }

    /// Create a new container file with custom maximum size
    pub fn create_with_max_size(path: &Path, max_size: u64) -> io::Result<Self> {
        let container_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?;

        let mut writer = BufWriter::new(file);

        // Write header
        writer.write_all(CONTAINER_MAGIC)?;
        writer.write_all(&CONTAINER_VERSION.to_le_bytes())?;
        writer.write_all(&[0u8; 6])?; // Reserved
        writer.flush()?;

        Ok(Self {
            container_id,
            writer,
            current_offset: HEADER_SIZE,
            chunks_written: 0,
            bytes_written: 0,
            max_size,
            path: path.to_path_buf(),
        })
    }

    /// Open an existing container file for appending
    pub fn open_append(path: &Path) -> io::Result<Self> {
        let container_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let file = OpenOptions::new().read(true).append(true).open(path)?;

        // Verify header
        let mut reader = BufReader::new(&file);
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != CONTAINER_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid container magic",
            ));
        }

        let current_offset = file.metadata()?.len();

        Ok(Self {
            container_id,
            writer: BufWriter::new(file),
            current_offset,
            chunks_written: 0,
            bytes_written: 0,
            max_size: DEFAULT_MAX_CONTAINER_SIZE,
            path: path.to_path_buf(),
        })
    }

    /// Append a chunk to the container.
    ///
    /// Returns a `PackedChunkRef` that can be stored in the Universe index.
    /// Returns `None` if the container is full (would exceed max_size).
    pub fn append_chunk(&mut self, data: &[u8]) -> io::Result<PackedChunkRef> {
        let chunk_len = data.len() as u32;

        // Check if container would exceed max size
        if self.current_offset + data.len() as u64 > self.max_size {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Container full: would exceed max size",
            ));
        }

        let offset = self.current_offset;
        self.writer.write_all(data)?;
        self.current_offset += data.len() as u64;
        self.chunks_written += 1;
        self.bytes_written += data.len() as u64;

        Ok(PackedChunkRef {
            container_id: self.container_id.clone(),
            offset,
            length: chunk_len,
        })
    }

    /// Flush the writer to ensure all data is on disk
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Get the container ID
    pub fn container_id(&self) -> &str {
        &self.container_id
    }

    /// Get the current write offset (total file size)
    pub fn current_size(&self) -> u64 {
        self.current_offset
    }

    /// Get the number of chunks written
    pub fn chunks_written(&self) -> u64 {
        self.chunks_written
    }

    /// Check if the container has room for more data
    pub fn has_capacity(&self, bytes: u64) -> bool {
        self.current_offset + bytes <= self.max_size
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Reads chunks from a container file by offset and length.
pub struct ContainerReader {
    #[allow(dead_code)]
    file: File,
    path: PathBuf,
}

impl ContainerReader {
    /// Open a container file for reading
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;

        // Verify header
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != CONTAINER_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid container magic",
            ));
        }

        Ok(Self {
            file,
            path: path.to_path_buf(),
        })
    }

    /// Read a chunk from the container using a packed reference
    pub fn read_chunk(&self, chunk_ref: &PackedChunkRef) -> io::Result<Vec<u8>> {
        self.read_at(chunk_ref.offset, chunk_ref.length)
    }

    /// Read data at a specific offset and length
    pub fn read_at(&self, offset: u64, length: u32) -> io::Result<Vec<u8>> {
        // Use pread-style access (separate file handle to avoid seeking issues)
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut buf = vec![0u8; length as usize];
        file.read_exact(&mut buf)?;

        Ok(buf)
    }
}

/// Manages a pool of container files, rotating to new ones when full.
pub struct ContainerPool {
    /// Directory where container files are stored
    directory: PathBuf,

    /// Maximum size per container file
    max_container_size: u64,

    /// Active writer (current container)
    active_writer: Option<ContainerWriter>,

    /// Counter for generating unique container IDs
    next_id: u64,

    /// Total chunks packed across all containers
    total_chunks: u64,

    /// Total bytes packed across all containers
    total_bytes: u64,
}

impl ContainerPool {
    /// Create a new container pool in the given directory
    pub fn new(directory: PathBuf, max_container_size: u64) -> Self {
        Self {
            directory,
            max_container_size,
            active_writer: None,
            next_id: 0,
            total_chunks: 0,
            total_bytes: 0,
        }
    }

    /// Pack a chunk into the pool, rotating containers as needed
    pub fn pack_chunk(&mut self, data: &[u8]) -> io::Result<PackedChunkRef> {
        // Ensure we have an active writer with capacity
        if self.active_writer.is_none()
            || !self
                .active_writer
                .as_ref()
                .unwrap()
                .has_capacity(data.len() as u64)
        {
            self.rotate()?;
        }

        let writer = self.active_writer.as_mut().unwrap();
        let chunk_ref = writer.append_chunk(data)?;
        self.total_chunks += 1;
        self.total_bytes += data.len() as u64;

        Ok(chunk_ref)
    }

    /// Rotate to a new container file
    fn rotate(&mut self) -> io::Result<()> {
        // Flush current writer
        if let Some(ref mut writer) = self.active_writer {
            writer.flush()?;
        }

        // Create new container
        let container_name = format!("container_{:06}.orbitpak", self.next_id);
        self.next_id += 1;

        let path = self.directory.join(&container_name);
        let writer = ContainerWriter::create_with_max_size(&path, self.max_container_size)?;
        self.active_writer = Some(writer);

        Ok(())
    }

    /// Flush the active container
    pub fn flush(&mut self) -> io::Result<()> {
        if let Some(ref mut writer) = self.active_writer {
            writer.flush()?;
        }
        Ok(())
    }

    /// Get statistics
    pub fn stats(&self) -> ContainerPoolStats {
        ContainerPoolStats {
            containers_created: self.next_id,
            total_chunks: self.total_chunks,
            total_bytes: self.total_bytes,
            active_container_size: self
                .active_writer
                .as_ref()
                .map(|w| w.current_size())
                .unwrap_or(0),
        }
    }
}

/// Statistics for the container pool
#[derive(Debug, Clone)]
pub struct ContainerPoolStats {
    /// Number of container files created
    pub containers_created: u64,
    /// Total chunks packed
    pub total_chunks: u64,
    /// Total bytes packed
    pub total_bytes: u64,
    /// Current active container size
    pub active_container_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_container_write_and_read() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.orbitpak");

        // Write chunks
        let mut writer = ContainerWriter::create(&path).unwrap();
        let ref1 = writer.append_chunk(b"hello world").unwrap();
        let ref2 = writer.append_chunk(b"goodbye").unwrap();
        writer.flush().unwrap();

        assert_eq!(ref1.offset, HEADER_SIZE);
        assert_eq!(ref1.length, 11);
        assert_eq!(ref2.offset, HEADER_SIZE + 11);
        assert_eq!(ref2.length, 7);

        // Read chunks back
        let reader = ContainerReader::open(&path).unwrap();
        assert_eq!(reader.read_chunk(&ref1).unwrap(), b"hello world");
        assert_eq!(reader.read_chunk(&ref2).unwrap(), b"goodbye");
    }

    #[test]
    fn test_container_max_size() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("small.orbitpak");

        let mut writer = ContainerWriter::create_with_max_size(&path, HEADER_SIZE + 10).unwrap();
        writer.append_chunk(&[0u8; 5]).unwrap(); // OK

        let result = writer.append_chunk(&[0u8; 10]); // Too big
        assert!(result.is_err());
    }

    #[test]
    fn test_container_pool_rotation() {
        let dir = TempDir::new().unwrap();

        let mut pool = ContainerPool::new(
            dir.path().to_path_buf(),
            HEADER_SIZE + 100, // Small containers for testing
        );

        // Pack enough chunks to trigger rotation
        let ref1 = pool.pack_chunk(&[0u8; 50]).unwrap();
        let ref2 = pool.pack_chunk(&[0u8; 50]).unwrap();
        // Next chunk should go to a new container
        let ref3 = pool.pack_chunk(&[0u8; 50]).unwrap();

        assert_eq!(ref1.container_id, "container_000000");
        assert_eq!(ref2.container_id, "container_000000");
        assert_eq!(ref3.container_id, "container_000001");

        let stats = pool.stats();
        assert_eq!(stats.containers_created, 2);
        assert_eq!(stats.total_chunks, 3);
        assert_eq!(stats.total_bytes, 150);
    }

    #[test]
    fn test_open_append() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("append.orbitpak");

        // Write initial chunks
        {
            let mut writer = ContainerWriter::create(&path).unwrap();
            writer.append_chunk(b"first").unwrap();
            writer.flush().unwrap();
        }

        // Reopen and append
        {
            let mut writer = ContainerWriter::open_append(&path).unwrap();
            let ref2 = writer.append_chunk(b"second").unwrap();
            writer.flush().unwrap();

            // Second chunk should start after first
            assert_eq!(ref2.offset, HEADER_SIZE + 5);
        }

        // Verify both are readable
        let reader = ContainerReader::open(&path).unwrap();
        let first = reader.read_at(HEADER_SIZE, 5).unwrap();
        assert_eq!(first, b"first");
    }

    #[test]
    fn test_packed_chunk_ref_serde() {
        let r = PackedChunkRef::new("container_001".to_string(), 1024, 4096);
        let json = serde_json::to_string(&r).unwrap();
        let parsed: PackedChunkRef = serde_json::from_str(&json).unwrap();
        assert_eq!(r, parsed);
    }

    #[test]
    fn test_has_capacity() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("cap.orbitpak");

        let writer = ContainerWriter::create_with_max_size(&path, HEADER_SIZE + 100).unwrap();
        assert!(writer.has_capacity(100));
        assert!(!writer.has_capacity(101));
    }

    #[test]
    fn test_empty_chunk() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("empty_chunk.orbitpak");

        let mut writer = ContainerWriter::create(&path).unwrap();
        let chunk_ref = writer.append_chunk(&[]).unwrap();
        writer.flush().unwrap();

        assert_eq!(chunk_ref.length, 0);
        assert_eq!(chunk_ref.offset, HEADER_SIZE);

        let reader = ContainerReader::open(&path).unwrap();
        let data = reader.read_chunk(&chunk_ref).unwrap();
        assert!(data.is_empty());
    }

    #[test]
    fn test_reader_invalid_magic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("garbage.orbitpak");

        std::fs::write(&path, b"NOT_ORBIT_GARBAGE_DATA").unwrap();

        let result = ContainerReader::open(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_reader_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does_not_exist.orbitpak");

        let result = ContainerReader::open(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_append_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("no_such_file.orbitpak");

        let result = ContainerWriter::open_append(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_at_past_end_of_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("short.orbitpak");

        let mut writer = ContainerWriter::create(&path).unwrap();
        writer.append_chunk(b"tiny").unwrap();
        writer.flush().unwrap();

        let reader = ContainerReader::open(&path).unwrap();
        // Offset well past the end of the file
        let result = reader.read_at(99999, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_flush_no_active_writer() {
        let dir = TempDir::new().unwrap();
        let mut pool = ContainerPool::new(dir.path().to_path_buf(), HEADER_SIZE + 1024);

        // flush with no prior pack_chunk should be a no-op and not panic
        pool.flush().unwrap();
    }

    #[test]
    fn test_pool_stats_no_activity() {
        let dir = TempDir::new().unwrap();
        let pool = ContainerPool::new(dir.path().to_path_buf(), HEADER_SIZE + 1024);

        let stats = pool.stats();
        assert_eq!(stats.containers_created, 0);
        assert_eq!(stats.total_chunks, 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.active_container_size, 0);
    }

    #[test]
    fn test_chunk_at_exact_capacity() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("exact_cap.orbitpak");

        let chunk_size: u64 = 50;
        let max_size = HEADER_SIZE + chunk_size;
        let mut writer = ContainerWriter::create_with_max_size(&path, max_size).unwrap();

        // Append a chunk that fills the container exactly to max_size
        let chunk_ref = writer
            .append_chunk(&vec![0xAB; chunk_size as usize])
            .unwrap();
        writer.flush().unwrap();

        assert_eq!(chunk_ref.length, chunk_size as u32);
        assert_eq!(writer.current_size(), max_size);

        // The next append of even 1 byte should fail
        let result = writer.append_chunk(&[0x01]);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_reads_same_reader() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("multi_read.orbitpak");

        let mut writer = ContainerWriter::create(&path).unwrap();
        let ref1 = writer.append_chunk(b"alpha").unwrap();
        let ref2 = writer.append_chunk(b"beta").unwrap();
        let ref3 = writer.append_chunk(b"gamma").unwrap();
        writer.flush().unwrap();

        let reader = ContainerReader::open(&path).unwrap();

        // Read in reverse order
        assert_eq!(reader.read_chunk(&ref3).unwrap(), b"gamma");
        assert_eq!(reader.read_chunk(&ref1).unwrap(), b"alpha");
        assert_eq!(reader.read_chunk(&ref2).unwrap(), b"beta");

        // Read same chunk twice
        assert_eq!(reader.read_chunk(&ref2).unwrap(), b"beta");
        assert_eq!(reader.read_chunk(&ref2).unwrap(), b"beta");

        // Read in original order
        assert_eq!(reader.read_chunk(&ref1).unwrap(), b"alpha");
        assert_eq!(reader.read_chunk(&ref2).unwrap(), b"beta");
        assert_eq!(reader.read_chunk(&ref3).unwrap(), b"gamma");
    }

    #[test]
    fn test_container_id_derivation() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("my_container.orbitpak");

        let writer = ContainerWriter::create(&path).unwrap();
        assert_eq!(writer.container_id(), "my_container");

        // Also verify via a different extension
        let path2 = dir.path().join("another_name.dat");
        let writer2 = ContainerWriter::create(&path2).unwrap();
        assert_eq!(writer2.container_id(), "another_name");

        // And with no extension
        let path3 = dir.path().join("bare_name");
        let writer3 = ContainerWriter::create(&path3).unwrap();
        assert_eq!(writer3.container_id(), "bare_name");
    }
}
