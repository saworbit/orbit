/*!
 * Orbit Explain Command
 *
 * Prints a human-readable explanation of what a transfer would do
 * given the resolved configuration. Helps users understand complex
 * flag combinations before committing to a transfer.
 */

use crate::cli_style::{section_header, Icons, Theme};
use crate::config::{CompressionType, CopyConfig, CopyMode, ErrorMode};

/// Print a human-readable explanation of what the given config will do.
pub fn explain_transfer(source: &str, dest: &str, config: &CopyConfig, is_remote: bool) {
    println!();
    println!("  {} {}", Icons::ORBIT, Theme::header("Transfer Plan"));
    println!();

    // Source & Destination
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

    // Mode
    let mode_desc = match config.copy_mode {
        CopyMode::Copy => "Copy — transfer all files unconditionally",
        CopyMode::Sync => "Sync — only transfer new or changed files",
        CopyMode::Update => "Update — only transfer files newer than destination",
        CopyMode::Mirror => "Mirror — exact replica (will DELETE extras at destination)",
    };
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

    // Recursive
    if config.recursive {
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Scope:"),
            Theme::value("Recursive (all subdirectories)")
        );
    } else {
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Scope:"),
            Theme::value("Single file")
        );
    }

    // Metadata
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

    // Checksum
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

    // Resume
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

    // Retries
    if config.retry_attempts > 0 {
        let backoff = if config.exponential_backoff {
            " with exponential backoff"
        } else {
            ""
        };
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Retries:"),
            Theme::value(format!(
                "Up to {} attempts, {}s delay{}",
                config.retry_attempts, config.retry_delay_secs, backoff
            ))
        );
    }

    // Error mode
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

    // Compression
    let comp_desc = match config.compression {
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
    };
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Compression:"),
        Theme::value(comp_desc)
    );

    // Zero-copy
    if config.use_zero_copy {
        println!(
            "  {} {} {}",
            Icons::LIGHTNING,
            Theme::muted("Zero-copy:"),
            Theme::success("Enabled (kernel-level transfer)")
        );
    }

    // Workers
    let worker_desc = if config.parallel == 0 {
        if is_remote {
            "Auto (256 for network)".to_string()
        } else {
            "Auto (CPU count for local)".to_string()
        }
    } else {
        format!("{} parallel workers", config.parallel)
    };
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Parallelism:"),
        Theme::value(worker_desc)
    );

    // Bandwidth
    if config.max_bandwidth > 0 {
        let bw_mb = config.max_bandwidth / (1024 * 1024);
        println!(
            "  {} {} {}",
            Icons::BULLET,
            Theme::muted("Bandwidth:"),
            Theme::warning(format!("Throttled to {} MB/s", bw_mb))
        );
    }

    // Filters
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

    // Advanced features (only if active)
    let mut advanced_lines: Vec<String> = Vec::new();

    if config.detect_renames {
        advanced_lines.push(format!(
            "  {} {} Detecting renamed files (threshold: {:.0}%)",
            Icons::BULLET,
            Theme::muted("Renames:"),
            config.rename_threshold * 100.0
        ));
    }
    if !config.link_dest.is_empty() {
        for ld in &config.link_dest {
            advanced_lines.push(format!(
                "  {} {} Hardlinking unchanged files against {}",
                Icons::BULLET,
                Theme::muted("Link-dest:"),
                ld.display()
            ));
        }
    }
    if config.preserve_hardlinks {
        advanced_lines.push(format!(
            "  {} {} Preserving hardlink groups",
            Icons::BULLET,
            Theme::muted("Hardlinks:"),
        ));
    }
    if config.inplace {
        let safety = format!("{:?}", config.inplace_safety).to_lowercase();
        advanced_lines.push(format!(
            "  {} {} Modifying files in-place (safety: {})",
            Icons::BULLET,
            Theme::muted("In-place:"),
            safety
        ));
    }
    if config.generate_manifest {
        advanced_lines.push(format!(
            "  {} {} Generating transfer manifests",
            Icons::BULLET,
            Theme::muted("Manifests:"),
        ));
    }
    if config.write_batch.is_some() {
        advanced_lines.push(format!(
            "  {} {} Recording batch journal for replay",
            Icons::BULLET,
            Theme::muted("Batch:"),
        ));
    }

    let has_advanced = !advanced_lines.is_empty();
    if has_advanced {
        println!();
        section_header(&format!("{} Advanced", Icons::GEAR));
        println!();
        for line in &advanced_lines {
            println!("{}", line);
        }
    }

    // Conditional copy
    let mut conditions: Vec<&str> = Vec::new();
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
