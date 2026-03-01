//! Security module for path validation and sandboxing.
//!
//! The PathJail ensures that the Star agent can only access files within
//! explicitly allowed directories, preventing directory traversal attacks
//! and unauthorized access to system files.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// A security sandbox that restricts file access to allowed root directories.
///
/// # Security Properties
///
/// - Prevents directory traversal attacks (e.g., `../../etc/passwd`)
/// - Blocks access to system files outside allowed roots
/// - Resolves symlinks to prevent jail escapes
/// - Validates paths before any filesystem operations
pub struct PathJail {
    allowed_roots: Vec<PathBuf>,
}

impl PathJail {
    /// Creates a new PathJail with the specified allowed root directories.
    ///
    /// # Arguments
    ///
    /// * `roots` - Vector of paths that the agent is allowed to access
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use orbit_star::security::PathJail;
    ///
    /// let jail = PathJail::new(vec![
    ///     PathBuf::from("/mnt/data"),
    ///     PathBuf::from("/backups"),
    /// ]);
    /// ```
    pub fn new(roots: Vec<PathBuf>) -> Self {
        // Canonicalize roots to resolve symlinks immediately
        let allowed_roots: Vec<PathBuf> = roots
            .into_iter()
            .filter_map(|p| match p.canonicalize() {
                Ok(canonical) => {
                    debug!("Added allowed root: {}", canonical.display());
                    Some(canonical)
                }
                Err(e) => {
                    warn!("Failed to canonicalize root {:?}: {}", p, e);
                    None
                }
            })
            .collect();

        if allowed_roots.is_empty() {
            warn!("PathJail created with no valid allowed roots!");
        }

        Self { allowed_roots }
    }

    /// Validates that a requested path is within one of the allowed roots.
    ///
    /// This method performs several security checks:
    /// 1. Canonicalizes the requested path to resolve symlinks and `.` / `..`
    /// 2. Checks if the canonical path starts with any allowed root
    /// 3. Returns the validated canonical path if allowed
    ///
    /// # Arguments
    ///
    /// * `requested` - The path requested by a client
    ///
    /// # Returns
    ///
    /// * `Ok(PathBuf)` - The validated canonical path
    /// * `Err` - If the path is outside allowed roots or invalid
    ///
    /// # Security
    ///
    /// This method prevents:
    /// - Directory traversal: `../../etc/passwd` → rejected
    /// - Symlink escapes: `/allowed/link` → `/etc/shadow` → rejected
    /// - Absolute path injection: `/etc/passwd` → rejected (unless `/etc` is allowed)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use orbit_star::security::PathJail;
    /// let jail = PathJail::new(vec![PathBuf::from("/mnt/data")]);
    ///
    /// // Allowed
    /// assert!(jail.secure_path("/mnt/data/file.txt").is_ok());
    /// assert!(jail.secure_path("/mnt/data/subdir/file.txt").is_ok());
    ///
    /// // Rejected
    /// assert!(jail.secure_path("/etc/passwd").is_err());
    /// assert!(jail.secure_path("/mnt/data/../etc/passwd").is_err());
    /// ```
    pub fn secure_path(&self, requested: &str) -> Result<PathBuf> {
        if self.allowed_roots.is_empty() {
            bail!("No allowed roots configured - all access denied");
        }

        let path = Path::new(requested);

        // Canonicalize the requested path to resolve symlinks and normalize.
        // If the path doesn't exist yet (e.g., a new destination file),
        // canonicalize the nearest existing ancestor and rebuild safely.
        let canonical = match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                let mut ancestor = path;
                let mut tail: Vec<std::ffi::OsString> = Vec::new();

                loop {
                    if let Ok(existing) = ancestor.canonicalize() {
                        let mut rebuilt = existing;
                        for component in tail.iter().rev() {
                            if component == std::ffi::OsStr::new("..") {
                                if !rebuilt.pop() {
                                    bail!("Failed to canonicalize path: {}", requested);
                                }
                            } else if component != std::ffi::OsStr::new(".") {
                                rebuilt.push(component);
                            }
                        }
                        break rebuilt;
                    }

                    if let Some(file_name) = ancestor.file_name() {
                        tail.push(file_name.to_os_string());
                    } else {
                        bail!("Failed to canonicalize path: {}", requested);
                    }

                    ancestor = ancestor
                        .parent()
                        .with_context(|| format!("Failed to canonicalize path: {}", requested))?;
                }
            }
        };

        // Check if the canonical path starts with any allowed root
        for allowed_root in &self.allowed_roots {
            if canonical.starts_with(allowed_root) {
                debug!(
                    "Access granted: {} (under {})",
                    canonical.display(),
                    allowed_root.display()
                );
                return Ok(canonical);
            }
        }

        // Path is outside all allowed roots
        warn!(
            "Access denied: {} (not under any allowed root)",
            canonical.display()
        );
        bail!(
            "Access denied: path '{}' is outside allowed directories",
            requested
        );
    }

    /// Returns the list of allowed root directories.
    #[allow(dead_code)]
    pub fn allowed_roots(&self) -> &[PathBuf] {
        &self.allowed_roots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_path_jail_blocks_traversal() {
        // Create a temp directory for testing
        let temp = std::env::temp_dir().join("orbit_jail_test");
        fs::create_dir_all(&temp).unwrap();

        let jail = PathJail::new(vec![temp.clone()]);

        // Should allow access to files within the jail
        let allowed_path = temp.join("file.txt");
        fs::write(&allowed_path, "test").unwrap();
        assert!(jail.secure_path(allowed_path.to_str().unwrap()).is_ok());

        // Should block access to parent directories
        let parent = temp.parent().unwrap();
        assert!(jail.secure_path(parent.to_str().unwrap()).is_err());

        // Cleanup
        fs::remove_dir_all(&temp).ok();
    }

    #[test]
    fn test_path_jail_empty_roots() {
        let jail = PathJail::new(vec![]);
        assert!(jail.secure_path("/any/path").is_err());
    }
}
