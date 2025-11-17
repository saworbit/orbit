//! Bloom filter for fast chunk existence checks
//!
//! A space-efficient probabilistic data structure for testing set membership.
//! May return false positives but never false negatives.

use crate::error::{Error, Result};
use bitvec::prelude::*;
use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};

/// Bloom filter for chunk content ID membership testing
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// Bit array for the filter
    bits: BitVec<u8, Msb0>,
    /// Number of hash functions
    num_hashes: u32,
    /// Number of elements inserted (for statistics)
    num_elements: u32,
}

impl BloomFilter {
    /// Create a new Bloom filter with specified capacity and false positive rate
    ///
    /// # Arguments
    /// * `expected_elements` - Expected number of elements to insert
    /// * `false_positive_rate` - Desired false positive rate (e.g., 0.01 for 1%)
    ///
    /// # Example
    /// ```
    /// use orbit_core_starmap::BloomFilter;
    ///
    /// // Create filter for 10,000 elements with 1% false positive rate
    /// let filter = BloomFilter::new(10_000, 0.01);
    /// ```
    pub fn new(expected_elements: u32, false_positive_rate: f64) -> Self {
        // Calculate optimal bit array size
        // m = -n * ln(p) / (ln(2)^2)
        let num_bits = Self::optimal_num_bits(expected_elements, false_positive_rate);

        // Calculate optimal number of hash functions
        // k = (m/n) * ln(2)
        let num_hashes = Self::optimal_num_hashes(num_bits, expected_elements);

        Self {
            bits: bitvec![u8, Msb0; 0; num_bits as usize],
            num_hashes,
            num_elements: 0,
        }
    }

    /// Create a Bloom filter from serialized data
    pub fn from_bytes(
        data: &[u8],
        num_hashes: u32,
        num_elements: u32,
        num_bits: usize,
    ) -> Result<Self> {
        if data.is_empty() {
            return Err(Error::bloom_filter("Empty bloom filter data"));
        }

        // Create BitVec from the data and truncate to exact bit length
        let mut bits = BitVec::from_vec(data.to_vec());
        bits.truncate(num_bits);

        Ok(Self {
            bits,
            num_hashes,
            num_elements,
        })
    }

    /// Insert an element into the Bloom filter
    ///
    /// # Example
    /// ```
    /// use orbit_core_starmap::BloomFilter;
    ///
    /// let mut filter = BloomFilter::new(1000, 0.01);
    /// let content_id = [0u8; 32];
    /// filter.insert(&content_id);
    /// assert!(filter.contains(&content_id));
    /// ```
    pub fn insert<T: Hash>(&mut self, item: &T) {
        let hashes = self.hash(item);
        for hash in hashes {
            let bit_index = (hash % self.bits.len() as u64) as usize;
            self.bits.set(bit_index, true);
        }
        self.num_elements += 1;
    }

    /// Check if an element might be in the set
    ///
    /// Returns `true` if the element might be present (may be false positive).
    /// Returns `false` if the element is definitely not present (no false negatives).
    pub fn contains<T: Hash>(&self, item: &T) -> bool {
        let hashes = self.hash(item);
        for hash in hashes {
            let bit_index = (hash % self.bits.len() as u64) as usize;
            if !self.bits[bit_index] {
                return false;
            }
        }
        true
    }

