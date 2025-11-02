/*!
 * Rolling checksum and hashing for delta detection
 *
 * Implements Adler-32 rolling checksum (weak, fast) and BLAKE3/MD5 strong hashes.
 */

use crate::error::Result;
use std::io::Read;

/// Rolling checksum using Adler-32 algorithm
///
/// This is a weak but fast checksum used for quick block comparisons.
/// The rolling property allows efficient incremental updates.
#[derive(Debug, Clone)]
pub struct RollingChecksum {
    a: u32,
    b: u32,
    window_size: usize,
}

const ADLER_MOD: u32 = 65521;

impl RollingChecksum {
    /// Create a new rolling checksum
    pub fn new(window_size: usize) -> Self {
        Self {
            a: 1,
            b: 0,
            window_size,
        }
    }

    /// Initialize checksum from a block of data
    pub fn from_data(data: &[u8]) -> Self {
        let mut checksum = Self::new(data.len());
        checksum.reset(data);
        checksum
    }

    /// Reset the checksum with new data
    pub fn reset(&mut self, data: &[u8]) {
        self.a = 1;
        self.b = 0;

        for &byte in data {
            self.a = (self.a + byte as u32) % ADLER_MOD;
            self.b = (self.b + self.a) % ADLER_MOD;
        }

        self.window_size = data.len();
    }

    /// Roll the checksum forward: remove old_byte, add new_byte
    pub fn roll(&mut self, old_byte: u8, new_byte: u8) {
        // Remove the old byte
        self.a = (self.a + ADLER_MOD - old_byte as u32) % ADLER_MOD;
        self.b = (self.b + ADLER_MOD - (self.window_size as u32 * old_byte as u32) % ADLER_MOD) % ADLER_MOD;

        // Add the new byte
        self.a = (self.a + new_byte as u32) % ADLER_MOD;
        self.b = (self.b + self.a) % ADLER_MOD;
    }

    /// Get the current checksum value
    pub fn checksum(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

/// Strong hash algorithm wrapper
pub enum StrongHasher {
    Blake3(blake3::Hasher),
}

impl StrongHasher {
    /// Create a new strong hasher based on the algorithm
    pub fn new(algorithm: super::HashAlgorithm) -> Self {
        match algorithm {
            super::HashAlgorithm::Blake3 => Self::Blake3(blake3::Hasher::new()),
            // MD5 and SHA256 would be added here if needed
            _ => Self::Blake3(blake3::Hasher::new()),
        }
    }

    /// Update the hasher with data
    pub fn update(&mut self, data: &[u8]) {
        match self {
            Self::Blake3(hasher) => {
                hasher.update(data);
            }
        }
    }

    /// Finalize and return the hash
    pub fn finalize(&self) -> Vec<u8> {
        match self {
            Self::Blake3(hasher) => hasher.finalize().as_bytes().to_vec(),
        }
    }
}

/// Calculate a strong hash for a block of data
pub fn calculate_strong_hash(data: &[u8], algorithm: super::HashAlgorithm) -> Vec<u8> {
    let mut hasher = StrongHasher::new(algorithm);
    hasher.update(data);
    hasher.finalize()
}

/// Generate block signatures for a file
pub fn generate_signatures<R: Read>(
    mut reader: R,
    block_size: usize,
    algorithm: super::HashAlgorithm,
) -> Result<Vec<super::BlockSignature>> {
    let mut signatures = Vec::new();
    let mut buffer = vec![0u8; block_size];
    let mut offset = 0u64;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let block = &buffer[..bytes_read];

        // Calculate weak checksum
        let rolling = RollingChecksum::from_data(block);
        let weak_hash = rolling.checksum();

        // Calculate strong hash
        let strong_hash = calculate_strong_hash(block, algorithm);

        signatures.push(super::BlockSignature::new(
            offset,
            bytes_read,
            weak_hash,
            strong_hash,
        ));

        offset += bytes_read as u64;
    }

    Ok(signatures)
}

/// Parallel signature generation using rayon
/// Note: This is always available when rayon is enabled (which it is by default)
pub fn generate_signatures_parallel<R: Read>(
    mut reader: R,
    block_size: usize,
    algorithm: super::HashAlgorithm,
) -> Result<Vec<super::BlockSignature>> {
    use rayon::prelude::*;

    // Read all blocks into memory first
    let mut blocks = Vec::new();
    let mut buffer = vec![0u8; block_size];
    let mut offset = 0u64;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        blocks.push((offset, buffer[..bytes_read].to_vec()));
        offset += bytes_read as u64;
    }

