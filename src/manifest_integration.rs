//! Manifest generation integration for Orbit transfers
//!
//! This module provides functionality to generate Flight Plans, Cargo Manifests,
//! and Star Maps during file transfer operations.

use crate::config::{ChunkingStrategy, CopyConfig};
use crate::error::{OrbitError, Result};
use orbit_core_audit::TelemetryLogger;
use orbit_core_manifest::{
    CargoManifest, Chunking, Encryption, Endpoint, FileRef, FlightPlan, Policy, WindowMeta,
};
use orbit_core_starmap::{ChunkMeta, StarMapBuilder};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Manifest generator for transfer operations
pub struct ManifestGenerator {
    /// Job ID for this transfer
    job_id: String,
    /// Output directory for manifests
    output_dir: PathBuf,
    /// Chunking strategy
    chunking_strategy: ChunkingStrategy,
    /// Flight plan being built
    flight_plan: FlightPlan,
    /// Telemetry logger
    telemetry: TelemetryLogger,
    /// Total bytes processed across all files
    total_bytes: u64,
}

impl ManifestGenerator {
    /// Create a new manifest generator
    pub fn new(source: &Path, dest: &Path, config: &CopyConfig) -> Result<Self> {
        // Generate job ID
        let job_id = format!("job-{}", chrono::Utc::now().format("%Y-%m-%dT%H_%M_%SZ"));

        // Determine output directory
        let output_dir = config
            .manifest_output_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("/var/lib/orbit/manifests").join(&job_id));

        // Create output directory
        std::fs::create_dir_all(&output_dir).map_err(|e| OrbitError::Io(e))?;

        // Create endpoints
        let source_endpoint = Endpoint::filesystem(source.to_string_lossy().to_string());
        let target_endpoint = Endpoint::filesystem(dest.to_string_lossy().to_string());

        // Create policy
        let encryption = Encryption::aes256_gcm("env:ORBIT_KEY");
        let policy =
            Policy::default_with_encryption(encryption).with_classification("UNCLASSIFIED");

        // Create flight plan
        let flight_plan =
            FlightPlan::new(source_endpoint, target_endpoint, policy).with_job_id(job_id.clone());

        // Create telemetry logger
        let telemetry_path = output_dir.join("audit.jsonl");
        let telemetry = TelemetryLogger::new(telemetry_path)
            .map_err(|e| OrbitError::Other(format!("Failed to create telemetry logger: {}", e)))?;

