use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::{OrbitError, Result};
#[cfg(test)]
use crate::hash::hash_file;
use crate::paths::resolve_endpoints;
use crate::plan::{Disposition, build_plan};
use crate::request::{CopyRequest, PlanRequest, VerifyMode};
use crate::scan::{EntryKind, scan_source};

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
    execute_copy_with_hook(request, cwd, |_, _| {})
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CopyPhase {
    AfterSourceSnapshot,
    BeforePublish,
}

fn execute_copy_with_hook(
    request: &CopyRequest,
    cwd: &Path,
    mut hook: impl FnMut(CopyPhase, &Path),
) -> Result<CopyResult> {
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
        Disposition::Conflict | Disposition::Replace => {
            return Err(OrbitError::DestinationExists(entry.destination.clone()));
        }
        Disposition::Unsupported => {
            return Err(OrbitError::CopyRequiresRegularFile(entry.source.clone()));
        }
        Disposition::Copy | Disposition::SkipIdentical => {}
    }

    let (mut source_file, source_stamp) = open_stable_regular(&entry.source, false)?;
    let needs_digest =
        request.verify == VerifyMode::Hash || entry.disposition == Disposition::SkipIdentical;
    let expected_digest = if needs_digest {
        Some(hash_open_file(&mut source_file, &entry.source)?)
    } else {
        None
    };
    ensure_stable_path(&source_file, &entry.source, &source_stamp, false)?;
    hook(CopyPhase::AfterSourceSnapshot, &entry.source);

    if entry.disposition == Disposition::SkipIdentical {
        let (mut destination_file, destination_stamp) =
            open_stable_regular(&entry.destination, true)?;
        let destination_digest = hash_open_file(&mut destination_file, &entry.destination)?;
        ensure_stable_path(
            &destination_file,
            &entry.destination,
            &destination_stamp,
            true,
        )?;
        ensure_stable_path(&source_file, &entry.source, &source_stamp, false)?;
        if expected_digest != Some(destination_digest) {
            return Err(OrbitError::DestinationExists(entry.destination.clone()));
        }
        return Ok(CopyResult {
            operation_id: plan.operation_id,
            source: entry.source.clone(),
            destination: entry.destination.clone(),
            bytes: source_stamp.length,
            verification: request.verify,
            status: CopyStatus::AlreadyIdentical,
        });
    }

    let (stage_path, mut stage_file) = create_stage(&entry.destination, &plan.operation_id)?;
    let stage_stamp = file_stamp(&stage_file, &stage_path, "inspect staging file")?;
    let copy_result = (|| {
        source_file
            .seek(SeekFrom::Start(0))
            .map_err(|source| OrbitError::Io {
                operation: "rewind source file",
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
        ensure_stable_path(&source_file, &entry.source, &source_stamp, false)?;
        verify_stage_handle(
            request.verify,
            source_stamp.length,
            expected_digest,
            copied,
            &mut stage_file,
            &stage_path,
        )?;
        if !path_names_open_file(&stage_file, &stage_path, stage_stamp.identity)? {
            return Err(OrbitError::StagingChanged(stage_path.clone()));
        }
        hook(CopyPhase::BeforePublish, &stage_path);
        if !path_names_open_file(&stage_file, &stage_path, stage_stamp.identity)? {
            return Err(OrbitError::StagingChanged(stage_path.clone()));
        }
        publish_stage(&stage_path, &entry.destination)?;
        let published = path_stamp(&entry.destination, "inspect published destination")?;
        if published.identity != stage_stamp.identity {
            return Err(OrbitError::StagingChanged(stage_path.clone()));
        }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FileIdentity {
    volume: u64,
    file: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileStamp {
    identity: FileIdentity,
    length: u64,
    modified: Option<SystemTime>,
}

fn open_stable_regular(path: &Path, destination: bool) -> Result<(File, FileStamp)> {
    let before = path_stamp(path, "inspect file before opening")?;
    let file = File::open(path).map_err(|source| OrbitError::Io {
        operation: "open regular file",
        path: path.to_path_buf(),
        source,
    })?;
    let opened = file_stamp(&file, path, "inspect opened file")?;
    let after = path_stamp(path, "inspect file after opening")?;
    if before.identity != opened.identity || after.identity != opened.identity {
        return Err(if destination {
            OrbitError::DestinationExists(path.to_path_buf())
        } else {
            OrbitError::SourceChanged(path.to_path_buf())
        });
    }
    Ok((file, opened))
}

fn ensure_stable_path(
    file: &File,
    path: &Path,
    expected: &FileStamp,
    destination: bool,
) -> Result<()> {
    let opened = file_stamp(file, path, "reinspect opened file")?;
    let current = path_stamp(path, "reinspect file path")?;
    if &opened == expected && current.identity == expected.identity {
        Ok(())
    } else if destination {
        Err(OrbitError::DestinationExists(path.to_path_buf()))
    } else {
        Err(OrbitError::SourceChanged(path.to_path_buf()))
    }
}

fn path_names_open_file(file: &File, path: &Path, expected: FileIdentity) -> Result<bool> {
    let opened = file_stamp(file, path, "reinspect opened file")?;
    let current = path_stamp(path, "reinspect file path")?;
    Ok(opened.identity == expected && current.identity == expected)
}

fn file_stamp(file: &File, path: &Path, operation: &'static str) -> Result<FileStamp> {
    let metadata = file.metadata().map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })?;
    stamp_from_metadata(
        &metadata,
        file_identity(file, &metadata, path, operation)?,
        path,
        operation,
    )
}

#[cfg(unix)]
fn path_stamp(path: &Path, operation: &'static str) -> Result<FileStamp> {
    let metadata = fs::symlink_metadata(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })?;
    let identity = file_identity_from_metadata(&metadata);
    stamp_from_metadata(&metadata, identity, path, operation)
}

#[cfg(windows)]
fn path_stamp(path: &Path, operation: &'static str) -> Result<FileStamp> {
    let before = fs::symlink_metadata(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })?;
    if !before.file_type().is_file() {
        return Err(OrbitError::Io {
            operation,
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "path is not a regular file"),
        });
    }
    let file = File::open(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })?;
    let opened = file_stamp(&file, path, operation)?;
    let after = fs::symlink_metadata(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })?;
    if !after.file_type().is_file() {
        return Err(OrbitError::Io {
            operation,
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "path is not a regular file"),
        });
    }
    Ok(opened)
}

