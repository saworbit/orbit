//! Cryptographic audit chaining with HMAC-SHA256
//!
//! This module implements immutable audit log chaining where each event
//! is cryptographically linked to the previous event. This provides:
//! - Tamper detection (any modification breaks the chain)
//! - Insertion detection (missing events break sequence)
//! - Reordering detection (sequence numbers enforced)
//!
//! The chain uses HMAC-SHA256 where each event's hash is computed as:
//! `HMAC(secret, previous_hash || canonical_event_bytes)`

use crate::event::OrbitEvent;
use crate::signer::AuditSigner;
use ring::hmac;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use thiserror::Error;

/// Errors that can occur during audit chain operations
#[derive(Debug, Error)]
pub enum ChainError {
    #[error("Failed to serialize event: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error(
        "Integrity check failed at sequence {sequence}: expected hash {expected}, got {actual}"
    )]
    IntegrityFailure {
        sequence: u64,
        expected: String,
        actual: String,
    },

    #[error("Sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },

    #[error("Missing integrity hash at sequence {0}")]
    MissingHash(u64),
}

/// Result type for audit chain operations
pub type Result<T> = std::result::Result<T, ChainError>;

/// Cryptographic audit chain
///
/// AuditChain maintains the state for HMAC-based event chaining.
/// It is thread-safe and can be shared across multiple threads.
///
/// ## Example
///
/// ```no_run
/// use orbit_observability::{AuditChain, AuditSigner, OrbitEvent, EventPayload};
///
/// let signer = AuditSigner::from_bytes(b"secret_key");
/// let chain = AuditChain::new(signer);
///
/// let mut event = OrbitEvent::new(EventPayload::Custom {
///     event_type: "test".to_string(),
///     data: serde_json::json!({"key": "value"}),
/// });
///
/// chain.sign_event(&mut event).unwrap();
/// assert!(event.integrity_hash.is_some());
/// assert_eq!(event.sequence, 0);
/// ```
pub struct AuditChain {
    /// HMAC key for signing
    key: Arc<hmac::Key>,

    /// Monotonic sequence counter
    sequence: AtomicU64,

    /// Previous event hash (for chaining)
    prev_hash: Arc<Mutex<Vec<u8>>>,
}

impl AuditChain {
    /// Create a new audit chain with the given signer
    ///
    /// The chain starts with sequence 0 and an initial hash of 32 zero bytes.
    pub fn new(signer: AuditSigner) -> Self {
        Self {
            key: signer.key(),
            sequence: AtomicU64::new(0),
            prev_hash: Arc::new(Mutex::new(vec![0u8; 32])),
        }
    }

    /// Sign an event and link it into the chain
    ///
    /// This method:
    /// 1. Assigns a monotonic sequence number
    /// 2. Canonicalizes the event (stable JSON serialization)
    /// 3. Computes HMAC(previous_hash || event_bytes)
    /// 4. Updates the event with hash and sequence
    /// 5. Stores the hash for the next event
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe. The sequence number is atomically incremented
    /// and the previous hash is protected by a mutex.
    pub fn sign_event(&self, event: &mut OrbitEvent) -> Result<()> {
        // Assign monotonic sequence
        event.sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Clear integrity_hash before serialization (it will be computed)
        event.integrity_hash = None;

        // Canonicalize event for stable hashing
        // Using to_vec for compact representation
        let canonical = serde_json::to_vec(&event)?;

        // Compute HMAC(prev_hash || canonical_bytes)
        let prev = self.prev_hash.lock().unwrap();
        let mut data = prev.clone();
        data.extend_from_slice(&canonical);

        let tag = hmac::sign(&self.key, &data);
        let hash_hex = hex::encode(tag.as_ref());

        // Update event
        event.integrity_hash = Some(hash_hex.clone());

        // Update chain state for next event
        drop(prev); // Release lock before updating
        *self.prev_hash.lock().unwrap() = tag.as_ref().to_vec();

        Ok(())
    }

    /// Verify the integrity of a chain of events
    ///
    /// This validates:
    /// - All events have integrity hashes
    /// - Sequence numbers are monotonically increasing
    /// - Each hash correctly links to the previous event
    ///
    /// Returns a ValidationReport with details about any failures.
    pub fn verify_chain(events: &[OrbitEvent], signer: &AuditSigner) -> Result<ValidationReport> {
        let key = signer.key();
        let mut prev_hash = vec![0u8; 32];
        let mut report = ValidationReport {
            total_events: events.len(),
            valid_events: 0,
            failures: Vec::new(),
        };

        for (index, event) in events.iter().enumerate() {
            // Check for integrity hash
            let reported_hash = match &event.integrity_hash {
                Some(h) => h,
                None => {
                    report
                        .failures
                        .push(ChainError::MissingHash(event.sequence));
                    continue;
                }
            };

            // Check sequence monotonicity
            if index > 0 {
                let expected_seq = events[index - 1].sequence + 1;
                if event.sequence != expected_seq {
                    report.failures.push(ChainError::SequenceGap {
                        expected: expected_seq,
                        actual: event.sequence,
                    });
                    // Continue checking other events
                }
            }

            // Recompute hash to verify
            let mut event_copy = event.clone();
            event_copy.integrity_hash = None; // Clear before hashing

            let canonical = match serde_json::to_vec(&event_copy) {
                Ok(c) => c,
                Err(e) => {
                    report.failures.push(ChainError::Serialization(e));
                    continue;
                }
            };

            let mut data = prev_hash.clone();
            data.extend_from_slice(&canonical);

            let tag = hmac::sign(&key, &data);
            let calculated_hash = hex::encode(tag.as_ref());

            if &calculated_hash != reported_hash {
                report.failures.push(ChainError::IntegrityFailure {
                    sequence: event.sequence,
                    expected: calculated_hash,
                    actual: reported_hash.clone(),
                });
                // Don't update prev_hash - chain is broken
                continue;
            }

            // Update for next event
            prev_hash = tag.as_ref().to_vec();
            report.valid_events += 1;
        }

        if report.failures.is_empty() {
            Ok(report)
        } else {
            Err(report.failures[0].clone())
        }
    }

