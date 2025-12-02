# Specification: Retry Logic Optimization

## 1. Problem Statement

The current retry implementation in `src/core/retry.rs` attempts to retry all non-fatal errors when `ErrorMode::Partial` is active. This includes errors that are "permanent" in nature, such as `std::io::ErrorKind::PermissionDenied` or `std::io::ErrorKind::AlreadyExists`.

Retrying these errors is inefficient because:
- The outcome is deterministic (it will fail every time).
- It wastes system resources (CPU, I/O cycles).
- It delays the feedback loop to the user or calling process.
- It needlessly consumes the exponential backoff delay (e.g., waiting 5s, 10s, 20s for a permission error).

## 2. Proposed Architecture

### 2.1. Strict Transience Check

The retry loop must rigorously enforce a "Transient-First" policy. The decision flow for an error E encountered during attempt N should be:

1. **Fatal Check**: Is E fatal? (e.g., source missing). If YES → Stop.
2. **Permanence Check**: Is E permanent? (i.e., `!E.is_transient()`). If YES → Stop.
3. **Budget Check**: Is N < MaxAttempts? If NO → Stop.
4. **Retry**: Wait backoff delay and retry.

### 2.2. Updated Error Categorization

The `OrbitError::is_transient()` method must be the single source of truth for this decision. The current implementation in `src/error.rs` effectively uses an "allow-list" for transient I/O errors (e.g., TimedOut, ConnectionRefused). This is safe and correct; any I/O error not on this list (like PermissionDenied) returns false.

### 2.3. Impact on Error Modes

The optimization applies specifically to the retry loop.

- **Abort Mode**: Fails immediately (current behavior). Optimization has no visible effect but logic is cleaner.
- **Skip Mode**: Skips immediately (current behavior). Optimization has no visible effect.
- **Partial Mode**: Currently retries everything. **New behavior**: Retries transient errors, fails fast on permanent errors.

## 3. Implementation Plan

1. **Modify `src/core/retry.rs`**: Insert the `!e.is_transient()` check before the ErrorMode evaluation logic inside the loop.
2. **Add `tests/retry_efficiency_test.rs`**: Create a specialized integration test that mocks a permanent I/O error and asserts that the operation attempt count is exactly 1 (no retries), whereas a transient error triggers retries.

## 4. Verification

- **Unit Tests**: Verify `is_transient` categorization.
- **Integration Tests**: Verify retry counts for different error types.

## 5. Expected Impact

- **Performance**: Eliminates wasted retry cycles for permanent errors.
- **User Experience**: Faster failure feedback for configuration issues.
- **Resource Efficiency**: Reduces CPU and I/O overhead in error scenarios.
