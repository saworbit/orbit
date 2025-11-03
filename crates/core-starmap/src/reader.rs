//! Star Map reader for querying binary indices

use crate::error::{Error, Result};
use crate::{BloomFilter, ChunkMeta, RankSelectBitmap, WindowMeta, StarMapData};
use crate::{STARMAP_MAGIC, STARMAP_VERSION};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Reader for querying Star Map binary indices via memory mapping
///
/// # Example
/// ```no_run
/// use orbit_core_starmap::StarMapReader;
///
/// let reader = StarMapReader::open("file.starmap.bin").unwrap();
/// 
/// // Check if a chunk exists
/// let content_id = [1u8; 32];
/// if reader.has_chunk(&content_id).unwrap() {
///     println!("Chunk exists!");
/// }
///
/// // Get chunk metadata
/// let chunk = reader.get_chunk(0).unwrap();
/// println!("Chunk offset: {}, length: {}", chunk.offset, chunk.length);
///
/// // Find missing chunks in a window
/// let missing = reader.next_missing(0).unwrap();
/// println!("Missing chunks: {:?}", missing);
/// ```
#[derive(Debug)]
pub struct StarMapReader {
    /// Memory-mapped file
    _mmap: Arc<Mmap>,
    /// Deserialized Star Map data
    data: StarMapData,
    /// Cached bloom filter
    bloom: BloomFilter,
    /// Cached bitmaps (one per window)
    bitmaps: Vec<RankSelectBitmap>,
}

impl StarMapReader {
    /// Open and memory-map a Star Map file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(Error::not_found(path));
        }

        // Open and memory-map the file
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let mmap = Arc::new(mmap);

        // Verify magic number
        if mmap.len() < 8 {
            return Err(Error::invalid_format("File too small"));
        }

        if &mmap[0..8] != STARMAP_MAGIC {
            return Err(Error::InvalidMagic {
                expected: STARMAP_MAGIC.to_vec(),
                found: mmap[0..8].to_vec(),
            });
        }

        // Deserialize from bincode (skip magic header)
        let data: StarMapData = bincode::deserialize(&mmap[8..])
            .map_err(|e| Error::invalid_format(format!("Deserialization failed: {}", e)))?;

        // Validate version
        if data.version != STARMAP_VERSION {
            return Err(Error::version_mismatch(STARMAP_VERSION, data.version));
        }

        // Reconstruct bloom filter from data
        let bloom = BloomFilter::from_bytes(
            &data.bloom_data,
            data.bloom_hashes,
            data.bloom_elements,
            data.bloom_bits,
        )?;

        // Reconstruct bitmaps
        let mut bitmaps = Vec::new();
        for (i, bitmap_data) in data.bitmaps.iter().enumerate() {
            if i >= data.windows.len() {
                return Err(Error::corrupt_data("More bitmaps than windows"));
            }
            let window_size = data.windows[i].count as usize;
            let bitmap = RankSelectBitmap::from_bytes(bitmap_data, window_size)?;
            bitmaps.push(bitmap);
        }

        Ok(Self {
            _mmap: mmap,
            data,
            bloom,
            bitmaps,
        })
    }

    /// Check if a chunk with the given content ID might exist
    ///
    /// Returns `true` if the chunk might be present (may be false positive).
    /// Returns `false` if the chunk is definitely not present.
    pub fn has_chunk(&self, content_id: &[u8; 32]) -> Result<bool> {
        Ok(self.bloom.contains(content_id))
    }

    /// Get the total number of chunks
    pub fn chunk_count(&self) -> Result<u32> {
        Ok(self.data.chunk_count)
    }

    /// Get the total number of windows
    pub fn window_count(&self) -> Result<u32> {
        Ok(self.data.window_count)
    }

    /// Get the file size
    pub fn file_size(&self) -> Result<u64> {
        Ok(self.data.file_size)
    }

    /// Get chunk metadata by index
    pub fn get_chunk(&self, index: u32) -> Result<ChunkMeta> {
        if index as usize >= self.data.chunks.len() {
            return Err(Error::chunk_index_out_of_bounds(
                index,
                self.data.chunks.len() as u32,
            ));
        }

        Ok(self.data.chunks[index as usize].clone())
    }

    /// Get window metadata by index
    pub fn get_window(&self, index: u32) -> Result<WindowMeta> {
        if index as usize >= self.data.windows.len() {
            return Err(Error::window_index_out_of_bounds(
                index,
                self.data.windows.len() as u32,
            ));
        }

        Ok(self.data.windows[index as usize].clone())
    }

    /// Get the bitmap for a specific window
    pub fn get_bitmap(&self, window_id: u32) -> Result<&RankSelectBitmap> {
        if window_id as usize >= self.bitmaps.len() {
            return Err(Error::window_index_out_of_bounds(
                window_id,
                self.bitmaps.len() as u32,
            ));
        }

        Ok(&self.bitmaps[window_id as usize])
    }

    /// Get all missing (not yet transferred) chunk indices for a window
    ///
    /// Returns a vector of chunk indices (relative to the window) that are not yet complete.
    pub fn next_missing(&self, window_id: u32) -> Result<Vec<u32>> {
        let bitmap = self.get_bitmap(window_id)?;
        let unset = bitmap.get_unset_positions();
        Ok(unset.into_iter().map(|i| i as u32).collect())
    }

    /// Get all completed chunk indices for a window
    pub fn get_completed(&self, window_id: u32) -> Result<Vec<u32>> {
        let bitmap = self.get_bitmap(window_id)?;
        let set = bitmap.get_set_positions();
        Ok(set.into_iter().map(|i| i as u32).collect())
    }

    /// Check if a specific chunk in a window is completed
    pub fn is_chunk_complete(&self, window_id: u32, chunk_index: u32) -> Result<bool> {
        let bitmap = self.get_bitmap(window_id)?;
        Ok(bitmap.get(chunk_index as usize))
    }

    /// Get completion statistics for a window
    pub fn window_stats(&self, window_id: u32) -> Result<WindowStats> {
        let bitmap = self.get_bitmap(window_id)?;
        let window = self.get_window(window_id)?;

        Ok(WindowStats {
            window_id,
            total_chunks: window.count as u32,
            completed_chunks: bitmap.count_ones(),
            missing_chunks: bitmap.count_zeros(),
            completion_percent: (bitmap.count_ones() as f64 / window.count as f64) * 100.0,
        })
    }

    /// Get overall completion statistics
    pub fn overall_stats(&self) -> Result<OverallStats> {
        let window_count = self.window_count()?;
        let mut total_chunks = 0u32;
        let mut completed_chunks = 0u32;

        for window_id in 0..window_count {
            let stats = self.window_stats(window_id)?;
            total_chunks += stats.total_chunks;
            completed_chunks += stats.completed_chunks;
        }

        Ok(OverallStats {
            total_windows: window_count,
            total_chunks,
            completed_chunks,
            missing_chunks: total_chunks - completed_chunks,
            completion_percent: if total_chunks > 0 {
                (completed_chunks as f64 / total_chunks as f64) * 100.0
            } else {
                0.0
            },
        })
    }
}

