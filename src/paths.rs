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
    let source_input = absolutize(source, cwd)?;
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
    let destination = resolve_leaf(&absolutize(destination, cwd)?)?;

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

fn absolutize(path: &Path, cwd: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        #[cfg(windows)]
        let cwd = ordinary_windows_cwd(cwd)?;
        cwd.join(path)
    };
    #[cfg(windows)]
    {
        // Rust 1.85 uses GetFullPathNameW for ordinary Windows paths, normalizing
        // dots, parents and trailing-dot/space aliases before filesystem lookup.
        // Explicit verbatim paths are returned unchanged by std::path::absolute.
        std::path::absolute(&path).map_err(|source| OrbitError::Io {
            operation: "normalize endpoint",
            path,
            source,
        })
    }
    #[cfg(not(windows))]
    Ok(path)
}

#[cfg(windows)]
fn ordinary_windows_cwd(cwd: &Path) -> Result<PathBuf> {
    use std::ffi::OsString;
    use std::path::{Component, Prefix};

    let mut components = cwd.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return Ok(cwd.to_path_buf());
    };
    // A canonical CWD must not turn an ordinary relative endpoint into a
    // verbatim input before GetFullPathNameW can normalize it. This spelling is
    // only the normalization base; canonical endpoint prefixes stay intact.
    let mut ordinary = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => OsString::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => {
            let mut value = OsString::from(r"\\");
            value.push(server);
            value.push(r"\");
            value.push(share);
            value
        }
        _ => return Ok(cwd.to_path_buf()),
    };
    ordinary.push(components.as_path());
    let ordinary = PathBuf::from(ordinary);
    let same_directory = fs::metadata(cwd).is_ok_and(|metadata| metadata.is_dir())
        && match (fs::canonicalize(cwd), fs::canonicalize(&ordinary)) {
            (Ok(verbatim), Ok(roundtrip)) => paths_equal(&verbatim, &roundtrip),
            _ => false,
        };
    if !same_directory {
        return Err(OrbitError::Io {
            operation: "normalize working directory",
            path: cwd.to_path_buf(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "verbatim working directory has no equivalent ordinary directory path",
            ),
        });
    }
    Ok(ordinary)
}

fn resolve_leaf(path: &Path) -> Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return resolve_ancestor(path);
    };
    let parent = path.parent().expect("a path with a file name has a parent");
    Ok(resolve_ancestor(parent)?.join(name))
}

#[cfg(windows)]
fn resolve_ancestor(path: &Path) -> Result<PathBuf> {
    // Ordinary inputs have already received Win32 lexical normalization. For
    // verbatim inputs, pass components unchanged to the filesystem as Rust does.
    resolve_nearest_existing_ancestor(path)
}

