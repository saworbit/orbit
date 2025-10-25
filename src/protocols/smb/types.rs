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
    Ntlmv2 {
        username: String,
        password: Secret,
    },
    
    /// Kerberos authentication
    /// If principal is None, uses OS credentials (SSO)
    Kerberos {
        principal: Option<String>,
    },
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
        #[cfg(feature = "smb-native")]
        {
            use zeroize::Zeroize;
            self.0.zeroize();
        }
        
        // Best-effort zeroing even without zeroize crate
        #[cfg(not(feature = "smb-native"))]
        {
            // SAFETY: We're zeroing our own String's bytes
            // This is best-effort and may not work if the string was moved
            unsafe {
                let bytes = self.0.as_bytes_mut();
                for b in bytes {
                    std::ptr::write_volatile(b, 0);
                }
            }
        }
    }
}

/// SMB security and encryption settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmbSecurity {
    /// Use encryption if available, but don't require it
    Opportunistic,
    
    /// Require SMB3 encryption, fail if not available
    RequireEncryption,
    
    /// Only signing (integrity), no payload encryption
    SignOnly,
}

/// SMB capability flags
#[cfg(feature = "smb-native")]
bitflags::bitflags! {
    /// SMB3 capability flags
    pub struct SmbCapability: u32 {
        /// Multi-channel support (multiple TCP connections)
        const MULTI_CHANNEL   = 0b0001;
        
        /// Durable file handles (survive temporary disconnects)
        const DURABLE_HANDLES = 0b0010;
        
        /// Directory leases (caching)
        const LEASES          = 0b0100;
        
        /// Distributed File System support
        const DFS             = 0b1000;
    }
}

#[cfg(not(feature = "smb-native"))]
/// Placeholder when feature is disabled
pub struct SmbCapability;

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