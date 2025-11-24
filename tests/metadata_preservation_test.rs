/*!
 * Integration tests for metadata preservation functionality
 *
 * Tests both default metadata support (timestamps, permissions) and
 * feature-gated extended metadata (xattrs, ownership) when the
 * `extended-metadata` feature is enabled.
 */

use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

// Import orbit's metadata types
use orbit::core::file_metadata::{FileMetadata, PreserveFlags};
use orbit::core::metadata_ops::{verify_metadata, MetadataPreserver};

/// Test that basic metadata (timestamps, permissions) is preserved by default
#[test]
fn test_basic_metadata_preservation() {
    let src_dir = tempdir().unwrap();
    let dest_dir = tempdir().unwrap();

    let src_file = src_dir.path().join("test.txt");
    let dest_file = dest_dir.path().join("test.txt");

    // Create source file with content
    {
        let mut file = File::create(&src_file).unwrap();
        file.write_all(b"Hello, Orbit!").unwrap();
        file.flush().unwrap();
    }

    // Set custom permissions on Unix
    #[cfg(unix)]
    {
        fs::set_permissions(&src_file, fs::Permissions::from_mode(0o644)).unwrap();
    }

    // Copy the file
    fs::copy(&src_file, &dest_file).unwrap();

    // Preserve metadata using default flags
    let preserver = MetadataPreserver::new();
    preserver.preserve_local(&src_file, &dest_file).unwrap();

    // Verify timestamps were preserved
    let src_meta = fs::metadata(&src_file).unwrap();
    let dest_meta = fs::metadata(&dest_file).unwrap();

    assert_eq!(
        src_meta.modified().unwrap(),
        dest_meta.modified().unwrap(),
        "Modification time should be preserved"
    );

    // Verify permissions on Unix
    #[cfg(unix)]
    {
        assert_eq!(
            src_meta.permissions().mode() & 0o777,
            dest_meta.permissions().mode() & 0o777,
            "Permissions should be preserved"
        );
    }
}

/// Test PreserveFlags parsing from string
#[test]
fn test_preserve_flags_parsing() {
    // Test basic flags
    let flags = PreserveFlags::from_str("times,perms").unwrap();
    assert!(flags.times, "times flag should be set");
    assert!(flags.permissions, "perms flag should be set");
    assert!(!flags.ownership, "ownership flag should not be set");
    assert!(!flags.xattrs, "xattrs flag should not be set");

    // Test 'all' flag
    let flags_all = PreserveFlags::from_str("all").unwrap();
    assert!(flags_all.times, "all should set times");
    assert!(flags_all.permissions, "all should set permissions");
    assert!(flags_all.ownership, "all should set ownership");
    assert!(flags_all.xattrs, "all should set xattrs");

    // Test alternative flag names
    let flags_alt = PreserveFlags::from_str("timestamps,mode,owner,extended").unwrap();
    assert!(flags_alt.times, "timestamps should set times");
    assert!(flags_alt.permissions, "mode should set permissions");
    assert!(flags_alt.ownership, "owner should set ownership");
    assert!(flags_alt.xattrs, "extended should set xattrs");

    // Test invalid flag
    let result = PreserveFlags::from_str("invalid_flag");
    assert!(result.is_err(), "invalid flag should return error");
}

/// Test FileMetadata extraction from path
#[test]
fn test_file_metadata_extraction() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");

    {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test content").unwrap();
        file.flush().unwrap();
    }

    let metadata = FileMetadata::from_path(&file_path).unwrap();

    assert_eq!(metadata.size, 12, "File size should be 12 bytes");
    assert!(metadata.is_file, "Should be identified as a file");
    assert!(!metadata.is_dir, "Should not be identified as a directory");
    assert!(
        metadata.modified.is_some(),
        "Modified time should be present"
    );

    #[cfg(unix)]
    {
        assert!(
            metadata.permissions.is_some(),
            "Permissions should be present on Unix"
        );
        assert!(
            metadata.owner_uid.is_some(),
            "UID should be present on Unix"
        );
        assert!(
            metadata.owner_gid.is_some(),
            "GID should be present on Unix"
        );
    }
}

/// Test metadata verification function
#[test]
fn test_metadata_verification() {
    let src_dir = tempdir().unwrap();
    let dest_dir = tempdir().unwrap();

    let src_file = src_dir.path().join("test.txt");
    let dest_file = dest_dir.path().join("test.txt");

    // Create source and destination files
    {
        let mut file = File::create(&src_file).unwrap();
        file.write_all(b"test").unwrap();
        file.flush().unwrap();
    }
    fs::copy(&src_file, &dest_file).unwrap();

    // Preserve metadata
    let preserver = MetadataPreserver::new();
    preserver.preserve_local(&src_file, &dest_file).unwrap();

    // Verify metadata matches
    let matches = verify_metadata(&src_file, &dest_file, PreserveFlags::basic()).unwrap();
    assert!(matches, "Metadata should match after preservation");
}

