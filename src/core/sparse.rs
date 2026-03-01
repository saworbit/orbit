/*!
 * Sparse file handling for efficient transfer of zero-heavy files.
 *
 * VM disk images (qcow2, VMDK), database files, and pre-allocated logs are
 * often 90%+ zeros. This module detects zero-regions during CDC chunking
 * (which already reads every byte) and writes proper sparse files at the
 * destination by seeking over zero regions instead of writing them.
 *
 * Unlike rsync, which cannot combine --sparse with --inplace, Orbit's
 * CDC chunks are independently addressable, so sparse + in-place works
 * naturally.
 */

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Sparse file handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SparseMode {
    /// Detect sparse-eligible chunks automatically during CDC.
    /// Zero chunks are written as holes, non-zero chunks are written normally.
    #[default]
    Auto,

    /// Always check for and create sparse holes (even for small files)
    Always,

    /// Disable sparse handling entirely — write all bytes including zeros.
    Never,
}

/// Statistics from a sparse-aware write operation.
#[derive(Debug, Clone, Default)]
pub struct SparseWriteStats {
    /// Total logical bytes (including holes)
    pub logical_bytes: u64,
    /// Actual bytes written to disk (excluding holes)
    pub physical_bytes: u64,
    /// Number of sparse holes created
    pub holes_created: u64,
    /// Total bytes saved by sparse holes
    pub bytes_saved: u64,
}

impl SparseWriteStats {
    /// Ratio of physical to logical bytes (1.0 = no savings, 0.0 = all holes)
    pub fn density_ratio(&self) -> f64 {
        if self.logical_bytes == 0 {
            1.0
        } else {
            self.physical_bytes as f64 / self.logical_bytes as f64
        }
    }
}

/// Minimum file size to consider for sparse optimization.
/// Files smaller than this are written densely regardless of SparseMode::Auto.
const SPARSE_MIN_FILE_SIZE: u64 = 64 * 1024; // 64 KB

/// Write data chunks to a file, creating sparse holes for all-zero chunks.
///
/// Each chunk is either written normally or skipped (creating a hole) based
/// on its `is_zero` flag. The file is truncated to the correct logical size
/// after writing to ensure trailing holes are properly represented.
///
/// # Arguments
/// * `dest` - Destination file path
/// * `chunks` - Iterator of (offset, length, data, is_zero) tuples
/// * `total_size` - Total logical file size (for final truncation)
/// * `mode` - Sparse handling mode
///
/// # Returns
/// Statistics about the sparse write operation
pub fn write_sparse<I>(
    dest: &Path,
    chunks: I,
    total_size: u64,
    mode: SparseMode,
) -> io::Result<SparseWriteStats>
where
    I: IntoIterator<Item = SparseChunk>,
{
    let sparse_enabled = match mode {
        SparseMode::Never => false,
        SparseMode::Always => true,
        SparseMode::Auto => total_size >= SPARSE_MIN_FILE_SIZE,
    };

    let mut file = File::create(dest)?;
    let mut stats = SparseWriteStats::default();
    stats.logical_bytes = total_size;

    for chunk in chunks {
        if sparse_enabled && chunk.is_zero {
            // Skip writing — seek past the region to create a hole.
            // The filesystem will return zeros when reading this region.
            file.seek(SeekFrom::Start(chunk.offset + chunk.length as u64))?;
            stats.holes_created += 1;
            stats.bytes_saved += chunk.length as u64;
        } else {
            // Write data at the correct offset
            file.seek(SeekFrom::Start(chunk.offset))?;
            file.write_all(&chunk.data)?;
            stats.physical_bytes += chunk.length as u64;
        }
    }

    // Set the final file size. This is critical for:
    // 1. Trailing holes: ensures the file reports the correct logical size
    // 2. Short files: ensures we don't leave the file too small
    file.set_len(total_size)?;
    file.sync_all()?;

    Ok(stats)
}

/// A chunk of data ready for sparse-aware writing.
pub struct SparseChunk {
    pub offset: u64,
    pub length: usize,
    pub data: Vec<u8>,
    pub is_zero: bool,
}

