//! V2 Verification Suite: Stage 2 (Logical Layer)
//!
//! Validates:
//! 1. SemanticRegistry correctly matches file extensions/paths.
//! 2. Priority assignment follows the Critical > Normal > Low hierarchy.

#[cfg(test)]
mod tests {
    use orbit_core_semantic::{Priority, SemanticRegistry, SyncStrategy};
    use std::path::Path;

    #[test]
    fn verify_semantic_classification() {
        let registry = SemanticRegistry::default();

        let scenarios = vec![
            // (Filename, Expected Priority, Expected Strategy)
            (
                "production.toml",
                Priority::Critical,
                SyncStrategy::AtomicReplace,
            ),
            (
                "docker-compose.yaml",
                Priority::Critical,
                SyncStrategy::AtomicReplace,
            ),
            ("database.wal", Priority::High, SyncStrategy::AppendOnly),
            (
                "pg_wal/000000010000000000000001",
                Priority::High,
                SyncStrategy::AppendOnly,
            ),
            (
                "source_code.rs",
                Priority::Normal,
                SyncStrategy::ContentDefined,
            ),
            (
                "backup_2025.iso",
                Priority::Low,
                SyncStrategy::ContentDefined,
            ),
            ("video.mp4", Priority::Low, SyncStrategy::ContentDefined),
        ];

        println!("\n🧪 Starting Semantic Classification Test...");

        for (path_str, expected_prio, expected_strat) in scenarios {
            let path = Path::new(path_str);
            let intent = registry.determine_intent(path, b"");

            println!(
                "   Checking {:<40} -> [{:?}] {:?}",
                path_str, intent.priority, intent.strategy
            );

            assert_eq!(
                intent.priority, expected_prio,
                "Priority mismatch for {}",
                path_str
            );
            assert_eq!(
                intent.strategy, expected_strat,
                "Strategy mismatch for {}",
                path_str
            );
        }

        println!("   ✅ PASS: All file types correctly classified.");
    }

    #[test]
    fn verify_priority_sorting_logic() {
        // This tests that Rust's default Ord implementation works as we expect
        // for our Priority enum (where 0 < 100).

        let p_crit = Priority::Critical; // 0
        let p_norm = Priority::Normal; // 50

        assert!(
            p_crit < p_norm,
            "Critical (0) should be 'less than' Normal (50) in value"
        );

        // When using a Max-Heap (BinaryHeap), .pop() returns the largest value.
        // If we want Critical to come out first, we will need to reverse the ordering
        // in the integration layer. This test confirms the raw values are correct.
    }
}
