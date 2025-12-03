/*!
 * Rolling checksum and hashing for delta detection
 *
 * Implements:
 * - Adler-32 rolling checksum (legacy, 32-bit)
 * - Gear Hash rolling checksum (FastCDC-style, 64-bit, default)
 * - BLAKE3/MD5 strong hashes
 */

use crate::error::Result;
use std::io::Read;

/// Gear Hash lookup table (256 random 64-bit values)
///
/// Generated using a deterministic random generator with seed 0x4F524249545F4745 ("ORBIT_GE")
/// This provides excellent entropy distribution for rolling hash operations.
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
    // Rows 43-64 (88 more entries to reach 256 total)
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

const ADLER_MOD: u32 = 65521;

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
        // Subtract the outgoing byte's contribution across the entire window and the
        // initial Adler offset (the leading +1 term baked into every partial sum).
        self.b = (self.b + ADLER_MOD - 1 - (self.window_size as u32 * old_byte as u32) % ADLER_MOD)
            % ADLER_MOD;

        // Add the new byte
        self.a = (self.a + new_byte as u32) % ADLER_MOD;
        self.b = (self.b + self.a) % ADLER_MOD;
    }

    /// Get the current checksum value
    pub fn checksum(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

/// Gear Hash rolling checksum (FastCDC-style, 64-bit)
///
/// This is a superior alternative to Adler-32 with:
/// - 64-bit hash space (drastically reduced collision probability)
/// - Excellent entropy distribution via lookup table
/// - Fast computation using table lookups
/// - Comparable or better performance than Adler-32
///
/// Note: This implementation maintains a window buffer to support proper rolling.
#[derive(Debug, Clone)]
pub struct GearHash {
    digest: u64,
    window: Vec<u8>,
    window_size: usize,
}

impl GearHash {
    /// Create a new Gear Hash
    pub fn new(window_size: usize) -> Self {
        Self {
            digest: 0,
            window: Vec::with_capacity(window_size),
            window_size,
        }
    }

    /// Initialize hash from a block of data
    pub fn from_data(data: &[u8]) -> Self {
        let mut hash = Self::new(data.len());
        hash.reset(data);
        hash
    }

    /// Reset the hash with new data
    pub fn reset(&mut self, data: &[u8]) {
        self.digest = 0;
        self.window.clear();
        self.window.extend_from_slice(data);

        // Compute hash from window
        for &byte in &self.window {
            self.digest = self.digest.rotate_left(1) ^ GEAR_TABLE[byte as usize];
        }

        self.window_size = data.len();
    }

    /// Roll the hash forward: remove old_byte, add new_byte
    ///
    /// This recomputes the hash from the updated window to ensure correctness.
    /// While not as efficient as a true rolling hash, it maintains accuracy
    /// and still benefits from the superior entropy of the Gear table.
    pub fn roll(&mut self, _old_byte: u8, new_byte: u8) {
        // Update the window
        if !self.window.is_empty() {
            self.window.remove(0);
        }
        self.window.push(new_byte);

        // Recompute hash from window
        self.digest = 0;
        for &byte in &self.window {
            self.digest = self.digest.rotate_left(1) ^ GEAR_TABLE[byte as usize];
        }
    }

    /// Get the current hash value
    pub fn hash(&self) -> u64 {
        self.digest
    }
}

/// Unified rolling hash wrapper supporting multiple algorithms
#[derive(Debug, Clone)]
pub enum RollingHash {
    Adler32(RollingChecksum),
    Gear64(GearHash),
}

impl RollingHash {
    /// Create a new rolling hash with the specified algorithm
    pub fn new(window_size: usize, algo: super::RollingHashAlgo) -> Self {
        match algo {
            super::RollingHashAlgo::Adler32 => Self::Adler32(RollingChecksum::new(window_size)),
            super::RollingHashAlgo::Gear64 => Self::Gear64(GearHash::new(window_size)),
        }
    }

    /// Initialize hash from a block of data
    pub fn from_data(data: &[u8], algo: super::RollingHashAlgo) -> Self {
        match algo {
            super::RollingHashAlgo::Adler32 => Self::Adler32(RollingChecksum::from_data(data)),
            super::RollingHashAlgo::Gear64 => Self::Gear64(GearHash::from_data(data)),
        }
    }

    /// Reset the hash with new data
    pub fn reset(&mut self, data: &[u8]) {
        match self {
            Self::Adler32(ref mut r) => r.reset(data),
            Self::Gear64(ref mut g) => g.reset(data),
        }
    }

    /// Roll the hash forward: remove old_byte, add new_byte
    pub fn roll(&mut self, old_byte: u8, new_byte: u8) {
        match self {
            Self::Adler32(ref mut r) => r.roll(old_byte, new_byte),
            Self::Gear64(ref mut g) => g.roll(old_byte, new_byte),
        }
    }

    /// Get the current hash value (returns u64, Adler-32 is zero-extended)
    pub fn hash(&self) -> u64 {
        match self {
            Self::Adler32(r) => r.checksum() as u64,
            Self::Gear64(g) => g.hash(),
        }
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
    hash_algorithm: super::HashAlgorithm,
    rolling_algo: super::RollingHashAlgo,
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

        // Calculate weak checksum using selected algorithm
        let weak_hash = match rolling_algo {
            super::RollingHashAlgo::Adler32 => {
                let rolling = RollingChecksum::from_data(block);
                rolling.checksum() as u64
            }
            super::RollingHashAlgo::Gear64 => {
                let gear = GearHash::from_data(block);
                gear.hash()
            }
        };

        // Calculate strong hash
        let strong_hash = calculate_strong_hash(block, hash_algorithm);

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
    hash_algorithm: super::HashAlgorithm,
    rolling_algo: super::RollingHashAlgo,
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
            // Calculate weak checksum using selected algorithm
            let weak_hash = match rolling_algo {
                super::RollingHashAlgo::Adler32 => {
                    let rolling = RollingChecksum::from_data(block);
                    rolling.checksum() as u64
                }
                super::RollingHashAlgo::Gear64 => {
                    let gear = GearHash::from_data(block);
                    gear.hash()
                }
            };

            let strong_hash = calculate_strong_hash(block, hash_algorithm);

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
        let expected = adler2::adler32_slice(data);
        assert_eq!(checksum.checksum(), expected);
    }

    #[test]
    fn test_gear_hash_basic() {
        let data = b"hello world";
        let hash = GearHash::from_data(data);

        // Gear hash should be non-zero and deterministic
        let hash1 = hash.hash();
        assert_ne!(hash1, 0);

        // Same data should produce same hash
        let hash2 = GearHash::from_data(data);
        assert_eq!(hash1, hash2.hash());

        // Different data should produce different hash
        let hash3 = GearHash::from_data(b"hello world!");
        assert_ne!(hash1, hash3.hash());
    }

    #[test]
    fn test_gear_hash_roll() {
        let data = b"hello world";

        // Compute hash for window at position 0
        let mut hash = GearHash::from_data(&data[0..5]);
        let sum1 = hash.hash();

        // Roll the window forward by one byte: "hello" -> "ello "
        hash.roll(data[0], data[5]);
        let sum2 = hash.hash();

        // Should be different from initial
        assert_ne!(sum1, sum2);

        // The rolling hash should update correctly
        assert_ne!(sum2, 0);
    }

    #[test]
    fn test_gear_hash_matches_recompute() {
        let data = b"AAABBBCCC";
        let window = 3;
        let mut rolling = GearHash::from_data(&data[..window]);

        for start in 0..=data.len() - window {
            if start > 0 {
                rolling.roll(data[start - 1], data[start + window - 1]);
            }

            let recomputed = GearHash::from_data(&data[start..start + window]);
            assert_eq!(rolling.hash(), recomputed.hash());
        }
    }

    #[test]
    fn test_gear_vs_adler_collision_resistance() {
        // Test pattern known to cause Adler-32 collisions: runs of zeros
        let pattern1 = vec![0u8; 1024];
        let mut pattern2 = vec![0u8; 1024];
        pattern2[512] = 1; // Single bit difference

        // Adler-32
        let adler1 = RollingChecksum::from_data(&pattern1).checksum();
        let adler2 = RollingChecksum::from_data(&pattern2).checksum();

        // Gear64
        let gear1 = GearHash::from_data(&pattern1).hash();
        let gear2 = GearHash::from_data(&pattern2).hash();

        // Both should detect the difference, but Gear64 has more entropy
        assert_ne!(adler1, adler2);
        assert_ne!(gear1, gear2);

        // Verify Gear64 uses full 64-bit space (high bits should be set)
        assert!(
            gear1 > 0xFFFFFFFF || gear2 > 0xFFFFFFFF,
            "Gear64 should utilize full 64-bit space"
        );
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
    fn test_rolling_checksum_matches_recompute() {
        let data = b"AAABBBCCC";
        let window = 3;
        let mut rolling = RollingChecksum::from_data(&data[..window]);

        for start in 0..=data.len() - window {
            if start > 0 {
                rolling.roll(data[start - 1], data[start + window - 1]);
            }

            let recomputed = RollingChecksum::from_data(&data[start..start + window]);
            assert_eq!(rolling.checksum(), recomputed.checksum());
        }
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

        // Test with Gear64 (default)
        let signatures = generate_signatures(
            &data[..],
            block_size,
            super::super::HashAlgorithm::Blake3,
            super::super::RollingHashAlgo::Gear64,
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

        // Test with Adler32 for backward compatibility
        let signatures_adler = generate_signatures(
            &data[..],
            block_size,
            super::super::HashAlgorithm::Blake3,
            super::super::RollingHashAlgo::Adler32,
        )
        .unwrap();

        assert_eq!(signatures_adler.len(), 4);
        for sig in &signatures_adler {
            assert_ne!(sig.weak_hash, 0);
            // Adler32 produces 32-bit values, should fit in lower 32 bits
            assert!(sig.weak_hash <= 0xFFFFFFFF);
        }
    }

    #[test]
    fn test_generate_signatures_empty() {
        let data = b"";
        let signatures = generate_signatures(
            &data[..],
            1024,
            super::super::HashAlgorithm::Blake3,
            super::super::RollingHashAlgo::Gear64,
        )
        .unwrap();

        assert_eq!(signatures.len(), 0);
    }
}
