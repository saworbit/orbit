//! Pipeline Router: Determines the execution strategy for a given file.
//!
//! The router acts as "The Sieve", directing files to either the fast lane
//! (Neutrino) or the standard lane (CDC + deduplication) based on file size.

/// Size threshold for fast lane routing (8KB)
pub const BYPASS_THRESHOLD_BYTES: u64 = 8192;

/// Transfer lane selection based on file characteristics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferLane {
    /// The "Neutrino" Fast Lane: Raw streaming, no chunking, no indexing.
    /// Used for files below the size threshold where CDC/dedup overhead
    /// exceeds potential savings.
    Fast,

    /// The Standard Lane: CDC chunking, Universe indexing, Deduplication.
    /// Used for larger files where content-defined chunking provides value.
    Standard,
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
    /// * `TransferLane::Fast` - If file is below threshold and fast lane is enabled
    /// * `TransferLane::Standard` - Otherwise
    ///
    /// # Example
    ///
    /// ```
    /// use orbit::core::neutrino::{FileRouter, TransferLane};
    ///
    /// let router = FileRouter::new(8192, true);
    /// assert_eq!(router.route(4096), TransferLane::Fast);
    /// assert_eq!(router.route(16384), TransferLane::Standard);
    /// ```
    pub fn route(&self, file_size: u64) -> TransferLane {
        if self.enabled && file_size < self.threshold {
            TransferLane::Fast
        } else {
            TransferLane::Standard
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

        // Files at or above threshold should use standard lane
        assert_eq!(router.route(8192), TransferLane::Standard);
        assert_eq!(router.route(8193), TransferLane::Standard);
        assert_eq!(router.route(16384), TransferLane::Standard);
        assert_eq!(router.route(1_000_000), TransferLane::Standard);
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