#[cfg(not(windows))]
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

    #[cfg(unix)]
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

    #[cfg(windows)]
    #[test]
    fn ordinary_windows_trailing_aliases_reject_the_same_endpoint() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("same");
        fs::write(&source, b"same endpoint").unwrap();
        for name in ["same.", "same ", "same. "] {
            let alias = temp.path().join(name);
            assert_eq!(fs::read(&alias).unwrap(), b"same endpoint");
            assert!(
                matches!(
                    resolve_endpoints(&source, &alias, temp.path()),
                    Err(OrbitError::SameEndpoint(_))
                ),
                "ordinary alias {name:?} bypassed same-endpoint rejection"
            );
            assert!(matches!(
                resolve_endpoints(&alias, &source, temp.path()),
                Err(OrbitError::SameEndpoint(_))
            ));
        }
    }

    #[cfg(windows)]
    #[test]
    fn ordinary_relative_windows_alias_stays_ordinary_with_a_canonical_cwd() {
        let temp = tempdir().unwrap();
        let source = temp.path().join("same");
        fs::write(&source, b"same").unwrap();
        let cwd = fs::canonicalize(temp.path()).unwrap();
        assert!(matches!(
            resolve_endpoints(&source, std::path::Path::new("same."), &cwd),
            Err(OrbitError::SameEndpoint(_))
        ));
    }

    #[cfg(windows)]
    #[test]
    fn relative_windows_endpoint_rejects_a_verbatim_cwd_with_an_ordinary_alias() {
        let temp = tempdir().unwrap();
        let parent = fs::canonicalize(temp.path()).unwrap();
        let literal_directory = parent.join("literal.");
        let alternate_directory = temp.path().join("literal");
        fs::create_dir(&literal_directory).unwrap();
        fs::create_dir(&alternate_directory).unwrap();
        fs::write(literal_directory.join("child.txt"), b"literal").unwrap();
        fs::write(alternate_directory.join("child.txt"), b"alternate").unwrap();
        let cwd = fs::canonicalize(&literal_directory).unwrap();

        let result = resolve_endpoints(
            std::path::Path::new("child.txt"),
            &parent.join("copy.txt"),
            &cwd,
        );
        match result {
            Err(OrbitError::Io {
                operation,
                path,
                source,
            }) => {
                assert_eq!(operation, "normalize working directory");
                assert_eq!(path, cwd);
                assert_eq!(source.kind(), std::io::ErrorKind::InvalidInput);
            }
            other => panic!("unsafe verbatim CWD was not rejected: {other:?}"),
        }

        let explicit = resolve_endpoints(
            &literal_directory.join("child.txt"),
            &parent.join("copy.txt"),
            &cwd,
        )
        .unwrap();
        assert_eq!(fs::read(explicit.source).unwrap(), b"literal");
        assert_eq!(
            fs::read(alternate_directory.join("child.txt")).unwrap(),
            b"alternate"
        );
    }

    #[cfg(windows)]
    #[test]
    fn ordinary_windows_link_parent_matches_filesystem_resolution() {
        let temp = tempdir().unwrap();
        let local = temp.path().join("local");
        let remote = temp.path().join("remote");
        fs::create_dir_all(remote.join("deep")).unwrap();
        fs::create_dir(&local).unwrap();
        fs::write(local.join("child.txt"), b"local").unwrap();
        fs::write(remote.join("child.txt"), b"remote").unwrap();
        if !create_directory_link(&remote.join("deep"), &local.join("link")) {
            return;
        }
        let source = local.join("link/../child.txt");
        assert_eq!(fs::read(&source).unwrap(), b"local");
        let resolved = resolve_endpoints(&source, &temp.path().join("copy"), temp.path()).unwrap();
        assert_eq!(fs::read(&resolved.source).unwrap(), b"local");
        assert_eq!(
            resolved.source,
            fs::canonicalize(local.join("child.txt")).unwrap()
        );
        assert!(matches!(
            resolve_endpoints(
                &local.join("link/.."),
                &local.join("nested/copy"),
                temp.path()
            ),
            Err(OrbitError::DestinationInsideSource { .. })
        ));
    }

    #[cfg(windows)]
    #[test]
    fn explicitly_verbatim_windows_leaf_keeps_literal_trailing_characters() {
        let temp = tempdir().unwrap();
        let ordinary = temp.path().join("literal");
        fs::write(&ordinary, b"ordinary").unwrap();
        let canonical_parent = fs::canonicalize(temp.path()).unwrap();
        for name in ["literal.", "literal "] {
            let literal = canonical_parent.join(name);
            // NTFS permits these literal names under the extended prefix.
            fs::write(&literal, b"verbatim").unwrap();
            assert_eq!(fs::read(&literal).unwrap(), b"verbatim");
            assert_eq!(fs::read(temp.path().join(name)).unwrap(), b"ordinary");
            let resolved = resolve_endpoints(&ordinary, &literal, temp.path()).unwrap();
            assert_eq!(resolved.destination, literal);
            assert_ne!(resolved.source, resolved.destination);
        }
    }

    #[cfg(windows)]
    #[test]
    fn explicitly_verbatim_windows_components_are_not_lexically_collapsed() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("directory")).unwrap();
        let mut raw = fs::canonicalize(temp.path()).unwrap().into_os_string();
        // OsString::push preserves the original components; PathBuf::push can
        // itself normalize relative components when its base is verbatim.
        raw.push(r"\directory\..\copy");
        let input = std::path::PathBuf::from(raw);
        assert_eq!(
            super::absolutize(&input, temp.path()).unwrap().as_os_str(),
            input.as_os_str()
        );
        let resolved = resolve_endpoints(&temp.path().join("directory"), &input, temp.path());
        // NTFS rejects literal dot components under the verbatim prefix.
        let parent_error = fs::symlink_metadata(input.parent().unwrap()).unwrap_err();
        match resolved {
            Err(OrbitError::Io { source, .. }) => assert_eq!(source.kind(), parent_error.kind()),
            Err(OrbitError::SourceMissing(_))
                if parent_error.kind() == std::io::ErrorKind::NotFound => {}
            other => panic!("verbatim parent components were normalized: {other:?}"),
        }
    }
}