fn stamp_from_metadata(
    metadata: &fs::Metadata,
    identity: FileIdentity,
    path: &Path,
    operation: &'static str,
) -> Result<FileStamp> {
    if !metadata.file_type().is_file() {
        return Err(OrbitError::Io {
            operation,
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "path is not a regular file"),
        });
    }
    Ok(FileStamp {
        identity,
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(unix)]
fn file_identity(
    _file: &File,
    metadata: &fs::Metadata,
    _path: &Path,
    _operation: &'static str,
) -> Result<FileIdentity> {
    Ok(file_identity_from_metadata(metadata))
}

#[cfg(unix)]
fn file_identity_from_metadata(metadata: &fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;

    FileIdentity {
        volume: metadata.dev(),
        file: metadata.ino(),
    }
}

#[cfg(windows)]
fn file_identity(
    file: &File,
    _metadata: &fs::Metadata,
    path: &Path,
    operation: &'static str,
) -> Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle() as _, &mut information) };
    if succeeded == 0 {
        return Err(OrbitError::Io {
            operation,
            path: path.to_path_buf(),
            source: io::Error::last_os_error(),
        });
    }
    Ok(FileIdentity {
        volume: u64::from(information.dwVolumeSerialNumber),
        file: (u64::from(information.nFileIndexHigh) << 32) | u64::from(information.nFileIndexLow),
    })
}

fn hash_open_file(file: &mut File, path: &Path) -> Result<blake3::Hash> {
    file.seek(SeekFrom::Start(0))
        .map_err(|source| OrbitError::Io {
            operation: "rewind file for hashing",
            path: path.to_path_buf(),
            source,
        })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|source| OrbitError::Io {
            operation: "hash open file",
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            return Ok(hasher.finalize());
        }
        hasher.update(&buffer[..read]);
    }
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
        let mut random = [0_u8; 16];
        getrandom::fill(&mut random).map_err(|source| OrbitError::Io {
            operation: "generate staging name",
            path: parent.to_path_buf(),
            source: io::Error::other(source.to_string()),
        })?;
        let mut suffix = String::with_capacity(random.len() * 2);
        for byte in random {
            use std::fmt::Write as _;
            write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
        }
        let name = format!(
            ".orbit-{}-{suffix}.partial",
            &operation_id[..operation_id.len().min(16)],
        );
        let path = parent.join(name);
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
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

