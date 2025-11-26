/*!
 * Delta algorithm implementation - rsync-inspired block matching
 *
 * Implements the core delta detection algorithm:
 * 1. Generate block signatures for destination file
 * 2. Find matching blocks in source file using rolling checksum
 * 3. Generate delta instructions (Copy existing blocks or insert new data)
 */

use super::checksum::{calculate_strong_hash, RollingHash};
use super::{BlockSignature, DeltaInstruction, DeltaStats, HashAlgorithm, RollingHashAlgo};
use crate::error::Result;
use std::collections::HashMap;
use std::io::Read;

/// Index of block signatures for fast lookup
#[derive(Clone)]
pub struct SignatureIndex {
    /// Map from weak hash to list of signatures with that weak hash
    weak_hash_map: HashMap<u64, Vec<BlockSignature>>,

    /// Block size used for signatures
    block_size: usize,
}

impl SignatureIndex {
    /// Create a new signature index from a list of signatures
    pub fn new(signatures: Vec<BlockSignature>) -> Self {
        let mut weak_hash_map: HashMap<u64, Vec<BlockSignature>> = HashMap::new();
        let block_size = signatures.first().map(|s| s.length).unwrap_or(0);

        for sig in signatures {
            weak_hash_map.entry(sig.weak_hash).or_default().push(sig);
        }

        Self {
            weak_hash_map,
            block_size,
        }
    }

    /// Find a matching block signature for the given weak and strong hashes
    pub fn find_match(&self, weak_hash: u64, strong_hash: &[u8]) -> Option<&BlockSignature> {
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
    hash_algorithm: HashAlgorithm,
    rolling_algo: RollingHashAlgo,
) -> Result<(Vec<DeltaInstruction>, DeltaStats)> {
    let block_size = dest_signatures.block_size();
    let mut instructions = Vec::new();
    let mut stats = DeltaStats::new();

    let mut buffer = Vec::new();
    source.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Ok((instructions, stats));
    }

    if block_size == 0 {
        stats.total_bytes = buffer.len() as u64;
        instructions.push(DeltaInstruction::Data {
            dest_offset: 0,
            bytes: buffer,
        });
        stats.bytes_transferred = stats.total_bytes;
        stats.calculate_savings_ratio();
        return Ok((instructions, stats));
    }

    stats.total_bytes = buffer.len() as u64;
    stats.total_blocks = (buffer.len() + block_size - 1) as u64 / block_size as u64;

    let mut pos = 0;
    let mut dest_offset = 0u64;
    // Track unmatched bytes by slice range to avoid per-byte buffering.
    let mut pending_start = 0usize;

