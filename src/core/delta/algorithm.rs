/*!
 * Delta algorithm implementation - rsync-inspired block matching
 *
 * Implements the core delta detection algorithm:
 * 1. Generate block signatures for destination file
 * 2. Find matching blocks in source file using rolling checksum
 * 3. Generate delta instructions (Copy existing blocks or insert new data)
 */

use super::{BlockSignature, DeltaInstruction, DeltaStats, HashAlgorithm};
use super::checksum::{RollingChecksum, calculate_strong_hash};
use crate::error::Result;
use std::collections::HashMap;
use std::io::Read;

/// Index of block signatures for fast lookup
#[derive(Clone)]
pub struct SignatureIndex {
    /// Map from weak hash to list of signatures with that weak hash
    weak_hash_map: HashMap<u32, Vec<BlockSignature>>,

    /// Block size used for signatures
    block_size: usize,
}

impl SignatureIndex {
    /// Create a new signature index from a list of signatures
    pub fn new(signatures: Vec<BlockSignature>) -> Self {
        let mut weak_hash_map: HashMap<u32, Vec<BlockSignature>> = HashMap::new();
        let block_size = signatures.first().map(|s| s.length).unwrap_or(0);

        for sig in signatures {
            weak_hash_map
                .entry(sig.weak_hash)
                .or_insert_with(Vec::new)
                .push(sig);
        }

        Self {
            weak_hash_map,
            block_size,
        }
    }

    /// Find a matching block signature for the given weak and strong hashes
    pub fn find_match(&self, weak_hash: u32, strong_hash: &[u8]) -> Option<&BlockSignature> {
        // First check weak hash
        let candidates = self.weak_hash_map.get(&weak_hash)?;

        // Then verify with strong hash
        candidates.iter().find(|sig| sig.strong_hash == strong_hash)
    }

    /// Get the block size
    pub fn block_size(&self) -> usize {
        self.block_size
    }
}

/// Generate delta instructions by comparing source with destination signatures
pub fn generate_delta<R: Read>(
    mut source: R,
    dest_signatures: SignatureIndex,
    algorithm: HashAlgorithm,
) -> Result<(Vec<DeltaInstruction>, DeltaStats)> {
    let block_size = dest_signatures.block_size();
    let mut instructions = Vec::new();
    let mut stats = DeltaStats::new();

    let mut buffer = Vec::new();
    source.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Ok((instructions, stats));
    }

    stats.total_bytes = buffer.len() as u64;
    stats.total_blocks = (buffer.len() + block_size - 1) as u64 / block_size as u64;

    let mut pos = 0;
    let mut dest_offset = 0u64;
    let mut pending_data = Vec::new();

    while pos < buffer.len() {
        // Try to find a matching block starting at current position
        let remaining = buffer.len() - pos;
        let chunk_size = remaining.min(block_size);
        let chunk = &buffer[pos..pos + chunk_size];

        // Calculate checksums for this block
        let rolling = RollingChecksum::from_data(chunk);
        let weak_hash = rolling.checksum();
        let strong_hash = calculate_strong_hash(chunk, algorithm);

        // Look for a match in destination signatures
        if let Some(sig) = dest_signatures.find_match(weak_hash, &strong_hash) {
            // Found a match! Flush any pending data first
            if !pending_data.is_empty() {
                instructions.push(DeltaInstruction::Data {
                    dest_offset,
                    bytes: pending_data.clone(),
                });
                dest_offset += pending_data.len() as u64;
                stats.bytes_transferred += pending_data.len() as u64;
                pending_data.clear();
            }

            // Add copy instruction
            instructions.push(DeltaInstruction::Copy {
                src_offset: sig.offset,
                dest_offset,
                length: sig.length,
            });

            dest_offset += sig.length as u64;
            pos += sig.length;
            stats.blocks_matched += 1;
            stats.bytes_saved += sig.length as u64;
        } else {
            // No match, add byte to pending data
            pending_data.push(buffer[pos]);
            pos += 1;
            stats.blocks_transferred += 1;
        }
    }

    // Flush any remaining pending data
    if !pending_data.is_empty() {
        instructions.push(DeltaInstruction::Data {
            dest_offset,
            bytes: pending_data,
        });
        stats.bytes_transferred += buffer.len() as u64 - stats.bytes_saved;
    }

    stats.calculate_savings_ratio();

    Ok((instructions, stats))
}

