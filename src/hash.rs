use std::path::Path;

use crate::error::{OrbitError, Result};

pub fn hash_file(path: &Path) -> Result<blake3::Hash> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(|source| OrbitError::Io {
        operation: "open file for hashing",
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|source| OrbitError::Io {
            operation: "hash file",
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::hash_file;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn hashes_the_complete_file() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("data");
        let bytes = vec![0x5a; 128 * 1024 + 17];
        fs::write(&path, &bytes).unwrap();
        assert_eq!(hash_file(&path).unwrap(), blake3::hash(&bytes));
    }
}