impl From<orbit_core_cdc::Chunk> for SparseChunk {
    fn from(chunk: orbit_core_cdc::Chunk) -> Self {
        Self {
            offset: chunk.offset,
            length: chunk.length,
            data: chunk.data,
            is_zero: chunk.is_zero,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_sparse_all_zeros() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("sparse.bin");
        let total_size = 1024 * 1024; // 1 MB

        let chunks = vec![SparseChunk {
            offset: 0,
            length: total_size as usize,
            data: vec![0u8; total_size as usize],
            is_zero: true,
        }];

        let stats = write_sparse(&dest, chunks, total_size, SparseMode::Always).unwrap();

        assert_eq!(stats.logical_bytes, total_size);
        assert_eq!(stats.physical_bytes, 0);
        assert_eq!(stats.holes_created, 1);
        assert_eq!(stats.bytes_saved, total_size);

        // File should report correct logical size
        let meta = std::fs::metadata(&dest).unwrap();
        assert_eq!(meta.len(), total_size);

        // Reading should return zeros
        let data = std::fs::read(&dest).unwrap();
        assert!(data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_write_sparse_mixed() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("mixed.bin");

        let nonzero_data = vec![0xABu8; 4096];
        let chunks = vec![
            SparseChunk {
                offset: 0,
                length: 4096,
                data: vec![0u8; 4096],
                is_zero: true,
            },
            SparseChunk {
                offset: 4096,
                length: 4096,
                data: nonzero_data.clone(),
                is_zero: false,
            },
            SparseChunk {
                offset: 8192,
                length: 4096,
                data: vec![0u8; 4096],
                is_zero: true,
            },
        ];

        let stats = write_sparse(&dest, chunks, 12288, SparseMode::Always).unwrap();

        assert_eq!(stats.logical_bytes, 12288);
        assert_eq!(stats.physical_bytes, 4096);
        assert_eq!(stats.holes_created, 2);
        assert_eq!(stats.bytes_saved, 8192);

        // Verify content
        let data = std::fs::read(&dest).unwrap();
        assert_eq!(data.len(), 12288);
        assert!(data[..4096].iter().all(|&b| b == 0));
        assert_eq!(&data[4096..8192], &nonzero_data[..]);
        assert!(data[8192..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_write_sparse_never_mode() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("dense.bin");

        let chunks = vec![SparseChunk {
            offset: 0,
            length: 4096,
            data: vec![0u8; 4096],
            is_zero: true,
        }];

        let stats = write_sparse(&dest, chunks, 4096, SparseMode::Never).unwrap();

        // Never mode writes all bytes, no holes
        assert_eq!(stats.physical_bytes, 4096);
        assert_eq!(stats.holes_created, 0);
    }

    #[test]
    fn test_write_sparse_auto_small_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("small.bin");

        // File smaller than SPARSE_MIN_FILE_SIZE — auto mode writes densely
        let chunks = vec![SparseChunk {
            offset: 0,
            length: 1024,
            data: vec![0u8; 1024],
            is_zero: true,
        }];

        let stats = write_sparse(&dest, chunks, 1024, SparseMode::Auto).unwrap();

        // Auto mode skips sparse for small files
        assert_eq!(stats.physical_bytes, 1024);
        assert_eq!(stats.holes_created, 0);
    }

    #[test]
    fn test_sparse_write_stats_density() {
        let stats = SparseWriteStats {
            logical_bytes: 1000,
            physical_bytes: 250,
            holes_created: 3,
            bytes_saved: 750,
        };
        assert!((stats.density_ratio() - 0.25).abs() < f64::EPSILON);

        let empty = SparseWriteStats::default();
        assert!((empty.density_ratio() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_write_sparse_empty_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("empty.bin");

        let chunks: Vec<SparseChunk> = vec![];
        let stats = write_sparse(&dest, chunks, 0, SparseMode::Always).unwrap();

        assert_eq!(stats.logical_bytes, 0);
        assert_eq!(stats.physical_bytes, 0);
        assert_eq!(stats.holes_created, 0);
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 0);
    }

    #[test]
    fn test_write_sparse_auto_large_file() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("large.bin");

        // File >= SPARSE_MIN_FILE_SIZE (64KB) triggers auto sparse
        let total_size = 128 * 1024; // 128 KB
        let chunks = vec![SparseChunk {
            offset: 0,
            length: total_size as usize,
            data: vec![0u8; total_size as usize],
            is_zero: true,
        }];

        let stats = write_sparse(&dest, chunks, total_size, SparseMode::Auto).unwrap();

        // Auto mode should create a hole for this large zero file
        assert_eq!(stats.holes_created, 1);
        assert_eq!(stats.physical_bytes, 0);
        assert_eq!(stats.bytes_saved, total_size);
    }

    #[test]
    fn test_write_sparse_multiple_sequential_nonzero_chunks() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("multi.bin");

        let chunks = vec![
            SparseChunk {
                offset: 0,
                length: 100,
                data: vec![0xAAu8; 100],
                is_zero: false,
            },
            SparseChunk {
                offset: 100,
                length: 100,
                data: vec![0xBBu8; 100],
                is_zero: false,
            },
            SparseChunk {
                offset: 200,
                length: 100,
                data: vec![0xCCu8; 100],
                is_zero: false,
            },
        ];

        let stats = write_sparse(&dest, chunks, 300, SparseMode::Always).unwrap();

        assert_eq!(stats.physical_bytes, 300);
        assert_eq!(stats.holes_created, 0);

        let data = std::fs::read(&dest).unwrap();
        assert_eq!(data.len(), 300);
        assert!(data[..100].iter().all(|&b| b == 0xAA));
        assert!(data[100..200].iter().all(|&b| b == 0xBB));
        assert!(data[200..300].iter().all(|&b| b == 0xCC));
    }

    #[test]
    fn test_write_sparse_trailing_hole() {
        let dir = tempdir().unwrap();
        let dest = dir.path().join("trailing_hole.bin");

        // Data only at the beginning, rest is a hole
        let chunks = vec![
            SparseChunk {
                offset: 0,
                length: 1024,
                data: vec![0xFFu8; 1024],
                is_zero: false,
            },
            SparseChunk {
                offset: 1024,
                length: 65536,
                data: vec![0u8; 65536],
                is_zero: true,
            },
        ];

        let stats = write_sparse(&dest, chunks, 66560, SparseMode::Always).unwrap();

        assert_eq!(stats.physical_bytes, 1024);
        assert_eq!(stats.holes_created, 1);

        let data = std::fs::read(&dest).unwrap();
        assert_eq!(data.len(), 66560);
        assert!(data[..1024].iter().all(|&b| b == 0xFF));
        assert!(data[1024..].iter().all(|&b| b == 0));
    }

    #[test]
    fn test_sparse_chunk_from_cdc_chunk() {
        let cdc_chunk = orbit_core_cdc::Chunk {
            offset: 42,
            length: 10,
            hash: [0xAB; 32],
            data: vec![0u8; 10],
            is_zero: true,
        };
        let sparse: SparseChunk = cdc_chunk.into();
        assert_eq!(sparse.offset, 42);
        assert_eq!(sparse.length, 10);
        assert!(sparse.is_zero);
    }
}