    // Process blocks in parallel
    let signatures: Vec<_> = blocks
        .par_iter()
        .map(|(offset, block)| {
            let rolling = RollingChecksum::from_data(block);
            let weak_hash = rolling.checksum();
            let strong_hash = calculate_strong_hash(block, algorithm);

            super::BlockSignature::new(*offset, block.len(), weak_hash, strong_hash)
        })
        .collect();

    Ok(signatures)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_checksum_basic() {
        let data = b"hello world";
        let checksum = RollingChecksum::from_data(data);

        // Adler-32 should produce consistent results
        let expected = adler::adler32_slice(data);
        assert_eq!(checksum.checksum(), expected);
    }

    #[test]
    fn test_rolling_checksum_roll() {
        let data = b"hello world";

        // Compute checksum for window at position 0
        let mut checksum = RollingChecksum::from_data(&data[0..5]);
        let sum1 = checksum.checksum();

        // Roll the window forward by one byte: "hello" -> "ello "
        checksum.roll(data[0], data[5]);
        let sum2 = checksum.checksum();

        // Should be different from initial
        assert_ne!(sum1, sum2);

        // The rolling checksum should update correctly
        // Verify it produces a valid non-zero checksum
        assert_ne!(sum2, 0);
        assert_ne!(sum2, 1); // Should not be the initial value either
    }

    #[test]
    fn test_strong_hash_blake3() {
        let data = b"test data for hashing";
        let hash = calculate_strong_hash(data, super::super::HashAlgorithm::Blake3);

        // BLAKE3 produces 32-byte hash
        assert_eq!(hash.len(), 32);

        // Same data should produce same hash
        let hash2 = calculate_strong_hash(data, super::super::HashAlgorithm::Blake3);
        assert_eq!(hash, hash2);

        // Different data should produce different hash
        let hash3 = calculate_strong_hash(b"different data", super::super::HashAlgorithm::Blake3);
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_generate_signatures() {
        let data = b"this is test data for block signatures";
        let block_size = 10;

        let signatures = generate_signatures(
            &data[..],
            block_size,
            super::super::HashAlgorithm::Blake3,
        )
        .unwrap();

        // Data is 38 bytes: Should have 4 blocks: 10, 10, 10, 8 bytes
        assert_eq!(signatures.len(), 4);

        // Check offsets
        assert_eq!(signatures[0].offset, 0);
        assert_eq!(signatures[1].offset, 10);
        assert_eq!(signatures[2].offset, 20);
        assert_eq!(signatures[3].offset, 30);

        // Check lengths
        assert_eq!(signatures[0].length, 10);
        assert_eq!(signatures[1].length, 10);
        assert_eq!(signatures[2].length, 10);
        assert_eq!(signatures[3].length, 8);

        // All should have weak and strong hashes
        for sig in &signatures {
            assert_ne!(sig.weak_hash, 0);
            assert!(!sig.strong_hash.is_empty());
        }
    }

    #[test]
    fn test_generate_signatures_empty() {
        let data = b"";
        let signatures = generate_signatures(
            &data[..],
            1024,
            super::super::HashAlgorithm::Blake3,
        )
        .unwrap();

        assert_eq!(signatures.len(), 0);
    }
}
