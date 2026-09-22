use std::fs;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

#[cfg(windows)]
use windows_sys::Win32::Globalization::{CSTR_EQUAL, CompareStringOrdinal};

use crate::error::{OrbitError, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedEndpoints {
    pub source: PathBuf,
    pub destination: PathBuf,
}

pub fn resolve_endpoints(
    source: &Path,
    destination: &Path,
    cwd: &Path,
) -> Result<ResolvedEndpoints> {
    let source_input = absolutize(source, cwd);
    let source_metadata = match fs::symlink_metadata(&source_input) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(OrbitError::SourceMissing(source_input));
        }
        Err(source_error) => {
            return Err(OrbitError::Io {
                operation: "inspect source endpoint",
                path: source_input,
                source: source_error,
            });
        }
    };
    let source = resolve_leaf(&source_input)?;
    let destination = resolve_leaf(&absolutize(destination, cwd))?;

    if paths_equal(&source, &destination) {
        return Err(OrbitError::SameEndpoint(source));
    }
    if source_metadata.file_type().is_dir() && is_descendant(&destination, &source) {
        return Err(OrbitError::DestinationInsideSource { destination });
    }
    Ok(ResolvedEndpoints {
        source,
        destination,
    })
}

fn absolutize(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}

fn resolve_leaf(path: &Path) -> Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return resolve_ancestor(path);
    };
    let parent = path.parent().expect("a path with a file name has a parent");
    Ok(resolve_ancestor(parent)?.join(name))
}

fn resolve_ancestor(path: &Path) -> Result<PathBuf> {
    let mut resolved = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => resolved.push(prefix.as_os_str()),
            std::path::Component::RootDir => resolved.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            std::path::Component::Normal(name) => {
                let candidate = resolved.join(name);
                match fs::symlink_metadata(&candidate) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        resolved = canonicalize(&candidate, "resolve endpoint symlink")?;
                    }
                    Ok(_) => resolved = candidate,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        resolved.push(name);
                    }
                    Err(source) => {
                        return Err(OrbitError::Io {
                            operation: "inspect endpoint ancestor",
                            path: candidate,
                            source,
                        });
                    }
                }
            }
        }
    }
    resolve_nearest_existing_ancestor(&resolved)
}

fn resolve_nearest_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor
                    .file_name()
                    .ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
                suffix.push(name.to_os_string());
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
            }
            Err(source) => {
                return Err(OrbitError::Io {
                    operation: "inspect endpoint ancestor",
                    path: ancestor.to_path_buf(),
                    source,
                });
            }
        }
    }

    let mut resolved = canonicalize(ancestor, "resolve endpoint ancestor")?;
    for name in suffix.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

fn canonicalize(path: &Path, operation: &'static str) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|source| OrbitError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    })
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    let mut left_components = left.components();
    let mut right_components = right.components();
    loop {
        match (left_components.next(), right_components.next()) {
            (Some(left), Some(right)) if components_equal(left.as_os_str(), right.as_os_str()) => {}
            (None, None) => return true,
            _ => return false,
        }
    }
}

fn is_descendant(candidate: &Path, parent: &Path) -> bool {
    let mut candidate_components = candidate.components();
    for parent_component in parent.components() {
        let Some(candidate_component) = candidate_components.next() else {
            return false;
        };
        if !components_equal(
            candidate_component.as_os_str(),
            parent_component.as_os_str(),
        ) {
            return false;
        }
    }
    candidate_components.next().is_some()
}

#[cfg(not(windows))]
fn components_equal(left: &std::ffi::OsStr, right: &std::ffi::OsStr) -> bool {
    left == right
}

