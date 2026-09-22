use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{OrbitError, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedFeature {
    SpecialFile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceEntry {
    pub relative_path: PathBuf,
    pub kind: EntryKind,
    pub length: u64,
    pub modified_ns: Option<i128>,
    pub read_only: bool,
    pub link_target: Option<PathBuf>,
    pub unsupported: Vec<UnsupportedFeature>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSnapshot {
    pub root: PathBuf,
    pub entries: Vec<SourceEntry>,
}

pub fn scan_source(root: &Path) -> Result<SourceSnapshot> {
    let mut entries = Vec::new();
    visit(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(SourceSnapshot {
        root: root.to_path_buf(),
        entries,
    })
}

fn visit(root: &Path, path: &Path, entries: &mut Vec<SourceEntry>) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| OrbitError::Io {
        operation: "inspect source",
        path: path.to_path_buf(),
        source,
    })?;
    let file_type = metadata.file_type();
    let relative_path = path
        .strip_prefix(root)
        .expect("entry is beneath scan root")
        .to_path_buf();
    let kind = if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_file() {
        EntryKind::File
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::Unsupported
    };
    let unsupported = if kind == EntryKind::Unsupported {
        vec![UnsupportedFeature::SpecialFile]
    } else {
        Vec::new()
    };
    let link_target = if kind == EntryKind::Symlink {
        Some(std::fs::read_link(path).map_err(|source| OrbitError::Io {
            operation: "read symbolic link",
            path: path.to_path_buf(),
            source,
        })?)
    } else {
        None
    };
    entries.push(SourceEntry {
        relative_path,
        kind,
        length: if kind == EntryKind::File {
            metadata.len()
        } else {
            0
        },
        modified_ns: metadata.modified().ok().and_then(system_time_ns),
        read_only: metadata.permissions().readonly(),
        link_target,
        unsupported,
    });
    if kind == EntryKind::Directory {
        let mut children = std::fs::read_dir(path)
            .map_err(|source| OrbitError::Io {
                operation: "read source directory",
                path: path.to_path_buf(),
                source,
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|source| OrbitError::Io {
                operation: "read source entry",
                path: path.to_path_buf(),
                source,
            })?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            visit(root, &child.path(), entries)?;
        }
    }
    Ok(())
}

fn system_time_ns(value: std::time::SystemTime) -> Option<i128> {
    value
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|duration| {
            i128::from(duration.as_secs()) * 1_000_000_000 + i128::from(duration.subsec_nanos())
        })
}

#[cfg(test)]
mod tests {
    use super::{EntryKind, scan_source};
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn returns_entries_in_relative_path_order() {
        let temp = tempdir().unwrap();
        fs::create_dir(temp.path().join("nested")).unwrap();
        fs::write(temp.path().join("nested/b.txt"), b"b").unwrap();
        fs::write(temp.path().join("a.txt"), b"a").unwrap();

        let snapshot = scan_source(temp.path()).unwrap();
        let paths: Vec<_> = snapshot
            .entries
            .into_iter()
            .map(|entry| entry.relative_path)
            .collect();
        assert_eq!(
            paths,
            vec![
                PathBuf::from(""),
                PathBuf::from("a.txt"),
                PathBuf::from("nested"),
                PathBuf::from("nested/b.txt"),
            ]
        );
    }

    #[test]
    fn records_a_symlink_without_following_its_target() {
        let temp = tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inside.txt"), b"inside").unwrap();
        if !create_directory_link(&target, &link) {
            return;
        }

        let snapshot = scan_source(&link).unwrap();
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].kind, EntryKind::Symlink);
        assert_eq!(
            snapshot.entries[0].link_target.as_deref(),
            Some(target.as_path())
        );
    }

    #[cfg(unix)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        std::os::unix::fs::symlink(target, link).unwrap();
        true
    }

    #[cfg(windows)]
    fn create_directory_link(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => true,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("cannot create test symlink: {error}"),
        }
    }
}