/// Test directory metadata preservation
#[test]
fn test_directory_metadata() {
    let dir = tempdir().unwrap();
    let test_dir = dir.path().join("subdir");
    fs::create_dir(&test_dir).unwrap();

    let metadata = FileMetadata::from_path(&test_dir).unwrap();

    assert!(
        !metadata.is_file,
        "Directory should not be identified as file"
    );
    assert!(metadata.is_dir, "Should be identified as directory");
}

/// Test manifest conversion round-trip
#[test]
fn test_manifest_conversion_roundtrip() {
    let mut original = FileMetadata::default();
    original.permissions = Some(0o755);
    original.owner_uid = Some(1000);
    original.owner_gid = Some(1000);
    original.size = 12345;
    original.is_file = true;

    // Convert to manifest format
    let map = original.to_manifest_map();

    // Convert back
    let restored = FileMetadata::from_manifest_map(&map).unwrap();

    assert_eq!(
        restored.permissions,
        Some(0o755),
        "Permissions should round-trip"
    );
    assert_eq!(restored.owner_uid, Some(1000), "UID should round-trip");
    assert_eq!(restored.owner_gid, Some(1000), "GID should round-trip");
}

/// Test that MetadataPreserver respects strict mode
#[test]
fn test_strict_mode_configuration() {
    let preserver = MetadataPreserver::new()
        .with_flags(PreserveFlags::all())
        .strict(true);

    assert!(preserver.strict, "Strict mode should be enabled");
    assert!(preserver.preserve_flags.times, "times should be set");
    assert!(
        preserver.preserve_flags.permissions,
        "permissions should be set"
    );
    assert!(
        preserver.preserve_flags.ownership,
        "ownership should be set"
    );
    assert!(preserver.preserve_flags.xattrs, "xattrs should be set");
}

// ============================================================================
// Extended Metadata Tests (feature-gated)
// ============================================================================

/// Test xattr preservation when extended-metadata feature is enabled
/// This test only runs when the feature is enabled and on Unix-like systems
#[cfg(all(feature = "extended-metadata", unix))]
mod extended_metadata_tests {
    use super::*;
    use std::collections::HashMap;

    /// Test that xattrs can be read from a file
    #[test]
    fn test_xattr_read() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_xattr.txt");

        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"test").unwrap();
            file.flush().unwrap();
        }

        // Set an xattr (user namespace on Linux)
        if let Err(e) = xattr::set(&file_path, "user.orbit.test", b"test_value") {
            // Skip if xattrs not supported on this filesystem
            eprintln!(
                "Skipping xattr test: filesystem may not support xattrs: {}",
                e
            );
            return;
        }

        // Read metadata with xattrs
        let metadata = FileMetadata::from_path(&file_path).unwrap();

        assert!(
            metadata.xattrs.is_some(),
            "xattrs should be populated when extended-metadata feature is enabled"
        );

        let xattrs = metadata.xattrs.unwrap();
        assert!(
            xattrs.contains_key("user.orbit.test"),
            "xattr should be present"
        );
        assert_eq!(
            xattrs.get("user.orbit.test").unwrap(),
            b"test_value",
            "xattr value should match"
        );
    }

    /// Test xattr preservation during copy
    #[test]
    fn test_xattr_preservation() {
        let src_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();

        let src_file = src_dir.path().join("source.txt");
        let dest_file = dest_dir.path().join("dest.txt");

        // Create source file
        {
            let mut file = File::create(&src_file).unwrap();
            file.write_all(b"Hello, xattrs!").unwrap();
            file.flush().unwrap();
        }

        // Set xattr on source
        if let Err(e) = xattr::set(&src_file, "user.orbit.preserve", b"preserved_value") {
            eprintln!("Skipping xattr preservation test: {}", e);
            return;
        }

        // Copy the file
        fs::copy(&src_file, &dest_file).unwrap();

        // Preserve metadata with xattrs enabled
        let preserver = MetadataPreserver::new().with_flags(PreserveFlags::all());
        preserver.preserve_local(&src_file, &dest_file).unwrap();

        // Verify xattr was preserved
        match xattr::get(&dest_file, "user.orbit.preserve") {
            Ok(Some(value)) => {
                assert_eq!(value, b"preserved_value", "xattr value should be preserved");
            }
            Ok(None) => {
                panic!("xattr was not preserved to destination");
            }
            Err(e) => {
                eprintln!("Could not read xattr from destination: {}", e);
            }
        }
    }

    /// Test multiple xattrs preservation
    #[test]
    fn test_multiple_xattrs() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("multi_xattr.txt");

        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"test").unwrap();
            file.flush().unwrap();
        }

        // Set multiple xattrs
        let xattrs_to_set = vec![
            ("user.orbit.attr1", b"value1".to_vec()),
            ("user.orbit.attr2", b"value2".to_vec()),
            ("user.orbit.binary", vec![0x00, 0xFF, 0x42]),
        ];

        for (name, value) in &xattrs_to_set {
            if let Err(e) = xattr::set(&file_path, name, value) {
                eprintln!("Skipping multiple xattrs test: {}", e);
                return;
            }
        }

        let metadata = FileMetadata::from_path(&file_path).unwrap();
        let xattrs = metadata.xattrs.unwrap();

        assert!(xattrs.len() >= 3, "Should have at least 3 xattrs");
        assert_eq!(xattrs.get("user.orbit.attr1").unwrap(), b"value1");
        assert_eq!(xattrs.get("user.orbit.attr2").unwrap(), b"value2");
        assert_eq!(
            xattrs.get("user.orbit.binary").unwrap(),
            &vec![0x00, 0xFF, 0x42]
        );
    }

    /// Test xattr manifest round-trip with base64 encoding
    #[test]
    fn test_xattr_manifest_roundtrip() {
        let mut metadata = FileMetadata::default();
        let mut xattrs = HashMap::new();
        xattrs.insert("user.test".to_string(), b"value".to_vec());
        xattrs.insert("user.binary".to_string(), vec![0x00, 0xFF]);
        metadata.xattrs = Some(xattrs);

        let map = metadata.to_manifest_map();
        let restored = FileMetadata::from_manifest_map(&map).unwrap();

        assert!(restored.xattrs.is_some(), "xattrs should be restored");
        let restored_xattrs = restored.xattrs.unwrap();
        assert_eq!(
            restored_xattrs.get("user.test").unwrap(),
            b"value",
            "text xattr should round-trip"
        );
        assert_eq!(
            restored_xattrs.get("user.binary").unwrap(),
            &vec![0x00, 0xFF],
            "binary xattr should round-trip"
        );
    }
}

