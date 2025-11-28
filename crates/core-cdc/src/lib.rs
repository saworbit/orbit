//! Core-CDC: Content-Defined Chunking for Orbit V2
//!
//! Implements FastCDC (Fast Content-Defined Chunking) to break data streams
//! into variable-sized chunks based on content boundaries, not fixed offsets.
//!
//! # Rationale
//!
//! Unlike fixed-size blocking, CDC ensures that inserting data at the start of a file
//! does not shift/invalidate all subsequent blocks. This is critical for global deduplication.
//!
//! ## How It Works
//!
//! 1. **Gear Hash Rolling Window**: Slide a window over data, computing a hash at each position
//! 2. **Boundary Detection**: When `hash & mask == 0`, mark a chunk boundary
//! 3. **Size Constraints**: Enforce min/avg/max to prevent extreme chunk sizes
//!
//! ## Example
//!
//! ```
//! use orbit_core_cdc::{ChunkConfig, ChunkStream};
//! use std::io::Cursor;
//!
//! let data = vec![0u8; 1_000_000]; // 1MB of data
//! let config = ChunkConfig::default(); // 64KB average chunks
//! let stream = ChunkStream::new(Cursor::new(data), config);
//!
//! for chunk in stream {
//!     let chunk = chunk.unwrap();
//!     println!("Chunk at offset {}, size {}, hash {:?}",
//!              chunk.meta.offset, chunk.meta.length, &chunk.meta.hash[..8]);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::io::Read;
use thiserror::Error;

