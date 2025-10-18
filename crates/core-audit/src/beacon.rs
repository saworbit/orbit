//! Beacon: Final job summary with optional signature
//!
//! A Beacon is a detached summary document that binds together the final
//! job digest, policy, and optionally a cryptographic signature for audit compliance.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Beacon: Final job summary and signature
///
/// # Example
/// ```
/// use orbit_core_audit::{Beacon, BeaconBuilder};
///
/// let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
///     .with_signer("CN=ORBIT-SIGNER")
///     .with_policy_classification("OFFICIAL:Sensitive")
///     .build();
///
/// beacon.save("beacon.json").unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Beacon {
    /// Schema version
    pub schema: String,

    /// Job ID
    pub job_id: String,

    /// Final job digest
    pub job_digest: String,

    /// Signer identity (e.g., CN from certificate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signer: Option<String>,

    /// Timestamp when beacon was created (UTC)
    pub ts: DateTime<Utc>,

    /// Policy information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<BeaconPolicy>,

    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// Detached signature (base64 encoded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Policy information in beacon
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeaconPolicy {
    /// Classification label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,

    /// Retention days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u32>,

    /// Encryption algorithm used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<String>,
}

/// Builder for constructing Beacon documents
///
/// # Example
/// ```
/// use orbit_core_audit::BeaconBuilder;
///
/// let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
///     .with_signer("CN=MyOrg")
///     .with_policy_classification("SECRET")
///     .with_policy_encryption("aes256-gcm")
///     .build();
/// ```
pub struct BeaconBuilder {
    job_id: String,
    job_digest: String,
    signer: Option<String>,
    policy: BeaconPolicy,
    metadata: HashMap<String, serde_json::Value>,
    signature: Option<String>,
}

impl Beacon {
    /// Create a minimal beacon
    pub fn new(job_id: String, job_digest: String) -> Self {
        Self {
            schema: "orbit.beacon.v1".to_string(),
            job_id,
            job_digest,
            signer: None,
            ts: Utc::now(),
            policy: None,
            metadata: None,
            signature: None,
        }
    }

    /// Check if the beacon has a signature
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Add a signature to the beacon
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Save beacon to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load beacon from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let beacon: Beacon = serde_json::from_str(&contents)?;
        Ok(beacon)
    }

    /// Validate beacon structure
    pub fn validate(&self) -> Result<()> {
        if self.schema != "orbit.beacon.v1" {
            return Err(Error::Other(format!(
                "Invalid schema version: {}",
                self.schema
            )));
        }

        if self.job_id.is_empty() {
            return Err(Error::missing_field("job_id"));
        }

        if self.job_digest.is_empty() {
            return Err(Error::missing_field("job_digest"));
        }

        Ok(())
    }
}

impl BeaconBuilder {
    /// Create a new beacon builder
    pub fn new<S: Into<String>>(job_id: S, job_digest: S) -> Self {
        Self {
            job_id: job_id.into(),
            job_digest: job_digest.into(),
            signer: None,
            policy: BeaconPolicy {
                classification: None,
                retention_days: None,
                encryption: None,
            },
            metadata: HashMap::new(),
            signature: None,
        }
    }

    /// Set the signer identity
    pub fn with_signer<S: Into<String>>(mut self, signer: S) -> Self {
        self.signer = Some(signer.into());
        self
    }

    /// Set policy classification
    pub fn with_policy_classification<S: Into<String>>(mut self, classification: S) -> Self {
        self.policy.classification = Some(classification.into());
        self
    }

    /// Set policy retention days
    pub fn with_policy_retention_days(mut self, days: u32) -> Self {
        self.policy.retention_days = Some(days);
        self
    }

    /// Set policy encryption
    pub fn with_policy_encryption<S: Into<String>>(mut self, encryption: S) -> Self {
        self.policy.encryption = Some(encryption.into());
        self
    }

    /// Add metadata field
    pub fn with_metadata<S: Into<String>>(
        mut self,
        key: S,
        value: serde_json::Value,
    ) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set signature
    pub fn with_signature<S: Into<String>>(mut self, signature: S) -> Self {
        self.signature = Some(signature.into());
        self
    }

