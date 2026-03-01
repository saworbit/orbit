/*!
 * Reference directory hardlinking for incremental backups (link-dest++).
 *
 * rsync's `--link-dest=DIR` creates hardlinks to unchanged files from a
 * reference directory, enabling space-efficient incremental backups.
 * However, rsync's approach is all-or-nothing: if a file is byte-identical,
 * it's hardlinked; if even one byte differs, the whole file is copied.
 *
 * Orbit's link-dest++ improves on this with chunk-level granularity:
 * - **Full match**: All chunks exist in reference → hardlink (same as rsync)
 * - **Partial match**: Some chunks match → use reference as delta basis,
 *   transferring only the changed chunks (rsync can't do this)
 * - **No match**: No chunk overlap → full transfer
 *
 * This means that even when a file has been partially modified since the
 * reference backup, Orbit only transfers the changed portions.
 */

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Decision on how to handle a file given reference directory content.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkDecision {
    /// All chunks exist in the reference file — create a hardlink.
    /// No data transfer needed. Path is the reference file to link from.
    Hardlink(PathBuf),

    /// Partial chunk overlap — use reference as delta basis.
    /// Transfer only non-matching chunks. Path is the reference basis file.
    DeltaBasis {
        reference: PathBuf,
        overlap: f64,
    },

    /// No useful match in any reference directory — full transfer.
    FullTransfer,
}

/// Resolves files against reference directories to determine the most
/// efficient transfer strategy for each file.
pub struct LinkDestResolver {
    /// Reference directories to check (in priority order)
    reference_dirs: Vec<PathBuf>,

    /// Minimum overlap ratio to use a reference as delta basis.
    /// Below this threshold, a full transfer is cheaper than computing deltas.
    delta_threshold: f64,
}

impl LinkDestResolver {
    /// Create a new resolver with the given reference directories.
    ///
    /// Directories are checked in order; the first match wins.
    pub fn new(reference_dirs: Vec<PathBuf>) -> Self {
        Self {
            reference_dirs,
            delta_threshold: 0.3, // 30% overlap = worth using as delta basis
        }
    }

    /// Set the minimum overlap ratio for delta basis usage.
    pub fn with_delta_threshold(mut self, threshold: f64) -> Self {
        self.delta_threshold = threshold;
        self
    }

    /// Resolve a source file against reference directories.
    ///
    /// Checks each reference directory for a file at the same relative path.
    /// If found, compares chunk hashes to determine the best strategy.
    ///
    /// # Arguments
    /// * `relative_path` - Path relative to the source/destination root
    /// * `source_chunk_hashes` - BLAKE3 hashes of the source file's chunks
    /// * `reference_chunk_hashes_fn` - Function to get chunk hashes for a reference file
    pub fn resolve<F>(
        &self,
        relative_path: &Path,
        source_chunk_hashes: &[[u8; 32]],
        reference_chunk_hashes_fn: F,
    ) -> LinkDecision
    where
        F: Fn(&Path) -> Option<Vec<[u8; 32]>>,
    {
        if source_chunk_hashes.is_empty() {
            return LinkDecision::FullTransfer;
        }

        for ref_dir in &self.reference_dirs {
            let ref_file = ref_dir.join(relative_path);
            if !ref_file.exists() {
                continue;
            }

            // Get chunk hashes for the reference file
            let ref_hashes = match reference_chunk_hashes_fn(&ref_file) {
                Some(h) => h,
                None => continue,
            };

            // Calculate overlap
            let ref_set: HashSet<[u8; 32]> = ref_hashes.iter().copied().collect();
            let shared = source_chunk_hashes
                .iter()
                .filter(|h| ref_set.contains(*h))
                .count();
            let overlap = shared as f64 / source_chunk_hashes.len() as f64;

            if (overlap - 1.0).abs() < f64::EPSILON && ref_hashes.len() == source_chunk_hashes.len()
            {
                // Perfect match — all chunks identical and same count
                return LinkDecision::Hardlink(ref_file);
            }

            if overlap >= self.delta_threshold {
                return LinkDecision::DeltaBasis {
                    reference: ref_file,
                    overlap,
                };
            }
        }

        LinkDecision::FullTransfer
    }