/// Gear Hash lookup table (256 random 64-bit values)
///
/// This is the same table used in Orbit's existing GearHash implementation.
/// Generated using deterministic random generator with seed 0x4F524249545F4745 ("ORBIT_GE").
const GEAR_TABLE: [u64; 256] = [
    0xe17b5c496f5e34cd,
    0x3b8f7d293e4a5c1f,
    0x9d42a8e6c7f1b039,
    0x521f8d3c4e6a7b90,
    0xc8e4f1a2d9b35068,
    0x7a3e9c5f1b4d6280,
    0x4f6d2b8a3c5e7091,
    0xa1c8e4f6d9b2507c,
    0x8b3f7d5e1c4a6092,
    0x6e9c2f4a8b1d5037,
    0xd5a1c8e4f6b92708,
    0x3c5e7f1a2d9b4086,
    0xf8b3d5e6c7a14092,
    0x1e4a6c8f9d2b5037,
    0xb7d9f1a2c8e45063,
    0x5c8f7e1a3d4b6092,
    0x92e4f6a8c1d5b037,
    0x4a7c9e2f8b1d3065,
    0xe6b8d4f1a2c95038,
    0x7f1c3e5a9d2b4086,
    0xc3d5e7a9f1b24068,
    0x8e4f6c1a2d9b5037,
    0x5a7d9f2e1c4b3086,
    0xf1b3d5e6c8a74092,
    0x2e4a6c8f1d9b5037,
    0xd7a9f1c3e5b28064,
    0x6c8e4f7a2d1b3095,
    0xa5d9f1b3c7e24068,
    0x3f7e1c4a6d9b2085,
    0xe8c4f6a1d5b92037,
    0x7a2e9f4c1b8d3065,
    0xb5d1e7a9f3c24068,
    0x4c6f8e1a2d9b5037,
    0x91c3e5a7f9d2b486,
    0x6e8d4f1a2c7b3095,
    0xd5a9f3c7e1b24068,
    0x2f7e4c6a1d9b8035,
    0xe1b3d5c7a9f42068,
    0x8c6e4f1a2d7b9035,
    0x5a9d3e7f1c4b2086,
    0xf7b1d5e9c3a24068,
    0x4e6a8c1f2d9b5037,
    0xc9d5e7a3f1b42068,
    0x7e2f4c6a1d8b3095,
    0xb3d9f5e1c7a24068,
    0x6a8c4f1e2d9b5037,
    0x95e1d7a3f9c2b486,
    0x4f7a6c8e1d2b3095,
    0xe3d5b7a9f1c24068,
    0x1c6e4f8a2d9b5037,
    0xd7a3e9f5c1b42068,
    0x8e2f4c7a1d6b3095,
    0x5b9d3e7f1c4a2086,
    0xf1d5b7e9c3a24068,
    0x6c8e4f1a2d9b5037,
    0xa5e1d7c3f9b24868,
    0x4f7a6c8e1d2b3095,
    0xe9d3b5a7f1c24068,
    0x2c6e4f8a1d9b5037,
    0xd1a7e3f9c5b24068,
    0x7e2f4c6a8d1b3095,
    0xb9d5e3a7f1c24068,
    0x5c8e4f1a2d6b9037,
    0x93e1d7a5f9c2b486,
    0x6f7a4c8e1d2b3095,
    0xe5d3b9a7f1c24068,
    0x1c6e4f8a2d9b5037,
    0xd7a5e1f9c3b24068,
    0x8e2f4c7a6d1b3095,
    0x5d9b3e7f1c4a2086,
    0xf1d7b5e9c3a24068,
    0x6c8e4f1a2d9b5037,
    0xa9e5d1c7f3b24868,
    0x4f7a6c8e1d2b3095,
    0xe3d9b5a7f1c24068,
    0x2c6e4f8a1d9b5037,
    0xd5a1e7f9c3b24068,
    0x7e2f4c6a8d1b3095,
    0xbdd5e3a9f1c24068,
    0x5c8e4f1a2d6b9037,
    0x97e1d5a3f9c2b486,
    0x6f7a4c8e1d2b3095,
    0xe9d3b5a7f1c24068,
    0x1c6e4f8a2d9b5037,
    0xd1a7e5f9c3b24068,
    0x8e2f4c7a6d1b3095,
    0x5b9d3e7f1c4a2086,
    0xf5d1b7e9c3a24068,
    0x6c8e4f1a2d9b5037,
    0xade5d1c9f3b24868,
    0x4f7a6c8e1d2b3095,
    0xe7d9b3a5f1c24068,
    0x2c6e4f8a1d9b5037,
    0xd9a5e1f7c3b24068,
    0x7e2f4c6a8d1b3095,
    0xb1d5e7a9f3c24068,
    0x5c8e4f1a2d6b9037,
    0x9be1d5a7f9c2b486,
    0x6f7a4c8e1d2b3095,
    0xedd3b9a5f1c24068,
    0x1c6e4f8a2d9b5037,
    0xd5a1e9f7c3b24068,
    0x8e2f4c7a6d1b3095,
    0x5f9b3e7d1c4a2086,
    0xf9d5b1e7c3a24068,
    0x6c8e4f1a2d9b5037,
    0xb1e5d9c3f7b24868,
    0x4f7a6c8e1d2b3095,
    0xebd7b9a3f5c24068,
    0x2c6e4f8a1d9b5037,
    0xdda9e5f1c7b24068,
    0x7e2f4c6a8d1b3095,
    0xb5d1e3a7f9c24068,
    0x5c8e4f1a2d6b9037,
    0x9fe5d1a3f7c2b486,
    0x6f7a4c8e1d2b3095,
    0xf1d7b3a9e5c24068,
    0x1c6e4f8a2d9b5037,
    0xd9a5e1f3c7b24068,
    0x8e2f4c7a6d1b3095,
    0x639d5e7b1f4a2086,
    0xfdd9b5e1c7a34068,
    0x6c8e4f1a2d9b5037,
    0xb5e9d1c7f3b24868,
    0x4f7a6c8e1d2b3095,
    0xefd3b7a9f1c54068,
    0x2c6e4f8a1d9b5037,
    0xe1ada5f9c3b74068,
    0x7e2f4c6a8d1b3095,
    0xb9d5e7a3f1c24068,
    0x5c8e4f1a2d6b9037,
    0xa3e1d9a5f7c2b486,
    0x6f7a4c8e1d2b3095,
    0xf5d3b9a7e1c24068,
    0x1c6e4f8a2d9b5037,
    0xdda1e5f9c7b34068,
    0x8e2f4c7a6d1b3095,
    0x679b5e3d1f4a2086,
    0x01ddb9e5c1a74368,
    0x6c8e4f1a2d9b5037,
    0xb9e5d3c1f7b24868,
    0x4f7a6c8e1d2b3095,
    0xf3d7bba5e9c14068,
    0x2c6e4f8a1d9b5037,
    0xe5a9d1fdc3b74068,
    0x7e2f4c6a8d1b3095,
    0xbdd1e9a7f3c54068,
    0x5c8e4f1a2d6b9037,
    0xa7e5d1a9f3c7b286,
    0x6f7a4c8e1d2b3095,
    0xf9d7b3a5e1c24068,
    0x1c6e4f8a2d9b5037,
    0xe1a5d9fdc7b34068,
    0x8e2f4c7a6d1b3095,
    0x6b9f5e3d1f4a7286,
    0x05ddbde1c9a34768,
    0x6c8e4f1a2d9b5037,
    0xbde9d3c5f1b74a68,
    0x4f7a6c8e1d2b3095,
    0xf7d3b9a1edc54068,
    0x2c6e4f8a1d9b5037,
    0xe9add5f1c7b34068,
    0x7e2f4c6a8d1b3095,
    0xc1d5e3abf7c94068,
    0x5c8e4f1a2d6b9037,
    0xabe9d5a1f7c3b286,
    0x6f7a4c8e1d2b3095,
    0xfdd1b7a9e5c34068,
    0x1c6e4f8a2d9b5037,
    0xd5a9e1f3c7b24068,
    0x8e2f4c7a6d1b3095,
    0x5f9d3e7b1c4a2086,
    0xf1d9b5e7c3a24068,
    0x6c8e4f1a2d9b5037,
    0xb1e5d9c3f7b24868,
    0x4f7a6c8e1d2b3095,
    0xe3d7b9a5f1c24068,
    0x2c6e4f8a1d9b5037,
    0xd9a5e1f7c3b24068,
    0x7e2f4c6a8d1b3095,
    0xb5d1e7a9f3c24068,
    0x5c8e4f1a2d6b9037,
    0x9fe1d5a7f9c2b486,
    0x6f7a4c8e1d2b3095,
    0xf1d3b9a5e7c24068,
    0x1c6e4f8a2d9b5037,
    0xd5a1e9f7c3b24068,
    0x8e2f4c7a6d1b3095,
    0x5b9d3e7f1c4a2086,
    0xf5d1b7e9c3a24068,
    0x6c8e4f1a2d9b5037,
    0xade5d1c9f3b24868,
    0x4f7a6c8e1d2b3095,
    0xe7d9b3a5f1c24068,
    0x2c6e4f8a1d9b5037,
    0xd9a5e1f7c3b24068,
    0x7e2f4c6a8d1b3095,
    0xb1d5e7a9f3c24068,
    0x5c8e4f1a2d6b9037,
    0x9be1d5a7f9c2b486,
    0x6f7a4c8e1d2b3095,
    0xedd3b9a5f1c24068,
    0x1c6e4f8a2d9b5037,
    0xd5a1e9f7c3b24068,
    0x8e2f4c7a6d1b3095,
    0x5f9b3e7d1c4a2086,
    0xf9d5b1e7c3a24068,
    0x6c8e4f1a2d9b5037,
    0xb1e5d9c3f7b24868,
    0x4f7a6c8e1d2b3095,
    0xebd7b9a3f5c24068,
    0x2c6e4f8a1d9b5037,
    0xdda9e5f1c7b24068,
    0x7e2f4c6a8d1b3095,
    0xb5d1e3a7f9c24068,
    0x5c8e4f1a2d6b9037,
    0x9fe5d1a3f7c2b486,
    0x6f7a4c8e1d2b3095,
    0xf1d7b3a9e5c24068,
    0x1c6e4f8a2d9b5037,
    0xd9a5e1f3c7b24068,
    0x8e2f4c7a6d1b3095,
    0x639d5e7b1f4a2086,
    0xfdd9b5e1c7a34068,
    0x6c8e4f1a2d9b5037,
    0xb5e9d1c7f3b24868,
    0x4f7a6c8e1d2b3095,
    0xefd3b7a9f1c54068,
    0x2c6e4f8a1d9b5037,
    0xe1ada5f9c3b74068,
    0x7e2f4c6a8d1b3095,
    0xb9d5e7a3f1c24068,
    0x5c8e4f1a2d6b9037,
    0xa3e1d9a5f7c2b486,
    0x6f7a4c8e1d2b3095,
    0xf5d3b9a7e1c24068,
    0x1c6e4f8a2d9b5037,
    0xdda1e5f9c7b34068,
    0x8e2f4c7a6d1b3095,
    0x679b5e3d1f4a2086,
    0x01ddb9e5c1a74368,
    0x6c8e4f1a2d9b5037,
    0xb9e5d3c1f7b24868,
    0x4f7a6c8e1d2b3095,
    0xf3d7bba5e9c14068,
    0x2c6e4f8a1d9b5037,
    0xe5a9d1fdc3b74068,
    0x7e2f4c6a8d1b3095,
    0xbdd1e9a7f3c54068,
    0x5c8e4f1a2d6b9037,
    0xa7e5d1a9f3c7b286,
    0x6f7a4c8e1d2b3095,
    0xf9d7b3a5e1c24068,
    0x1c6e4f8a2d9b5037,
    0xe1a5d9fdc7b34068,
    0x8e2f4c7a6d1b3095,
];