    /// Build the beacon
    pub fn build(self) -> Beacon {
        let has_policy = self.policy.classification.is_some()
            || self.policy.retention_days.is_some()
            || self.policy.encryption.is_some();

        Beacon {
            schema: "orbit.beacon.v1".to_string(),
            job_id: self.job_id,
            job_digest: self.job_digest,
            signer: self.signer,
            ts: Utc::now(),
            policy: if has_policy { Some(self.policy) } else { None },
            metadata: if self.metadata.is_empty() {
                None
            } else {
                Some(self.metadata)
            },
            signature: self.signature,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[test]
    fn test_beacon_creation() {
        let beacon = Beacon::new("job-123".to_string(), "sha256:abc123".to_string());

        assert_eq!(beacon.schema, "orbit.beacon.v1");
        assert_eq!(beacon.job_id, "job-123");
        assert_eq!(beacon.job_digest, "sha256:abc123");
        assert!(!beacon.is_signed());
    }

    #[test]
    fn test_beacon_builder() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_signer("CN=ORBIT-SIGNER")
            .with_policy_classification("OFFICIAL:Sensitive")
            .with_policy_retention_days(180)
            .with_policy_encryption("aes256-gcm")
            .build();

        assert_eq!(beacon.job_id, "job-123");
        assert_eq!(beacon.signer, Some("CN=ORBIT-SIGNER".to_string()));
        
        let policy = beacon.policy.unwrap();
        assert_eq!(policy.classification, Some("OFFICIAL:Sensitive".to_string()));
        assert_eq!(policy.retention_days, Some(180));
        assert_eq!(policy.encryption, Some("aes256-gcm".to_string()));
    }

    #[test]
    fn test_beacon_with_signature() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_signature("base64encodedSignature")
            .build();

        assert!(beacon.is_signed());
        assert_eq!(beacon.signature, Some("base64encodedSignature".to_string()));
    }

    #[test]
    fn test_beacon_with_metadata() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_metadata("files_count", json!(5))
            .with_metadata("bytes_total", json!(1024000))
            .build();

        let metadata = beacon.metadata.unwrap();
        assert_eq!(metadata.get("files_count").unwrap(), &json!(5));
        assert_eq!(metadata.get("bytes_total").unwrap(), &json!(1024000));
    }

    #[test]
    fn test_beacon_save_load() {
        let temp_file = NamedTempFile::new().unwrap();

        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_signer("CN=TEST")
            .build();

        beacon.save(temp_file.path()).unwrap();

        let loaded = Beacon::load(temp_file.path()).unwrap();
        assert_eq!(loaded.job_id, "job-123");
        assert_eq!(loaded.job_digest, "sha256:abc123");
        assert_eq!(loaded.signer, Some("CN=TEST".to_string()));
    }

    #[test]
    fn test_beacon_serialization() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_policy_classification("SECRET")
            .build();

        let json = serde_json::to_string(&beacon).unwrap();
        let deserialized: Beacon = serde_json::from_str(&json).unwrap();

        assert_eq!(beacon.job_id, deserialized.job_id);
        assert_eq!(beacon.job_digest, deserialized.job_digest);
    }

    #[test]
    fn test_beacon_validation() {
        let beacon = Beacon::new("job-123".to_string(), "sha256:abc123".to_string());
        assert!(beacon.validate().is_ok());

        let invalid = Beacon::new("".to_string(), "sha256:abc123".to_string());
        let result = invalid.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("job_id"));
    }

    #[test]
    fn test_beacon_minimal() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123").build();

        // Minimal beacon should have no policy or metadata
        assert!(beacon.policy.is_none());
        assert!(beacon.metadata.is_none());
        assert!(beacon.signature.is_none());
    }

    #[test]
    fn test_beacon_policy_optional_fields() {
        let beacon = BeaconBuilder::new("job-123", "sha256:abc123")
            .with_policy_classification("OFFICIAL")
            .build();

        let policy = beacon.policy.unwrap();
        assert_eq!(policy.classification, Some("OFFICIAL".to_string()));
        assert_eq!(policy.retention_days, None);
        assert_eq!(policy.encryption, None);
    }
}