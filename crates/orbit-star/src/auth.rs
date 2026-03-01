//! JWT-based authentication for P2P star-to-star transfers.
//!
//! This module implements stateless authorization tokens that allow Stars to
//! securely transfer data without requiring a shared database or centralized
//! authentication service.
//!
//! # Security Model
//!
//! - **Nucleus generates tokens**: Signed with ORBIT_AUTH_SECRET
//! - **Destination receives token**: Included in ReplicateFile command
//! - **Source verifies token**: Before serving file data
//!
//! # Token Claims
//!
//! - `sub`: "transfer" (subject)
//! - `allow_file`: Path to authorized file
//! - `exp`: Expiration timestamp (1 hour default)
//! - `iat`: Issued at timestamp
//! - `iss`: "orbit-nucleus" (issuer)

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// JWT claims for transfer authorization
#[derive(Debug, Serialize, Deserialize)]
struct TransferClaims {
    /// Subject (always "transfer")
    sub: String,
    /// File path that this token authorizes
    allow_file: String,
    /// Expiration timestamp (Unix epoch)
    exp: u64,
    /// Issued at timestamp (Unix epoch)
    iat: u64,
    /// Issuer (always "orbit-nucleus")
    iss: String,
}

/// Service for generating and verifying JWT transfer tokens
pub struct AuthService {
    /// Shared secret for HMAC signing
    secret: Vec<u8>,
    /// Token validity duration in seconds (default: 3600 = 1 hour)
    #[allow(dead_code)] // Used by with_validity() constructor
    validity_seconds: u64,
}

impl AuthService {
    /// Creates a new AuthService with the given secret
    ///
    /// # Arguments
    ///
    /// * `secret` - Shared secret for HMAC-SHA256 signing
    ///
    /// # Example
    ///
    /// ```
    /// use orbit_star::auth::AuthService;
    ///
    /// let auth = AuthService::new("my-secret-key");
    /// ```
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            validity_seconds: 3600, // 1 hour
        }
    }

    /// Creates a new AuthService with custom validity duration
    ///
    /// # Arguments
    ///
    /// * `secret` - Shared secret for HMAC-SHA256 signing
    /// * `validity_seconds` - Token validity duration in seconds
    #[allow(dead_code)] // Public API used by tests
    pub fn with_validity(secret: &str, validity_seconds: u64) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
            validity_seconds,
        }
    }

    /// Generates a transfer token for the specified file
    ///
    /// This method is called by the Nucleus when orchestrating a P2P transfer.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file this token authorizes
    ///
    /// # Returns
    ///
    /// A signed JWT token string
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_star::auth::AuthService;
    /// let auth = AuthService::new("secret");
    /// let token = auth.generate_transfer_token("/data/file.txt").unwrap();
    /// ```
    #[allow(dead_code)] // Public API for Nucleus to call (magnetar integration)
    pub fn generate_transfer_token(&self, file_path: &str) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("System time error: {}", e))?
            .as_secs();

        let claims = TransferClaims {
            sub: "transfer".to_string(),
            allow_file: file_path.to_string(),
            exp: now + self.validity_seconds,
            iat: now,
            iss: "orbit-nucleus".to_string(),
        };

        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| format!("Token generation failed: {}", e))
    }

    /// Verifies a transfer token and checks authorization for the requested path
    ///
    /// This method is called by Source Stars when receiving ReadStream requests.
    ///
    /// # Arguments
    ///
    /// * `token` - JWT token to verify
    /// * `requested_path` - Path that the client is trying to access
    ///
    /// # Returns
    ///
    /// Ok(()) if the token is valid and authorizes access to the requested path
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token signature is invalid
    /// - Token has expired
    /// - Token doesn't authorize the requested path
    /// - Token issuer is not "orbit-nucleus"
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use orbit_star::auth::AuthService;
    /// let auth = AuthService::new("secret");
    /// let token = auth.generate_transfer_token("/data/file.txt").unwrap();
    ///
    /// // Later, on the source star...
    /// auth.verify_transfer_token(&token, "/data/file.txt").unwrap();
    /// ```
    pub fn verify_transfer_token(&self, token: &str, requested_path: &str) -> Result<(), String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&["orbit-nucleus"]);
        validation.validate_exp = true;
        validation.leeway = 0;

        let token_data =
            decode::<TransferClaims>(token, &DecodingKey::from_secret(&self.secret), &validation)
                .map_err(|e| format!("Token decode failed: {}", e))?;

        // Verify the token authorizes this specific file
        if token_data.claims.allow_file != requested_path {
            return Err(format!(
                "Token allows '{}', but requested '{}'",
                token_data.claims.allow_file, requested_path
            ));
        }

        // Verify subject
        if token_data.claims.sub != "transfer" {
            return Err(format!(
                "Invalid subject: expected 'transfer', got '{}'",
                token_data.claims.sub
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_verification() {
        let auth = AuthService::new("test-secret-123");
        let token = auth
            .generate_transfer_token("/data/test.txt")
            .expect("Token generation failed");

        assert!(!token.is_empty());

        // Verify with correct path
        auth.verify_transfer_token(&token, "/data/test.txt")
            .expect("Token verification failed");
    }

    #[test]
    fn test_token_wrong_path_rejected() {
        let auth = AuthService::new("test-secret-123");
        let token = auth
            .generate_transfer_token("/data/allowed.txt")
            .expect("Token generation failed");

        // Try to access a different file
        let result = auth.verify_transfer_token(&token, "/data/forbidden.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("allows"));
    }

    #[test]
    fn test_token_wrong_secret_rejected() {
        let auth1 = AuthService::new("secret-1");
        let auth2 = AuthService::new("secret-2");

        let token = auth1
            .generate_transfer_token("/data/test.txt")
            .expect("Token generation failed");

        // Try to verify with different secret
        let result = auth2.verify_transfer_token(&token, "/data/test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token_rejected() {
        // Create a token with 0 seconds validity (immediately expired)
        let auth = AuthService::with_validity("test-secret", 0);
        let token = auth
            .generate_transfer_token("/data/test.txt")
            .expect("Token generation failed");

        // Wait long enough to cross the second boundary used by exp
        std::thread::sleep(std::time::Duration::from_secs(1));

        let result = auth.verify_transfer_token(&token, "/data/test.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_token_custom_validity() {
        let auth = AuthService::with_validity("test-secret", 7200); // 2 hours
        let token = auth
            .generate_transfer_token("/data/test.txt")
            .expect("Token generation failed");

        // Should still be valid
        auth.verify_transfer_token(&token, "/data/test.txt")
            .expect("Token verification failed");
    }
}
