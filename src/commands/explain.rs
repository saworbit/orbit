/*!
 * Orbit Explain Command
 *
 * Prints a human-readable explanation of what a transfer would do
 * given the resolved configuration. Helps users understand complex
 * flag combinations before committing to a transfer.
 */

use crate::cli_style::{section_header, Icons, Theme};
use crate::config::{CompressionType, CopyConfig, CopyMode, ErrorMode};

fn mode_description(copy_mode: CopyMode) -> &'static str {
    match copy_mode {
        CopyMode::Copy => "Copy — transfer all files unconditionally",
        CopyMode::Sync => "Sync — only transfer new or changed files",
        CopyMode::Update => "Update — only transfer files newer than destination",
        CopyMode::Mirror => "Mirror — exact replica (will DELETE extras at destination)",
    }
}

fn retry_description(config: &CopyConfig) -> Option<String> {
    if config.retry_attempts == 0 {
        return None;
    }

    let backoff = if config.exponential_backoff {
        " with exponential backoff"
    } else {
        ""
    };

    Some(format!(
        "Up to {} attempts, {}s delay{}",
        config.retry_attempts, config.retry_delay_secs, backoff
    ))
}

fn compression_description(compression: CompressionType) -> String {
    match compression {
        CompressionType::None => "None (raw transfer)".to_string(),
        CompressionType::Lz4 => "LZ4 (fast, ~2× throughput on slow links)".to_string(),
        CompressionType::Zstd { level } => {
            let quality = match level {
                1..=3 => "fast",
                4..=9 => "balanced",
                10..=19 => "high ratio, slower",
                _ => "custom",
            };
            format!("Zstd level {} ({})", level, quality)
        }
    }
}

fn worker_description(parallel: usize, is_remote: bool) -> String {
    if parallel == 0 {
        if is_remote {
            "Auto (256 for network)".to_string()
        } else {
            "Auto (CPU count for local)".to_string()
        }
    } else {
        format!("{} parallel workers", parallel)
    }
}

fn advanced_lines(config: &CopyConfig) -> Vec<String> {
    let mut lines = Vec::new();

    if config.detect_renames {
        lines.push(format!(
            "  {} {} Detecting renamed files (threshold: {:.0}%)",
            Icons::BULLET,
            Theme::muted("Renames:"),
            config.rename_threshold * 100.0
        ));
    }
    if !config.link_dest.is_empty() {
        for ld in &config.link_dest {
            lines.push(format!(
                "  {} {} Hardlinking unchanged files against {}",
                Icons::BULLET,
                Theme::muted("Link-dest:"),
                ld.display()
            ));
        }
    }
    if config.preserve_hardlinks {
        lines.push(format!(
            "  {} {} Preserving hardlink groups",
            Icons::BULLET,
            Theme::muted("Hardlinks:"),
        ));
    }
    if config.inplace {
        let safety = format!("{:?}", config.inplace_safety).to_lowercase();
        lines.push(format!(
            "  {} {} Modifying files in-place (safety: {})",
            Icons::BULLET,
            Theme::muted("In-place:"),
            safety
        ));
    }
    if config.generate_manifest {
        lines.push(format!(
            "  {} {} Generating transfer manifests",
            Icons::BULLET,
            Theme::muted("Manifests:"),
        ));
    }
    if config.write_batch.is_some() {
        lines.push(format!(
            "  {} {} Recording batch journal for replay",
            Icons::BULLET,
            Theme::muted("Batch:"),
        ));
    }

    lines
}

fn active_conditions(config: &CopyConfig) -> Vec<&'static str> {
    let mut conditions = Vec::new();
    if config.no_clobber {
        conditions.push("Won't overwrite existing files");
    }
    if config.if_size_differ {
        conditions.push("Only if sizes differ");
    }
    if config.if_source_newer {
        conditions.push("Only if source is newer");
    }
    if config.ignore_existing {
        conditions.push("Skipping files that already exist");
    }
    if config.flatten {
        conditions.push("Flattening directory hierarchy");
    }
    conditions
}

