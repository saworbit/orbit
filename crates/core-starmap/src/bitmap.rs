//! Rank-select bitmap for tracking chunk completion
//!
//! Provides efficient rank and select operations on bitmaps to track
//! which chunks in a window have been transferred.

use crate::error::{Error, Result};
use bitvec::prelude::*;

/// Rank-select bitmap for tracking chunk state
///
/// # Operations
/// - **Set**: Mark a chunk as present/complete
/// - **Get**: Check if a chunk is present
/// - **Rank**: Count number of 1s up to position (how many chunks complete)
/// - **Select**: Find position of nth 1 (find nth complete chunk)
#[derive(Debug, Clone)]
pub struct RankSelectBitmap {
    /// The underlying bit array
    bits: BitVec<u8, Msb0>,
    /// Cached rank at block boundaries (for fast rank queries)
    rank_cache: Vec<u32>,
    /// Block size for rank caching (typically 512 bits)
    block_size: usize,
}

impl RankSelectBitmap {
    /// Create a new bitmap with the specified size
    ///
    /// # Example
    /// ```
    /// use orbit_core_starmap::RankSelectBitmap;
    ///
    /// let mut bitmap = RankSelectBitmap::new(100);
    /// bitmap.set(10, true);
    /// assert!(bitmap.get(10));
    /// assert_eq!(bitmap.rank(10), 1);
    /// ```
    pub fn new(size: usize) -> Self {
        let block_size = 512; // Standard block size for rank caching
        let num_blocks = (size + block_size - 1) / block_size;
        
        Self {
            bits: bitvec![u8, Msb0; 0; size],
            rank_cache: vec![0; num_blocks + 1],
            block_size,
        }
    }

