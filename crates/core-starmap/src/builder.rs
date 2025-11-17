//! Star Map builder for constructing binary indices

use crate::error::{Error, Result};
use crate::{BloomFilter, ChunkMeta, RankSelectBitmap, StarMapData, WindowMeta};
use crate::{STARMAP_MAGIC, STARMAP_VERSION};

/// Builder for constructing Star Map binary indices
///
/// # Example
/// ```
/// use orbit_core_starmap::StarMapBuilder;
///
/// let mut builder = StarMapBuilder::new(1024000);
/// builder.add_chunk(0, 4096, &[1u8; 32]);
/// builder.add_chunk(4096, 4096, &[2u8; 32]);
/// builder.add_window(0, 0, 2, &[0u8; 32], 0);
///
/// let data = builder.build().unwrap();
/// std::fs::write("file.starmap.bin", &data).unwrap();
/// ```
#[derive(Debug)]
pub struct StarMapBuilder {
    /// File size in bytes
    file_size: u64,
    /// Collected chunks
    chunks: Vec<ChunkMeta>,
    /// Collected windows
    windows: Vec<WindowMeta>,
    /// Expected number of chunks (for bloom filter sizing)
    expected_chunks: u32,
}

impl StarMapBuilder {
    /// Create a new Star Map builder
    ///
    /// # Arguments
    /// * `file_size` - Total size of the file being mapped
    pub fn new(file_size: u64) -> Self {
        // Estimate chunks based on average chunk size of 256 KiB
        let expected_chunks = ((file_size / (256 * 1024)) + 1) as u32;

        Self {
            file_size,
            chunks: Vec::new(),
            windows: Vec::new(),
            expected_chunks,
        }
    }

    /// Create a new Star Map builder with expected chunk count
    ///
    /// Use this if you know the approximate number of chunks in advance
    /// for better bloom filter sizing.
    pub fn with_expected_chunks(file_size: u64, expected_chunks: u32) -> Self {
        Self {
            file_size,
            chunks: Vec::new(),
            windows: Vec::new(),
            expected_chunks,
        }
    }

    /// Add a chunk to the Star Map
    ///
    /// # Arguments
    /// * `offset` - Byte offset in the source file
    /// * `length` - Length of the chunk in bytes
    /// * `content_id` - BLAKE3 hash of the chunk (32 bytes)
    pub fn add_chunk(&mut self, offset: u64, length: u32, content_id: &[u8; 32]) -> Result<()> {
        self.chunks.push(ChunkMeta {
            offset,
            length,
            content_id: *content_id,
        });
        Ok(())
    }

    /// Add a window to the Star Map
    ///
    /// # Arguments
    /// * `id` - Window identifier (must be sequential starting from 0)
    /// * `first_chunk` - Index of the first chunk in this window
    /// * `count` - Number of chunks in this window
    /// * `merkle_root` - Merkle tree root for this window (32 bytes)
    /// * `overlap` - Number of chunks overlapping with previous window
    pub fn add_window(
        &mut self,
        id: u32,
        first_chunk: u32,
        count: u16,
        merkle_root: &[u8; 32],
        overlap: u16,
    ) -> Result<()> {
        // Validate window ID is sequential
        if id as usize != self.windows.len() {
            return Err(Error::invalid_window(format!(
                "Window IDs must be sequential: expected {}, got {}",
                self.windows.len(),
                id
            )));
        }

        // Validate chunk references
        if first_chunk as usize >= self.chunks.len() && !self.chunks.is_empty() {
            return Err(Error::invalid_window(format!(
                "Window references non-existent chunk: first_chunk={}, total_chunks={}",
                first_chunk,
                self.chunks.len()
            )));
        }

        self.windows.push(WindowMeta {
            id,
            first_chunk,
            count,
            merkle_root: *merkle_root,
            overlap,
        });
        Ok(())
    }

    /// Get the number of chunks added so far
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Get the number of windows added so far
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Build the Star Map and serialize to bytes
    pub fn build(self) -> Result<Vec<u8>> {
        // Validate we have data
        if self.chunks.is_empty() {
            return Err(Error::Empty);
        }
        if self.windows.is_empty() {
            return Err(Error::Empty);
        }

        // Build bloom filter from all content IDs
        let bloom = self.build_bloom_filter();

        // Build bitmaps for each window (initially all zeros - nothing transferred yet)
        let bitmaps = self.build_bitmaps();

        // Create StarMapData structure
        let starmap_data = StarMapData {
            version: STARMAP_VERSION,
            file_size: self.file_size,
            chunk_count: self.chunks.len() as u32,
            window_count: self.windows.len() as u32,
            chunks: self.chunks,
            windows: self.windows,
            bloom_data: bloom.to_bytes(),
            bloom_hashes: bloom.num_hashes(),
            bloom_elements: bloom.num_elements(),
            bloom_bits: bloom.num_bits(),
            bitmaps: bitmaps.iter().map(|b| b.to_bytes()).collect(),
        };

        // Serialize to bytes with magic header
        let mut buffer = Vec::new();

        // Write magic number
        buffer.extend_from_slice(STARMAP_MAGIC);

        // Serialize the data structure
        let serialized = bincode::serialize(&starmap_data)
            .map_err(|e| Error::Other(format!("Serialization failed: {}", e)))?;

        buffer.extend_from_slice(&serialized);

        Ok(buffer)
    }