    /// Serialize the Bloom filter to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.bits.as_raw_slice().to_vec()
    }

    /// Get the number of bits in the filter
    pub fn num_bits(&self) -> usize {
        self.bits.len()
    }

    /// Get the number of hash functions
    pub fn num_hashes(&self) -> u32 {
        self.num_hashes
    }

    /// Get the number of elements inserted
    pub fn num_elements(&self) -> u32 {
        self.num_elements
    }

    /// Calculate the estimated false positive rate based on current state
    pub fn estimated_false_positive_rate(&self) -> f64 {
        if self.num_elements == 0 {
            return 0.0;
        }

        // p ≈ (1 - e^(-k*n/m))^k
        let m = self.bits.len() as f64;
        let n = self.num_elements as f64;
        let k = self.num_hashes as f64;

        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// Generate hash values for an item using double hashing
    ///
    /// Uses two independent hash functions (SipHash with different keys)
    /// to generate k hash values: h_i = h1 + i*h2
    fn hash<T: Hash>(&self, item: &T) -> Vec<u64> {
        let mut hashes = Vec::with_capacity(self.num_hashes as usize);

        // First hash function
        let mut hasher1 = SipHasher13::new_with_keys(0, 0);
        item.hash(&mut hasher1);
        let h1 = hasher1.finish();

        // Second hash function (different key)
        let mut hasher2 = SipHasher13::new_with_keys(1, 1);
        item.hash(&mut hasher2);
        let h2 = hasher2.finish();

        // Generate k hash values using double hashing
        for i in 0..self.num_hashes {
            hashes.push(h1.wrapping_add((i as u64).wrapping_mul(h2)));
        }

        hashes
    }

    /// Calculate optimal number of bits for the filter
    fn optimal_num_bits(expected_elements: u32, false_positive_rate: f64) -> u32 {
        let n = expected_elements as f64;
        let p = false_positive_rate;
        let ln2_squared = std::f64::consts::LN_2.powi(2);

        let m = -(n * p.ln()) / ln2_squared;
        m.ceil() as u32
    }

    /// Calculate optimal number of hash functions
    fn optimal_num_hashes(num_bits: u32, expected_elements: u32) -> u32 {
        let m = num_bits as f64;
        let n = expected_elements as f64;

        let k = (m / n) * std::f64::consts::LN_2;
        k.ceil().max(1.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bloom_filter_basic() {
        let mut filter = BloomFilter::new(100, 0.01);

        let item1 = [1u8; 32];
        let item2 = [2u8; 32];
        let item3 = [3u8; 32];

        // Initially, nothing is present
        assert!(!filter.contains(&item1));
        assert!(!filter.contains(&item2));

        // Insert item1
        filter.insert(&item1);
        assert!(filter.contains(&item1));
        assert!(!filter.contains(&item2));

        // Insert item2
        filter.insert(&item2);
        assert!(filter.contains(&item1));
        assert!(filter.contains(&item2));
        assert!(!filter.contains(&item3));
    }

    #[test]
    fn test_bloom_filter_serialization() {
        let mut filter = BloomFilter::new(100, 0.01);

        let item = [42u8; 32];
        filter.insert(&item);

        // Serialize
        let bytes = filter.to_bytes();
        let num_hashes = filter.num_hashes();
        let num_elements = filter.num_elements();
        let num_bits = filter.num_bits();

        // Deserialize
        let restored = BloomFilter::from_bytes(&bytes, num_hashes, num_elements, num_bits).unwrap();

        // Should still contain the item
        assert!(restored.contains(&item));
        assert_eq!(restored.num_bits(), filter.num_bits());
        assert_eq!(restored.num_hashes(), filter.num_hashes());
    }

    #[test]
    fn test_optimal_sizing() {
        let filter = BloomFilter::new(1000, 0.01);

        // With 1000 elements and 1% FPR, should have ~9585 bits and 7 hashes
        assert!(filter.num_bits() > 9000 && filter.num_bits() < 10000);
        assert!(filter.num_hashes() >= 6 && filter.num_hashes() <= 8);
    }

    #[test]
    fn test_false_positive_rate() {
        let mut filter = BloomFilter::new(100, 0.01);

        // Insert 100 items
        for i in 0..100 {
            let item = [i as u8; 32];
            filter.insert(&item);
        }

        // Estimated FPR should be close to 1%
        let estimated_fpr = filter.estimated_false_positive_rate();
        assert!(estimated_fpr < 0.02, "FPR too high: {}", estimated_fpr);
    }

    #[test]
    fn test_no_false_negatives() {
        let mut filter = BloomFilter::new(1000, 0.01);

        let mut items = Vec::new();
        for i in 0..500 {
            let mut item = [0u8; 32];
            item[0] = (i % 256) as u8;
            item[1] = (i / 256) as u8;
            items.push(item);
            filter.insert(&item);
        }

        // All inserted items must be found (no false negatives)
        for item in &items {
            assert!(filter.contains(item), "False negative detected!");
        }
    }

    #[test]
    fn test_empty_bloom_filter() {
        let filter = BloomFilter::new(100, 0.01);

        let item = [0u8; 32];
        assert!(!filter.contains(&item));
        assert_eq!(filter.num_elements(), 0);
        assert_eq!(filter.estimated_false_positive_rate(), 0.0);
    }

    #[test]
    fn test_from_empty_bytes() {
        let result = BloomFilter::from_bytes(&[], 7, 0, 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty"));
    }
}
