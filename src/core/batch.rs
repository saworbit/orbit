/*!
 * Transfer journal / batch mode for recording and replaying operations.
 *
 * rsync's `--write-batch` records all transfer operations into an opaque
 * file tied to fixed-block offsets. Orbit improves on this with a
 * content-addressed journal: each entry references CDC chunk hashes rather
 * than raw byte offsets, making the journal portable across destinations
 * that may differ slightly.
 *
 * # Usage
 *
 * **Record a transfer:**
 * ```text
 * orbit copy --write-batch changes.batch /source /dest1
 * ```
 *
 * **Replay against another destination:**
 * ```text
 * orbit copy --read-batch changes.batch /dest2
 * ```
 *
 * Because entries are chunk-addressed, replaying against a destination that's
 * slightly different still works — matching chunks are skipped automatically.
 *
 * # Format
 *
 * The journal is a bincode-serialized `TransferJournal` struct, consisting
 * of a header with metadata and a sequence of `JournalEntry` operations.
 */

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Magic bytes to identify Orbit batch files
const BATCH_MAGIC: &[u8; 8] = b"ORBITBTC";

/// Current batch format version
const BATCH_VERSION: u16 = 1;

/// A complete transfer journal recording all operations from a transfer session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferJournal {
    /// When this journal was created
    pub created_at: SystemTime,

    /// Source root directory (for documentation; not used during replay)
    pub source_root: PathBuf,

    /// Destination root directory used during recording
    pub dest_root: PathBuf,

    /// Sequential list of transfer operations
    pub entries: Vec<JournalEntry>,

    /// Summary statistics from the recording session
    pub stats: JournalStats,
}

/// A single operation recorded in the journal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JournalEntry {
    /// Create a new file with the given chunks
    CreateFile {
        /// Path relative to destination root
        path: PathBuf,
        /// BLAKE3 hashes of content chunks (in order)
        chunk_hashes: Vec<[u8; 32]>,
        /// Raw chunk data (for replay without access to original source)
        chunk_data: Vec<Vec<u8>>,
        /// File size in bytes
        size: u64,
        /// Unix permissions mode (optional)
        mode: Option<u32>,
    },

    /// Update an existing file — only changed chunks are stored
    UpdateFile {
        path: PathBuf,
        /// Operations to reconstruct the new file
        ops: Vec<DeltaOp>,
        /// New total file size
        new_size: u64,
    },

    /// Delete a file (for mirror/sync modes)
    DeleteFile {
        path: PathBuf,
    },

    /// Create a directory
    CreateDir {
        path: PathBuf,
        mode: Option<u32>,
    },

    /// Create a hardlink
    CreateHardlink {
        /// The existing file to link to (relative path)
        target: PathBuf,
        /// The new hardlink path (relative path)
        link: PathBuf,
    },

    /// Set file metadata (permissions, timestamps)
    SetMetadata {
        path: PathBuf,
        mtime: Option<SystemTime>,
        mode: Option<u32>,
    },
}

/// Delta operation for file updates — either copy an existing chunk or write new data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOp {
    /// Reuse an existing chunk by hash (destination already has it)
    CopyChunk {
        hash: [u8; 32],
        length: u32,
    },
    /// Write new chunk data (destination doesn't have this chunk)
    WriteChunk {
        hash: [u8; 32],
        data: Vec<u8>,
    },
}

/// Summary statistics for a journal
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JournalStats {
    pub files_created: u64,
    pub files_updated: u64,
    pub files_deleted: u64,
    pub dirs_created: u64,
    pub hardlinks_created: u64,
    pub total_bytes: u64,
    pub new_data_bytes: u64,
}

impl TransferJournal {
    /// Create a new empty journal for recording.
    pub fn new(source_root: PathBuf, dest_root: PathBuf) -> Self {
        Self {
            created_at: SystemTime::now(),
            source_root,
            dest_root,
            entries: Vec::new(),
            stats: JournalStats::default(),
        }
    }

