/*!
 * Content-aware rename/move detection using Star Map chunk overlap.
 *
 * rsync's `--fuzzy` option finds basis files by filename similarity within
 * the same directory — primitive and unreliable. Orbit does MUCH better:
 * it uses the Star Map's global chunk index to find files at the destination
 * that share content chunks with source files, regardless of name or location.
 *
 * When a user renames `src/old_name.rs` to `src/new_name.rs` and syncs,
 * traditional tools transfer the entire file. Orbit's rename detector
 * recognizes that the destination already has an identical (or near-identical)
 * file and uses it as a delta basis — transferring near-zero bytes.
 *
 * # Algorithm
 *
 * 1. For each "new" source file (no exact path match at destination):
 *    a. Sample a subset of its chunk hashes (first, middle, last)
 *    b. Query Star Map: "which destination files contain these chunks?"
 *    c. For each candidate, compute the full chunk overlap ratio
 *    d. If overlap >= threshold (default 80%), use that file as delta basis
 *
 * 2. The algorithm is O(sample_size * files_per_chunk) per new file,
 *    which is typically very fast since most chunks appear in few files.
 */

use std::collections::HashMap;
use std::path::PathBuf;

use orbit_core_starmap::universe::UniverseMap;

/// Configuration for rename detection
#[derive(Debug, Clone)]
pub struct RenameDetectorConfig {
    /// Minimum fraction of chunks that must overlap to consider a rename.
    /// Default: 0.8 (80%). Range: 0.0–1.0.
    pub threshold: f64,

    /// Maximum number of candidates to evaluate per source file.
    /// Limits CPU cost when a popular chunk appears in many files.
    pub max_candidates: usize,
}

impl Default for RenameDetectorConfig {
    fn default() -> Self {
        Self {
            threshold: 0.8,
            max_candidates: 10,
        }
    }
}

/// A detected rename/move match between a source file and a destination file.
#[derive(Debug, Clone)]
pub struct RenameMatch {
    /// The source file that needs to be transferred
    pub source_path: PathBuf,
    /// The destination file to use as delta basis
    pub basis_path: PathBuf,
    /// Fraction of source chunks found in the basis file (0.0–1.0)
    pub overlap_ratio: f64,
    /// Number of chunks shared between source and basis
    pub shared_chunks: usize,
    /// Total chunks in the source file
    pub total_chunks: usize,
}

/// Detect renames/moves by finding destination files with high chunk overlap.
///
/// # Arguments
/// * `starmap` - Universe Map indexed with destination file chunks
/// * `source_chunks` - Map of source file path → list of chunk hashes
/// * `config` - Detection parameters (threshold, max candidates)
///
/// # Returns
/// A list of rename matches, sorted by overlap ratio (best first).
pub fn detect_renames(
    starmap: &UniverseMap,
    source_chunks: &HashMap<PathBuf, Vec<[u8; 32]>>,
    config: &RenameDetectorConfig,
) -> Vec<RenameMatch> {
    let mut matches = Vec::new();

    for (source_path, chunk_hashes) in source_chunks {
        if chunk_hashes.is_empty() {
            continue;
        }

        // Sample chunk hashes for initial candidate search.
        // Sampling first/middle/last is much cheaper than querying all chunks.
        let sample = sample_chunks(chunk_hashes);

        // Collect candidate file IDs from sampled chunks
        let mut candidate_counts: HashMap<u64, usize> = HashMap::new();
        for hash in &sample {
            for file_id in starmap.files_containing_chunk(hash) {
                *candidate_counts.entry(file_id).or_default() += 1;
            }
        }

        // Sort candidates by sample hit count (most hits = most likely match)
        let mut candidates: Vec<(u64, usize)> = candidate_counts.into_iter().collect();
        candidates.sort_by(|a, b| b.1.cmp(&a.1));
        candidates.truncate(config.max_candidates);

        // For top candidates, compute full overlap ratio
        for (file_id, _) in candidates {
            let dest_path = match starmap.get_file_path(file_id) {
                Some(p) => PathBuf::from(p),
                None => continue,
            };

            // Skip if this is the same path (not a rename)
            if dest_path == *source_path {
                continue;
            }

            let dest_chunks = starmap.chunks_for_file(file_id);
            let dest_chunk_set: std::collections::HashSet<[u8; 32]> =
                dest_chunks.into_iter().collect();

            let shared = chunk_hashes
                .iter()
                .filter(|h| dest_chunk_set.contains(*h))
                .count();

            let overlap = shared as f64 / chunk_hashes.len() as f64;

            if overlap >= config.threshold {
                matches.push(RenameMatch {
                    source_path: source_path.clone(),
                    basis_path: dest_path,
                    overlap_ratio: overlap,
                    shared_chunks: shared,
                    total_chunks: chunk_hashes.len(),
                });
                break; // Take the best match for this source file
            }
        }
    }

    // Sort by overlap ratio, best first
    matches.sort_by(|a, b| b.overlap_ratio.partial_cmp(&a.overlap_ratio).unwrap());
    matches
}

