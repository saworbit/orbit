use std::path::{Path, PathBuf};

use crate::error::{OrbitError, Result};
use crate::scan::EntryKind;

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
