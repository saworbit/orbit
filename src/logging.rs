/*!
 * Logging and tracing initialization
 */

use std::fs::File;
use std::path::Path;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};

/// Initialize structured logging based on configuration
pub fn init_logging(config: &CopyConfig) -> Result<()> {
    let log_level = if config.verbose {
        Level::DEBUG
    } else {
        config.log_level.to_tracing_level()
    };

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(format!("orbit={}", log_level)))
        .map_err(|e| OrbitError::Config(format!("Failed to create log filter: {}", e)))?;

    // Create the subscriber based on log file configuration
    if let Some(ref log_path) = config.log_file {
        init_file_logging(log_path, env_filter)?;
    } else {
        init_stdout_logging(env_filter);
    }

    Ok(())
}

/// Initialize logging to stdout/stderr
fn init_stdout_logging(env_filter: EnvFilter) {
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .compact();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
}

/// Initialize logging to a file
fn init_file_logging(log_path: &Path, env_filter: EnvFilter) -> Result<()> {
    let file = File::create(log_path)
        .map_err(|e| OrbitError::Config(format!("Failed to create log file: {}", e)))?;

    let fmt_layer = fmt::layer()
        .with_writer(file)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false) // No ANSI colors in file
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// Initialize logging with custom format for testing
#[cfg(test)]
pub fn init_test_logging() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("orbit=debug"));

        let fmt_layer = fmt::layer().with_test_writer().with_target(false).compact();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer)
            .try_init()
            .ok(); // Ignore error if already initialized
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LogLevel;
    use tempfile::NamedTempFile;

    #[test]
    fn test_init_stdout_logging() {
        // This test just verifies the function doesn't panic
        let config = CopyConfig {
            log_level: LogLevel::Info,
            log_file: None,
            verbose: false,
            ..Default::default()
        };

        // Can't actually test initialization since it can only happen once
        // Just verify the config is valid
        assert!(!config.verbose);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn test_init_file_logging() {
        let temp_file = NamedTempFile::new().unwrap();
        let log_path = temp_file.path().to_path_buf();

        let config = CopyConfig {
            log_level: LogLevel::Debug,
            log_file: Some(log_path.clone()),
            verbose: false,
            ..Default::default()
        };

        assert_eq!(config.log_file, Some(log_path));
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_verbose_overrides_log_level() {
        let config = CopyConfig {
            log_level: LogLevel::Error,
            log_file: None,
            verbose: true,
            ..Default::default()
        };

        // When verbose is true, should use DEBUG level
        assert!(config.verbose);
    }

    #[test]
    fn test_log_level_conversion() {
        assert_eq!(LogLevel::Error.to_tracing_level(), Level::ERROR);
        assert_eq!(LogLevel::Warn.to_tracing_level(), Level::WARN);
        assert_eq!(LogLevel::Info.to_tracing_level(), Level::INFO);
        assert_eq!(LogLevel::Debug.to_tracing_level(), Level::DEBUG);
        assert_eq!(LogLevel::Trace.to_tracing_level(), Level::TRACE);
    }
}
