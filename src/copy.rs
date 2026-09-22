use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{OrbitError, Result};
use crate::hash::hash_file;
use crate::paths::resolve_endpoints;
use crate::plan::{Disposition, build_plan};
use crate::request::{CopyRequest, PlanRequest, VerifyMode};
use crate::scan::{EntryKind, scan_source};

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CopyStatus {
    Copied,
    AlreadyIdentical,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopyResult {
    pub operation_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub bytes: u64,
    pub verification: VerifyMode,
    pub status: CopyStatus,
}

pub fn execute_copy(request: &CopyRequest, cwd: &Path) -> Result<CopyResult> {
    let endpoints = resolve_endpoints(&request.source, &request.destination, cwd)?;
    ensure_destination_parent(&endpoints.destination)?;

    let source_metadata =
        fs::symlink_metadata(&endpoints.source).map_err(|source| OrbitError::Io {
            operation: "inspect copy source",
            path: endpoints.source.clone(),
            source,
        })?;
    if !source_metadata.file_type().is_file() {
        return Err(OrbitError::CopyRequiresRegularFile(endpoints.source));
    }

    let snapshot = scan_source(&endpoints.source)?;
    if snapshot.entries.len() != 1 || snapshot.entries[0].kind != EntryKind::File {
        return Err(OrbitError::CopyRequiresRegularFile(endpoints.source));
    }

    let plan_request = PlanRequest {
        source: request.source.clone(),
        destination: request.destination.clone(),
        replace: false,
        verify: request.verify,
        ignore_unsupported: false,
        output: request.output,
    };
    let plan = build_plan(&plan_request, &endpoints, &snapshot)?;
    let entry = &plan.entries[0];

    match entry.disposition {
        Disposition::SkipIdentical => {
            let expected_digest = entry
                .source_digest
                .as_deref()
                .ok_or_else(|| OrbitError::VerificationFailed(entry.destination.clone()))?;
            if !destination_matches_digest(&entry.destination, expected_digest)? {
                return Err(OrbitError::DestinationExists(entry.destination.clone()));
            }
            return Ok(CopyResult {
                operation_id: plan.operation_id,
                source: entry.source.clone(),
                destination: entry.destination.clone(),
                bytes: entry.length,
                verification: request.verify,
                status: CopyStatus::AlreadyIdentical,
            });
        }
        Disposition::Copy => {}
        Disposition::Conflict | Disposition::Replace => {
            return Err(OrbitError::DestinationExists(entry.destination.clone()));
        }
        Disposition::Unsupported => {
            return Err(OrbitError::CopyRequiresRegularFile(entry.source.clone()));
        }
    }

    let (stage_path, mut stage_file) = create_stage(&entry.destination, &plan.operation_id)?;
    let copy_result = (|| {
        let mut source_file = File::open(&entry.source).map_err(|source| OrbitError::Io {
            operation: "open source file",
            path: entry.source.clone(),
            source,
        })?;
        let copied =
            io::copy(&mut source_file, &mut stage_file).map_err(|source| OrbitError::Io {
                operation: "copy source into staging file",
                path: stage_path.clone(),
                source,
            })?;
        stage_file.sync_all().map_err(|source| OrbitError::Io {
            operation: "flush staging file",
            path: stage_path.clone(),
            source,
        })?;
        verify_stage(
            request.verify,
            entry.length,
            entry.source_digest.as_deref(),
            copied,
            &stage_path,
        )?;
        drop(stage_file);
        publish_stage(&stage_path, &entry.destination)?;
        Ok(CopyResult {
            operation_id: plan.operation_id,
            source: entry.source.clone(),
            destination: entry.destination.clone(),
            bytes: copied,
            verification: request.verify,
            status: CopyStatus::Copied,
        })
    })();

    match fs::remove_file(&stage_path) {
        Ok(()) => copy_result,
        Err(source) if source.kind() == io::ErrorKind::NotFound => copy_result,
        Err(source) => Err(OrbitError::StagingCleanup {
            path: stage_path,
            source,
        }),
    }
}

fn destination_matches_digest(destination: &Path, expected_digest: &str) -> Result<bool> {
    hash_file(destination).map(|actual| actual.to_hex().as_str() == expected_digest)
}

fn ensure_destination_parent(destination: &Path) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| OrbitError::DestinationParentMissing(destination.to_path_buf()))?;
    match fs::metadata(parent) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(OrbitError::DestinationParentMissing(parent.to_path_buf())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            Err(OrbitError::DestinationParentMissing(parent.to_path_buf()))
        }
        Err(source) => Err(OrbitError::Io {
            operation: "inspect destination parent",
            path: parent.to_path_buf(),
            source,
        }),
    }
}