    /// Quick check: does any reference directory have a file at this path
    /// with the exact same size and modification time?
    ///
    /// This is a cheap pre-filter before the more expensive chunk comparison.
    pub fn quick_match(&self, relative_path: &Path, source_size: u64, source_mtime: std::time::SystemTime) -> Option<PathBuf> {
        for ref_dir in &self.reference_dirs {
            let ref_file = ref_dir.join(relative_path);
            if let Ok(meta) = std::fs::metadata(&ref_file) {
                if meta.len() == source_size {
                    if let Ok(mtime) = meta.modified() {
                        if mtime == source_mtime {
                            return Some(ref_file);
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_full_match_hardlink() {
        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("file.txt"), b"data").unwrap();

        let chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32]];
        let ref_chunks = chunks.clone();

        let resolver = LinkDestResolver::new(vec![ref_dir]);

        let decision = resolver.resolve(
            Path::new("file.txt"),
            &chunks,
            |_path| Some(ref_chunks.clone()),
        );

        assert!(matches!(decision, LinkDecision::Hardlink(_)));
    }

    #[test]
    fn test_partial_match_delta_basis() {
        let source_chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];
        // Reference shares 3/5 = 60%
        let ref_chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32], [0xAA; 32], [0xBB; 32]];

        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("file.txt"), b"placeholder").unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);

        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(ref_chunks.clone()),
        );

        match decision {
            LinkDecision::DeltaBasis { overlap, .. } => {
                assert!((overlap - 0.6).abs() < f64::EPSILON);
            }
            other => panic!("Expected DeltaBasis, got {:?}", other),
        }
    }

    #[test]
    fn test_no_match_full_transfer() {
        let source_chunks = vec![[0x01; 32], [0x02; 32]];
        let ref_chunks = vec![[0xAA; 32], [0xBB; 32]]; // No overlap

        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("file.txt"), b"different").unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);

        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(ref_chunks.clone()),
        );

        assert_eq!(decision, LinkDecision::FullTransfer);
    }

    #[test]
    fn test_no_reference_file() {
        let source_chunks = vec![[0x01; 32]];

        // Reference dir exists but doesn't contain the file
        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);

        let decision = resolver.resolve(
            Path::new("missing.txt"),
            &source_chunks,
            |_| None,
        );

        assert_eq!(decision, LinkDecision::FullTransfer);
    }

    #[test]
    fn test_multiple_reference_dirs() {
        let source_chunks = vec![[0x01; 32], [0x02; 32]];

        let dir = tempdir().unwrap();
        let ref1 = dir.path().join("ref1");
        let ref2 = dir.path().join("ref2");
        std::fs::create_dir_all(&ref1).unwrap();
        std::fs::create_dir_all(&ref2).unwrap();
        // Only ref2 has the file
        std::fs::write(ref2.join("file.txt"), b"data").unwrap();

        let resolver = LinkDestResolver::new(vec![ref1, ref2]);

        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(source_chunks.clone()), // Perfect match
        );

        assert!(matches!(decision, LinkDecision::Hardlink(_)));
    }

    #[test]
    fn test_empty_chunks() {
        let resolver = LinkDestResolver::new(vec![PathBuf::from("/ref")]);
        let decision = resolver.resolve(Path::new("empty.txt"), &[], |_| None);
        assert_eq!(decision, LinkDecision::FullTransfer);
    }

    #[test]
    fn test_quick_match() {
        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();

        let ref_file = ref_dir.join("test.txt");
        std::fs::write(&ref_file, b"test content").unwrap();

        let meta = std::fs::metadata(&ref_file).unwrap();
        let mtime = meta.modified().unwrap();
        let size = meta.len();

        let resolver = LinkDestResolver::new(vec![ref_dir]);
        let result = resolver.quick_match(Path::new("test.txt"), size, mtime);

        assert!(result.is_some());
    }

    #[test]
    fn test_quick_match_size_mismatch() {
        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();

        let ref_file = ref_dir.join("test.txt");
        std::fs::write(&ref_file, b"test content").unwrap();

        let meta = std::fs::metadata(&ref_file).unwrap();
        let mtime = meta.modified().unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);
        // Source has different size
        let result = resolver.quick_match(Path::new("test.txt"), 999, mtime);
        assert!(result.is_none());
    }

    #[test]
    fn test_quick_match_no_file() {
        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);
        let result = resolver.quick_match(
            Path::new("nonexistent.txt"),
            100,
            std::time::SystemTime::now(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_custom_delta_threshold() {
        let source_chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];
        // Reference shares 1/5 = 20%
        let ref_chunks = vec![[0x01; 32], [0xAA; 32], [0xBB; 32], [0xCC; 32], [0xDD; 32]];

        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("file.txt"), b"data").unwrap();

        // Default threshold (0.3) — 20% is below it
        let resolver = LinkDestResolver::new(vec![ref_dir.clone()]);
        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(ref_chunks.clone()),
        );
        assert_eq!(decision, LinkDecision::FullTransfer);

        // Custom threshold at 0.1 — 20% is above it
        let resolver = LinkDestResolver::new(vec![ref_dir]).with_delta_threshold(0.1);
        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(ref_chunks.clone()),
        );
        match decision {
            LinkDecision::DeltaBasis { overlap, .. } => {
                assert!((overlap - 0.2).abs() < f64::EPSILON);
            }
            other => panic!("Expected DeltaBasis, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_with_different_chunk_counts() {
        // Source has 3 chunks, ref has 5 — even if all 3 source chunks exist in ref,
        // it's not a perfect match (different chunk count).
        let source_chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32]];
        let ref_chunks = vec![[0x01; 32], [0x02; 32], [0x03; 32], [0x04; 32], [0x05; 32]];

        let dir = tempdir().unwrap();
        let ref_dir = dir.path().join("ref");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("file.txt"), b"data").unwrap();

        let resolver = LinkDestResolver::new(vec![ref_dir]);
        let decision = resolver.resolve(
            Path::new("file.txt"),
            &source_chunks,
            |_| Some(ref_chunks.clone()),
        );

        // All source chunks match (100%) but different count, so DeltaBasis not Hardlink
        match decision {
            LinkDecision::DeltaBasis { overlap, .. } => {
                assert!((overlap - 1.0).abs() < f64::EPSILON);
            }
            other => panic!("Expected DeltaBasis (not Hardlink due to different chunk count), got {:?}", other),
        }
    }

    #[test]
    fn test_empty_reference_dirs() {
        let resolver = LinkDestResolver::new(vec![]);
        let decision = resolver.resolve(
            Path::new("file.txt"),
            &[[0x01; 32]],
            |_| None,
        );
        assert_eq!(decision, LinkDecision::FullTransfer);
    }
}