    /// Create a bitmap from serialized bytes
    pub fn from_bytes(data: &[u8], size: usize) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::bitmap("Empty bitmap data"));
        }

        let mut bitmap = Self::new(size);
        
        // Copy bits from data
        let bits_to_copy = size.min(data.len() * 8);
        for i in 0..bits_to_copy {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            let bit_value = (data[byte_idx] >> (7 - bit_idx)) & 1;
            bitmap.bits.set(i, bit_value == 1);
        }

        // Rebuild rank cache
        bitmap.rebuild_rank_cache();

        Ok(bitmap)
    }

    /// Set a bit at the specified position
    pub fn set(&mut self, position: usize, value: bool) -> Result<()> {
        if position >= self.bits.len() {
            return Err(Error::bitmap(format!(
                "Position {} out of bounds (size: {})",
                position,
                self.bits.len()
            )));
        }

        let old_value = self.bits[position];
        self.bits.set(position, value);

        // Update rank cache if the value changed
        if old_value != value {
            self.update_rank_cache_from(position);
        }

        Ok(())
    }

    /// Get the bit at the specified position
    pub fn get(&self, position: usize) -> bool {
        if position >= self.bits.len() {
            return false;
        }
        self.bits[position]
    }

    /// Count the number of 1s up to (but not including) the specified position
    ///
    /// # Example
    /// ```
    /// use orbit_core_starmap::RankSelectBitmap;
    ///
    /// let mut bitmap = RankSelectBitmap::new(100);
    /// bitmap.set(5, true);
    /// bitmap.set(10, true);
    /// bitmap.set(15, true);
    ///
    /// assert_eq!(bitmap.rank(0), 0);
    /// assert_eq!(bitmap.rank(6), 1);  // One 1 before position 6
    /// assert_eq!(bitmap.rank(11), 2); // Two 1s before position 11
    /// assert_eq!(bitmap.rank(20), 3); // Three 1s before position 20
    /// ```
    pub fn rank(&self, position: usize) -> u32 {
        if position == 0 {
            return 0;
        }
        if position >= self.bits.len() {
            return self.count_ones();
        }

        // Get cached rank at block boundary
        let block_idx = position / self.block_size;
        let mut count = self.rank_cache[block_idx];

        // Count remaining bits within the block
        let block_start = block_idx * self.block_size;
        for i in block_start..position {
            if self.bits[i] {
                count += 1;
            }
        }

        count
    }

    /// Find the position of the nth 1 (0-indexed)
    ///
    /// Returns `None` if there are fewer than n+1 ones in the bitmap.
    ///
    /// # Example
    /// ```
    /// use orbit_core_starmap::RankSelectBitmap;
    ///
    /// let mut bitmap = RankSelectBitmap::new(100);
    /// bitmap.set(5, true);
    /// bitmap.set(10, true);
    /// bitmap.set(15, true);
    ///
    /// assert_eq!(bitmap.select(0), Some(5));  // First 1 at position 5
    /// assert_eq!(bitmap.select(1), Some(10)); // Second 1 at position 10
    /// assert_eq!(bitmap.select(2), Some(15)); // Third 1 at position 15
    /// assert_eq!(bitmap.select(3), None);     // No fourth 1
    /// ```
    pub fn select(&self, n: u32) -> Option<usize> {
        if n >= self.count_ones() {
            return None;
        }

        let mut count = 0;
        for (i, bit) in self.bits.iter().enumerate() {
            if *bit {
                if count == n {
                    return Some(i);
                }
                count += 1;
            }
        }

        None
    }

    /// Get the total number of 1s in the bitmap
    pub fn count_ones(&self) -> u32 {
        *self.rank_cache.last().unwrap_or(&0)
    }

    /// Get the total number of 0s in the bitmap
    pub fn count_zeros(&self) -> u32 {
        self.bits.len() as u32 - self.count_ones()
    }

    /// Get the size of the bitmap
    pub fn len(&self) -> usize {
        self.bits.len()
    }

    /// Check if the bitmap is empty
    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }

    /// Serialize the bitmap to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bits.as_raw_slice().to_vec()
    }

    /// Get all positions where the bit is set to 1
    pub fn get_set_positions(&self) -> Vec<usize> {
        self.bits
            .iter()
            .enumerate()
            .filter_map(|(i, bit)| if *bit { Some(i) } else { None })
            .collect()
    }

    /// Get all positions where the bit is set to 0
    pub fn get_unset_positions(&self) -> Vec<usize> {
        self.bits
            .iter()
            .enumerate()
            .filter_map(|(i, bit)| if !*bit { Some(i) } else { None })
            .collect()
    }

    /// Rebuild the rank cache from scratch
    fn rebuild_rank_cache(&mut self) {
        let mut count = 0u32;
        for block_idx in 0..self.rank_cache.len() {
            self.rank_cache[block_idx] = count;
            
            let block_start = block_idx * self.block_size;
            let block_end = (block_start + self.block_size).min(self.bits.len());
            
            for i in block_start..block_end {
                if self.bits[i] {
                    count += 1;
                }
            }
        }
    }

    /// Update rank cache from a specific position onwards
    fn update_rank_cache_from(&mut self, position: usize) {
        let start_block = position / self.block_size;
        
        // Recalculate from the start of this block
        let mut count = if start_block > 0 {
            self.rank_cache[start_block]
        } else {
            0
        };

        for block_idx in start_block..self.rank_cache.len() {
            self.rank_cache[block_idx] = count;
            
            let block_start = block_idx * self.block_size;
            let block_end = (block_start + self.block_size).min(self.bits.len());
            
            for i in block_start..block_end {
                if self.bits[i] {
                    count += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitmap_basic() {
        let mut bitmap = RankSelectBitmap::new(100);
        
        assert_eq!(bitmap.len(), 100);
        assert_eq!(bitmap.count_ones(), 0);
        assert_eq!(bitmap.count_zeros(), 100);

        bitmap.set(10, true).unwrap();
        assert!(bitmap.get(10));
        assert!(!bitmap.get(11));
        assert_eq!(bitmap.count_ones(), 1);
    }

    #[test]
    fn test_rank() {
        let mut bitmap = RankSelectBitmap::new(100);
        
        bitmap.set(5, true).unwrap();
        bitmap.set(10, true).unwrap();
        bitmap.set(15, true).unwrap();

        assert_eq!(bitmap.rank(0), 0);
        assert_eq!(bitmap.rank(5), 0);  // Before position 5
        assert_eq!(bitmap.rank(6), 1);  // After position 5
        assert_eq!(bitmap.rank(10), 1); // Before position 10
        assert_eq!(bitmap.rank(11), 2); // After position 10
        assert_eq!(bitmap.rank(16), 3); // After position 15
        assert_eq!(bitmap.rank(100), 3); // End of bitmap
    }

    #[test]
    fn test_select() {
        let mut bitmap = RankSelectBitmap::new(100);
        
        bitmap.set(5, true).unwrap();
        bitmap.set(10, true).unwrap();
        bitmap.set(15, true).unwrap();

        assert_eq!(bitmap.select(0), Some(5));
        assert_eq!(bitmap.select(1), Some(10));
        assert_eq!(bitmap.select(2), Some(15));
        assert_eq!(bitmap.select(3), None);
    }

    #[test]
    fn test_serialization() {
        let mut bitmap = RankSelectBitmap::new(100);
        
        bitmap.set(10, true).unwrap();
        bitmap.set(20, true).unwrap();
        bitmap.set(30, true).unwrap();

        let bytes = bitmap.to_bytes();
        let restored = RankSelectBitmap::from_bytes(&bytes, 100).unwrap();

        assert!(restored.get(10));
        assert!(restored.get(20));
        assert!(restored.get(30));
        assert!(!restored.get(15));
        assert_eq!(restored.count_ones(), 3);
    }

    #[test]
    fn test_get_set_positions() {
        let mut bitmap = RankSelectBitmap::new(100);
        
        bitmap.set(5, true).unwrap();
        bitmap.set(15, true).unwrap();
        bitmap.set(25, true).unwrap();

        let set_positions = bitmap.get_set_positions();
        assert_eq!(set_positions, vec![5, 15, 25]);

        let unset_count = bitmap.get_unset_positions().len();
        assert_eq!(unset_count, 97);
    }

    #[test]
    fn test_out_of_bounds() {
        let mut bitmap = RankSelectBitmap::new(10);
        
        let result = bitmap.set(10, true);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("out of bounds"));

        assert!(!bitmap.get(10)); // Should return false, not panic
    }

    #[test]
    fn test_rank_cache_update() {
        let mut bitmap = RankSelectBitmap::new(1000);
        
        // Set every 10th bit
        for i in (0..1000).step_by(10) {
            bitmap.set(i, true).unwrap();
        }

        assert_eq!(bitmap.count_ones(), 100);
        assert_eq!(bitmap.rank(500), 50);
        assert_eq!(bitmap.rank(1000), 100);
    }

    #[test]
    fn test_empty_bitmap() {
        let bitmap = RankSelectBitmap::new(0);
        
        assert!(bitmap.is_empty());
        assert_eq!(bitmap.len(), 0);
        assert_eq!(bitmap.count_ones(), 0);
        assert_eq!(bitmap.rank(0), 0);
        assert_eq!(bitmap.select(0), None);
    }

    #[test]
    fn test_from_empty_bytes() {
        let result = RankSelectBitmap::from_bytes(&[], 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty"));
    }
}