/// Print a human-readable explanation of what the given config will do.
pub fn explain_transfer(source: &str, dest: &str, config: &CopyConfig, is_remote: bool) {
    println!();
    println!("  {} {}", Icons::ORBIT, Theme::header("Transfer Plan"));
    println!();

    println!(
        "  {} {} {}",
        Icons::ARROW_RIGHT,
        Theme::muted("From:"),
        Theme::value(source)
    );
    println!(
        "  {} {} {}",
        Icons::ARROW_RIGHT,
        Theme::muted("  To:"),
        Theme::value(dest)
    );
    println!();

    let mode_desc = mode_description(config.copy_mode);
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Mode:"),
        if matches!(config.copy_mode, CopyMode::Mirror) {
            Theme::warning(mode_desc)
        } else {
            Theme::value(mode_desc)
        }
    );

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Scope:"),
        if config.recursive {
            Theme::value("Recursive (all subdirectories)")
        } else {
            Theme::value("Single file")
        }
    );

    if config.preserve_metadata {
        let detail = if let Some(ref flags) = config.preserve_flags {
            format!("Preserving: {}", flags)
        } else {
            "Preserving timestamps & permissions".to_string()
        };
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Metadata:"),
            Theme::value(detail)
        );
    }

    println!();
    section_header(&format!("{} Safety", Icons::SHIELD));
    println!();

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Integrity:"),
        if config.verify_checksum {
            Theme::success("Checksum verification enabled")
        } else {
            Theme::warning("Checksum verification DISABLED")
        }
    );

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Resume:"),
        if config.resume_enabled {
            Theme::success("Can resume if interrupted")
        } else {
            Theme::muted("No resume (restart from beginning if interrupted)")
        }
    );

    if let Some(retries) = retry_description(config) {
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Retries:"),
            Theme::value(retries)
        );
    }

    let error_desc = match config.error_mode {
        ErrorMode::Abort => "Stop on first error",
        ErrorMode::Skip => "Skip failed files, continue with rest",
        ErrorMode::Partial => "Keep partial files for resume",
    };
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("On error:"),
        Theme::value(error_desc)
    );

    println!();
    section_header(&format!("{} Performance", Icons::ROCKET));
    println!();

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Compression:"),
        Theme::value(compression_description(config.compression))
    );

    if config.use_zero_copy {
        println!(
            "  {} {} {}",
            Icons::LIGHTNING,
            Theme::muted("Zero-copy:"),
            Theme::success("Enabled (kernel-level transfer)")
        );
    }

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Parallelism:"),
        Theme::value(worker_description(config.parallel, is_remote))
    );

    if config.max_bandwidth > 0 {
        let bw_mb = config.max_bandwidth / (1024 * 1024);
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Bandwidth:"),
            Theme::warning(format!("Throttled to {} MB/s", bw_mb))
        );
    }

    if !config.exclude_patterns.is_empty() || !config.include_patterns.is_empty() {
        println!();
        section_header(&format!("{} Filters", Icons::GEAR));
        println!();
        for pat in &config.include_patterns {
            println!(
                "  {} {} {}",
                Theme::success("+"),
                Theme::muted("Include:"),
                Theme::value(pat)
            );
        }
        for pat in &config.exclude_patterns {
            println!(
                "  {} {} {}",
                Theme::error("−"),
                Theme::muted("Exclude:"),
                Theme::value(pat)
            );
        }
    }

    let advanced = advanced_lines(config);
    let has_advanced = !advanced.is_empty();
    if has_advanced {
        println!();
        section_header(&format!("{} Advanced", Icons::GEAR));
        println!();
        for line in &advanced {
            println!("{}", line);
        }
    }

    let conditions = active_conditions(config);
    if !conditions.is_empty() {
        if !has_advanced {
            println!();
        }
        println!();
        section_header(&format!("{} Conditions", Icons::INFO));
        println!();
        for cond in &conditions {
            println!("  {} {}", Icons::BULLET, Theme::value(cond));
        }
    }

    if config.dry_run {
        println!();
        println!(
            "  {} {}",
            Theme::warning(Icons::WARNING),
            Theme::warning("DRY RUN — no files will be modified")
        );
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InplaceSafety;
    use std::path::PathBuf;

    #[test]
    fn test_mode_description_covers_all_transfer_modes() {
        assert_eq!(
            mode_description(CopyMode::Copy),
            "Copy — transfer all files unconditionally"
        );
        assert_eq!(
            mode_description(CopyMode::Sync),
            "Sync — only transfer new or changed files"
        );
        assert_eq!(
            mode_description(CopyMode::Update),
            "Update — only transfer files newer than destination"
        );
        assert!(mode_description(CopyMode::Mirror).contains("DELETE"));
    }

    #[test]
    fn test_retry_description_handles_disabled_and_backoff() {
        let mut config = CopyConfig {
            retry_attempts: 0,
            ..CopyConfig::default()
        };
        assert!(retry_description(&config).is_none());

        config.retry_attempts = 4;
        config.retry_delay_secs = 9;
        config.exponential_backoff = true;
        assert_eq!(
            retry_description(&config).as_deref(),
            Some("Up to 4 attempts, 9s delay with exponential backoff")
        );
    }

    #[test]
    fn test_compression_description_classifies_levels() {
        assert_eq!(
            compression_description(CompressionType::None),
            "None (raw transfer)"
        );
        assert!(compression_description(CompressionType::Lz4).contains("LZ4"));
        assert_eq!(
            compression_description(CompressionType::Zstd { level: 3 }),
            "Zstd level 3 (fast)"
        );
        assert_eq!(
            compression_description(CompressionType::Zstd { level: 9 }),
            "Zstd level 9 (balanced)"
        );
        assert_eq!(
            compression_description(CompressionType::Zstd { level: 19 }),
            "Zstd level 19 (high ratio, slower)"
        );
        assert_eq!(
            compression_description(CompressionType::Zstd { level: 42 }),
            "Zstd level 42 (custom)"
        );
    }

    #[test]
    fn test_worker_description_switches_on_remote_and_parallelism() {
        assert_eq!(worker_description(0, false), "Auto (CPU count for local)");
        assert_eq!(worker_description(0, true), "Auto (256 for network)");
        assert_eq!(worker_description(12, false), "12 parallel workers");
    }

    #[test]
    fn test_advanced_lines_and_conditions_report_enabled_features() {
        let config = CopyConfig {
            detect_renames: true,
            rename_threshold: 0.85,
            link_dest: vec![PathBuf::from("/snapshots/latest")],
            preserve_hardlinks: true,
            inplace: true,
            inplace_safety: InplaceSafety::Journaled,
            generate_manifest: true,
            write_batch: Some(PathBuf::from("batch.orbit")),
            no_clobber: true,
            if_size_differ: true,
            if_source_newer: true,
            ignore_existing: true,
            flatten: true,
            ..CopyConfig::default()
        };

        let advanced = advanced_lines(&config).join("\n");
        assert!(advanced.contains("Detecting renamed files"));
        assert!(advanced.contains("Hardlinking unchanged files against"));
        assert!(advanced.contains("Preserving hardlink groups"));
        assert!(advanced.contains("safety: journaled"));
        assert!(advanced.contains("Generating transfer manifests"));
        assert!(advanced.contains("Recording batch journal"));

        assert_eq!(
            active_conditions(&config),
            vec![
                "Won't overwrite existing files",
                "Only if sizes differ",
                "Only if source is newer",
                "Skipping files that already exist",
                "Flattening directory hierarchy",
            ]
        );
    }
}
