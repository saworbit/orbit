//! Pipeline Router: Determines the execution strategy for a given file.
//!
//! The router acts as "The Sieve", directing files to the appropriate lane
//! based on file size: Neutrino (Fast), Equilibrium (Standard), or Gigantor (Large).

/// Size threshold for fast lane routing (8KB)
pub const BYPASS_THRESHOLD_BYTES: u64 = 8192;

/// Size threshold for large file routing (1GB)
pub const LARGE_FILE_THRESHOLD_BYTES: u64 = 1024 * 1024 * 1024;

/// Transfer lane selection based on file characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferLane {
    /// The "Neutrino" Fast Lane: Raw streaming, no chunking, no indexing.
    /// Used for files below 8KB where CDC/dedup overhead exceeds potential savings.
    Fast,

    /// The "Equilibrium" Standard Lane: CDC chunking, Universe indexing, Deduplication.
    /// Used for medium-sized files (8KB to 1GB) where content-defined chunking provides value.
    Standard,

    /// The "Gigantor" Large File Lane: Tiered deduplication for files over 1GB.
    /// Uses specialized strategies for large files (future implementation).
    Large,
}

/// Router that determines optimal transfer strategy based on file metadata
#[derive(Debug, Clone)]
pub struct FileRouter {
    /// Size threshold in bytes (files below this use fast lane)
    threshold: u64,

    /// Whether Neutrino fast lane is enabled
    enabled: bool,
}

impl FileRouter {
    /// Creates a new router with specified threshold and enabled state
    ///
    /// # Arguments
    ///
    /// * `threshold` - Size threshold in bytes (typically 8KB)
    /// * `enabled` - Whether to route small files to fast lane
    ///
    /// # Example
    ///
    /// ```
    /// use orbit::core::neutrino::FileRouter;
    ///
    /// let router = FileRouter::new(8192, true);
    /// ```
    pub fn new(threshold: u64, enabled: bool) -> Self {
        Self { threshold, enabled }
    }

    /// Determines the optimal strategy based on file size
    ///
    /// # Arguments
    ///
    /// * `file_size` - Size of the file in bytes
    ///
    /// # Returns
    ///
    /// * `TransferLane::Fast` - If file is below 8KB and fast lane is enabled
    /// * `TransferLane::Standard` - If file is 8KB to 1GB (Equilibrium)
    /// * `TransferLane::Large` - If file is over 1GB (Gigantor - future)
    ///
    /// # Example
    ///
    /// ```
    /// use orbit::core::neutrino::{FileRouter, TransferLane};
    ///
    /// let router = FileRouter::new(8192, true);
    /// assert_eq!(router.route(4096), TransferLane::Fast);
    /// assert_eq!(router.route(16384), TransferLane::Standard);
    /// assert_eq!(router.route(2_000_000_000), TransferLane::Large);
    /// ```
    pub fn route(&self, file_size: u64) -> TransferLane {
        if self.enabled && file_size < self.threshold {
            TransferLane::Fast
        } else if file_size < LARGE_FILE_THRESHOLD_BYTES {
            TransferLane::Standard
        } else {
            TransferLane::Large
        }
    }

    /// Returns the configured threshold
    pub fn threshold(&self) -> u64 {
        self.threshold
    }

    /// Returns whether fast lane is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for FileRouter {
    fn default() -> Self {
        Self::new(BYPASS_THRESHOLD_BYTES, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_threshold() {
        let router = FileRouter::new(8192, true);

        // Files below threshold should use fast lane
        assert_eq!(router.route(0), TransferLane::Fast);
        assert_eq!(router.route(1), TransferLane::Fast);
        assert_eq!(router.route(4096), TransferLane::Fast);
        assert_eq!(router.route(8191), TransferLane::Fast);

        // Files at or above threshold but below 1GB should use standard lane (Equilibrium)
        assert_eq!(router.route(8192), TransferLane::Standard);
        assert_eq!(router.route(8193), TransferLane::Standard);
        assert_eq!(router.route(16384), TransferLane::Standard);
        assert_eq!(router.route(1_000_000), TransferLane::Standard);
        assert_eq!(router.route(500 * 1024 * 1024), TransferLane::Standard); // 500MB
        assert_eq!(
            router.route(LARGE_FILE_THRESHOLD_BYTES - 1),
            TransferLane::Standard
        );

        // Files at or above 1GB should use large file lane (Gigantor)
        assert_eq!(
            router.route(LARGE_FILE_THRESHOLD_BYTES),
            TransferLane::Large
        );
        assert_eq!(
            router.route(LARGE_FILE_THRESHOLD_BYTES + 1),
            TransferLane::Large
        );
        assert_eq!(router.route(2 * 1024 * 1024 * 1024), TransferLane::Large); // 2GB
    }

    #[test]
    fn test_router_disabled() {
        let router = FileRouter::new(8192, false);

        // When disabled, all files use standard lane
        assert_eq!(router.route(0), TransferLane::Standard);
        assert_eq!(router.route(100), TransferLane::Standard);
        assert_eq!(router.route(4096), TransferLane::Standard);
        assert_eq!(router.route(8191), TransferLane::Standard);
        assert_eq!(router.route(16384), TransferLane::Standard);
    }

    #[test]
    fn test_router_custom_threshold() {
        let router = FileRouter::new(16384, true); // 16KB threshold

        assert_eq!(router.route(8192), TransferLane::Fast);
        assert_eq!(router.route(16383), TransferLane::Fast);
        assert_eq!(router.route(16384), TransferLane::Standard);
    }

    #[test]
    fn test_router_zero_threshold() {
        let router = FileRouter::new(0, true);

        // With zero threshold, even zero-byte files use standard lane
        assert_eq!(router.route(0), TransferLane::Standard);
        assert_eq!(router.route(1), TransferLane::Standard);
    }

    #[test]
    fn test_router_default() {
        let router = FileRouter::default();

        // Default is disabled with 8KB threshold
        assert!(!router.is_enabled());
        assert_eq!(router.threshold(), BYPASS_THRESHOLD_BYTES);
        assert_eq!(router.route(4096), TransferLane::Standard);
    }

    #[test]
    fn test_router_getters() {
        let router = FileRouter::new(16384, true);

        assert_eq!(router.threshold(), 16384);
        assert!(router.is_enabled());
    }
}