        Ok(Self {
            job_id,
            output_dir,
            chunking_strategy: config.chunking_strategy.clone(),
            flight_plan,
            telemetry,
            total_bytes: 0,
        })
    }

    /// Generate manifest for a single file
    pub fn generate_file_manifest(&mut self, file_path: &Path, relative_path: &str) -> Result<()> {
        // Open file
        let mut file = File::open(file_path).map_err(|e| OrbitError::Io(e))?;

        // Get file size
        let metadata = file.metadata().map_err(|e| OrbitError::Io(e))?;
        let file_size = metadata.len();

        // Accumulate total bytes
        self.total_bytes += file_size;

        // Log file start
        self.telemetry
            .log_file_start(&self.job_id, relative_path, file_size)
            .map_err(|e| OrbitError::Other(format!("Telemetry error: {}", e)))?;

        // Create chunking configuration
        let chunking = match &self.chunking_strategy {
            ChunkingStrategy::Cdc { avg_kib, algo } => Chunking::cdc(*avg_kib, algo),
            ChunkingStrategy::Fixed { size_kib } => Chunking::fixed(*size_kib),
        };

        // Create cargo manifest
        let mut cargo = CargoManifest::new(relative_path, file_size, chunking);

        // Chunk the file and build star map
        let (chunks, windows) = self.chunk_file(&mut file, file_size)?;

        // Build star map
        let mut starmap_builder = StarMapBuilder::new(file_size);
        for chunk in &chunks {
            starmap_builder
                .add_chunk(chunk.offset, chunk.length, &chunk.content_id)
                .map_err(|e| OrbitError::Other(format!("Star map error: {}", e)))?;
        }

        // Add windows to cargo manifest and star map
        for window in windows.iter() {
            // Convert merkle_root bytes to hex string for cargo manifest
            let merkle_hex = hex::encode(window.merkle_root);

            cargo.add_window(
                WindowMeta::new(window.id, window.first_chunk, window.count, merkle_hex)
                    .with_overlap(window.overlap.unwrap_or(0)),
            );

            starmap_builder
                .add_window(
                    window.id,
                    window.first_chunk,
                    window.count,
                    &window.merkle_root,
                    window.overlap.unwrap_or(0),
                )
                .map_err(|e| OrbitError::Other(format!("Star map error: {}", e)))?;
        }

        // Build star map
        let starmap_data = starmap_builder
            .build()
            .map_err(|e| OrbitError::Other(format!("Star map build error: {}", e)))?;

        // Generate file names
        let safe_name = relative_path.replace('/', "_").replace('\\', "_");
        let cargo_filename = format!("{}.cargo.json", safe_name);
        let starmap_filename = format!("{}.starmap.bin", safe_name);

        // Save cargo manifest
        let cargo_path = self.output_dir.join(&cargo_filename);
        cargo
            .save(&cargo_path)
            .map_err(|e| OrbitError::Other(format!("Failed to save cargo manifest: {}", e)))?;

        // Save star map
        let starmap_path = self.output_dir.join(&starmap_filename);
        std::fs::write(&starmap_path, starmap_data).map_err(|e| OrbitError::Io(e))?;

        // Add file reference to flight plan
        self.flight_plan
            .add_file(FileRef::new(relative_path, &cargo_filename).with_starmap(&starmap_filename));

        Ok(())
    }

    /// Finalize the manifest generation
    pub fn finalize(mut self, job_digest: &str) -> Result<()> {
        // Finalize flight plan
        self.flight_plan.finalize(job_digest.to_string());

        // Save flight plan
        let flight_plan_path = self.output_dir.join("job.flightplan.json");
        self.flight_plan
            .save(&flight_plan_path)
            .map_err(|e| OrbitError::Other(format!("Failed to save flight plan: {}", e)))?;

        // Log job completion with accumulated total bytes
        let file_count = self.flight_plan.files.len() as u32;
        self.telemetry
            .log_job_complete(&self.job_id, job_digest, file_count, self.total_bytes)
            .map_err(|e| OrbitError::Other(format!("Telemetry error: {}", e)))?;

        Ok(())
    }

    /// Chunk a file and generate windows
    fn chunk_file(
        &self,
        file: &mut File,
        file_size: u64,
    ) -> Result<(Vec<ChunkMeta>, Vec<InternalWindowMeta>)> {
        let chunk_size = match &self.chunking_strategy {
            ChunkingStrategy::Cdc { avg_kib, .. } => (*avg_kib as usize) * 1024,
            ChunkingStrategy::Fixed { size_kib } => (*size_kib as usize) * 1024,
        };

        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut buffer = vec![0u8; chunk_size];

        // Read and chunk the file
        file.seek(SeekFrom::Start(0))
            .map_err(|e| OrbitError::Io(e))?;

        loop {
            let bytes_read = file.read(&mut buffer).map_err(|e| OrbitError::Io(e))?;

            if bytes_read == 0 {
                break;
            }

            // Calculate BLAKE3 hash of chunk
            let hash = blake3::hash(&buffer[..bytes_read]);
            let content_id = *hash.as_bytes();

            chunks.push(ChunkMeta {
                offset,
                length: bytes_read as u32,
                content_id,
            });

            offset += bytes_read as u64;
        }

        // Validate that we read the expected amount
        if offset != file_size {
            return Err(OrbitError::Other(format!(
                "File size mismatch: expected {} bytes, read {} bytes",
                file_size, offset
            )));
        }

        // Create windows (64 chunks per window, 4 chunk overlap)
        let mut windows = Vec::new();
        let chunks_per_window = 64u32;
        let overlap = 4u16;

        let mut window_id = 0u32;
        let mut first_chunk = 0u32;

        while first_chunk < chunks.len() as u32 {
            let remaining = chunks.len() as u32 - first_chunk;
            let count = remaining.min(chunks_per_window) as u16;

            // Calculate merkle root for this window (simplified - just hash the chunk IDs)
            let window_chunks: Vec<_> = chunks
                .iter()
                .skip(first_chunk as usize)
                .take(count as usize)
                .collect();

            let mut hasher = blake3::Hasher::new();
            for chunk in &window_chunks {
                hasher.update(&chunk.content_id);
            }
            let merkle_root = *hasher.finalize().as_bytes();

            windows.push(InternalWindowMeta {
                id: window_id,
                first_chunk,
                count,
                merkle_root,
                overlap: Some(overlap),
            });

            window_id += 1;
            // Ensure we always advance by at least 1 chunk to prevent infinite loops
            // when count <= overlap (e.g., small files with few chunks)
            first_chunk += (count as u32).saturating_sub(overlap as u32).max(1);
        }

        Ok((chunks, windows))
    }
}

/// Internal window metadata with byte array merkle root
#[derive(Debug, Clone)]
struct InternalWindowMeta {
    pub id: u32,
    pub first_chunk: u32,
    pub count: u16,
    pub merkle_root: [u8; 32],
    pub overlap: Option<u16>,
}

/// Check if manifest generation is enabled
pub fn should_generate_manifest(config: &CopyConfig) -> bool {
    config.generate_manifest && config.manifest_output_dir.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_should_generate_manifest() {
        let mut config = CopyConfig::default();
        assert!(!should_generate_manifest(&config));

        config.generate_manifest = true;
        assert!(!should_generate_manifest(&config)); // Still false, no output dir

        config.manifest_output_dir = Some(PathBuf::from("/tmp/manifests"));
        assert!(should_generate_manifest(&config));
    }

    #[test]
    fn test_manifest_generator_creation() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");

        let mut config = CopyConfig::default();
        config.manifest_output_dir = Some(temp_dir.path().join("manifests"));

        let generator = ManifestGenerator::new(&source, &dest, &config);
        assert!(generator.is_ok());
    }

    #[test]
    fn test_chunk_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut temp_file = NamedTempFile::new().unwrap();

        // Write 1KB of test data
        let data = vec![0x42u8; 1024];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let source = temp_dir.path().join("source");
        let dest = temp_dir.path().join("dest");

        let mut config = CopyConfig::default();
        config.manifest_output_dir = Some(temp_dir.path().join("manifests"));
        config.chunking_strategy = ChunkingStrategy::Fixed { size_kib: 1 };

        let generator = ManifestGenerator::new(&source, &dest, &config).unwrap();

        let mut file = File::open(temp_file.path()).unwrap();
        let (chunks, windows) = generator.chunk_file(&mut file, 1024).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].length, 1024);
        assert!(!windows.is_empty());
    }
}
