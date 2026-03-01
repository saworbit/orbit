/*!
 * In-place file update mode for space-constrained transfers.
 *
 * Orbit's default transfer creates a temp file, writes to it, then renames
 * over the destination. This is safe but requires N bytes of free space for
 * an N-byte file — prohibitive for large files (VM images, database files)
 * where only a small portion changed.
 *
 * In-place mode modifies the destination file directly, with configurable
 * safety levels:
 *
 * - **Reflink**: Uses copy-on-write (FICLONE/FICLONERANGE) to snapshot the
 *   file before modification. Zero-cost on btrfs/XFS/APFS. Falls back to
 *   journaled if the filesystem doesn't support reflinks.
 *
 * - **Journaled**: Records original bytes in a Magnetar-compatible undo journal
 *   before each overwrite. On crash, the journal can reconstruct the original.
 *
 * - **Unsafe**: Direct overwrite with no recovery. User opt-in only.
 *
 * Unlike rsync's `--inplace` which has no crash safety, Orbit's default
 * (Reflink) provides both space efficiency AND crash safety.
 */

use crate::config::InplaceSafety;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const JOURNAL_MAGIC: &[u8; 8] = b"ORBITJNL";
const JOURNAL_VERSION: u16 = 1;

/// An in-place file writer that applies chunk-level updates directly
/// to an existing destination file.
pub struct InplaceWriter {
    file: File,
    safety: InplaceSafety,
    journal: Option<UndoJournal>,
    reflink_created: bool,
    reflink_attempted: bool,
    dest_path: PathBuf,
    bytes_written: u64,
}

/// A simple undo journal that records original bytes before overwrite.
/// Each entry stores (offset, original_data) so the file can be restored.
struct UndoJournal {
    file: File,
    journal_path: PathBuf,
    entries: u64,
}

/// A single undo journal entry
#[derive(Debug, Serialize, Deserialize)]
struct JournalEntry {
    offset: u64,
    original_data: Vec<u8>,
}

impl UndoJournal {
    fn open(dest_path: &Path) -> io::Result<Self> {
        let journal_path = dest_path.with_extension("orbit_undo_journal");
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&journal_path)?;

        file.write_all(JOURNAL_MAGIC)?;
        file.write_all(&JOURNAL_VERSION.to_le_bytes())?;
        file.flush()?;
        file.sync_data()?;

        Ok(Self {
            file,
            journal_path,
            entries: 0,
        })
    }

    fn record(&mut self, offset: u64, original_data: Vec<u8>) -> io::Result<()> {
        let entry = JournalEntry {
            offset,
            original_data,
        };

        let encoded = bincode::serialize(&entry).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("journal serialization error: {}", e),
            )
        })?;

        let len = encoded.len() as u32;
        self.file.write_all(&len.to_le_bytes())?;
        self.file.write_all(&encoded)?;
        self.file.flush()?;
        self.file.sync_data()?;
        self.entries += 1;
        Ok(())
    }

    fn load_entries(path: &Path) -> io::Result<Vec<JournalEntry>> {
        let mut file = File::open(path)?;

        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if &magic != JOURNAL_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "not an Orbit undo journal",
            ));
        }

        let mut version_bytes = [0u8; 2];
        file.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != JOURNAL_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported journal version (expected {}, found {})",
                    JOURNAL_VERSION, version
                ),
            ));
        }

        let mut entries = Vec::new();
        loop {
            let mut len_bytes = [0u8; 4];
            match file.read_exact(&mut len_bytes) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            let len = u32::from_le_bytes(len_bytes) as usize;
            let mut buf = vec![0u8; len];
            file.read_exact(&mut buf)?;
            let entry: JournalEntry = bincode::deserialize(&buf).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("journal deserialization error: {}", e),
                )
            })?;
            entries.push(entry);
        }

        Ok(entries)
    }
}