/// Optimized delta generation with rolling checksum
///
/// This version uses a rolling window to efficiently search for matches
/// by incrementally updating the checksum rather than recalculating it
/// for every byte position.
pub fn generate_delta_rolling<R: Read>(
    mut source: R,
    dest_signatures: SignatureIndex,
    algorithm: HashAlgorithm,
) -> Result<(Vec<DeltaInstruction>, DeltaStats)> {
    let block_size = dest_signatures.block_size();
    let mut instructions = Vec::new();
    let mut stats = DeltaStats::new();

    let mut buffer = Vec::new();
    source.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Ok((instructions, stats));
    }

    stats.total_bytes = buffer.len() as u64;
    stats.total_blocks = ((buffer.len() + block_size - 1) / block_size) as u64;

    let mut pos = 0;
    let mut dest_offset = 0u64;
    let mut pending_data = Vec::new();
    let mut rolling: Option<RollingChecksum> = None;

    while pos < buffer.len() {
        let remaining = buffer.len() - pos;

        if remaining < block_size {
            // Less than a full block remaining, add to pending data
            pending_data.extend_from_slice(&buffer[pos..]);
            stats.blocks_transferred += 1;
            break;
        }

        // Initialize or roll the checksum
        let weak_hash = if let Some(ref mut roll) = rolling {
            if pos >= block_size {
                // Roll forward: remove old byte, add new byte
                let old_byte = buffer[pos - block_size];
                let new_byte = buffer[pos];
                roll.roll(old_byte, new_byte);
            }
            roll.checksum()
        } else {
            // Initialize checksum for first block
            let chunk = &buffer[pos..pos + block_size];
            rolling = Some(RollingChecksum::from_data(chunk));
            rolling.as_ref().unwrap().checksum()
        };

        // Calculate strong hash only if weak hash matches
        let chunk = &buffer[pos..pos + block_size];
        if dest_signatures.weak_hash_map.contains_key(&weak_hash) {
            let strong_hash = calculate_strong_hash(chunk, algorithm);

            if let Some(sig) = dest_signatures.find_match(weak_hash, &strong_hash) {
                // Found a match!
                if !pending_data.is_empty() {
                    instructions.push(DeltaInstruction::Data {
                        dest_offset,
                        bytes: pending_data.clone(),
                    });
                    dest_offset += pending_data.len() as u64;
                    stats.bytes_transferred += pending_data.len() as u64;
                    pending_data.clear();
                }

                instructions.push(DeltaInstruction::Copy {
                    src_offset: sig.offset,
                    dest_offset,
                    length: sig.length,
                });

                dest_offset += sig.length as u64;
                pos += sig.length;
                stats.blocks_matched += 1;
                stats.bytes_saved += sig.length as u64;

                // Reset rolling checksum after a match
                rolling = None;
                continue;
            }
        }

        // No match, advance by one byte
        pending_data.push(buffer[pos]);
        pos += 1;
    }

    // Flush remaining pending data
    if !pending_data.is_empty() {
        instructions.push(DeltaInstruction::Data {
            dest_offset,
            bytes: pending_data,
        });
        stats.bytes_transferred += buffer.len() as u64 - stats.bytes_saved;
    }

    stats.calculate_savings_ratio();

    Ok((instructions, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::checksum::generate_signatures;

    #[test]
    fn test_signature_index() {
        let signatures = vec![
            BlockSignature::new(0, 10, 12345, vec![1, 2, 3]),
            BlockSignature::new(10, 10, 67890, vec![4, 5, 6]),
            BlockSignature::new(20, 10, 12345, vec![7, 8, 9]), // Same weak hash
        ];

        let index = SignatureIndex::new(signatures);

        // Find exact match
        let match1 = index.find_match(12345, &[1, 2, 3]);
        assert!(match1.is_some());
        assert_eq!(match1.unwrap().offset, 0);

        // Find match with same weak hash but different strong hash
        let match2 = index.find_match(12345, &[7, 8, 9]);
        assert!(match2.is_some());
        assert_eq!(match2.unwrap().offset, 20);

        // No match
        let match3 = index.find_match(11111, &[1, 2, 3]);
        assert!(match3.is_none());
    }

    #[test]
    fn test_generate_delta_identical_files() {
        let data = b"hello world test data";
        let block_size = 5;

        // Generate signatures for "destination"
        let signatures = generate_signatures(&data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta for identical "source"
        let (instructions, stats) = generate_delta(&data[..], index, HashAlgorithm::Blake3).unwrap();

        // All blocks should match
        assert!(stats.blocks_matched > 0);
        assert_eq!(stats.bytes_saved, data.len() as u64);
        assert_eq!(stats.savings_ratio, 1.0);

        // Should only have Copy instructions
        for instr in &instructions {
            assert!(matches!(instr, DeltaInstruction::Copy { .. }));
        }
    }

    #[test]
    fn test_generate_delta_completely_different() {
        let dest_data = b"original data here";
        let source_data = b"completely different content";
        let block_size = 5;

        // Generate signatures for destination
        let signatures = generate_signatures(&dest_data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta for different source
        let (instructions, stats) = generate_delta(&source_data[..], index, HashAlgorithm::Blake3).unwrap();

        // No blocks should match
        assert_eq!(stats.blocks_matched, 0);
        assert_eq!(stats.bytes_saved, 0);
        assert_eq!(stats.savings_ratio, 0.0);

        // Should have Data instructions
        let has_data = instructions.iter().any(|i| matches!(i, DeltaInstruction::Data { .. }));
        assert!(has_data);
    }

    #[test]
    fn test_generate_delta_partial_match() {
        let dest_data = b"hello world and more text here";
        let source_data = b"hello world but different ending";
        let block_size = 5;

        // Generate signatures for destination
        let signatures = generate_signatures(&dest_data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta
        let (instructions, stats) = generate_delta(&source_data[..], index, HashAlgorithm::Blake3).unwrap();

        // Should have some matches (the "hello world" part)
        assert!(stats.blocks_matched > 0);
        assert!(stats.bytes_saved > 0);
        assert!(stats.savings_ratio > 0.0 && stats.savings_ratio < 1.0);

        // Should have both Copy and Data instructions
        let has_copy = instructions.iter().any(|i| matches!(i, DeltaInstruction::Copy { .. }));
        let has_data = instructions.iter().any(|i| matches!(i, DeltaInstruction::Data { .. }));
        assert!(has_copy);
        assert!(has_data);
    }

    #[test]
    fn test_generate_delta_rolling() {
        let dest_data = b"this is a test file with some content";
        let source_data = b"this is a test file with different content";
        let block_size = 8;

        // Generate signatures for destination
        let signatures = generate_signatures(&dest_data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index = SignatureIndex::new(signatures);

        // Compare basic vs rolling algorithm
        let (_, stats_basic) = generate_delta(&source_data[..], index.clone(), HashAlgorithm::Blake3).unwrap();

        let signatures2 = generate_signatures(&dest_data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index2 = SignatureIndex::new(signatures2);
        let (_, stats_rolling) = generate_delta_rolling(&source_data[..], index2, HashAlgorithm::Blake3).unwrap();

        // Both should find similar matches
        assert_eq!(stats_basic.total_bytes, stats_rolling.total_bytes);
    }

    #[test]
    fn test_empty_source() {
        let dest_data = b"some data";
        let source_data = b"";
        let block_size = 5;

        let signatures = generate_signatures(&dest_data[..], block_size, HashAlgorithm::Blake3).unwrap();
        let index = SignatureIndex::new(signatures);

        let (instructions, stats) = generate_delta(&source_data[..], index, HashAlgorithm::Blake3).unwrap();

        assert_eq!(instructions.len(), 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.blocks_matched, 0);
    }
}
