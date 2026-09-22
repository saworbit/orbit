use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

use crate::error::{OrbitError, Result};
use crate::hash::hash_file;
use crate::paths::ResolvedEndpoints;
use crate::request::{PlanRequest, VerifyMode};
use crate::scan::{EntryKind, SourceSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Copy,
    SkipIdentical,
    Replace,
    Conflict,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct PlanEntry {
    pub relative_path: PathBuf,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub kind: EntryKind,
    pub disposition: Disposition,
    pub length: u64,
    pub source_digest: Option<String>,
    pub reason: String,
    pub blocking: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct CopyPlan {
    pub operation_id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub entries: Vec<PlanEntry>,
}

impl CopyPlan {
    pub fn is_executable(&self) -> bool {
        self.entries.iter().all(|entry| !entry.blocking)
    }
}

pub fn build_plan(
    request: &PlanRequest,
    endpoints: &ResolvedEndpoints,
    snapshot: &SourceSnapshot,
) -> Result<CopyPlan> {
    let mut entries = Vec::with_capacity(snapshot.entries.len());
    for source_entry in &snapshot.entries {
        let source_path = at_root(&snapshot.root, &source_entry.relative_path);
        let destination_path = at_root(&endpoints.destination, &source_entry.relative_path);

        let mut source_hash =
            if source_entry.kind == EntryKind::File && request.verify == VerifyMode::Hash {
                Some(hash_file(&source_path)?)
            } else {
                None
            };

        let (disposition, reason, blocking) = if !source_entry.unsupported.is_empty() {
            if request.ignore_unsupported {
                (
                    Disposition::Unsupported,
                    "unsupported source entry will be omitted",
                    false,
                )
            } else {
                (
                    Disposition::Unsupported,
                    "source entry has unsupported fidelity",
                    true,
                )
            }
        } else {
            match inspect_destination(&destination_path)? {
                None => (Disposition::Copy, "destination does not exist", false),
                Some(destination) if destination.kind != source_entry.kind => (
                    Disposition::Conflict,
                    "source and destination object types differ",
                    true,
                ),
                Some(destination) => match source_entry.kind {
                    EntryKind::Directory => (
                        Disposition::SkipIdentical,
                        "destination directory already exists",
                        false,
                    ),
                    EntryKind::Symlink if destination.link_target == source_entry.link_target => (
                        Disposition::SkipIdentical,
                        "symbolic link targets are identical",
                        false,
                    ),
                    EntryKind::Symlink => (
                        Disposition::Conflict,
                        "destination symbolic link has a different target",
                        true,
                    ),
                    EntryKind::File if destination.length == source_entry.length => {
                        let source_value = match source_hash {
                            Some(value) => value,
                            None => {
                                let value = hash_file(&source_path)?;
                                source_hash = Some(value);
                                value
                            }
                        };
                        let destination_value = hash_file(&destination_path)?;
                        if source_value == destination_value {
                            (Disposition::SkipIdentical, "contents are identical", false)
                        } else if request.replace {
                            (
                                Disposition::Replace,
                                "destination file will be replaced",
                                false,
                            )
                        } else {
                            (
                                Disposition::Conflict,
                                "destination file has different contents",
                                true,
                            )
                        }
                    }
                    EntryKind::File if request.replace => (
                        Disposition::Replace,
                        "destination file will be replaced",
                        false,
                    ),
                    EntryKind::File => (
                        Disposition::Conflict,
                        "destination file has different contents",
                        true,
                    ),
                    EntryKind::Unsupported => (
                        Disposition::Unsupported,
                        "source entry has unsupported fidelity",
                        !request.ignore_unsupported,
                    ),
                },
            }
        };

        entries.push(PlanEntry {
            relative_path: source_entry.relative_path.clone(),
            source: source_path,
            destination: destination_path,
            kind: source_entry.kind,
            disposition,
            length: source_entry.length,
            source_digest: source_hash.map(|hash| hash.to_hex().to_string()),
            reason: reason.to_owned(),
            blocking,
        });
    }

    Ok(CopyPlan {
        operation_id: operation_id(request, endpoints),
        source: endpoints.source.clone(),
        destination: endpoints.destination.clone(),
        entries,
    })
}

fn at_root(root: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

fn operation_id(request: &PlanRequest, endpoints: &ResolvedEndpoints) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"orbit.operation-id.v2");
    update_path_hash(&mut hasher, b"source", &endpoints.source);
    update_path_hash(&mut hasher, b"destination", &endpoints.destination);
    hasher.update(b"options");
    hasher.update(&[
        u8::from(request.replace),
        match request.verify {
            VerifyMode::Hash => 1,
            VerifyMode::Size => 2,
        },
        u8::from(request.ignore_unsupported),
    ]);
    hasher.finalize().to_hex()[..24].to_owned()
}

fn update_path_hash(hasher: &mut blake3::Hasher, domain: &[u8], path: &Path) {
    let encoded = native_path_bytes(path);
    let length = u64::try_from(encoded.len()).expect("path length fits in u64");
    hasher.update(domain);
    hasher.update(PATH_ENCODING_DOMAIN);
    hasher.update(&length.to_le_bytes());
    hasher.update(&encoded);
}

#[cfg(unix)]
const PATH_ENCODING_DOMAIN: &[u8] = b"unix-bytes-v1";

#[cfg(windows)]
const PATH_ENCODING_DOMAIN: &[u8] = b"windows-utf16le-v1";

#[cfg(unix)]
fn native_path_bytes(path: &Path) -> Vec<u8> {
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(windows)]
fn native_path_bytes(path: &Path) -> Vec<u8> {
    path.as_os_str()
        .encode_wide()
        .flat_map(|unit| unit.to_le_bytes())
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(dead_code)]
struct DestinationEntry {
    kind: EntryKind,
    length: u64,
    link_target: Option<PathBuf>,
}

#[allow(dead_code)]
fn inspect_destination(path: &Path) -> Result<Option<DestinationEntry>> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(OrbitError::Io {
                operation: "inspect destination",
                path: path.to_path_buf(),
                source,
            });
        }
    };
    let file_type = metadata.file_type();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::Unsupported
    };
    let link_target = if kind == EntryKind::Symlink {
        Some(std::fs::read_link(path).map_err(|source| OrbitError::Io {
            operation: "read destination symbolic link",
            path: path.to_path_buf(),
            source,
        })?)
    } else {
        None
    };
    Ok(Some(DestinationEntry {
        kind,
        length: metadata.len(),
        link_target,
    }))
}