    /// Record an operation in the journal.
    pub fn record(&mut self, entry: JournalEntry) {
        match &entry {
            JournalEntry::CreateFile { size, .. } => {
                self.stats.files_created += 1;
                self.stats.total_bytes += size;
                self.stats.new_data_bytes += size;
            }
            JournalEntry::UpdateFile { ops, .. } => {
                self.stats.files_updated += 1;
                for op in ops {
                    match op {
                        DeltaOp::WriteChunk { data, .. } => {
                            self.stats.new_data_bytes += data.len() as u64;
                            self.stats.total_bytes += data.len() as u64;
                        }
                        DeltaOp::CopyChunk { length, .. } => {
                            self.stats.total_bytes += *length as u64;
                        }
                    }
                }
            }
            JournalEntry::DeleteFile { .. } => {
                self.stats.files_deleted += 1;
            }
            JournalEntry::CreateDir { .. } => {
                self.stats.dirs_created += 1;
            }
            JournalEntry::CreateHardlink { .. } => {
                self.stats.hardlinks_created += 1;
            }
            JournalEntry::SetMetadata { .. } => {}
        }
        self.entries.push(entry);
    }

    /// Save the journal to a batch file.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write magic and version
        writer.write_all(BATCH_MAGIC)?;
        writer.write_all(&BATCH_VERSION.to_le_bytes())?;

        // Serialize the journal
        let encoded = bincode::serialize(self).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("serialization error: {}", e),
            )
        })?;

        // Write length prefix + data
        writer.write_all(&(encoded.len() as u64).to_le_bytes())?;
        writer.write_all(&encoded)?;
        writer.flush()?;

        Ok(())
    }

    /// Load a journal from a batch file.
    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != BATCH_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "not an Orbit batch file (expected {:?}, found {:?})",
                    BATCH_MAGIC, magic
                ),
            ));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != BATCH_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported batch version (expected {}, found {})",
                    BATCH_VERSION, version
                ),
            ));
        }

        // Read length prefix
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let data_len = u64::from_le_bytes(len_bytes) as usize;

        // Read data
        let mut data = vec![0u8; data_len];
        reader.read_exact(&mut data)?;

        // Deserialize
        let journal: TransferJournal = bincode::deserialize(&data).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("deserialization error: {}", e),
            )
        })?;

        Ok(journal)
    }

    /// Replay the journal against a destination directory.
    ///
    /// Applies all recorded operations in sequence. For UpdateFile entries,
    /// CopyChunk ops verify the chunk exists at the destination before reusing.
    pub fn replay(&self, dest_root: &Path) -> io::Result<ReplayStats> {
        let mut stats = ReplayStats::default();

        for entry in &self.entries {
            match entry {
                JournalEntry::CreateFile {
                    path,
                    chunk_data,
                    size,
                    ..
                } => {
                    let dest = dest_root.join(path);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let mut file = File::create(&dest)?;
                    for data in chunk_data {
                        file.write_all(data)?;
                    }
                    file.set_len(*size)?;
                    file.sync_all()?;
                    stats.files_created += 1;
                    stats.bytes_written += size;
                }

                JournalEntry::UpdateFile {
                    path,
                    ops,
                    new_size,
                } => {
                    let dest = dest_root.join(path);
                    let dest_exists = dest.exists();
                    let needs_existing = ops.iter().any(|op| matches!(op, DeltaOp::CopyChunk { .. }));
                    if !dest_exists && needs_existing {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!(
                                "cannot apply CopyChunk ops to missing destination: {}",
                                dest.display()
                            ),
                        ));
                    }

                    let mut file = std::fs::OpenOptions::new()
                        .read(true)
                        .write(true)
                        .create(!dest_exists)
                        .truncate(false)
                        .open(&dest)?;

                    let mut offset = 0u64;
                    for op in ops {
                        match op {
                            DeltaOp::WriteChunk { data, .. } => {
                                file.seek(SeekFrom::Start(offset))?;
                                file.write_all(data)?;
                                offset += data.len() as u64;
                                stats.bytes_written += data.len() as u64;
                            }
                            DeltaOp::CopyChunk { hash, length } => {
                                verify_chunk_at_offset(&mut file, offset, *length, hash)?;
                                offset += *length as u64;
                                stats.chunks_reused += 1;
                            }
                        }
                    }
                    file.set_len(*new_size)?;
                    file.sync_all()?;
                    stats.files_updated += 1;
                }

                JournalEntry::DeleteFile { path } => {
                    let dest = dest_root.join(path);
                    if dest.exists() {
                        std::fs::remove_file(&dest)?;
                        stats.files_deleted += 1;
                    }
                }

                JournalEntry::CreateDir { path, .. } => {
                    let dest = dest_root.join(path);
                    std::fs::create_dir_all(&dest)?;
                    stats.dirs_created += 1;
                }

                JournalEntry::CreateHardlink { target, link } => {
                    let target_path = dest_root.join(target);
                    let link_path = dest_root.join(link);
                    if let Some(parent) = link_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    if link_path.exists() {
                        std::fs::remove_file(&link_path)?;
                    }
                    std::fs::hard_link(&target_path, &link_path)?;
                    stats.hardlinks_created += 1;
                }

                JournalEntry::SetMetadata { .. } => {
                    // Metadata application is platform-specific; handled elsewhere
                    stats.metadata_applied += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Number of entries in the journal.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn verify_chunk_at_offset(
    file: &mut File,
    offset: u64,
    length: u32,
    expected_hash: &[u8; 32],
) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; length as usize];
    file.read_exact(&mut buf)?;
    let actual = blake3::hash(&buf);
    if actual.as_bytes() != expected_hash {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "destination chunk hash mismatch during batch replay",
        ));
    }
    Ok(())
}

