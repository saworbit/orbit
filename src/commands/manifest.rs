/*!
 * Manifest command handlers for Orbit CLI
 *
 * Handles flight plan creation, verification, diffing, and info display.
 */

use std::path::PathBuf;

use clap::Subcommand;

use crate::cli_style::{self, format_bytes, print_error, print_info, section_header, Icons, Theme};
use crate::config::{ChunkingStrategy, CopyConfig};
use crate::error::{OrbitError, Result, EXIT_FATAL};
use crate::manifest_integration::ManifestGenerator;

fn parse_chunking_strategy(chunking: &str, chunk_size: u32) -> Result<ChunkingStrategy> {
    match chunking {
        "cdc" => Ok(ChunkingStrategy::Cdc {
            avg_kib: chunk_size,
            algo: "gear".to_string(),
        }),
        "fixed" => Ok(ChunkingStrategy::Fixed {
            size_kib: chunk_size,
        }),
        _ => Err(OrbitError::Config(format!(
            "Invalid chunking strategy: {}",
            chunking
        ))),
    }
}

#[derive(Subcommand)]
pub enum ManifestCommands {
    /// Create a flight plan without transferring data
    Plan {
        /// Source path
        #[arg(short, long)]
        source: PathBuf,

        /// Destination path
        #[arg(short, long)]
        dest: PathBuf,

        /// Output directory for manifests
        #[arg(short, long)]
        output: PathBuf,

        /// Chunking strategy: cdc or fixed
        #[arg(long, default_value = "cdc")]
        chunking: String,

        /// Average chunk size in KiB (for CDC) or fixed size (for fixed)
        #[arg(long, default_value = "256")]
        chunk_size: u32,
    },

    /// Verify a completed transfer using manifests
    Verify {
        /// Directory containing manifests
        #[arg(short, long)]
        manifest_dir: PathBuf,
    },

    /// Show differences between manifest and target
    Diff {
        /// Directory containing manifests
        #[arg(short, long)]
        manifest_dir: PathBuf,

        /// Target directory to compare
        #[arg(short, long)]
        target: PathBuf,
    },

    /// Display manifest information
    Info {
        /// Path to flight plan or cargo manifest
        #[arg(short, long)]
        path: PathBuf,
    },
}

pub fn handle_manifest_command(command: ManifestCommands) -> Result<()> {
    match command {
        ManifestCommands::Plan {
            source,
            dest,
            output,
            chunking,
            chunk_size,
        } => handle_manifest_plan(source, dest, output, chunking, chunk_size),
        ManifestCommands::Verify { manifest_dir } => handle_manifest_verify(manifest_dir),
        ManifestCommands::Diff {
            manifest_dir,
            target,
        } => handle_manifest_diff(manifest_dir, target),
        ManifestCommands::Info { path } => handle_manifest_info(path),
    }
}

pub fn handle_manifest_plan(
    source: PathBuf,
    dest: PathBuf,
    output: PathBuf,
    chunking: String,
    chunk_size: u32,
) -> Result<()> {
    section_header(&format!("{} Creating Flight Plan", Icons::MANIFEST));
    println!();
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Source:"),
        Theme::value(source.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Dest:"),
        Theme::value(dest.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Output:"),
        Theme::value(output.display())
    );
    println!();

    let chunking_strategy = match parse_chunking_strategy(&chunking, chunk_size) {
        Ok(strategy) => strategy,
        Err(_) => {
            print_error(
                &format!("Invalid chunking strategy: {}", chunking),
                Some("Use 'cdc' or 'fixed'"),
            );
            std::process::exit(EXIT_FATAL);
        }
    };

    let config = CopyConfig {
        generate_manifest: true,
        manifest_output_dir: Some(output.clone()),
        chunking_strategy,
        ..Default::default()
    };

    let mut generator = ManifestGenerator::new(&source, &dest, &config)?;

    if source.is_file() {
        let file_name = source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        print_info(&format!("Processing: {}", file_name));
        generator.generate_file_manifest(&source, file_name)?;
    } else if source.is_dir() {
        use walkdir::WalkDir;

        for entry in WalkDir::new(&source).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                let relative_path = entry
                    .path()
                    .strip_prefix(&source)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                println!("  {} {}", Theme::muted(Icons::ARROW_RIGHT), relative_path);
                generator.generate_file_manifest(entry.path(), &relative_path)?;
            }
        }
    } else {
        print_error(
            "Source path does not exist or is not accessible",
            Some("Check the path and try again"),
        );
        std::process::exit(EXIT_FATAL);
    }

    generator.finalize("sha256:pending")?;

    println!();
    cli_style::print_success(&format!("Flight plan created at: {}", output.display()));

    Ok(())
}

