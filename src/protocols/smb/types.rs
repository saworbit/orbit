//! SMB protocol types and configuration structures

use std::time::SystemTime;

/// SMB target configuration
///
/// Specifies the connection details for an SMB share.
///
/// # Example
///
/// ```
/// use orbit::protocols::smb::{SmbTarget, SmbAuth, SmbSecurity, Secret};
///
/// let target = SmbTarget {
///     host: "fileserver.acme.corp".to_string(),
///     share: "projects".to_string(),
///     subpath: "alpha/reports/Q4".to_string(),
///     port: None, // defaults to 445
///     auth: SmbAuth::Ntlmv2 {
///         username: "jdoe".to_string(),
///         password: Secret("secret".to_string()),
///     },
///     security: SmbSecurity::RequireEncryption,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SmbTarget {
    /// Hostname or IP address (e.g., "fileserver.acme.corp")
    pub host: String,

    /// Share name (e.g., "projects")
    pub share: String,

    /// Subpath within the share (e.g., "alpha/reports/Q4")
    /// No leading slash
    pub subpath: String,

    /// SMB port (default: 445)
    pub port: Option<u16>,

    /// Authentication method
    pub auth: SmbAuth,

    /// Security/encryption settings
    pub security: SmbSecurity,
}

/// SMB authentication methods
#[derive(Debug, Clone)]
pub enum SmbAuth {
    /// Anonymous access (no credentials)
    Anonymous,

    /// NTLMv2 authentication
    Ntlmv2 { username: String, password: Secret },

    /// Kerberos authentication
    /// If principal is None, uses OS credentials (SSO)
    Kerberos { principal: Option<String> },
}

/// Secret wrapper for credentials
///
/// Automatically zeroes memory on drop to prevent credential leakage.
#[derive(Clone)]
pub struct Secret(pub String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // Best-effort zeroing of the password string
        // Note: This may not work if the string was moved, but provides basic protection
        unsafe {
            let bytes = self.0.as_bytes_mut();
            for b in bytes {
                std::ptr::write_volatile(b, 0);
            }
        }
    }
}

/// SMB security and encryption settings
///
/// This enum defines the security policy enforced during SMB connection.
/// The native client will configure the underlying SMB session to match
/// the requested policy and **fail the connection** if the server cannot
/// satisfy the requirements.
///
/// # Security Policy Enforcement
///
/// - **RequireEncryption**: Connection will fail if the server doesn't support SMB3 encryption
/// - **SignOnly**: Encryption is explicitly disabled; signing is enforced
/// - **Opportunistic**: Uses encryption if the server supports it, otherwise falls back to signing
///
/// # Examples
///
/// ```
/// use orbit::protocols::smb::{SmbTarget, SmbSecurity, SmbAuth};
///
/// // Most secure: require encryption
/// let secure_target = SmbTarget {
///     host: "fileserver".to_string(),
///     share: "sensitive".to_string(),
///     subpath: String::new(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::RequireEncryption,
/// };
///
/// // Performance-optimized: disable encryption on trusted network
/// let perf_target = SmbTarget {
///     host: "fileserver".to_string(),
///     share: "data".to_string(),
///     subpath: String::new(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::SignOnly,
/// };
///
/// // Flexible: use encryption if available
/// let compat_target = SmbTarget {
///     host: "fileserver".to_string(),
///     share: "shared".to_string(),
///     subpath: String::new(),
///     port: None,
///     auth: SmbAuth::Anonymous,
///     security: SmbSecurity::Opportunistic,
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmbSecurity {
    /// Use encryption if available, but don't require it (default)
    ///
    /// - Attempts to negotiate SMB3 encryption with the server
    /// - Falls back to signing-only if encryption is unavailable
    /// - Connection succeeds as long as signing is supported
    /// - Recommended for most use cases
    Opportunistic,

    /// Require SMB3 encryption, fail if not available
    ///
    /// - Forces encryption for all SMB traffic
    /// - Connection fails if server doesn't support SMB3 encryption
    /// - Provides both confidentiality and integrity protection
    /// - Recommended for sensitive data over untrusted networks
    /// - Uses AES-128/256-GCM or AES-128/256-CCM ciphers
    RequireEncryption,

    /// Only signing (integrity), no payload encryption
    ///
    /// - Explicitly disables encryption
    /// - Enforces packet signing for integrity protection
    /// - Provides tamper detection but not confidentiality
    /// - Recommended for performance-critical scenarios on trusted networks
    /// - Uses HMAC-SHA256, AES-128-GMAC, or AES-128-CMAC
    SignOnly,
}

/// SMB3 capability flags (simplified for v0.11.0)
///
/// Note: Advanced capabilities like multi-channel, durable handles, etc.
/// are handled internally by the smb crate and don't need explicit flags here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmbCapability {
    _private: (),
}

impl SmbCapability {
    /// Create a new capability set (currently a placeholder)
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for SmbCapability {
    fn default() -> Self {
        Self::new()
    }
}

/// File/directory metadata from SMB
#[derive(Debug, Clone)]
pub struct SmbMetadata {
    /// File size in bytes
    pub size: u64,

    /// Is this a directory?
    pub is_dir: bool,

    /// Last modified timestamp
    pub modified: Option<SystemTime>,

    /// Is the session/tree encrypted?
    pub encrypted: bool,
}

impl Default for SmbTarget {
    fn default() -> Self {
        Self {
            host: String::new(),
            share: String::new(),
            subpath: String::new(),
            port: Some(445),
            auth: SmbAuth::Anonymous,
            security: SmbSecurity::Opportunistic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_debug() {
        let secret = Secret("password123".to_string());
        let debug_str = format!("{:?}", secret);
        assert!(!debug_str.contains("password123"));
        assert!(debug_str.contains("REDACTED"));
    }

    #[test]
    fn test_smb_target_default() {
        let target = SmbTarget::default();
        assert_eq!(target.port, Some(445));
        assert!(matches!(target.auth, SmbAuth::Anonymous));
        assert_eq!(target.security, SmbSecurity::Opportunistic);
    }

    #[test]
    fn test_security_levels() {
        let opportunistic = SmbSecurity::Opportunistic;
        let required = SmbSecurity::RequireEncryption;
        let sign_only = SmbSecurity::SignOnly;

        assert_ne!(opportunistic, required);
        assert_ne!(required, sign_only);
        assert_ne!(sign_only, opportunistic);
    }
}