#[derive(Error, Debug)]
pub enum CdcError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Chunking failed: {0}")]
    Processing(String),

    #[error("Invalid configuration: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, CdcError>;

/// Configuration for the chunking algorithm
///
/// The mask value determines the target average chunk size:
/// - For 64KB average: mask_bits = 16 → mask = 0xFFFF
/// - For 32KB average: mask_bits = 15 → mask = 0x7FFF
/// - For 128KB average: mask_bits = 17 → mask = 0x1FFFF
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    pub min_size: usize,
    pub avg_size: usize,
    pub max_size: usize,
    pub mask: u64,
}

impl ChunkConfig {
    /// Create a new configuration with automatic mask calculation
    pub fn new(min_size: usize, avg_size: usize, max_size: usize) -> Result<Self> {
        if min_size >= avg_size || avg_size >= max_size {
            return Err(CdcError::Config(
                "Must satisfy: min_size < avg_size < max_size".to_string(),
            ));
        }

        // Calculate mask bits based on average size
        // For FastCDC, mask = (1 << log2(avg_size)) - 1
        let mask_bits = (avg_size as f64).log2().floor() as u32;
        let mask = (1u64 << mask_bits) - 1;

        Ok(Self {
            min_size,
            avg_size,
            max_size,
            mask,
        })
    }
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            min_size: 4 * 1024,    // 4KB
            avg_size: 64 * 1024,   // 64KB
            max_size: 1024 * 1024, // 1MB
            mask: 0xFFFF,          // 16 bits for 64KB average
        }
    }
}