/// Statistics for a single window
#[derive(Debug, Clone)]
pub struct WindowStats {
    pub window_id: u32,
    pub total_chunks: u32,
    pub completed_chunks: u32,
    pub missing_chunks: u32,
    pub completion_percent: f64,
}

/// Overall completion statistics
#[derive(Debug, Clone)]
pub struct OverallStats {
    pub total_windows: u32,
    pub total_chunks: u32,
    pub completed_chunks: u32,
    pub missing_chunks: u32,
    pub completion_percent: f64,
}

// Star Map reader is thread-safe for reading
unsafe impl Send for StarMapReader {}
unsafe impl Sync for StarMapReader {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StarMapBuilder;
    use tempfile::NamedTempFile;

    fn create_test_starmap() -> NamedTempFile {
        let mut builder = StarMapBuilder::new(8192);

        builder.add_chunk(0, 4096, &[1u8; 32]).unwrap();
        builder.add_chunk(4096, 4096, &[2u8; 32]).unwrap();
        builder.add_window(0, 0, 2, &[0u8; 32], 0).unwrap();

        let data = builder.build().unwrap();

        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), data).unwrap();

        temp_file
    }

    #[test]
    fn test_reader_open() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        assert_eq!(reader.chunk_count().unwrap(), 2);
        assert_eq!(reader.window_count().unwrap(), 1);
        assert_eq!(reader.file_size().unwrap(), 8192);
    }

    #[test]
    fn test_get_chunk() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        let chunk0 = reader.get_chunk(0).unwrap();
        assert_eq!(chunk0.offset, 0);
        assert_eq!(chunk0.length, 4096);
        assert_eq!(chunk0.content_id, [1u8; 32]);

        let chunk1 = reader.get_chunk(1).unwrap();
        assert_eq!(chunk1.offset, 4096);
        assert_eq!(chunk1.content_id, [2u8; 32]);
    }

    #[test]
    fn test_get_chunk_out_of_bounds() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        let result = reader.get_chunk(10);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ChunkIndexOutOfBounds { .. }));
    }

    #[test]
    fn test_get_window() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        let window = reader.get_window(0).unwrap();
        assert_eq!(window.id, 0);
        assert_eq!(window.first_chunk, 0);
        assert_eq!(window.count, 2);
        assert_eq!(window.merkle_root, [0u8; 32]);
    }

    #[test]
    fn test_has_chunk() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        assert!(reader.has_chunk(&[1u8; 32]).unwrap());
        assert!(reader.has_chunk(&[2u8; 32]).unwrap());
    }

    #[test]
    fn test_next_missing() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        // All chunks should be missing initially (bitmap starts at 0)
        let missing = reader.next_missing(0).unwrap();
        assert_eq!(missing.len(), 2);
        assert!(missing.contains(&0));
        assert!(missing.contains(&1));
    }

    #[test]
    fn test_window_stats() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        let stats = reader.window_stats(0).unwrap();
        assert_eq!(stats.window_id, 0);
        assert_eq!(stats.total_chunks, 2);
        assert_eq!(stats.completed_chunks, 0);
        assert_eq!(stats.missing_chunks, 2);
        assert_eq!(stats.completion_percent, 0.0);
    }

    #[test]
    fn test_overall_stats() {
        let temp_file = create_test_starmap();
        let reader = StarMapReader::open(temp_file.path()).unwrap();

        let stats = reader.overall_stats().unwrap();
        assert_eq!(stats.total_windows, 1);
        assert_eq!(stats.total_chunks, 2);
        assert_eq!(stats.completed_chunks, 0);
        assert_eq!(stats.missing_chunks, 2);
        assert_eq!(stats.completion_percent, 0.0);
    }

    #[test]
    fn test_reader_not_found() {
        let result = StarMapReader::open("/nonexistent/file.starmap");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NotFound { .. }));
    }

    #[test]
    fn test_magic_number_validation() {
        // Create a file with invalid magic number
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"BADMAGIC").unwrap();

        let result = StarMapReader::open(temp_file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidMagic { .. }));
    }
}