#[cfg(test)]
mod destination_tests {
    use super::inspect_destination;
    use crate::scan::EntryKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_destination_returns_none() {
        let temp = tempdir().unwrap();
        assert_eq!(
            inspect_destination(&temp.path().join("missing")).unwrap(),
            None
        );
    }

    #[test]
    fn regular_file_reports_kind_and_length() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("file");
        fs::write(&path, b"four").unwrap();
        let entry = inspect_destination(&path).unwrap().unwrap();
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.length, 4);
        assert_eq!(entry.link_target, None);
    }

    #[test]
    fn directory_is_not_classified_as_file() {
        let temp = tempdir().unwrap();
        let entry = inspect_destination(temp.path()).unwrap().unwrap();
        assert_eq!(entry.kind, EntryKind::Directory);
    }
}

#[cfg(test)]
mod tests {
    use super::{Disposition, build_plan, operation_id};
    use crate::paths::ResolvedEndpoints;
    use crate::request::{OutputMode, PlanRequest, VerifyMode};
    use crate::scan::{EntryKind, SourceEntry, SourceSnapshot, UnsupportedFeature, scan_source};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn request_for(
        source: &Path,
        destination: &Path,
        replace: bool,
        verify: VerifyMode,
        ignore_unsupported: bool,
    ) -> PlanRequest {
        PlanRequest {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            replace,
            verify,
            ignore_unsupported,
            output: OutputMode::Human,
        }
    }

    fn plan_for(
        source: &Path,
        destination: &Path,
        replace: bool,
        verify: VerifyMode,
        ignore_unsupported: bool,
    ) -> super::CopyPlan {
        let request = request_for(source, destination, replace, verify, ignore_unsupported);
        let endpoints = ResolvedEndpoints {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
        };
        let snapshot = scan_source(source).unwrap();
        build_plan(&request, &endpoints, &snapshot).unwrap()
    }