fn create_stage(destination: &Path, operation_id: &str) -> Result<(PathBuf, File)> {
    let parent = destination
        .parent()
        .expect("destination parent was checked before staging");
    for _ in 0..128 {
        let sequence = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            ".orbit-{}-{}-{sequence}.partial",
            &operation_id[..operation_id.len().min(16)],
            std::process::id()
        );
        let path = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(OrbitError::Io {
                    operation: "create staging file",
                    path,
                    source,
                });
            }
        }
    }
    Err(OrbitError::Io {
        operation: "create unique staging file",
        path: parent.to_path_buf(),
        source: io::Error::new(io::ErrorKind::AlreadyExists, "staging name space exhausted"),
    })
}

fn verify_stage(
    mode: VerifyMode,
    expected_length: u64,
    expected_digest: Option<&str>,
    copied: u64,
    stage: &Path,
) -> Result<()> {
    let verified = match mode {
        VerifyMode::Hash => expected_digest.is_some_and(|expected| {
            hash_file(stage).is_ok_and(|actual| actual.to_hex().as_str() == expected)
        }),
        VerifyMode::Size => {
            copied == expected_length
                && fs::metadata(stage).is_ok_and(|metadata| metadata.len() == expected_length)
        }
    };
    if verified {
        Ok(())
    } else {
        Err(OrbitError::VerificationFailed(stage.to_path_buf()))
    }
}

fn publish_stage(stage: &Path, destination: &Path) -> Result<()> {
    fs::hard_link(stage, destination).map_err(|source| {
        if source.kind() == io::ErrorKind::AlreadyExists {
            OrbitError::DestinationExists(destination.to_path_buf())
        } else {
            OrbitError::Io {
                operation: "publish verified staging file",
                path: destination.to_path_buf(),
                source,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{destination_matches_digest, publish_stage, verify_stage};
    use crate::error::OrbitError;
    use crate::request::VerifyMode;
    use std::fs;

    #[test]
    fn publishing_never_overwrites_a_destination_that_appeared() {
        let temp = tempfile::tempdir().unwrap();
        let stage = temp.path().join("stage");
        let destination = temp.path().join("destination");
        fs::write(&stage, b"new").unwrap();
        fs::write(&destination, b"keep").unwrap();

        assert!(matches!(
            publish_stage(&stage, &destination),
            Err(OrbitError::DestinationExists(path)) if path == destination
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"keep");
        assert_eq!(fs::read(&stage).unwrap(), b"new");
    }

    #[test]
    fn hash_verification_detects_changed_content() {
        let temp = tempfile::tempdir().unwrap();
        let stage = temp.path().join("stage");
        fs::write(&stage, b"changed").unwrap();

        assert!(matches!(
            verify_stage(VerifyMode::Hash, 7, Some(&blake3::hash(b"planned").to_hex()), 7, &stage),
            Err(OrbitError::VerificationFailed(path)) if path == stage
        ));
    }

    #[test]
    fn identical_no_op_is_revalidated_before_success() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("destination");
        let planned = blake3::hash(b"planned").to_hex().to_string();
        fs::write(&destination, b"changed after planning").unwrap();

        assert!(!destination_matches_digest(&destination, &planned).unwrap());
    }
}