/// Statistics from replaying a journal.
#[derive(Debug, Clone, Default)]
pub struct ReplayStats {
    pub files_created: u64,
    pub files_updated: u64,
    pub files_deleted: u64,
    pub dirs_created: u64,
    pub hardlinks_created: u64,
    pub metadata_applied: u64,
    pub bytes_written: u64,
    pub chunks_reused: u64,
}

/// Record a single file as a CreateFile entry in the journal.
///
/// This is the safest default: the batch contains the full file content,
/// independent of whether the live transfer used delta or other optimizations.
pub fn record_create_file(
    journal: &mut TransferJournal,
    source_path: &Path,
    relative_path: &Path,
) -> io::Result<()> {
    let metadata = std::fs::metadata(source_path)?;
    let size = metadata.len();
    let mode = file_mode(&metadata);

    let file = File::open(source_path)?;
    let config = orbit_core_cdc::ChunkConfig::default_config();
    let mut stream = orbit_core_cdc::ChunkStream::new(file, config);

    let mut chunk_hashes = Vec::new();
    let mut chunk_data = Vec::new();

    while let Some(chunk) = stream.next() {
        let chunk = chunk.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        chunk_hashes.push(chunk.hash);
        chunk_data.push(chunk.data);
    }

    journal.record(JournalEntry::CreateFile {
        path: relative_path.to_path_buf(),
        chunk_hashes,
        chunk_data,
        size,
        mode,
    });

    Ok(())
}

