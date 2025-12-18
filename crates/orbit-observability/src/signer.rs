//! Cryptographic signing for audit events
//!
//! This module provides the `AuditSigner` which wraps the HMAC secret key
//! used for cryptographic audit chaining. The signer prevents accidental
//! logging or serialization of the secret key material.

use ring::hmac;
use std::sync::Arc;
use thiserror::Error;

/// Error types for audit signing operations
#[derive(Debug, Error)]
pub enum SignerError {
    #[error("ORBIT_AUDIT_SECRET environment variable not set")]
    MissingSecret,

    #[error("Invalid secret key: {0}")]
    InvalidSecret(String),
}

/// Write-only wrapper for HMAC secret key
///
/// AuditSigner provides secure storage for the HMAC-SHA256 secret key
/// used in audit chain integrity. The key is:
/// - Loaded once at startup from environment or bytes
/// - Wrapped in Arc for thread-safe sharing
/// - Never logged or serialized
/// - Used only for HMAC operations via the ring crate
///
/// ## Security Considerations
///
/// - The secret should be at least 256 bits (32 bytes) for security
/// - Store in a secure vault (AWS Secrets Manager, HashiCorp Vault) in production
/// - Never hardcode the secret in source code
/// - Rotate regularly according to your security policy
///
/// ## Example
///
/// ```no_run
/// use orbit_observability::signer::AuditSigner;
///
/// // Load from environment variable ORBIT_AUDIT_SECRET
/// let signer = AuditSigner::from_env().expect("ORBIT_AUDIT_SECRET not set");
///
/// // Or for testing, load from explicit bytes
/// let signer = AuditSigner::from_bytes(b"test_secret_key_32_bytes_long!!!");
/// ```
pub struct AuditSigner {
    pub(crate) key: Arc<hmac::Key>,
}

impl AuditSigner {
    /// Load secret from ORBIT_AUDIT_SECRET environment variable
    ///
    /// # Errors
    ///
    /// Returns `SignerError::MissingSecret` if the environment variable is not set
    ///
    /// # Security
    ///
    /// The environment variable should contain a base64-encoded or hex-encoded
    /// secret of at least 256 bits. For simplicity, this implementation uses
    /// the raw bytes of the environment variable string.
    pub fn from_env() -> Result<Self, SignerError> {
        let secret = std::env::var("ORBIT_AUDIT_SECRET").map_err(|_| SignerError::MissingSecret)?;

        if secret.is_empty() {
            return Err(SignerError::InvalidSecret(
                "ORBIT_AUDIT_SECRET cannot be empty".to_string(),
            ));
        }

        Ok(Self::from_bytes(secret.as_bytes()))
    }

    /// Load from explicit byte array
    ///
    /// This is primarily for testing and development. In production,
    /// prefer `from_env()` to load from a secure vault.
    ///
    /// # Example
    ///
    /// ```
    /// use orbit_observability::signer::AuditSigner;
    ///
    /// let signer = AuditSigner::from_bytes(b"my_secret_key_for_testing");
    /// ```
    pub fn from_bytes(secret: &[u8]) -> Self {
        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        Self { key: Arc::new(key) }
    }

    /// Get a cloned reference to the HMAC key
    ///
    /// This is used internally by AuditChain to access the key
    /// for signing operations.
    pub(crate) fn key(&self) -> Arc<hmac::Key> {
        Arc::clone(&self.key)
    }
}

// Prevent accidental Debug output of secret key
impl std::fmt::Debug for AuditSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditSigner")
            .field("key", &"<redacted>")
            .finish()
    }
}

// Implement Clone to allow sharing the signer
impl Clone for AuditSigner {
    fn clone(&self) -> Self {
        Self {
            key: Arc::clone(&self.key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bytes() {
        let signer = AuditSigner::from_bytes(b"test_secret_key");
        assert!(Arc::strong_count(&signer.key) == 1);
    }

    #[test]
    fn test_clone_shares_key() {
        let signer1 = AuditSigner::from_bytes(b"test_secret");
        let signer2 = signer1.clone();

        // Both signers should share the same key (Arc)
        assert!(Arc::ptr_eq(&signer1.key, &signer2.key));
    }

    #[test]
    fn test_debug_redacts_key() {
        let signer = AuditSigner::from_bytes(b"super_secret_key");
        let debug_output = format!("{:?}", signer);

        assert!(debug_output.contains("<redacted>"));
        assert!(!debug_output.contains("super_secret_key"));
    }

    #[test]
    fn test_from_env_missing() {
        // Ensure the env var is not set
        std::env::remove_var("ORBIT_AUDIT_SECRET");

        let result = AuditSigner::from_env();
        assert!(matches!(result, Err(SignerError::MissingSecret)));
    }

    #[test]
    fn test_from_env_empty() {
        std::env::set_var("ORBIT_AUDIT_SECRET", "");

        let result = AuditSigner::from_env();
        assert!(matches!(result, Err(SignerError::InvalidSecret(_))));

        std::env::remove_var("ORBIT_AUDIT_SECRET");
    }

    #[test]
    fn test_from_env_success() {
        std::env::set_var("ORBIT_AUDIT_SECRET", "my_test_secret_key_123");

        let result = AuditSigner::from_env();
        assert!(result.is_ok());

        std::env::remove_var("ORBIT_AUDIT_SECRET");
    }
}