impl InplaceWriter {
    /// Open a destination file for in-place modification.
    ///
    /// The file must already exist. For new files, use the standard
    /// temp+rename approach instead.
    pub fn open(dest_path: &Path, safety: InplaceSafety) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(dest_path)?;

        let journal = match safety {
            InplaceSafety::Journaled => Some(UndoJournal::open(dest_path)?),
            _ => None,
        };

        Ok(Self {
            file,
            safety,
            journal,
            reflink_created: false,
            reflink_attempted: false,
            dest_path: dest_path.to_owned(),
            bytes_written: 0,
        })
    }

    /// Apply a data chunk at the given offset in-place.
    ///
    /// Depending on the safety level:
    /// - Reflink: ensures a CoW snapshot exists before first write
    /// - Journaled: records original bytes before overwriting
    /// - Unsafe: writes directly with no safety net
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
        match self.safety {
            InplaceSafety::Reflink => {
                if !self.reflink_attempted {
                    let created = self.try_create_reflink_snapshot()?;
                    self.reflink_created = created;
                    self.reflink_attempted = true;
                    if !created {
                        // Fall back to journaled safety when reflink is unavailable.
                        self.safety = InplaceSafety::Journaled;
                        if self.journal.is_none() {
                            self.journal = Some(UndoJournal::open(&self.dest_path)?);
                        }
                    }
                }
                if self.safety == InplaceSafety::Journaled {
                    self.record_journal_entry(offset, data)?;
                }
            }
            InplaceSafety::Journaled => {
                self.record_journal_entry(offset, data)?;
            }
            InplaceSafety::Unsafe => {
                // No safety measures
            }
        }

        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;
        self.bytes_written += data.len() as u64;
        Ok(())
    }

    fn record_journal_entry(&mut self, offset: u64, data: &[u8]) -> io::Result<()> {
        // Read original bytes before overwriting
        let mut original = vec![0u8; data.len()];
        self.file.seek(SeekFrom::Start(offset))?;
        // Read as much as possible (file may be shorter than write range)
        let bytes_read = read_fully(&mut self.file, &mut original)?;
        original.truncate(bytes_read);

        if let Some(ref mut journal) = self.journal {
            journal.record(offset, original)?;
        }
        Ok(())
    }

    /// Finalize the in-place update: set correct file size and sync.
    ///
    /// If the new file is shorter than the old one, this truncates.
    /// If longer, the file was already extended by writes past the old end.
    pub fn finalize(self, final_size: u64) -> io::Result<InplaceStats> {
        self.file.set_len(final_size)?;
        self.file.sync_all()?;

        // Clean up journal on successful completion
        if let Some(ref journal) = self.journal {
            let _ = std::fs::remove_file(&journal.journal_path);
        }

        // Clean up reflink snapshot on success
        if self.reflink_created {
            let snapshot_path = reflink_snapshot_path(&self.dest_path);
            let _ = std::fs::remove_file(&snapshot_path);
        }

        Ok(InplaceStats {
            bytes_written: self.bytes_written,
        })
    }

    /// Try to create a reflink (CoW) snapshot of the file before modification.
    ///
    /// On filesystems that support reflinks (btrfs, XFS 4.x+, APFS), this is
    /// an O(1) metadata operation — no data is copied. The snapshot shares
    /// physical blocks with the original until either is modified.
    ///
    /// If reflinks are not supported, falls back to no snapshot (the Reflink
    /// safety level degrades to Unsafe on non-CoW filesystems).
    fn try_create_reflink_snapshot(&self) -> io::Result<bool> {
        let snapshot_path = reflink_snapshot_path(&self.dest_path);

        // Try reflink copy (platform-specific)
        match try_reflink(&self.dest_path, &snapshot_path) {
            Ok(()) => {
                tracing::debug!(
                    "Created reflink snapshot: {:?} -> {:?}",
                    self.dest_path,
                    snapshot_path
                );
                Ok(true)
            }
            Err(e) => {
                tracing::debug!(
                    "Reflink not supported on this filesystem ({}), falling back to journaled",
                    e
                );
                // Not an error — just means this FS doesn't support CoW
                Ok(false)
            }
        }
    }
}