/// Sample a representative subset of chunk hashes from a file.
///
/// Takes first, middle, and last chunks for O(1) sample that covers
/// the file's beginning, middle, and end — catching renames where the
/// file is identical but also catching partial edits.
fn sample_chunks(chunks: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let len = chunks.len();
    if len <= 3 {
        return chunks.to_vec();
    }

    vec![chunks[0], chunks[len / 2], chunks[len - 1]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_core_starmap::universe::{Location, UniverseMap};

    fn setup_starmap_with_file(
        starmap: &mut UniverseMap,
        path: &str,
        chunk_hashes: &[[u8; 32]],
    ) -> u64 {
        let file_id = starmap.register_file(path);
        for (i, hash) in chunk_hashes.iter().enumerate() {
            starmap.add_chunk(hash, Location::new(file_id, i as u64 * 4096, 4096));
        }
        file_id
    }

    #[test]
    fn test_detect_exact_rename() {
        let mut starmap = UniverseMap::new();
        let chunks = [[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];

        // Destination has file at old path
        setup_starmap_with_file(&mut starmap, "/dest/old_name.rs", &chunks);

        // Source file at new path has identical chunks
        let mut source_chunks = HashMap::new();
        source_chunks.insert(PathBuf::from("/src/new_name.rs"), chunks.to_vec());

        let config = RenameDetectorConfig::default();
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].source_path, PathBuf::from("/src/new_name.rs"));
        assert_eq!(matches[0].basis_path, PathBuf::from("/dest/old_name.rs"));
        assert!((matches[0].overlap_ratio - 1.0).abs() < f64::EPSILON);
        assert_eq!(matches[0].shared_chunks, 5);
    }

    #[test]
    fn test_detect_partial_rename() {
        let mut starmap = UniverseMap::new();
        // Destination file has 5 chunks
        let dest_chunks = [[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];
        setup_starmap_with_file(&mut starmap, "/dest/old.rs", &dest_chunks);

        // Source file shares 4/5 chunks (80%) + 1 new chunk
        let mut source_chunks = HashMap::new();
        source_chunks.insert(
            PathBuf::from("/src/new.rs"),
            vec![[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0xFF; 32]],
        );

        let config = RenameDetectorConfig {
            threshold: 0.8,
            ..Default::default()
        };
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert_eq!(matches.len(), 1);
        assert!((matches[0].overlap_ratio - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_match_below_threshold() {
        let mut starmap = UniverseMap::new();
        let dest_chunks = [[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];
        setup_starmap_with_file(&mut starmap, "/dest/old.rs", &dest_chunks);

        // Source shares only 2/5 chunks (40%) — below threshold
        let mut source_chunks = HashMap::new();
        source_chunks.insert(
            PathBuf::from("/src/different.rs"),
            vec![[0x01; 32], [0x02; 32], [0xA0; 32], [0xB0; 32], [0xC0; 32]],
        );

        let config = RenameDetectorConfig::default();
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_sample_chunks() {
        let chunks: Vec<[u8; 32]> = (0..10).map(|i| [i; 32]).collect();
        let sample = sample_chunks(&chunks);

        assert_eq!(sample.len(), 3);
        assert_eq!(sample[0], [0; 32]); // first
        assert_eq!(sample[1], [5; 32]); // middle
        assert_eq!(sample[2], [9; 32]); // last
    }

    #[test]
    fn test_sample_small_file() {
        let chunks = vec![[0x01; 32], [0x02; 32]];
        let sample = sample_chunks(&chunks);
        assert_eq!(sample.len(), 2); // Returns all for small files
    }

    #[test]
    fn test_empty_source() {
        let starmap = UniverseMap::new();
        let source_chunks = HashMap::new();
        let matches = detect_renames(&starmap, &source_chunks, &RenameDetectorConfig::default());
        assert!(matches.is_empty());
    }

    #[test]
    fn test_detect_with_custom_threshold() {
        let mut starmap = UniverseMap::new();
        let dest_chunks = [[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];
        setup_starmap_with_file(&mut starmap, "/dest/old.rs", &dest_chunks);

        // Source shares 3/5 = 60% — below default 80% but above custom 50%
        let mut source_chunks = HashMap::new();
        source_chunks.insert(
            PathBuf::from("/src/new.rs"),
            vec![[0x01; 32], [0x02; 32], [0x03; 32], [0xF0; 32], [0xF1; 32]],
        );

        let config = RenameDetectorConfig {
            threshold: 0.5,
            ..Default::default()
        };
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert_eq!(matches.len(), 1);
        assert!((matches[0].overlap_ratio - 0.6).abs() < f64::EPSILON);
    }

    #[test]
    fn test_detect_same_path_not_rename() {
        let mut starmap = UniverseMap::new();
        let chunks = [[0x01; 32], [0x02; 32], [0x03; 32]];
        setup_starmap_with_file(&mut starmap, "/path/file.rs", &chunks);

        // Source at the SAME path — should not be detected as a rename
        let mut source_chunks = HashMap::new();
        source_chunks.insert(PathBuf::from("/path/file.rs"), chunks.to_vec());

        let config = RenameDetectorConfig::default();
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_detect_multiple_renames() {
        let mut starmap = UniverseMap::new();
        let chunks1 = [[0x01; 32], [0x02; 32], [0x03; 32]];
        let chunks2 = [[0x11; 32], [0x12; 32], [0x13; 32]];
        setup_starmap_with_file(&mut starmap, "/dest/old1.rs", &chunks1);
        setup_starmap_with_file(&mut starmap, "/dest/old2.rs", &chunks2);

        let mut source_chunks = HashMap::new();
        source_chunks.insert(PathBuf::from("/src/new1.rs"), chunks1.to_vec());
        source_chunks.insert(PathBuf::from("/src/new2.rs"), chunks2.to_vec());

        let config = RenameDetectorConfig::default();
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_detect_empty_chunks_in_source() {
        let starmap = UniverseMap::new();

        let mut source_chunks = HashMap::new();
        source_chunks.insert(PathBuf::from("/src/empty.rs"), vec![]);

        let config = RenameDetectorConfig::default();
        let matches = detect_renames(&starmap, &source_chunks, &config);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_rename_detector_config_default() {
        let config = RenameDetectorConfig::default();
        assert!((config.threshold - 0.8).abs() < f64::EPSILON);
        assert_eq!(config.max_candidates, 10);
    }
}