pub fn handle_manifest_verify(manifest_dir: PathBuf) -> Result<()> {
    use crate::manifests::{CargoManifest, FlightPlan};

    section_header(&format!("{} Verifying Manifests", Icons::SHIELD));
    println!();
    println!(
        "  {} {}",
        Theme::muted("Directory:"),
        Theme::value(manifest_dir.display())
    );
    println!();

    let flight_plan_path = manifest_dir.join("job.flightplan.json");
    if !flight_plan_path.exists() {
        print_error(
            &format!("Flight plan not found: {}", flight_plan_path.display()),
            Some("Run 'orbit manifest plan' first"),
        );
        std::process::exit(EXIT_FATAL);
    }

    let flight_plan = FlightPlan::load(&flight_plan_path)
        .map_err(|e| OrbitError::Other(format!("Failed to load flight plan: {}", e)))?;

    // Display flight plan info
    let status = if flight_plan.is_finalized() {
        format!("{} Finalized", Icons::SUCCESS)
    } else {
        format!("{} Pending", Icons::PENDING)
    };

    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Job ID:"),
        Theme::value(&flight_plan.job_id)
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Files:"),
        Theme::value(flight_plan.files.len())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Status:"),
        if flight_plan.is_finalized() {
            Theme::success(&status)
        } else {
            Theme::warning(&status)
        }
    );
    println!();

    let mut verified = 0;
    let mut failed = 0;

    for file_ref in &flight_plan.files {
        let cargo_path = manifest_dir.join(&file_ref.cargo);

        if !cargo_path.exists() {
            println!(
                "  {} {} {}",
                Theme::error(Icons::ERROR),
                file_ref.path,
                Theme::error("(missing)")
            );
            failed += 1;
            continue;
        }

        match CargoManifest::load(&cargo_path) {
            Ok(cargo) => {
                println!(
                    "  {} {} {} windows, {}",
                    Theme::success(Icons::SUCCESS),
                    file_ref.path,
                    cargo.windows.len(),
                    format_bytes(cargo.size)
                );
                verified += 1;
            }
            Err(e) => {
                println!(
                    "  {} {} {}",
                    Theme::error(Icons::ERROR),
                    file_ref.path,
                    Theme::error(format!("({})", e))
                );
                failed += 1;
            }
        }
    }

    println!();
    if failed == 0 {
        cli_style::print_success(&format!(
            "Verification complete: {} files verified",
            verified
        ));
    } else {
        cli_style::print_warning(&format!(
            "Verification complete: {} verified, {} failed",
            verified, failed
        ));
    }

    Ok(())
}

// TODO: Implement full manifest-vs-filesystem comparison (alpha stub)
pub fn handle_manifest_diff(manifest_dir: PathBuf, target: PathBuf) -> Result<()> {
    section_header(&format!("{} Comparing Manifests", Icons::STATS));
    println!();
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Manifests:"),
        Theme::value(manifest_dir.display())
    );
    println!(
        "  {} {} {}",
        Icons::BULLET,
        Theme::muted("Target:"),
        Theme::value(target.display())
    );
    println!();

    cli_style::print_warning("Diff operation not yet fully implemented");
    println!(
        "  {} This will compare manifest metadata with actual files",
        Theme::muted(Icons::ARROW_RIGHT)
    );
    println!();

    Ok(())
}

pub fn handle_manifest_info(path: PathBuf) -> Result<()> {
    use crate::manifests::{CargoManifest, FlightPlan};

    if !path.exists() {
        print_error(
            &format!("Path not found: {}", path.display()),
            Some("Check the file path and try again"),
        );
        std::process::exit(EXIT_FATAL);
    }

    if let Ok(flight_plan) = FlightPlan::load(&path) {
        section_header(&format!("{} Flight Plan", Icons::MANIFEST));
        println!();

        let items = vec![
            ("Schema", flight_plan.schema.clone()),
            ("Job ID", flight_plan.job_id.clone()),
            ("Created", flight_plan.created_utc.to_string()),
            (
                "Source",
                format!(
                    "{} ({})",
                    flight_plan.source.root, flight_plan.source.endpoint_type
                ),
            ),
            (
                "Target",
                format!(
                    "{} ({})",
                    flight_plan.target.root, flight_plan.target.endpoint_type
                ),
            ),
            ("Files", flight_plan.files.len().to_string()),
            ("Encryption", flight_plan.policy.encryption.aead.clone()),
        ];

        for (key, value) in items {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted(format!("{}:", key)),
                Theme::value(&value)
            );
        }

        if let Some(classification) = &flight_plan.policy.classification {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted("Classification:"),
                Theme::value(classification)
            );
        }

        println!();
        return Ok(());
    }

    if let Ok(cargo) = CargoManifest::load(&path) {
        section_header(&format!("{} Cargo Manifest", Icons::FILE));
        println!();

        let items = vec![
            ("Schema", cargo.schema.clone()),
            ("Path", cargo.path.clone()),
            ("Size", format_bytes(cargo.size)),
            ("Chunking", cargo.chunking.chunking_type.clone()),
            ("Windows", cargo.windows.len().to_string()),
            ("Chunks", cargo.total_chunks().to_string()),
        ];

        for (key, value) in items {
            println!(
                "  {} {} {}",
                Icons::BULLET,
                Theme::muted(format!("{}:", key)),
                Theme::value(&value)
            );
        }

        println!();
        return Ok(());
    }

    print_error(
        "Not a valid flight plan or cargo manifest",
        Some("Ensure the file is a valid Orbit manifest"),
    );
    std::process::exit(EXIT_FATAL);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chunking_strategy_cdc() {
        match parse_chunking_strategy("cdc", 256).unwrap() {
            ChunkingStrategy::Cdc { avg_kib, algo } => {
                assert_eq!(avg_kib, 256);
                assert_eq!(algo, "gear");
            }
            other => panic!("expected cdc strategy, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chunking_strategy_fixed() {
        match parse_chunking_strategy("fixed", 512).unwrap() {
            ChunkingStrategy::Fixed { size_kib } => assert_eq!(size_kib, 512),
            other => panic!("expected fixed strategy, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_chunking_strategy_rejects_invalid_value() {
        let err = parse_chunking_strategy("bogus", 256).unwrap_err();
        assert!(err.to_string().contains("Invalid chunking strategy: bogus"));
    }

    #[test]
    fn test_handle_manifest_diff_is_placeholder_but_ok() {
        let result = handle_manifest_diff(PathBuf::from("manifests"), PathBuf::from("target"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_manifest_command_dispatches_diff() {
        let result = handle_manifest_command(ManifestCommands::Diff {
            manifest_dir: PathBuf::from("manifests"),
            target: PathBuf::from("target"),
        });
        assert!(result.is_ok());
    }
}