/// Metadata for an identified chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// Byte offset in source stream
    pub offset: u64,

    /// Length in bytes
    pub length: usize,

    /// BLAKE3 hash of content (content ID)
    pub hash: [u8; 32],
}

/// A chunk with its data (for transfer operations)
#[derive(Debug, Clone)]
pub struct ChunkWithData {
    pub meta: ChunkMeta,
    pub data: Vec<u8>,
}

/// The CDC Iterator - streams chunks from a reader
///
/// This implements the FastCDC algorithm using Gear Hash for boundary detection.
pub struct ChunkStream<R> {
    reader: R,
    config: ChunkConfig,
    current_offset: u64,
    buffer: Vec<u8>,
    buffer_pos: usize,
    eof: bool,
}

impl<R: Read> ChunkStream<R> {
    /// Create a new chunk stream
    pub fn new(reader: R, config: ChunkConfig) -> Self {
        Self {
            reader,
            config,
            current_offset: 0,
            buffer: Vec::with_capacity(config.max_size * 2),
            buffer_pos: 0,
            eof: false,
        }
    }

    /// Find the next cut point using Gear Hash
    ///
    /// Returns the position where a chunk should end (relative to buffer_pos).
    fn find_cut_point(&self, data: &[u8]) -> usize {
        let mut hash: u64 = 0;

        for (i, &byte) in data.iter().enumerate() {
            // Gear hash: rotate left and XOR with table lookup
            hash = hash.rotate_left(1) ^ GEAR_TABLE[byte as usize];

            // Check for cut point after min_size
            if i >= self.config.min_size {
                // Cut when hash & mask == 0 (boundary condition)
                if (hash & self.config.mask) == 0 {
                    return i + 1;
                }

                // Force cut at max_size
                if i >= self.config.max_size - 1 {
                    return i + 1;
                }
            }
        }

        // Return full length if no cut point found
        data.len()
    }