    while pos < buffer.len() {
        // Try to find a matching block starting at current position
        let remaining = buffer.len() - pos;
        let chunk_size = remaining.min(block_size);
        let chunk = &buffer[pos..pos + chunk_size];

        // Calculate checksums for this block
        let rolling = RollingHash::from_data(chunk, rolling_algo);
        let weak_hash = rolling.hash();
        let strong_hash = calculate_strong_hash(chunk, hash_algorithm);

        // Look for a match in destination signatures
        if let Some(sig) = dest_signatures.find_match(weak_hash, &strong_hash) {
            // Found a match! Flush any pending data first
            if pos > pending_start {
                let bytes = buffer[pending_start..pos].to_vec();
                instructions.push(DeltaInstruction::Data { dest_offset, bytes });
                dest_offset += (pos - pending_start) as u64;
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
            pending_start = pos;
        } else {
            // No match, advance window; copy will be emitted when we find a match
            pos += 1;
        }
    }

    // Flush any remaining pending data
    if pending_start < buffer.len() {
        let tail = buffer[pending_start..].to_vec();
        instructions.push(DeltaInstruction::Data {
            dest_offset,
            bytes: tail,
        });
    }

    stats.blocks_transferred = stats.total_blocks.saturating_sub(stats.blocks_matched);
    stats.bytes_transferred = stats.total_bytes.saturating_sub(stats.bytes_saved);
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
    hash_algorithm: HashAlgorithm,
    rolling_algo: RollingHashAlgo,
) -> Result<(Vec<DeltaInstruction>, DeltaStats)> {
    let block_size = dest_signatures.block_size();
    let mut instructions = Vec::new();
    let mut stats = DeltaStats::new();

    let mut buffer = Vec::new();
    source.read_to_end(&mut buffer)?;

    if buffer.is_empty() {
        return Ok((instructions, stats));
    }

    if block_size == 0 {
        stats.total_bytes = buffer.len() as u64;
        instructions.push(DeltaInstruction::Data {
            dest_offset: 0,
            bytes: buffer,
        });
        stats.bytes_transferred = stats.total_bytes;
        stats.calculate_savings_ratio();
        return Ok((instructions, stats));
    }

    stats.total_bytes = buffer.len() as u64;
    stats.total_blocks = buffer.len().div_ceil(block_size) as u64;

    let mut pos = 0;
    let mut dest_offset = 0u64;
    // Track the start of the current non-matching span to emit in one shot.
    let mut pending_start = 0usize;
    let mut rolling: Option<RollingHash> = None;

    while pos + block_size <= buffer.len() {
        // Initialize or roll the checksum
        let weak_hash = if let Some(ref mut roll) = rolling {
            // Roll forward: remove the byte leaving the window, add the new byte.
            // Safety: loop guard ensures pos > 0 has a valid previous byte and
            // pos + block_size - 1 is in-bounds for the new trailing byte.
            let old_byte = if pos > 0 { buffer[pos - 1] } else { buffer[0] };
            let new_byte = buffer[pos + block_size - 1];
            roll.roll(old_byte, new_byte);
            roll.hash()
        } else {
            // Initialize checksum for first block
            let chunk = &buffer[pos..pos + block_size];
            rolling = Some(RollingHash::from_data(chunk, rolling_algo));
            rolling.as_ref().unwrap().hash()
        };

        // Calculate strong hash only if weak hash matches
        let chunk = &buffer[pos..pos + block_size];
        if dest_signatures.weak_hash_map.contains_key(&weak_hash) {
            let strong_hash = calculate_strong_hash(chunk, hash_algorithm);

            if let Some(sig) = dest_signatures.find_match(weak_hash, &strong_hash) {
                // Found a match! Flush any pending data in a single memcpy.
                if pos > pending_start {
                    let bytes = buffer[pending_start..pos].to_vec();
                    instructions.push(DeltaInstruction::Data { dest_offset, bytes });
                    dest_offset += (pos - pending_start) as u64;
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
                pending_start = pos;

                // Reset rolling checksum after a match
                rolling = None;
                continue;
            }
        }

        // No match; just advance and keep growing the pending span.
        pos += 1;
    }

    // Flush remaining pending data
    if pending_start < buffer.len() {
        let tail = buffer[pending_start..].to_vec();
        instructions.push(DeltaInstruction::Data {
            dest_offset,
            bytes: tail,
        });
    }

    stats.blocks_transferred = stats.total_blocks.saturating_sub(stats.blocks_matched);
    stats.bytes_transferred = stats.total_bytes.saturating_sub(stats.bytes_saved);
    stats.calculate_savings_ratio();

    Ok((instructions, stats))
}

#[cfg(test)]
mod tests {
    use super::super::checksum::{calculate_strong_hash, generate_signatures, RollingHash};
    use super::*;

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
        let signatures = generate_signatures(
            &data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta for identical "source"
        let (instructions, stats) = generate_delta(
            &data[..],
            index,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

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
        let signatures = generate_signatures(
            &dest_data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta for different source
        let (instructions, stats) = generate_delta(
            &source_data[..],
            index,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

        // No blocks should match
        assert_eq!(stats.blocks_matched, 0);
        assert_eq!(stats.bytes_saved, 0);
        assert_eq!(stats.savings_ratio, 0.0);

        // Should have Data instructions
        let has_data = instructions
            .iter()
            .any(|i| matches!(i, DeltaInstruction::Data { .. }));
        assert!(has_data);
    }

    #[test]
    fn test_generate_delta_partial_match() {
        let dest_data = b"hello world and more text here";
        let source_data = b"hello world but different ending";
        let block_size = 5;

        // Generate signatures for destination
        let signatures = generate_signatures(
            &dest_data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index = SignatureIndex::new(signatures);

        // Generate delta
        let (instructions, stats) = generate_delta(
            &source_data[..],
            index,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

        // Should have some matches (the "hello world" part)
        assert!(stats.blocks_matched > 0);
        assert!(stats.bytes_saved > 0);
        assert!(stats.savings_ratio > 0.0 && stats.savings_ratio < 1.0);

        // Should have both Copy and Data instructions
        let has_copy = instructions
            .iter()
            .any(|i| matches!(i, DeltaInstruction::Copy { .. }));
        let has_data = instructions
            .iter()
            .any(|i| matches!(i, DeltaInstruction::Data { .. }));
        assert!(has_copy);
        assert!(has_data);
    }

    #[test]
    fn test_generate_delta_rolling() {
        let dest_data = b"this is a test file with some content";
        let source_data = b"this is a test file with different content";
        let block_size = 8;

        // Generate signatures for destination
        let signatures = generate_signatures(
            &dest_data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index = SignatureIndex::new(signatures);

        // Compare basic vs rolling algorithm
        let (_, stats_basic) = generate_delta(
            &source_data[..],
            index.clone(),
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

        let signatures2 = generate_signatures(
            &dest_data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index2 = SignatureIndex::new(signatures2);
        let (_, stats_rolling) = generate_delta_rolling(
            &source_data[..],
            index2,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

        // Both should find similar matches
        assert_eq!(stats_basic.total_bytes, stats_rolling.total_bytes);
    }

    #[test]
    fn test_delta_generation_gap_logic() {
        // Source contains a single matching window in the middle.
        let data = b"AAABBBCCC";
        let sig = BlockSignature::new(
            100,
            3,
            RollingHash::from_data(b"BBB", super::super::RollingHashAlgo::Gear64).hash(),
            calculate_strong_hash(b"BBB", HashAlgorithm::Blake3),
        );
        let index = SignatureIndex::new(vec![sig]);

        let (insts, _) = generate_delta_rolling(
            &data[..],
            index,
            HashAlgorithm::Blake3,
            super::super::RollingHashAlgo::Gear64,
        )
        .unwrap();

        assert_eq!(insts.len(), 3);

        match &insts[0] {
            DeltaInstruction::Data { dest_offset, bytes } => {
                assert_eq!(*dest_offset, 0);
                assert_eq!(bytes, b"AAA");
            }
            _ => panic!("Expected Data instruction first"),
        }

        match &insts[1] {
            DeltaInstruction::Copy {
                src_offset,
                dest_offset,
                length,
            } => {
                assert_eq!(*src_offset, 100);
                assert_eq!(*dest_offset, 3);
                assert_eq!(*length, 3);
            }
            _ => panic!("Expected Copy instruction second"),
        }

        match &insts[2] {
            DeltaInstruction::Data { dest_offset, bytes } => {
                assert_eq!(*dest_offset, 6);
                assert_eq!(bytes, b"CCC");
            }
            _ => panic!("Expected Data instruction last"),
        }
    }

    #[test]
    fn test_empty_source() {
        let dest_data = b"some data";
        let source_data = b"";
        let block_size = 5;

        let signatures = generate_signatures(
            &dest_data[..],
            block_size,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();
        let index = SignatureIndex::new(signatures);

        let (instructions, stats) = generate_delta(
            &source_data[..],
            index,
            HashAlgorithm::Blake3,
            RollingHashAlgo::Gear64,
        )
        .unwrap();

        assert_eq!(instructions.len(), 0);
        assert_eq!(stats.total_bytes, 0);
        assert_eq!(stats.blocks_matched, 0);
    }
}