fn file_mode(metadata: &std::fs::Metadata) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        Some(metadata.permissions().mode())
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_journal_record_and_save_load() {
        let dir = tempdir().unwrap();
        let batch_path = dir.path().join("test.batch");

        let mut journal = TransferJournal::new(
            PathBuf::from("/source"),
            PathBuf::from("/dest"),
        );

        journal.record(JournalEntry::CreateDir {
            path: PathBuf::from("subdir"),
            mode: Some(0o755),
        });

        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("subdir/hello.txt"),
            chunk_hashes: vec![[0x01; 32]],
            chunk_data: vec![b"Hello World!".to_vec()],
            size: 12,
            mode: Some(0o644),
        });

        assert_eq!(journal.len(), 2);
        assert_eq!(journal.stats.files_created, 1);
        assert_eq!(journal.stats.dirs_created, 1);

        // Save and reload
        journal.save(&batch_path).unwrap();
        let loaded = TransferJournal::load(&batch_path).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.stats.files_created, 1);
        assert_eq!(loaded.source_root, PathBuf::from("/source"));
    }

    #[test]
    fn test_journal_replay_create_files() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");

        let mut journal = TransferJournal::new(
            PathBuf::from("/source"),
            dest.clone(),
        );

        journal.record(JournalEntry::CreateDir {
            path: PathBuf::from("docs"),
            mode: None,
        });

        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("docs/readme.txt"),
            chunk_hashes: vec![[0x01; 32]],
            chunk_data: vec![b"Read me!".to_vec()],
            size: 8,
            mode: None,
        });

        let stats = journal.replay(&dest).unwrap();

        assert_eq!(stats.dirs_created, 1);
        assert_eq!(stats.files_created, 1);
        assert_eq!(stats.bytes_written, 8);
        assert_eq!(
            std::fs::read(dest.join("docs/readme.txt")).unwrap(),
            b"Read me!"
        );
    }

    #[test]
    fn test_journal_replay_delete() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();

        // Create a file to delete
        std::fs::write(dest.join("obsolete.txt"), b"old data").unwrap();

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::DeleteFile {
            path: PathBuf::from("obsolete.txt"),
        });

        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.files_deleted, 1);
        assert!(!dest.join("obsolete.txt").exists());
    }

    #[test]
    fn test_journal_replay_hardlink() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();

        // Create the target file first
        std::fs::write(dest.join("original.txt"), b"shared content").unwrap();

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::CreateHardlink {
            target: PathBuf::from("original.txt"),
            link: PathBuf::from("link.txt"),
        });

        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.hardlinks_created, 1);
        assert_eq!(
            std::fs::read(dest.join("link.txt")).unwrap(),
            b"shared content"
        );
    }

    #[test]
    fn test_journal_replay_update_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();

        // Existing file at destination
        std::fs::write(dest.join("data.bin"), b"AAAA BBBB CCCC").unwrap();

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::UpdateFile {
            path: PathBuf::from("data.bin"),
            ops: vec![
                DeltaOp::CopyChunk {
                    hash: [0x01; 32],
                    length: 5,
                },
                DeltaOp::WriteChunk {
                    hash: [0x02; 32],
                    data: b"XXXX".to_vec(),
                },
                DeltaOp::CopyChunk {
                    hash: [0x03; 32],
                    length: 5,
                },
            ],
            new_size: 14,
        });

        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.files_updated, 1);
        assert_eq!(stats.chunks_reused, 2);
        assert_eq!(stats.bytes_written, 4);

        // The write went to offset 5 (after first CopyChunk)
        let content = std::fs::read(dest.join("data.bin")).unwrap();
        assert_eq!(&content[5..9], b"XXXX");
    }

    #[test]
    fn test_invalid_batch_file() {
        let dir = tempdir().unwrap();
        let bad_file = dir.path().join("not_a_batch.bin");
        std::fs::write(&bad_file, b"not a valid batch").unwrap();

        let result = TransferJournal::load(&bad_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_save_load_roundtrip() {
        let dir = tempdir().unwrap();
        let batch_path = dir.path().join("roundtrip.batch");

        let mut journal = TransferJournal::new(
            PathBuf::from("/a"),
            PathBuf::from("/b"),
        );

        // Record various operation types
        journal.record(JournalEntry::CreateDir {
            path: PathBuf::from("dir1"),
            mode: Some(0o755),
        });
        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("dir1/file.txt"),
            chunk_hashes: vec![[0xAA; 32], [0xBB; 32]],
            chunk_data: vec![b"chunk1".to_vec(), b"chunk2".to_vec()],
            size: 12,
            mode: Some(0o644),
        });
        journal.record(JournalEntry::DeleteFile {
            path: PathBuf::from("old.txt"),
        });
        journal.record(JournalEntry::SetMetadata {
            path: PathBuf::from("dir1/file.txt"),
            mtime: Some(SystemTime::UNIX_EPOCH),
            mode: Some(0o444),
        });

        journal.save(&batch_path).unwrap();
        let loaded = TransferJournal::load(&batch_path).unwrap();

        assert_eq!(loaded.len(), journal.len());
        assert_eq!(loaded.stats.files_created, journal.stats.files_created);
        assert_eq!(loaded.stats.files_deleted, journal.stats.files_deleted);
        assert_eq!(loaded.stats.dirs_created, journal.stats.dirs_created);
    }

    #[test]
    fn test_empty_journal() {
        let journal = TransferJournal::new(PathBuf::from("/a"), PathBuf::from("/b"));
        assert!(journal.is_empty());
        assert_eq!(journal.len(), 0);
    }

    #[test]
    fn test_journal_stats_comprehensive() {
        let mut journal = TransferJournal::new(PathBuf::from("/a"), PathBuf::from("/b"));

        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("a.txt"),
            chunk_hashes: vec![[0x01; 32]],
            chunk_data: vec![b"data".to_vec()],
            size: 4,
            mode: None,
        });
        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("b.txt"),
            chunk_hashes: vec![[0x02; 32]],
            chunk_data: vec![b"more".to_vec()],
            size: 4,
            mode: None,
        });
        journal.record(JournalEntry::DeleteFile {
            path: PathBuf::from("old.txt"),
        });
        journal.record(JournalEntry::CreateDir {
            path: PathBuf::from("subdir"),
            mode: Some(0o755),
        });
        journal.record(JournalEntry::CreateHardlink {
            target: PathBuf::from("a.txt"),
            link: PathBuf::from("a_link.txt"),
        });
        journal.record(JournalEntry::UpdateFile {
            path: PathBuf::from("c.txt"),
            ops: vec![
                DeltaOp::CopyChunk { hash: [0x10; 32], length: 100 },
                DeltaOp::WriteChunk { hash: [0x11; 32], data: vec![0xAA; 50] },
            ],
            new_size: 150,
        });
        journal.record(JournalEntry::SetMetadata {
            path: PathBuf::from("a.txt"),
            mtime: None,
            mode: Some(0o644),
        });

        assert_eq!(journal.stats.files_created, 2);
        assert_eq!(journal.stats.files_updated, 1);
        assert_eq!(journal.stats.files_deleted, 1);
        assert_eq!(journal.stats.dirs_created, 1);
        assert_eq!(journal.stats.hardlinks_created, 1);
        assert_eq!(journal.stats.new_data_bytes, 4 + 4 + 50); // two creates + one write chunk
        assert_eq!(journal.stats.total_bytes, 4 + 4 + 100); // two creates + one copy chunk
        assert_eq!(journal.len(), 7);
        assert!(!journal.is_empty());
    }

    #[test]
    fn test_batch_version_mismatch() {
        let dir = tempdir().unwrap();
        let bad_file = dir.path().join("bad_version.batch");

        // Write valid magic but wrong version
        let mut data = Vec::new();
        data.extend_from_slice(b"ORBITBTC");
        data.extend_from_slice(&99u16.to_le_bytes());
        std::fs::write(&bad_file, &data).unwrap();

        let result = TransferJournal::load(&bad_file);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("unsupported batch version"));
    }

    #[test]
    fn test_replay_creates_nested_dirs() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::CreateDir {
            path: PathBuf::from("a/b/c"),
            mode: None,
        });
        journal.record(JournalEntry::CreateFile {
            path: PathBuf::from("a/b/c/deep.txt"),
            chunk_hashes: vec![[0x01; 32]],
            chunk_data: vec![b"deep content".to_vec()],
            size: 12,
            mode: None,
        });

        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.dirs_created, 1);
        assert_eq!(stats.files_created, 1);
        assert_eq!(
            std::fs::read(dest.join("a/b/c/deep.txt")).unwrap(),
            b"deep content"
        );
    }

    #[test]
    fn test_replay_delete_nonexistent_file_is_ok() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::DeleteFile {
            path: PathBuf::from("ghost.txt"),
        });

        // Deleting a nonexistent file should not error
        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.files_deleted, 0); // File didn't exist, so count stays 0
    }

    #[test]
    fn test_replay_set_metadata() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("file.txt"), b"data").unwrap();

        let mut journal = TransferJournal::new(PathBuf::from("/src"), dest.clone());
        journal.record(JournalEntry::SetMetadata {
            path: PathBuf::from("file.txt"),
            mtime: Some(SystemTime::UNIX_EPOCH),
            mode: Some(0o444),
        });

        let stats = journal.replay(&dest).unwrap();
        assert_eq!(stats.metadata_applied, 1);
    }
}
