/*!
 * Streaming checksum calculation for efficient hashing during copy
 */

use crate::error::Result;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Streaming hasher that calculates checksum incrementally
pub struct StreamingHasher {
    hasher: Sha256,
}

impl StreamingHasher {
    /// Create a new streaming hasher
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    /// Update the hash with new data
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Finalize and return the hash
    pub fn finalize(self) -> sha2::digest::Output<Sha256> {
        self.hasher.finalize()
    }
}

impl Default for StreamingHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate checksum of a file (standalone function)
pub fn calculate_checksum(path: &Path) -> Result<String> {
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024]; // 64KB buffer

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_streaming_hasher() {
        let mut hasher = StreamingHasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");

        let result = format!("{:x}", hasher.finalize());

        // SHA256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_calculate_checksum() {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(b"test data").unwrap();
        temp.flush().unwrap();

        let checksum = calculate_checksum(temp.path()).unwrap();
        assert_eq!(checksum.len(), 64); // SHA256 is 64 hex chars
    }
}
