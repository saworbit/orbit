//! Content-Defined Chunking (CDC) Library for Orbit V2
//!
//! This crate provides a fast, data-agnostic chunking engine using the Gear Hash
//! rolling hash algorithm. It solves the "shift problem" by creating chunk boundaries
//! based on content rather than fixed offsets.

mod gear;

use gear::GearHash;
use std::io::{self, Read};
use thiserror::Error;

/// Errors that can occur during chunking
#[derive(Error, Debug)]
pub enum ChunkError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Configuration for the chunking algorithm
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Minimum chunk size in bytes (prevents too many small chunks)
    pub min_size: usize,

    /// Average/target chunk size in bytes (used for cut-point mask)
    pub avg_size: usize,

    /// Maximum chunk size in bytes (prevents unbounded chunks)
    pub max_size: usize,
}

impl ChunkConfig {
    /// Create a new configuration with the given parameters
    pub fn new(min_size: usize, avg_size: usize, max_size: usize) -> Result<Self, ChunkError> {
        if min_size >= avg_size {
            return Err(ChunkError::InvalidConfig(
                "min_size must be less than avg_size".to_string(),
            ));
        }
        if avg_size >= max_size {
            return Err(ChunkError::InvalidConfig(
                "avg_size must be less than max_size".to_string(),
            ));
        }
        if !avg_size.is_power_of_two() {
            return Err(ChunkError::InvalidConfig(
                "avg_size must be a power of 2 for efficient masking".to_string(),
            ));
        }

        Ok(Self {
            min_size,
            avg_size,
            max_size,
        })
    }

    /// Default configuration: 8KB min, 64KB avg, 256KB max
    pub fn default_config() -> Self {
        Self {
            min_size: 8 * 1024,   // 8 KB
            avg_size: 64 * 1024,  // 64 KB
            max_size: 256 * 1024, // 256 KB
        }
    }

    /// Calculate the mask for cut-point detection.
    /// Uses bottom 12 bits of the gear hash, which have good entropy
    /// from carry propagation in the shift-add algorithm.
    /// The actual average chunk size is controlled by the threshold in
    /// find_cut_point, not by this mask alone.
    fn cut_mask(&self) -> u64 {
        // Derive mask from avg_size, capped to ensure sufficient hash entropy.
        // The gear hash's shift-add produces reliable entropy only in lower bits.
        // For avg_size <= 4KB, use avg_size - 1 directly.
        // For larger avg_size, cap at 12 bits (0xFFF) to ensure the threshold
        // approach can find cut points even in repetitive data patterns.
        let bits = (self.avg_size as u64).trailing_zeros().min(12);
        (1u64 << bits) - 1
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// A content-addressed chunk of data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Byte offset in the original stream where this chunk starts
    pub offset: u64,

    /// Length of this chunk in bytes
    pub length: usize,

    /// BLAKE3 hash of the chunk data
    pub hash: [u8; 32],

    /// The actual chunk data
    pub data: Vec<u8>,

    /// Whether this chunk is entirely zero bytes (for sparse file optimization).
    /// Detected during chunking at near-zero cost since we already read every byte.
    pub is_zero: bool,
}

/// Iterator that produces chunks from a Read stream
pub struct ChunkStream<R: Read> {
    reader: R,
    config: ChunkConfig,
    buffer: Vec<u8>,
    buffer_len: usize,
    buffer_pos: usize,
    stream_offset: u64,
    finished: bool,
}

impl<R: Read> ChunkStream<R> {
    /// Create a new chunk stream with the given reader and configuration
    pub fn new(reader: R, config: ChunkConfig) -> Self {
        // Allocate buffer to hold at least 2x max chunk size for efficient scanning
        let buffer_capacity = config.max_size * 2;

        Self {
            reader,
            config,
            buffer: vec![0u8; buffer_capacity],
            buffer_len: 0,
            buffer_pos: 0,
            stream_offset: 0,
            finished: false,
        }
    }