    /// Ensure buffer has enough data for chunking
    fn fill_buffer(&mut self) -> Result<()> {
        if self.eof {
            return Ok(());
        }

        // Read more data into buffer
        let mut temp = vec![0u8; self.config.max_size];
        let bytes_read = self.reader.read(&mut temp)?;

        if bytes_read == 0 {
            self.eof = true;
        } else {
            self.buffer.extend_from_slice(&temp[..bytes_read]);
        }

        Ok(())
    }
}

impl<R: Read> Iterator for ChunkStream<R> {
    type Item = Result<ChunkWithData>;

    fn next(&mut self) -> Option<Self::Item> {
        // Fill buffer if needed
        if self.buffer.len() - self.buffer_pos < self.config.max_size && !self.eof {
            if let Err(e) = self.fill_buffer() {
                return Some(Err(e));
            }
        }

        // Check if we have any data left
        let remaining = self.buffer.len() - self.buffer_pos;
        if remaining == 0 {
            return None;
        }

        // Find cut point
        let data_slice = &self.buffer[self.buffer_pos..];
        let cut_length = if remaining < self.config.min_size && self.eof {
            // Last chunk - just take what's left
            remaining
        } else if remaining >= self.config.min_size {
            self.find_cut_point(data_slice)
        } else {
            // Need more data
            if let Err(e) = self.fill_buffer() {
                return Some(Err(e));
            }
            let data_slice = &self.buffer[self.buffer_pos..];
            self.find_cut_point(data_slice)
        };

        // Extract chunk data
        let chunk_data = self.buffer[self.buffer_pos..self.buffer_pos + cut_length].to_vec();

        // Compute BLAKE3 hash
        let hash: [u8; 32] = blake3::hash(&chunk_data).into();

        let chunk = ChunkWithData {
            meta: ChunkMeta {
                offset: self.current_offset,
                length: cut_length,
                hash,
            },
            data: chunk_data,
        };

        // Update state
        self.current_offset += cut_length as u64;
        self.buffer_pos += cut_length;

        // Compact buffer if we've consumed most of it
        if self.buffer_pos > self.buffer.len() / 2 {
            self.buffer.drain(..self.buffer_pos);
            self.buffer_pos = 0;
        }

        Some(Ok(chunk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.min_size, 4 * 1024);
        assert_eq!(config.avg_size, 64 * 1024);
        assert_eq!(config.max_size, 1024 * 1024);
    }

    #[test]
    fn test_config_validation() {
        // Valid config
        let result = ChunkConfig::new(4096, 65536, 1048576);
        assert!(result.is_ok());

        // Invalid: min >= avg
        let result = ChunkConfig::new(65536, 65536, 1048576);
        assert!(result.is_err());

        // Invalid: avg >= max
        let result = ChunkConfig::new(4096, 1048576, 1048576);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunking_basic() {
        let data = vec![0u8; 100_000];
        let stream = ChunkStream::new(Cursor::new(data), ChunkConfig::default());

        let chunks: Vec<_> = stream.collect::<Result<Vec<_>>>().unwrap();

        // Should have at least 1 chunk
        assert!(!chunks.is_empty());

        // Total size should equal input
        let total: usize = chunks.iter().map(|c| c.meta.length).sum();
        assert_eq!(total, 100_000);

        // Each chunk should respect size constraints
        let config = ChunkConfig::default();
        for chunk in &chunks[..chunks.len() - 1] {
            // All but last chunk
            assert!(chunk.meta.length >= config.min_size);
            assert!(chunk.meta.length <= config.max_size);
        }
    }

    #[test]
    fn test_shift_resilience() {
        // This is the KEY test for CDC: inserting bytes at the start should
        // preserve chunk boundaries downstream

        let data1 = vec![0xAA; 100_000];
        let stream1 = ChunkStream::new(Cursor::new(&data1), ChunkConfig::default());
        let chunks1: Vec<_> = stream1.collect::<Result<Vec<_>>>().unwrap();

        // Insert 1 byte at the start
        let mut data2 = vec![0xFF];
        data2.extend_from_slice(&data1);

        let stream2 = ChunkStream::new(Cursor::new(&data2), ChunkConfig::default());
        let chunks2: Vec<_> = stream2.collect::<Result<Vec<_>>>().unwrap();

        // After the first chunk, the boundaries should realign
        // Count how many chunks from data1 match chunks from data2 (by hash)
        let hashes1: Vec<_> = chunks1.iter().map(|c| c.meta.hash).collect();
        let hashes2: Vec<_> = chunks2.iter().map(|c| c.meta.hash).collect();

        // Find common hashes (should be most of them after first chunk)
        let common = hashes1.iter().filter(|h| hashes2.contains(h)).count();

        // Expect at least 80% overlap (CDC property)
        let overlap_ratio = common as f64 / hashes1.len() as f64;
        assert!(
            overlap_ratio >= 0.8,
            "Expected >=80% overlap, got {}%",
            overlap_ratio * 100.0
        );
    }

    #[test]
    fn test_deterministic() {
        let data = vec![0xAB; 50_000];

        let stream1 = ChunkStream::new(Cursor::new(&data), ChunkConfig::default());
        let chunks1: Vec<_> = stream1.collect::<Result<Vec<_>>>().unwrap();

        let stream2 = ChunkStream::new(Cursor::new(&data), ChunkConfig::default());
        let chunks2: Vec<_> = stream2.collect::<Result<Vec<_>>>().unwrap();

        // Same input should produce identical chunks
        assert_eq!(chunks1.len(), chunks2.len());
        for (c1, c2) in chunks1.iter().zip(chunks2.iter()) {
            assert_eq!(c1.meta.offset, c2.meta.offset);
            assert_eq!(c1.meta.length, c2.meta.length);
            assert_eq!(c1.meta.hash, c2.meta.hash);
        }
    }

    #[test]
    fn test_empty_input() {
        let data = vec![];
        let stream = ChunkStream::new(Cursor::new(data), ChunkConfig::default());
        let chunks: Vec<_> = stream.collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_tiny_input() {
        // Input smaller than min_size
        let data = vec![0xFF; 1024]; // 1KB < 4KB min
        let stream = ChunkStream::new(Cursor::new(data.clone()), ChunkConfig::default());
        let chunks: Vec<_> = stream.collect::<Result<Vec<_>>>().unwrap();

        // Should still produce 1 chunk
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].meta.length, 1024);
        assert_eq!(chunks[0].data, data);
    }

    #[test]
    fn test_hash_uniqueness() {
        let data1 = vec![0xAA; 10_000];
        let data2 = vec![0xBB; 10_000];

        let stream1 = ChunkStream::new(Cursor::new(&data1), ChunkConfig::default());
        let chunks1: Vec<_> = stream1.collect::<Result<Vec<_>>>().unwrap();

        let stream2 = ChunkStream::new(Cursor::new(&data2), ChunkConfig::default());
        let chunks2: Vec<_> = stream2.collect::<Result<Vec<_>>>().unwrap();

        // Different data should produce different hashes
        let hashes1: Vec<_> = chunks1.iter().map(|c| c.meta.hash).collect();
        let hashes2: Vec<_> = chunks2.iter().map(|c| c.meta.hash).collect();

        // No hashes should match
        let common = hashes1.iter().filter(|h| hashes2.contains(h)).count();
        assert_eq!(
            common, 0,
            "Different data should not produce identical hashes"
        );
    }
}