fn verify_stage_handle(
    mode: VerifyMode,
    expected_length: u64,
    expected_digest: Option<blake3::Hash>,
    copied: u64,
    stage_file: &mut File,
    stage_path: &Path,
) -> Result<()> {
    let metadata = stage_file.metadata().map_err(|source| OrbitError::Io {
        operation: "inspect staging file",
        path: stage_path.to_path_buf(),
        source,
    })?;
    let verified = match mode {
        VerifyMode::Hash => match expected_digest {
            Some(expected) => {
                copied == expected_length
                    && metadata.len() == expected_length
                    && hash_open_file(stage_file, stage_path)? == expected
            }
            None => false,
        },
        VerifyMode::Size => copied == expected_length && metadata.len() == expected_length,
    };
    if verified {
        Ok(())
    } else {
        Err(OrbitError::VerificationFailed(stage_path.to_path_buf()))
    }
}

#[cfg(test)]
fn verify_stage(
    mode: VerifyMode,
    expected_length: u64,
    expected_digest: Option<&str>,
    copied: u64,
    stage: &Path,
) -> Result<()> {
    let verified = match mode {
        VerifyMode::Hash => match expected_digest {
            Some(expected) => hash_file(stage)?.to_hex().as_str() == expected,
            None => false,
        },
        VerifyMode::Size => {
            let length = fs::metadata(stage)
                .map_err(|source| OrbitError::Io {
                    operation: "inspect staging file",
                    path: stage.to_path_buf(),
                    source,
                })?
                .len();
            copied == expected_length && length == expected_length
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
    use super::{CopyPhase, execute_copy_with_hook, publish_stage, verify_stage};
    use crate::error::OrbitError;
    use crate::request::{CopyRequest, OutputMode, VerifyMode};
    use std::fs;
    use std::path::PathBuf;

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
    fn source_mutation_after_snapshot_fails_without_a_destination() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"planned").unwrap();

        let result = execute_copy_with_hook(
            &request(source.clone(), destination.clone()),
            temp.path(),
            |phase, path| {
                if phase == CopyPhase::AfterSourceSnapshot {
                    fs::write(path, b"changed").unwrap();
                }
            },
        );

        assert!(matches!(
            result,
            Err(OrbitError::SourceChanged(_)) | Err(OrbitError::VerificationFailed(_))
        ));
        assert!(!destination.exists());
        assert!(partial_paths(temp.path()).is_empty());
    }

    #[test]
    fn staging_path_replacement_is_detected_before_publication() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        let held = temp.path().join("held-stage");
        fs::write(&source, b"verified").unwrap();

        let result = execute_copy_with_hook(
            &request(source, destination.clone()),
            temp.path(),
            |phase, path| {
                if phase == CopyPhase::BeforePublish {
                    fs::rename(path, &held).unwrap();
                    fs::write(path, b"unverified").unwrap();
                }
            },
        );

        assert!(
            matches!(result, Err(OrbitError::StagingChanged(_))),
            "unexpected result: {result:?}"
        );
        assert!(!destination.exists());
        assert!(partial_paths(temp.path()).is_empty());
        fs::remove_file(held).unwrap();
    }

    #[test]
    fn destination_created_during_copy_is_never_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"new content").unwrap();

        let result = execute_copy_with_hook(
            &request(source, destination.clone()),
            temp.path(),
            |phase, _| {
                if phase == CopyPhase::BeforePublish {
                    fs::write(&destination, b"concurrent content").unwrap();
                }
            },
        );

        assert!(matches!(result, Err(OrbitError::DestinationExists(_))));
        assert_eq!(fs::read(&destination).unwrap(), b"concurrent content");
        assert!(partial_paths(temp.path()).is_empty());
    }

    #[test]
    fn verification_io_error_is_not_reported_as_a_content_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-stage");

        assert!(matches!(
            verify_stage(
                VerifyMode::Hash,
                0,
                Some(&blake3::hash(b"").to_hex()),
                0,
                &missing
            ),
            Err(OrbitError::Io { .. })
        ));
    }

    fn request(source: PathBuf, destination: PathBuf) -> CopyRequest {
        CopyRequest {
            source,
            destination,
            verify: VerifyMode::Hash,
            output: OutputMode::Quiet,
        }
    }

    fn partial_paths(root: &std::path::Path) -> Vec<PathBuf> {
        fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with(".orbit-") && name.ends_with(".partial")
                })
            })
            .collect()
    }
}