    fn unsupported_plan(ignore_unsupported: bool) -> super::CopyPlan {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"source").unwrap();
        let request = request_for(
            &source,
            &destination,
            false,
            VerifyMode::Hash,
            ignore_unsupported,
        );
        let endpoints = ResolvedEndpoints {
            source: source.clone(),
            destination,
        };
        let snapshot = SourceSnapshot {
            root: source,
            entries: vec![SourceEntry {
                relative_path: PathBuf::new(),
                kind: EntryKind::Unsupported,
                length: 0,
                modified_ns: None,
                read_only: false,
                link_target: None,
                unsupported: vec![UnsupportedFeature::SpecialFile],
            }],
        };
        build_plan(&request, &endpoints, &snapshot).unwrap()
    }

    #[test]
    fn missing_exact_destination_is_copy() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::write(&source, b"same").unwrap();

        let plan = plan_for(
            &source,
            &temp.path().join("missing"),
            false,
            VerifyMode::Hash,
            false,
        );

        assert_eq!(plan.entries[0].disposition, Disposition::Copy);
        assert_eq!(plan.entries[0].reason, "destination does not exist");
        assert!(!plan.entries[0].blocking);
        assert_eq!(
            plan.entries[0].source_digest.as_deref(),
            Some(blake3::hash(b"same").to_hex().as_str())
        );
    }

    #[test]
    fn existing_directory_is_identical() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&destination).unwrap();

        let plan = plan_for(&source, &destination, false, VerifyMode::Hash, false);

        assert_eq!(plan.entries[0].disposition, Disposition::SkipIdentical);
        assert_eq!(
            plan.entries[0].reason,
            "destination directory already exists"
        );
        assert!(!plan.entries[0].blocking);
    }

    #[test]
    fn equal_size_is_hashed_before_identical_is_selected_in_size_mode() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"same").unwrap();
        fs::write(&destination, b"same").unwrap();

        let plan = plan_for(&source, &destination, false, VerifyMode::Size, false);

        assert_eq!(plan.entries[0].disposition, Disposition::SkipIdentical);
        assert_eq!(plan.entries[0].reason, "contents are identical");
        assert_eq!(
            plan.entries[0].source_digest.as_deref(),
            Some(blake3::hash(b"same").to_hex().as_str())
        );
        assert!(!plan.entries[0].blocking);
    }

    #[test]
    fn equal_size_different_content_is_conflict_without_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"left").unwrap();
        fs::write(&destination, b"rite").unwrap();

        let plan = plan_for(&source, &destination, false, VerifyMode::Size, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(
            plan.entries[0].reason,
            "destination file has different contents"
        );
        assert!(plan.entries[0].blocking);
        assert!(plan.entries[0].source_digest.is_some());
        assert!(!plan.is_executable());
    }

    #[test]
    fn equal_size_different_content_is_replace_with_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"left").unwrap();
        fs::write(&destination, b"rite").unwrap();

        let plan = plan_for(&source, &destination, true, VerifyMode::Size, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Replace);
        assert_eq!(plan.entries[0].reason, "destination file will be replaced");
        assert!(!plan.entries[0].blocking);
        assert!(plan.entries[0].source_digest.is_some());
        assert!(plan.is_executable());
    }

    #[test]
    fn different_length_is_conflict_without_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old-value").unwrap();

        let plan = plan_for(&source, &destination, false, VerifyMode::Size, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(
            plan.entries[0].reason,
            "destination file has different contents"
        );
        assert!(plan.entries[0].blocking);
        assert_eq!(plan.entries[0].source_digest, None);
    }

    #[test]
    fn different_length_is_replace_with_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old-value").unwrap();

        let plan = plan_for(&source, &destination, true, VerifyMode::Hash, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Replace);
        assert_eq!(plan.entries[0].reason, "destination file will be replaced");
        assert!(!plan.entries[0].blocking);
        assert_eq!(
            plan.entries[0].source_digest.as_deref(),
            Some(blake3::hash(b"new").to_hex().as_str())
        );
    }

    #[test]
    fn type_conflict_blocks_even_with_replace() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"file").unwrap();
        fs::create_dir(&destination).unwrap();

        let plan = plan_for(&source, &destination, true, VerifyMode::Hash, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(
            plan.entries[0].reason,
            "source and destination object types differ"
        );
        assert!(plan.entries[0].blocking);
        assert!(!plan.is_executable());
    }

    #[test]
    fn unsupported_entry_blocks_by_default() {
        let plan = unsupported_plan(false);

        assert_eq!(plan.entries[0].disposition, Disposition::Unsupported);
        assert_eq!(
            plan.entries[0].reason,
            "source entry has unsupported fidelity"
        );
        assert!(plan.entries[0].blocking);
        assert!(!plan.is_executable());
    }

    #[test]
    fn ignored_unsupported_entry_remains_visible_and_non_blocking() {
        let plan = unsupported_plan(true);

        assert_eq!(plan.entries[0].disposition, Disposition::Unsupported);
        assert_eq!(
            plan.entries[0].reason,
            "unsupported source entry will be omitted"
        );
        assert!(!plan.entries[0].blocking);
        assert!(plan.is_executable());
    }

    #[test]
    fn plan_entries_remain_in_snapshot_relative_path_order() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("z.txt"), b"z").unwrap();
        fs::write(source.join("a.txt"), b"a").unwrap();

        let plan = plan_for(
            &source,
            &temp.path().join("destination"),
            false,
            VerifyMode::Hash,
            false,
        );
        let paths: Vec<_> = plan
            .entries
            .iter()
            .map(|entry| entry.relative_path.clone())
            .collect();

        assert_eq!(
            paths,
            vec![
                PathBuf::from(""),
                PathBuf::from("a.txt"),
                PathBuf::from("z.txt")
            ]
        );
        assert_eq!(plan.entries[0].reason, "destination does not exist");
        assert_eq!(plan.entries[1].reason, "destination does not exist");
        assert_eq!(plan.entries[2].reason, "destination does not exist");
    }

    #[test]
    fn entries_map_to_the_exact_destination_root() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("chosen-name");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("child.txt"), b"child").unwrap();

        let plan = plan_for(&source, &destination, false, VerifyMode::Hash, false);

        assert_eq!(plan.destination, destination);
        assert_eq!(plan.entries[0].destination, plan.destination);
        assert_eq!(
            plan.entries[1].destination,
            plan.destination.join("child.txt")
        );
        assert_eq!(plan.entries[0].reason, "destination does not exist");
        assert_eq!(plan.entries[1].reason, "destination does not exist");
    }

    #[test]
    fn matching_dangling_symbolic_link_targets_are_identical_without_traversal() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source-link");
        let destination = temp.path().join("destination-link");
        let missing_target = Path::new("missing-target");
        if create_file_link(missing_target, &source) == SymlinkCapability::PermissionDenied
            || create_file_link(missing_target, &destination) == SymlinkCapability::PermissionDenied
        {
            eprintln!("skipping symlink assertion: symbolic-link creation is not permitted");
            return;
        }

        let plan = plan_for(&source, &destination, false, VerifyMode::Hash, false);

        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].disposition, Disposition::SkipIdentical);
        assert_eq!(
            plan.entries[0].reason,
            "symbolic link targets are identical"
        );
        assert!(!plan.entries[0].blocking);
    }

    #[test]
    fn different_symbolic_link_target_is_conflict() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source-link");
        let destination = temp.path().join("destination-link");
        if create_file_link(Path::new("first-missing-target"), &source)
            == SymlinkCapability::PermissionDenied
            || create_file_link(Path::new("second-missing-target"), &destination)
                == SymlinkCapability::PermissionDenied
        {
            eprintln!("skipping symlink assertion: symbolic-link creation is not permitted");
            return;
        }

        let plan = plan_for(&source, &destination, true, VerifyMode::Hash, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Conflict);
        assert_eq!(
            plan.entries[0].reason,
            "destination symbolic link has a different target"
        );
        assert!(plan.entries[0].blocking);
    }

    #[test]
    fn planning_never_changes_or_deletes_a_read_only_destination() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"left").unwrap();
        fs::write(&destination, b"rite").unwrap();
        let original_permissions = fs::metadata(&destination).unwrap().permissions();
        let mut permissions = original_permissions.clone();
        permissions.set_readonly(true);
        fs::set_permissions(&destination, permissions).unwrap();

        let plan = plan_for(&source, &destination, true, VerifyMode::Hash, false);

        assert_eq!(plan.entries[0].disposition, Disposition::Replace);
        assert_eq!(plan.entries[0].reason, "destination file will be replaced");
        assert_eq!(fs::read(&destination).unwrap(), b"rite");
        assert!(fs::metadata(&destination).unwrap().permissions().readonly());

        fs::set_permissions(&destination, original_permissions).unwrap();
    }

    #[test]
    fn plan_serializes_the_public_contract_and_snake_case_disposition() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::write(&source, b"data").unwrap();
        let plan = plan_for(&source, &destination, false, VerifyMode::Hash, false);

        let value = serde_json::to_value(&plan).unwrap();

        assert_eq!(value["operation_id"].as_str().unwrap().len(), 24);
        assert_eq!(value["source"], source.to_string_lossy().as_ref());
        assert_eq!(value["destination"], destination.to_string_lossy().as_ref());
        assert_eq!(value["entries"][0]["relative_path"], "");
        assert_eq!(
            value["entries"][0]["source"],
            source.to_string_lossy().as_ref()
        );
        assert_eq!(
            value["entries"][0]["destination"],
            destination.to_string_lossy().as_ref()
        );
        assert_eq!(value["entries"][0]["kind"], "file");
        assert_eq!(value["entries"][0]["disposition"], "copy");
        assert_eq!(value["entries"][0]["length"], 4);
        assert_eq!(
            value["entries"][0]["source_digest"],
            blake3::hash(b"data").to_hex().as_str()
        );
        assert_eq!(value["entries"][0]["reason"], "destination does not exist");
        assert_eq!(value["entries"][0]["blocking"], false);
    }

    #[test]
    fn operation_id_is_deterministic_and_includes_planning_options() {
        let source = PathBuf::from("source");
        let destination = PathBuf::from("destination");
        let endpoints = ResolvedEndpoints {
            source: source.clone(),
            destination: destination.clone(),
        };
        let base = request_for(&source, &destination, false, VerifyMode::Hash, false);
        let replace = request_for(&source, &destination, true, VerifyMode::Hash, false);
        let size = request_for(&source, &destination, false, VerifyMode::Size, false);
        let ignore = request_for(&source, &destination, false, VerifyMode::Hash, true);

        let id = operation_id(&base, &endpoints);

        assert_eq!(id, operation_id(&base, &endpoints));
        assert_eq!(id.len(), 24);
        assert_ne!(id, operation_id(&replace, &endpoints));
        assert_ne!(id, operation_id(&size, &endpoints));
        assert_ne!(id, operation_id(&ignore, &endpoints));
    }

    #[cfg(windows)]
    #[test]
    fn operation_id_matches_stable_windows_utf16le_golden() {
        let source = PathBuf::from("source");
        let destination = PathBuf::from("destination");
        let request = request_for(&source, &destination, false, VerifyMode::Hash, false);
        let endpoints = ResolvedEndpoints {
            source,
            destination,
        };

        assert_eq!(
            operation_id(&request, &endpoints),
            "316bcd8b631e8e3a848046c9"
        );
    }

    #[cfg(unix)]
    #[test]
    fn operation_id_matches_stable_unix_bytes_golden() {
        let source = PathBuf::from("source");
        let destination = PathBuf::from("destination");
        let request = request_for(&source, &destination, false, VerifyMode::Hash, false);
        let endpoints = ResolvedEndpoints {
            source,
            destination,
        };

        assert_eq!(
            operation_id(&request, &endpoints),
            "d9f6d84708288c436eabf6ca"
        );
    }

    #[cfg(windows)]
    #[test]
    fn operation_id_matches_stable_windows_unpaired_surrogate_golden() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let first = PathBuf::from(OsString::from_wide(&[0xd800]));
        let second = PathBuf::from(OsString::from_wide(&[0xd801]));
        assert_eq!(first.to_string_lossy(), second.to_string_lossy());
        let destination = PathBuf::from("destination");
        let first_request = request_for(&first, &destination, false, VerifyMode::Hash, false);
        let second_request = request_for(&second, &destination, false, VerifyMode::Hash, false);
        let first_endpoints = ResolvedEndpoints {
            source: first,
            destination: destination.clone(),
        };
        let second_endpoints = ResolvedEndpoints {
            source: second,
            destination,
        };

        assert_eq!(
            operation_id(&first_request, &first_endpoints),
            "b56c5a0f05884fda575ddb54"
        );
        assert_ne!(
            operation_id(&first_request, &first_endpoints),
            operation_id(&second_request, &second_endpoints)
        );
    }

    #[cfg(unix)]
    #[test]
    fn operation_id_matches_stable_unix_non_unicode_golden() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let first = PathBuf::from(OsString::from_vec(vec![0x80]));
        let second = PathBuf::from(OsString::from_vec(vec![0x81]));
        assert_eq!(first.to_string_lossy(), second.to_string_lossy());
        let destination = PathBuf::from("destination");
        let first_request = request_for(&first, &destination, false, VerifyMode::Hash, false);
        let second_request = request_for(&second, &destination, false, VerifyMode::Hash, false);
        let first_endpoints = ResolvedEndpoints {
            source: first,
            destination: destination.clone(),
        };
        let second_endpoints = ResolvedEndpoints {
            source: second,
            destination,
        };

        assert_eq!(
            operation_id(&first_request, &first_endpoints),
            "ad63eb63f0de53d9c0a05436"
        );
        assert_ne!(
            operation_id(&first_request, &first_endpoints),
            operation_id(&second_request, &second_endpoints)
        );
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum SymlinkCapability {
        Created,
        PermissionDenied,
    }

    #[cfg(unix)]
    fn create_file_link(target: &Path, link: &Path) -> SymlinkCapability {
        std::os::unix::fs::symlink(target, link).unwrap();
        SymlinkCapability::Created
    }

    #[cfg(windows)]
    fn create_file_link(target: &Path, link: &Path) -> SymlinkCapability {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => SymlinkCapability::Created,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                SymlinkCapability::PermissionDenied
            }
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }
}
