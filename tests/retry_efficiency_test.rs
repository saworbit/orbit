use orbit::config::{CopyConfig, ErrorMode};
use orbit::core::retry::with_retry;
use orbit::error::OrbitError;
use std::io;

#[test]
fn test_permanent_error_no_retry() {
    // Configuration with high retry count
    let config = CopyConfig {
        retry_attempts: 10,
        retry_delay_secs: 0,            // No delay for test speed
        error_mode: ErrorMode::Partial, // Mode that usually allows retries
        ..Default::default()
    };

    let mut attempts = 0;

    // Simulate a PERMANENT error (PermissionDenied)
    let result = with_retry(&config, || {
        attempts += 1;
        Err(OrbitError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "access denied",
        )))
    });

    assert!(result.is_err());

    // CRITICAL ASSERTION:
    // Should fail on the first attempt (attempts == 1).
    // It should NOT retry 10 times.
    assert_eq!(attempts, 1, "Permanent error triggered retries!");
}

#[test]
fn test_transient_error_retries() {
    let config = CopyConfig {
        retry_attempts: 3,
        retry_delay_secs: 0,
        error_mode: ErrorMode::Partial,
        ..Default::default()
    };

    let mut attempts = 0;

    // Simulate a TRANSIENT error (TimedOut)
    let result = with_retry(&config, || {
        attempts += 1;
        Err(OrbitError::Io(io::Error::new(
            io::ErrorKind::TimedOut,
            "timeout",
        )))
    });

    assert!(result.is_err());

    // Should try initial + 3 retries = 4 attempts
    assert_eq!(attempts, 4, "Transient error did not retry correctly");
}

#[test]
fn test_skip_mode_permanent_error() {
    let config = CopyConfig {
        retry_attempts: 5,
        retry_delay_secs: 0,
        error_mode: ErrorMode::Skip, // Skip mode
        ..Default::default()
    };

    let mut attempts = 0;

    let result = with_retry(&config, || {
        attempts += 1;
        Err(OrbitError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "access denied",
        )))
    });

    // Should return Ok (Skipped stats)
    assert!(result.is_ok());
    let stats = result.unwrap();
    assert_eq!(stats.files_skipped, 1);

    // Should NOT retry before skipping
    assert_eq!(attempts, 1);
}

#[test]
fn test_already_exists_no_retry() {
    let config = CopyConfig {
        retry_attempts: 5,
        retry_delay_secs: 0,
        error_mode: ErrorMode::Partial,
        ..Default::default()
    };

    let mut attempts = 0;

    let result = with_retry(&config, || {
        attempts += 1;
        Err(OrbitError::Io(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "file exists",
        )))
    });

    assert!(result.is_err());

    // AlreadyExists is permanent, should fail immediately
    assert_eq!(attempts, 1, "AlreadyExists error triggered retries!");
}

#[test]
fn test_connection_refused_retries() {
    let config = CopyConfig {
        retry_attempts: 2,
        retry_delay_secs: 0,
        error_mode: ErrorMode::Partial,
        ..Default::default()
    };

    let mut attempts = 0;

    let result = with_retry(&config, || {
        attempts += 1;
        Err(OrbitError::Io(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "connection refused",
        )))
    });

    assert!(result.is_err());

    // ConnectionRefused is transient, should retry
    assert_eq!(attempts, 3, "ConnectionRefused should trigger retries");
}