#[cfg(windows)]
fn components_equal(left: &OsStr, right: &OsStr) -> bool {
    let left: Vec<u16> = left.encode_wide().collect();
    let right: Vec<u16> = right.encode_wide().collect();
    let (Ok(left_len), Ok(right_len)) = (i32::try_from(left.len()), i32::try_from(right.len()))
    else {
        return false;
    };

    // The UTF-16 slices remain valid for the call, and the API receives their exact lengths.
    unsafe {
        CompareStringOrdinal(left.as_ptr(), left_len, right.as_ptr(), right_len, 1) == CSTR_EQUAL
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_endpoints;
    use crate::error::OrbitError;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn resolves_a_nonexistent_exact_destination() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").unwrap();
        let result =
            resolve_endpoints(&source, &temp.path().join("new/target.txt"), temp.path()).unwrap();
        assert!(result.source.is_absolute());
        assert_eq!(
            result.destination,
            fs::canonicalize(temp.path())
                .unwrap()
                .join("new/target.txt")
        );
    }

    #[test]
    fn rejects_the_same_endpoint() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("same");
        fs::write(&source, b"source").unwrap();
        assert!(matches!(
            resolve_endpoints(&source, &source, temp.path()),
            Err(OrbitError::SameEndpoint(_))
        ));
    }

    #[test]
    fn rejects_destination_inside_source_directory() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        assert!(matches!(
            resolve_endpoints(&source, &source.join("nested/copy"), temp.path()),
            Err(OrbitError::DestinationInsideSource { .. })
        ));
    }

    #[test]
    fn resolution_preserves_a_final_symlink_instead_of_following_it() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");
        fs::write(&target, b"target").unwrap();
        if !create_file_link(&target, &link) {
            return;
        }
        let resolved =
            resolve_endpoints(&link, &temp.path().join("copy.txt"), temp.path()).unwrap();
        let resolved_parent = fs::canonicalize(temp.path()).unwrap();
        assert_eq!(resolved.source, resolved_parent.join("link.txt"));
        assert_ne!(resolved.source, resolved_parent.join("target.txt"));
    }

    #[test]
    fn resolves_parent_components_after_symlinks_before_checking_containment() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("b/source");
        let link = temp.path().join("a/link");
        fs::create_dir_all(source.join("deep")).unwrap();
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        if !create_directory_link(&source.join("deep"), &link) {
            return;
        }
        assert!(matches!(
            resolve_endpoints(&link.join(".."), &source.join("nested/copy"), temp.path(),),
            Err(OrbitError::DestinationInsideSource { .. })
        ));
    }

    #[cfg(unix)]
    fn create_file_link(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(unix)]
    fn create_directory_link(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn create_directory_link(target: &std::path::Path, link: &std::path::Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }

    #[cfg(windows)]
    fn create_file_link(target: &std::path::Path, link: &std::path::Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn rejects_the_same_windows_endpoint_with_different_case() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("MixedCase.txt");
        fs::write(&source, b"source").unwrap();
        let differently_cased = temp.path().join("MIXEDCASE.TXT");
        assert!(matches!(
            resolve_endpoints(&source, &differently_cased, temp.path()),
            Err(OrbitError::SameEndpoint(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn preserves_windows_extended_canonical_prefixes() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("source.txt");
        fs::write(&source, b"source").unwrap();
        let canonical_parent = fs::canonicalize(temp.path()).unwrap();
        assert!(canonical_parent.to_string_lossy().starts_with(r"\\?\"));

        let resolved =
            resolve_endpoints(&source, &temp.path().join("new/target.txt"), temp.path()).unwrap();
        assert_eq!(resolved.source, canonical_parent.join("source.txt"));
        assert_eq!(
            resolved.destination,
            canonical_parent.join("new/target.txt")
        );
    }

    #[cfg(windows)]
    #[test]
    fn rejects_the_same_windows_endpoint_with_non_ascii_case() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("MÜNCHEN.TXT");
        fs::write(&source, b"source").unwrap();
        let differently_cased = temp.path().join("münchen.txt");
        assert!(matches!(
            resolve_endpoints(&source, &differently_cased, temp.path()),
            Err(OrbitError::SameEndpoint(_))
        ));
    }
}