    /// Fill the buffer from the reader
    fn fill_buffer(&mut self) -> io::Result<()> {
        // Move unprocessed data to the start of the buffer
        if self.buffer_pos > 0 && self.buffer_len > self.buffer_pos {
            let remaining = self.buffer_len - self.buffer_pos;
            self.buffer.copy_within(self.buffer_pos..self.buffer_len, 0);
            self.buffer_len = remaining;
            self.buffer_pos = 0;
        } else if self.buffer_pos >= self.buffer_len {
            // Buffer is fully consumed
            self.buffer_len = 0;
            self.buffer_pos = 0;
        }

        // Fill the rest of the buffer
        while self.buffer_len < self.buffer.len() {
            match self.reader.read(&mut self.buffer[self.buffer_len..]) {
                Ok(0) => {
                    // EOF reached
                    self.finished = true;
                    break;
                }
                Ok(n) => {
                    self.buffer_len += n;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    // Retry on interruption
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Find the next chunk boundary using the Gear hash
    fn find_cut_point(&self, start: usize, end: usize) -> Option<usize> {
        let mut hasher = GearHash::new();
        let mask = self.config.cut_mask();

        // Threshold-based cut detection: check if the masked hash value
        // falls below a threshold. This is more robust than exact-zero
        // checks because it tolerates data with short repeating patterns
        // where only a limited set of hash values appear.
        let threshold = 32u64;

        // Scan from start to end looking for a cut point
        for i in start..end {
            let hash = hasher.next(self.buffer[i]);

            // Check if we're past min_size
            let chunk_len = i - self.buffer_pos + 1;

            if chunk_len >= self.config.min_size && (hash & mask) < threshold {
                return Some(i + 1); // Cut after this byte
            }

            // Force cut at max_size
            if chunk_len >= self.config.max_size {
                return Some(i + 1);
            }
        }

        None
    }

    /// Extract the next chunk from the buffer
    fn next_chunk(&mut self) -> Result<Option<Chunk>, ChunkError> {
        // Try to fill buffer if needed (check available data, not total buffer size)
        if self.buffer_len - self.buffer_pos < self.config.max_size && !self.finished {
            self.fill_buffer()?;
        }

        // Check if we're done
        if self.buffer_pos >= self.buffer_len {
            return Ok(None);
        }

        // Calculate the scan range
        let available = self.buffer_len - self.buffer_pos;
        let scan_end = self.buffer_pos + available.min(self.config.max_size);

        // Find cut point
        let cut_point = if available >= self.config.max_size {
            // We have enough data to scan for a cut point
            self.find_cut_point(self.buffer_pos, scan_end)
                .unwrap_or(self.buffer_pos + self.config.max_size)
        } else if self.finished {
            // This is the last chunk - take everything remaining
            self.buffer_len
        } else {
            // Not enough data yet and not finished - refill buffer
            self.fill_buffer()?;
            if self.buffer_len - self.buffer_pos < self.config.min_size && !self.finished {
                // Still not enough, wait for more
                return Ok(None);
            }
            // Use everything we have
            self.buffer_len
        };

        // Extract chunk data
        let chunk_len = cut_point - self.buffer_pos;

        // Safety check
        if chunk_len == 0 && self.finished {
            return Ok(None);
        }

        if chunk_len == 0 {
            // Need more data
            return Ok(None);
        }

        let chunk_data = self.buffer[self.buffer_pos..cut_point].to_vec();
        let chunk_offset = self.stream_offset;

        // Detect all-zero chunks for sparse file optimization.
        // This auto-vectorizes to SIMD on modern CPUs, adding negligible cost
        // since we already read every byte for hashing.
        let is_zero = chunk_data.iter().all(|&b| b == 0);

        // Compute BLAKE3 hash
        let hash = blake3::hash(&chunk_data);

        // Update position
        self.buffer_pos = cut_point;
        self.stream_offset += chunk_len as u64;

        Ok(Some(Chunk {
            offset: chunk_offset,
            length: chunk_len,
            hash: *hash.as_bytes(),
            data: chunk_data,
            is_zero,
        }))
    }
}

impl<R: Read> Iterator for ChunkStream<R> {
    type Item = Result<Chunk, ChunkError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_chunk() {
            Ok(Some(chunk)) => Some(Ok(chunk)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Async file hashing support (requires "async" feature)
#[cfg(feature = "async")]
pub mod async_hash {
    use std::path::Path;
    use tokio::fs::File;
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    /// Calculates the BLAKE3 hash of a file range.
    ///
    /// This is an async function that reads a specific range of bytes from a file
    /// and returns its BLAKE3 hash.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `offset` - Byte offset to start reading from
    /// * `length` - Number of bytes to read
    ///
    /// # Returns
    ///
    /// The 32-byte BLAKE3 hash of the file range
    pub async fn hash_file_range(
        path: &Path,
        offset: u64,
        length: u64,
    ) -> std::io::Result<[u8; 32]> {
        let mut file = File::open(path).await?;
        file.seek(std::io::SeekFrom::Start(offset)).await?;

        let mut hasher = blake3::Hasher::new();
        let mut remaining = length;
        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer

        while remaining > 0 {
            let to_read = remaining.min(buffer.len() as u64) as usize;
            let n = file.read(&mut buffer[..to_read]).await?;

            if n == 0 {
                break; // EOF
            }

            hasher.update(&buffer[..n]);
            remaining -= n as u64;
        }

        Ok(*hasher.finalize().as_bytes())
    }
}

#[cfg(feature = "async")]
pub use async_hash::hash_file_range;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_config_validation() {
        // Valid config
        assert!(ChunkConfig::new(8192, 65536, 262144).is_ok());

        // min >= avg
        assert!(ChunkConfig::new(65536, 65536, 262144).is_err());

        // avg >= max
        assert!(ChunkConfig::new(8192, 262144, 262144).is_err());

        // avg not power of 2
        assert!(ChunkConfig::new(8192, 60000, 262144).is_err());
    }

    #[test]
    fn test_basic_chunking() {
        let data = vec![0u8; 1024 * 1024]; // 1MB of zeros
        let config = ChunkConfig::default_config();
        let stream = ChunkStream::new(Cursor::new(data), config);

        let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

        assert!(!chunks.is_empty(), "Should produce at least one chunk");

        // Verify offsets are sequential
        let mut expected_offset = 0u64;
        for chunk in &chunks {
            assert_eq!(chunk.offset, expected_offset);
            expected_offset += chunk.length as u64;
        }
    }

    #[test]
    fn test_empty_input() {
        let data: Vec<u8> = vec![];
        let config = ChunkConfig::default_config();
        let stream = ChunkStream::new(Cursor::new(data), config);

        let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_small_input() {
        let data = vec![42u8; 1024]; // 1KB
        let config = ChunkConfig::default_config();
        let stream = ChunkStream::new(Cursor::new(data.clone()), config);

        let chunks: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

        // Should produce exactly one chunk (below min_size boundary behavior)
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data, data);
    }
}