    /// Get the current sequence number (next event will have this sequence)
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }
}

/// Report from chain validation
#[derive(Debug, Clone)]
pub struct ValidationReport {
    /// Total number of events checked
    pub total_events: usize,

    /// Number of events that passed validation
    pub valid_events: usize,

    /// List of validation failures
    pub failures: Vec<ChainError>,
}

impl ValidationReport {
    /// Check if the chain is valid (no failures)
    pub fn is_valid(&self) -> bool {
        self.failures.is_empty() && self.valid_events == self.total_events
    }

    /// Get failure rate as a percentage
    pub fn failure_rate(&self) -> f64 {
        if self.total_events == 0 {
            return 0.0;
        }
        ((self.total_events - self.valid_events) as f64 / self.total_events as f64) * 100.0
    }
}

// Manual Clone implementation for ChainError (Error trait doesn't derive Clone)
impl Clone for ChainError {
    fn clone(&self) -> Self {
        match self {
            Self::Serialization(e) => {
                Self::Serialization(serde_json::Error::io(std::io::Error::other(e.to_string())))
            }
            Self::IntegrityFailure {
                sequence,
                expected,
                actual,
            } => Self::IntegrityFailure {
                sequence: *sequence,
                expected: expected.clone(),
                actual: actual.clone(),
            },
            Self::SequenceGap { expected, actual } => Self::SequenceGap {
                expected: *expected,
                actual: *actual,
            },
            Self::MissingHash(seq) => Self::MissingHash(*seq),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventPayload;

    fn create_test_signer() -> AuditSigner {
        AuditSigner::from_bytes(b"test_secret_key_for_chain_tests")
    }

    #[test]
    fn test_basic_signing() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer);

        let mut event = OrbitEvent::new(EventPayload::JobStart {
            files: 1,
            total_bytes: 1024,
            protocol: "local".to_string(),
        });

        chain.sign_event(&mut event).unwrap();

        assert_eq!(event.sequence, 0);
        assert!(event.integrity_hash.is_some());
        assert_eq!(event.integrity_hash.as_ref().unwrap().len(), 64); // SHA256 hex
    }

    #[test]
    fn test_sequential_signing() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer);

        let mut event1 = OrbitEvent::new(EventPayload::JobStart {
            files: 1,
            total_bytes: 1024,
            protocol: "local".to_string(),
        });

        let mut event2 = OrbitEvent::new(EventPayload::JobComplete {
            duration_ms: 100,
            digest: "abc123".to_string(),
        });

        chain.sign_event(&mut event1).unwrap();
        chain.sign_event(&mut event2).unwrap();

        assert_eq!(event1.sequence, 0);
        assert_eq!(event2.sequence, 1);
        assert_ne!(event1.integrity_hash, event2.integrity_hash);
    }

    #[test]
    fn test_verify_valid_chain() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer.clone());

        let mut events = vec![];
        for i in 0..5 {
            let mut event = OrbitEvent::new(EventPayload::Custom {
                event_type: format!("test_{}", i),
                data: serde_json::json!({"value": i}),
            });
            chain.sign_event(&mut event).unwrap();
            events.push(event);
        }

        let report = AuditChain::verify_chain(&events, &signer).unwrap();
        assert!(report.is_valid());
        assert_eq!(report.valid_events, 5);
        assert_eq!(report.total_events, 5);
    }

    #[test]
    fn test_detect_tampering() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer.clone());

        let mut events = vec![];
        for i in 0..5 {
            let mut event = OrbitEvent::new(EventPayload::Custom {
                event_type: format!("test_{}", i),
                data: serde_json::json!({"value": i}),
            });
            chain.sign_event(&mut event).unwrap();
            events.push(event);
        }

        // Tamper with middle event
        events[2].metadata = Some(serde_json::json!({"tampered": true}));

        let result = AuditChain::verify_chain(&events, &signer);
        assert!(result.is_err());

        // Check the error is an integrity failure
        match result {
            Err(ChainError::IntegrityFailure { sequence, .. }) => {
                assert_eq!(sequence, 2);
            }
            _ => panic!("Expected IntegrityFailure"),
        }
    }

    #[test]
    fn test_detect_missing_hash() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer.clone());

        let mut events = vec![];
        for i in 0..3 {
            let mut event = OrbitEvent::new(EventPayload::Custom {
                event_type: format!("test_{}", i),
                data: serde_json::json!({"value": i}),
            });
            chain.sign_event(&mut event).unwrap();
            events.push(event);
        }

        // Remove hash from middle event
        events[1].integrity_hash = None;

        let result = AuditChain::verify_chain(&events, &signer);
        assert!(result.is_err());
    }

    #[test]
    fn test_sequence_counter() {
        let signer = create_test_signer();
        let chain = AuditChain::new(signer);

        assert_eq!(chain.current_sequence(), 0);

        let mut event = OrbitEvent::new(EventPayload::JobStart {
            files: 1,
            total_bytes: 100,
            protocol: "s3".to_string(),
        });

        chain.sign_event(&mut event).unwrap();
        assert_eq!(chain.current_sequence(), 1);
    }

    #[test]
    fn test_validation_report() {
        let report = ValidationReport {
            total_events: 10,
            valid_events: 8,
            failures: vec![],
        };

        assert!(!report.is_valid()); // failures vec is empty but valid_events != total
        assert_eq!(report.failure_rate(), 20.0);
    }
}