/// Statistics from an in-place update operation.
#[derive(Debug, Clone)]
pub struct InplaceStats {
    pub bytes_written: u64,
}

/// Generate the path for a reflink snapshot backup.
fn reflink_snapshot_path(dest: &Path) -> PathBuf {
    dest.with_extension("orbit_inplace_snapshot")
}

/// Read as many bytes as possible into the buffer, handling short reads.
fn read_fully(reader: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match reader.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(total)
}

/// Attempt a reflink (copy-on-write) clone of a file.
///
/// Platform-specific:
/// - Linux: ioctl(FICLONE) on btrfs/XFS
/// - macOS: clonefile(2)
/// - Windows/other: not supported, returns error
#[cfg(target_os = "linux")]
fn try_reflink(src: &Path, dst: &Path) -> io::Result<()> {
    use std::os::unix::io::AsRawFd;

    // FICLONE ioctl number: _IOW(0x94, 9, int) = 0x40049409
    const FICLONE: libc::c_ulong = 0x40049409;

    let src_file = File::open(src)?;
    let dst_file = File::create(dst)?;

    let ret = unsafe { libc::ioctl(dst_file.as_raw_fd(), FICLONE, src_file.as_raw_fd()) };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn try_reflink(src: &Path, dst: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    extern "C" {
        fn clonefile(src: *const libc::c_char, dst: *const libc::c_char, flags: u32) -> i32;
    }

    let src_c = CString::new(src.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;
    let dst_c = CString::new(dst.as_os_str().as_bytes())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid path"))?;

    let ret = unsafe { clonefile(src_c.as_ptr(), dst_c.as_ptr(), 0) };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn try_reflink(_src: &Path, _dst: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "reflinks not supported on this platform",
    ))
}

/// Recover a file from its undo journal after a crash.
///
/// Reads the journal entries and restores original bytes at each recorded
/// offset, effectively rolling back the in-place update.
pub fn recover_from_journal(dest_path: &Path) -> io::Result<bool> {
    let journal_path = dest_path.with_extension("orbit_undo_journal");
    if !journal_path.exists() {
        return Ok(false); // No journal, nothing to recover
    }

    // Check for reflink snapshot first — it's a complete copy
    let snapshot_path = reflink_snapshot_path(dest_path);
    if snapshot_path.exists() {
        std::fs::rename(&snapshot_path, dest_path)?;
        let _ = std::fs::remove_file(&journal_path);
        tracing::info!("Recovered {:?} from reflink snapshot", dest_path);
        return Ok(true);
    }

    let entries = UndoJournal::load_entries(&journal_path)?;
    if entries.is_empty() {
        let _ = std::fs::remove_file(&journal_path);
        return Ok(false);
    }

    let mut file = OpenOptions::new().write(true).open(dest_path)?;
    for entry in entries.into_iter().rev() {
        file.seek(SeekFrom::Start(entry.offset))?;
        file.write_all(&entry.original_data)?;
    }
    file.sync_all()?;

    tracing::info!("Recovered {:?} from journal entries", dest_path);
    let _ = std::fs::remove_file(&journal_path);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_inplace_write_unsafe() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest.bin");

        // Create initial file
        std::fs::write(&dest, b"AAAA BBBB CCCC").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Unsafe).unwrap();
        writer.write_at(5, b"XXXX").unwrap();
        writer.finalize(14).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"AAAA XXXX CCCC");
    }

    #[test]
    fn test_inplace_write_journaled() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest.bin");

        std::fs::write(&dest, b"Hello World").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Journaled).unwrap();
        writer.write_at(6, b"Orbit").unwrap();
        writer.finalize(11).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"Hello Orbit");

        // Journal should be cleaned up on success
        assert!(!dest.with_extension("orbit_undo_journal").exists());
    }

    #[test]
    fn test_inplace_truncate_shorter() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest.bin");

        std::fs::write(&dest, b"Long content here").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Unsafe).unwrap();
        writer.write_at(0, b"Short").unwrap();
        writer.finalize(5).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"Short");
    }

    #[test]
    fn test_inplace_extend_longer() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dest.bin");

        std::fs::write(&dest, b"Hi").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Unsafe).unwrap();
        writer.write_at(0, b"Hello World!").unwrap();
        writer.finalize(12).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"Hello World!");
    }

    #[test]
    fn test_recover_no_journal() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("clean.bin");
        std::fs::write(&dest, b"clean").unwrap();

        let recovered = recover_from_journal(&dest).unwrap();
        assert!(!recovered);
    }

    #[test]
    fn test_inplace_open_nonexistent_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("nonexistent.bin");

        let result = InplaceWriter::open(&dest, InplaceSafety::Unsafe);
        assert!(result.is_err());
    }

    #[test]
    fn test_inplace_multiple_writes() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("multi.bin");

        std::fs::write(&dest, b"AAAA BBBB CCCC DDDD").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Unsafe).unwrap();
        writer.write_at(0, b"1111").unwrap();
        writer.write_at(10, b"3333").unwrap();
        let stats = writer.finalize(19).unwrap();

        let content = std::fs::read(&dest).unwrap();
        assert_eq!(&content[0..4], b"1111");
        assert_eq!(&content[5..9], b"BBBB");
        assert_eq!(&content[10..14], b"3333");
        assert_eq!(stats.bytes_written, 8);
    }

    #[test]
    fn test_inplace_write_zero_length() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("empty_write.bin");

        std::fs::write(&dest, b"Hello").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Unsafe).unwrap();
        writer.write_at(0, b"").unwrap();
        let stats = writer.finalize(5).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"Hello");
        assert_eq!(stats.bytes_written, 0);
    }

    #[test]
    fn test_inplace_journaled_multiple_writes() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("journal_multi.bin");

        std::fs::write(&dest, b"AABBCCDD").unwrap();

        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Journaled).unwrap();
        writer.write_at(0, b"XX").unwrap();
        writer.write_at(4, b"YY").unwrap();
        let stats = writer.finalize(8).unwrap();

        assert_eq!(std::fs::read(&dest).unwrap(), b"XXBBYYDD");
        assert_eq!(stats.bytes_written, 4);

        // Journal cleaned up
        assert!(!dest.with_extension("orbit_undo_journal").exists());
    }

    #[test]
    fn test_inplace_reflink_on_non_cow_fs() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("reflink_test.bin");

        std::fs::write(&dest, b"test data").unwrap();

        // On most test filesystems, reflinks aren't supported.
        // This should still work — it degrades gracefully.
        let mut writer = InplaceWriter::open(&dest, InplaceSafety::Reflink).unwrap();
        writer.write_at(0, b"new!").unwrap();
        writer.finalize(9).unwrap();

        let content = std::fs::read(&dest).unwrap();
        assert_eq!(&content[..4], b"new!");
        assert_eq!(&content[4..], b" data");
    }

    #[test]
    fn test_recover_from_reflink_snapshot() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("recoverable.bin");
        let snapshot = dir.path().join("recoverable.orbit_inplace_snapshot");
        let journal = dir.path().join("recoverable.orbit_undo_journal");

        // Simulate a crash: snapshot and journal exist
        std::fs::write(&dest, b"corrupted").unwrap();
        std::fs::write(&snapshot, b"original good data").unwrap();
        std::fs::write(&journal, b"fake journal").unwrap();

        let recovered = recover_from_journal(&dest).unwrap();
        assert!(recovered);

        // Dest should be restored from snapshot
        assert_eq!(std::fs::read(&dest).unwrap(), b"original good data");
        // Both snapshot and journal should be cleaned up
        assert!(!snapshot.exists());
        assert!(!journal.exists());
    }
}
