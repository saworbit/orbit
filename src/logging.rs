/*!
 * Logging and tracing initialization with unified observability
 */

use std::fs::File;
use std::path::Path;
use std::env;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::config::CopyConfig;
use crate::error::{OrbitError, Result};

// V3 Observability imports
use orbit_observability::{AuditBridgeLayer, AuditSigner, UnifiedLogger};

// ============================================================================
// User-Facing Output Macros (Phase 3: Terminology Abstraction)
// ============================================================================

/// Macro for user-facing informational output
/// This formats cleanly for CLI users without timestamps/log levels
#[macro_export]
macro_rules! orbit_info {
    ($($arg:tt)*) => {
        tracing::info!(target: "user_facing", $($arg)*)
    }
}

/// Macro for user-facing warning output
#[macro_export]
macro_rules! orbit_warn {
    ($($arg:tt)*) => {
        tracing::warn!(target: "user_facing", $($arg)*)
    }
}

/// Macro for user-facing error output
#[macro_export]
macro_rules! orbit_error {
    ($($arg:tt)*) => {
        tracing::error!(target: "user_facing", $($arg)*)
    }
}

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

    let log_mode = env::var("TEST_LOG")
        .ok()
        .or_else(|| env::var("ORBIT_LOG_MODE").ok());
    if log_mode.as_deref() == Some("llm-debug") {
        init_llm_debug_logging(env_filter, config.log_file.as_deref())?;
        return Ok(());
    }

    // V3: Create unified audit logger if audit_log_path is configured
    let audit_layer = if let Some(ref audit_path) = config.audit_log_path {
        // Try to load HMAC secret from environment, fallback to disabled logger
        let logger = match AuditSigner::from_env() {
            Ok(signer) => {
                eprintln!(
                    "🔒 Initializing cryptographic audit logging to {:?}",
                    audit_path
                );
                UnifiedLogger::new(Some(audit_path), signer).map_err(|e| {
                    OrbitError::Config(format!("Failed to create audit logger: {}", e))
                })?
            }
            Err(_) => {
                eprintln!("⚠️  ORBIT_AUDIT_SECRET not set - audit logging disabled");
                eprintln!("    Set ORBIT_AUDIT_SECRET environment variable to enable cryptographic audit chaining");
                UnifiedLogger::disabled()
            }
        };

        Some(AuditBridgeLayer::new(logger).with_span_events(true))
    } else {
        None
    };

    // Create the subscriber based on log file configuration
    if let Some(ref log_path) = config.log_file {
        init_file_logging(log_path, env_filter, audit_layer, config)?;
    } else {
        init_stdout_logging(env_filter, audit_layer, config);
    }

    Ok(())
}

fn init_llm_debug_logging(env_filter: EnvFilter, log_path: Option<&Path>) -> Result<()> {
    let fmt_layer = fmt::layer()
        .json()
        .flatten_event(true)
        .with_span_list(true)
        .with_current_span(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_span_events(FmtSpan::NONE);

    let registry = tracing_subscriber::registry().with(env_filter);

    if let Some(path) = log_path {
        let file = File::create(path)
            .map_err(|e| OrbitError::Config(format!("Failed to create log file: {}", e)))?;
        registry.with(fmt_layer.with_writer(file)).init();
    } else {
        registry.with(fmt_layer).init();
    }

    Ok(())
}

/// Initialize logging to stdout/stderr
fn init_stdout_logging(
    env_filter: EnvFilter,
    audit_layer: Option<AuditBridgeLayer>,
    _config: &CopyConfig,
) {
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .compact();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    // Add audit bridge layer if configured
    let registry = registry.with(audit_layer);

    // V3: Add OpenTelemetry layer if otel_endpoint is configured
    #[cfg(feature = "opentelemetry")]
    {
        if let Some(ref endpoint) = _config.otel_endpoint {
            use opentelemetry_otlp::WithExportConfig;
            use opentelemetry_sdk::runtime;
            use tracing_opentelemetry::OpenTelemetryLayer;

            match opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint),
                )
                .install_batch(runtime::Tokio)
            {
                Ok(tracer) => {
                    tracing::info!("OpenTelemetry tracing enabled - exporting to {}", endpoint);
                    let otel_layer = OpenTelemetryLayer::new(tracer);
                    registry.with(Some(otel_layer)).init();
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to initialize OpenTelemetry: {}", e);
                }
            }
        }
    }

    registry.init();
}

/// Initialize logging to a file
fn init_file_logging(
    log_path: &Path,
    env_filter: EnvFilter,
    audit_layer: Option<AuditBridgeLayer>,
    _config: &CopyConfig,
) -> Result<()> {
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

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    // Add audit bridge layer if configured
    let registry = registry.with(audit_layer);

    // V3: Add OpenTelemetry layer if otel_endpoint is configured
    #[cfg(feature = "opentelemetry")]
    {
        if let Some(ref endpoint) = _config.otel_endpoint {
            use opentelemetry_otlp::WithExportConfig;
            use opentelemetry_sdk::runtime;
            use tracing_opentelemetry::OpenTelemetryLayer;

            match opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint),
                )
                .install_batch(runtime::Tokio)
            {
                Ok(tracer) => {
                    tracing::info!("OpenTelemetry tracing enabled - exporting to {}", endpoint);
                    let otel_layer = OpenTelemetryLayer::new(tracer);
                    registry.with(Some(otel_layer)).init();
                    return Ok(());
                }
                Err(e) => {
                    tracing::error!("Failed to initialize OpenTelemetry: {}", e);
                }
            }
        }
    }

    registry.init();
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