    /// Build bloom filter from all chunk content IDs
    fn build_bloom_filter(&self) -> BloomFilter {
        // Use 1% false positive rate
        let mut bloom = BloomFilter::new(self.expected_chunks.max(self.chunks.len() as u32), 0.01);

        for chunk in &self.chunks {
            bloom.insert(&chunk.content_id);
        }

        bloom
    }

    /// Build empty bitmaps for each window
    ///
    /// These track which chunks have been transferred. Initially all zeros.
    fn build_bitmaps(&self) -> Vec<RankSelectBitmap> {
        self.windows
            .iter()
            .map(|window| RankSelectBitmap::new(window.count as usize))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut builder = StarMapBuilder::new(8192);

        assert_eq!(builder.chunk_count(), 0);
        assert_eq!(builder.window_count(), 0);

        builder.add_chunk(0, 4096, &[1u8; 32]).unwrap();
        builder.add_chunk(4096, 4096, &[2u8; 32]).unwrap();

        assert_eq!(builder.chunk_count(), 2);
    }

    #[test]
    fn test_add_window() {
        let mut builder = StarMapBuilder::new(8192);

        builder.add_chunk(0, 4096, &[1u8; 32]).unwrap();
        builder.add_chunk(4096, 4096, &[2u8; 32]).unwrap();

        builder.add_window(0, 0, 2, &[0u8; 32], 0).unwrap();

        assert_eq!(builder.window_count(), 1);
    }

    #[test]
    fn test_window_sequential_ids() {
        let mut builder = StarMapBuilder::new(8192);

        builder.add_chunk(0, 4096, &[1u8; 32]).unwrap();

        // First window must have ID 0
        let result = builder.add_window(1, 0, 1, &[0u8; 32], 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("sequential"));

        // Add window 0 first
        builder.add_window(0, 0, 1, &[0u8; 32], 0).unwrap();

        // Now window 1 should work
        builder.add_window(1, 0, 1, &[0u8; 32], 0).unwrap();
        assert_eq!(builder.window_count(), 2);
    }

    #[test]
    fn test_build_empty() {
        let builder = StarMapBuilder::new(8192);

        let result = builder.build();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Empty));
    }

    #[test]
    fn test_build_success() {
        let mut builder = StarMapBuilder::new(8192);

        builder.add_chunk(0, 4096, &[1u8; 32]).unwrap();
        builder.add_chunk(4096, 4096, &[2u8; 32]).unwrap();
        builder.add_window(0, 0, 2, &[0u8; 32], 0).unwrap();

        let data = builder.build().unwrap();
        assert!(!data.is_empty());

        // Check magic number
        assert_eq!(&data[0..8], STARMAP_MAGIC);
    }

    #[test]
    fn test_with_expected_chunks() {
        let builder = StarMapBuilder::with_expected_chunks(1_000_000, 1000);
        assert_eq!(builder.expected_chunks, 1000);
    }

    #[test]
    fn test_multiple_windows() {
        let mut builder = StarMapBuilder::new(16384);

        // Add 4 chunks
        for i in 0..4 {
            let mut cid = [0u8; 32];
            cid[0] = i as u8;
            builder.add_chunk(i as u64 * 4096, 4096, &cid).unwrap();
        }

        // Add 2 windows
        builder.add_window(0, 0, 2, &[1u8; 32], 0).unwrap();
        builder.add_window(1, 2, 2, &[2u8; 32], 0).unwrap();

        let data = builder.build().unwrap();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_serialization_format() {
        let mut builder = StarMapBuilder::new(4096);

        builder.add_chunk(0, 4096, &[42u8; 32]).unwrap();
        builder.add_window(0, 0, 1, &[99u8; 32], 0).unwrap();

        let data = builder.build().unwrap();

        // Should start with magic number
        assert_eq!(&data[0..8], STARMAP_MAGIC);
        // Should have reasonable size
        assert!(data.len() > 100);
    }
}