/// Test that verifies the feature flag behavior
/// When extended-metadata is NOT enabled, xattrs should not be populated
#[cfg(not(feature = "extended-metadata"))]
#[test]
fn test_xattrs_not_populated_without_feature() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");

    {
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"test").unwrap();
        file.flush().unwrap();
    }

    let metadata = FileMetadata::from_path(&file_path).unwrap();

    // Without the extended-metadata feature, xattrs should always be None
    assert!(
        metadata.xattrs.is_none(),
        "xattrs should be None when extended-metadata feature is not enabled"
    );
}

// ============================================================================
// Unix-specific ownership tests
// ============================================================================

#[cfg(unix)]
mod unix_ownership_tests {
    use super::*;

    /// Test that ownership information is extracted on Unix
    #[test]
    fn test_ownership_extraction() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"test").unwrap();
            file.flush().unwrap();
        }

        let metadata = FileMetadata::from_path(&file_path).unwrap();

        assert!(
            metadata.owner_uid.is_some(),
            "UID should be extracted on Unix"
        );
        assert!(
            metadata.owner_gid.is_some(),
            "GID should be extracted on Unix"
        );

        // Verify the UID/GID match the current user (who created the file)
        let fs_meta = fs::metadata(&file_path).unwrap();
        assert_eq!(
            metadata.owner_uid.unwrap(),
            fs_meta.uid(),
            "UID should match filesystem metadata"
        );
        assert_eq!(
            metadata.owner_gid.unwrap(),
            fs_meta.gid(),
            "GID should match filesystem metadata"
        );
    }

    /// Test that ownership preservation requires privileges
    /// This test verifies the graceful failure when not running as root
    #[test]
    fn test_ownership_preservation_requires_privileges() {
        // Skip if running as root (ownership changes would succeed)
        if unsafe { libc::geteuid() } == 0 {
            eprintln!("Skipping: test requires non-root user");
            return;
        }

        let src_dir = tempdir().unwrap();
        let dest_dir = tempdir().unwrap();

        let src_file = src_dir.path().join("source.txt");
        let dest_file = dest_dir.path().join("dest.txt");

        {
            let mut file = File::create(&src_file).unwrap();
            file.write_all(b"test").unwrap();
            file.flush().unwrap();
        }
        fs::copy(&src_file, &dest_file).unwrap();

        // Try to preserve ownership (should warn but not fail in non-strict mode)
        let preserver = MetadataPreserver::new()
            .with_flags(PreserveFlags::all())
            .strict(false);

        let result = preserver.preserve_local(&src_file, &dest_file);
        assert!(
            result.is_ok(),
            "Should succeed with warning in non-strict mode"
        );
    }
}

// ============================================================================
// Windows-specific tests
// ============================================================================

#[cfg(windows)]
mod windows_tests {
    use super::*;

    /// Test that Windows file attributes are extracted
    #[test]
    fn test_windows_attributes_extraction() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"test").unwrap();
            file.flush().unwrap();
        }

        let metadata = FileMetadata::from_path(&file_path).unwrap();

        assert!(
            metadata.windows_attributes.is_some(),
            "Windows attributes should be extracted on Windows"
        );
    }
}
