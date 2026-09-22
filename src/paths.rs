use std::fs;
use std::path::{Component, Path, PathBuf};

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

    if path_key(&source) == path_key(&destination) {
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
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn resolve_leaf(path: &Path) -> Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return canonicalize(path, "resolve filesystem root");
    };
    let parent = path.parent().expect("a path with a file name has a parent");
    Ok(resolve_ancestor(parent)?.join(name))
}

fn resolve_ancestor(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
        suffix.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| OrbitError::SourceMissing(path.to_path_buf()))?;
    }
    let mut resolved = canonicalize(ancestor, "resolve endpoint ancestor")?;
    for name in suffix.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

fn canonicalize(path: &Path, operation: &'static str) -> Result<PathBuf> {
    fs::canonicalize(path)
        .map(normalize_canonical_path)
        .map_err(|source| OrbitError::Io {
            operation,
            path: path.to_path_buf(),
            source,
        })
}

fn normalize_canonical_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        let Some(value) = value.strip_prefix(r"\\?\") else {
            return path;
        };
        if let Some(unc) = value.strip_prefix("UNC\\") {
            PathBuf::from(format!(r"\\{unc}"))
        } else {
            PathBuf::from(value)
        }
    }
    #[cfg(not(windows))]
    {
        path
    }
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value.into_owned()
    }
}

fn is_descendant(candidate: &Path, parent: &Path) -> bool {
    let candidate_components: Vec<_> = candidate
        .components()
        .map(|c| path_key(Path::new(c.as_os_str())))
        .collect();
    let parent_components: Vec<_> = parent
        .components()
        .map(|c| path_key(Path::new(c.as_os_str())))
        .collect();
    candidate_components.len() > parent_components.len()
        && candidate_components.starts_with(&parent_components)
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
        assert_eq!(result.destination, temp.path().join("new/target.txt"));
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
        assert_eq!(resolved.source, link);
        assert_ne!(resolved.source, target);
    }

    #[cfg(unix)]
    fn create_file_link(target: &std::path::Path, link: &std::path::Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
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
}
