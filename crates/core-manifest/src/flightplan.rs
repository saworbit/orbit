//! Flight Plan data structures and operations
//!
//! A Flight Plan is the top-level manifest for a transfer job, containing metadata,
//! policy, and references to individual file manifests.

use crate::error::{Error, Result};
use crate::FLIGHT_PLAN_SCHEMA_VERSION;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

/// Flight Plan: job-level transfer manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlightPlan {
    /// Schema version identifier
    pub schema: String,

    /// Unique job identifier (e.g., "job-2025-10-18T14:30:00Z")
    pub job_id: String,

    /// Job creation timestamp (UTC)
    pub created_utc: DateTime<Utc>,

    /// Source endpoint
    pub source: Endpoint,

    /// Target endpoint
    pub target: Endpoint,

    /// Transfer policy (encryption, retention, etc.)
    pub policy: Policy,

    /// Optional capacity estimation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_vector: Option<CapacityVector>,

    /// List of files in this job
    pub files: Vec<FileRef>,

    /// Final job digest (set after completion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_digest: Option<String>,
}

/// Storage endpoint (source or target)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Endpoint {
    /// Endpoint type (fs, smb, s3, etc.)
    #[serde(rename = "type")]
    pub endpoint_type: String,

    /// Root path or URI
    pub root: String,

    /// Optional fingerprint for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

/// Endpoint types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    /// Local filesystem
    Filesystem,
    /// SMB/CIFS network share
    Smb,
    /// Amazon S3
    S3,
    /// Custom endpoint
    Custom,
}

impl EndpointType {
    /// Convert to string representation
    pub fn as_str(&self) -> &str {
        match self {
            EndpointType::Filesystem => "fs",
            EndpointType::Smb => "smb",
            EndpointType::S3 => "s3",
            EndpointType::Custom => "custom",
        }
    }
}

impl FromStr for EndpointType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "fs" => Ok(EndpointType::Filesystem),
            "smb" | "cifs" => Ok(EndpointType::Smb),
            "s3" => Ok(EndpointType::S3),
            "custom" => Ok(EndpointType::Custom),
            _ => Err(Error::InvalidEndpointType(s.to_string())),
        }
    }
}

/// Transfer policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Policy {
    /// Encryption configuration
    pub encryption: Encryption,

    /// Retention period in days
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u32>,

    /// Redaction profile name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redaction_profile: Option<String>,

    /// Whether to verify on arrival
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_on_arrival: Option<bool>,

    /// Classification label (e.g., "OFFICIAL:Sensitive")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Encryption {
    /// AEAD algorithm (e.g., "aes256-gcm")
    pub aead: String,

    /// Key reference (e.g., "env:ORBIT_KEY")
    pub key_ref: String,
}

/// Capacity estimation vector
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapacityVector {
    /// Total bytes including duplicates
    pub bytes_total: u64,

    /// Unique bytes after deduplication
    pub bytes_unique: u64,

    /// Estimated overhead percentage
    pub est_overhead_pct: f32,

    /// Time estimates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_minutes: Option<Eta>,
}

/// Time estimation for different network conditions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Eta {
    /// Clean network conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clean: Option<u32>,

    /// Moderate network conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderate: Option<u32>,

    /// Rough network conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rough: Option<u32>,
}

/// Reference to a file and its manifests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileRef {
    /// Relative path within the job
    pub path: String,

    /// Content ID or path to Cargo Manifest
    pub cargo: String,

    /// Optional content ID or path to Star Map
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starmap: Option<String>,
}

impl FlightPlan {
    /// Create a new Flight Plan with minimal required fields
    pub fn new(source: Endpoint, target: Endpoint, policy: Policy) -> Self {
        let now = Utc::now();
        let job_id = format!("job-{}", now.format("%Y-%m-%dT%H_%M_%SZ"));

        Self {
            schema: FLIGHT_PLAN_SCHEMA_VERSION.to_string(),
            job_id,
            created_utc: now,
            source,
            target,
            policy,
            capacity_vector: None,
            files: Vec::new(),
            job_digest: None,
        }
    }

    /// Create a new Flight Plan with a custom job ID
    pub fn with_job_id(mut self, job_id: String) -> Self {
        self.job_id = job_id;
        self
    }

    /// Add a file reference to this Flight Plan
    pub fn add_file(&mut self, file_ref: FileRef) {
        self.files.push(file_ref);
    }

    /// Set capacity vector
    pub fn with_capacity_vector(mut self, capacity: CapacityVector) -> Self {
        self.capacity_vector = Some(capacity);
        self
    }

    /// Finalize the Flight Plan with a job digest
    pub fn finalize(&mut self, job_digest: String) {
        self.job_digest = Some(job_digest);
    }

    /// Check if the Flight Plan is finalized
    pub fn is_finalized(&self) -> bool {
        self.job_digest.is_some()
    }

    /// Save Flight Plan to a JSON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load Flight Plan from a JSON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(Error::manifest_not_found(path));
        }

        let contents = std::fs::read_to_string(path)?;
        let flight_plan: FlightPlan = serde_json::from_str(&contents)?;

        // Validate schema version
        if flight_plan.schema != FLIGHT_PLAN_SCHEMA_VERSION {
            return Err(Error::version_mismatch(
                FLIGHT_PLAN_SCHEMA_VERSION,
                &flight_plan.schema,
            ));
        }

        Ok(flight_plan)
    }
}

impl Endpoint {
    /// Create a filesystem endpoint
    pub fn filesystem<S: Into<String>>(root: S) -> Self {
        Self {
            endpoint_type: EndpointType::Filesystem.as_str().to_string(),
            root: root.into(),
            fingerprint: None,
        }
    }

    /// Create an SMB endpoint
    pub fn smb<S: Into<String>>(root: S) -> Self {
        Self {
            endpoint_type: EndpointType::Smb.as_str().to_string(),
            root: root.into(),
            fingerprint: None,
        }
    }

    /// Create an S3 endpoint
    pub fn s3<S: Into<String>>(root: S) -> Self {
        Self {
            endpoint_type: EndpointType::S3.as_str().to_string(),
            root: root.into(),
            fingerprint: None,
        }
    }

    /// Set endpoint fingerprint
    pub fn with_fingerprint<S: Into<String>>(mut self, fingerprint: S) -> Self {
        self.fingerprint = Some(fingerprint.into());
        self
    }
}

impl Policy {
    /// Create a default policy with encryption
    pub fn default_with_encryption(encryption: Encryption) -> Self {
        Self {
            encryption,
            retention_days: None,
            redaction_profile: None,
            verify_on_arrival: Some(true),
            classification: None,
        }
    }

    /// Set retention days
    pub fn with_retention_days(mut self, days: u32) -> Self {
        self.retention_days = Some(days);
        self
    }

    /// Set classification
    pub fn with_classification<S: Into<String>>(mut self, classification: S) -> Self {
        self.classification = Some(classification.into());
        self
    }
}

impl Encryption {
    /// Create AES-256-GCM encryption config
    pub fn aes256_gcm<S: Into<String>>(key_ref: S) -> Self {
        Self {
            aead: "aes256-gcm".to_string(),
            key_ref: key_ref.into(),
        }
    }

    /// Create ChaCha20-Poly1305 encryption config
    pub fn chacha20_poly1305<S: Into<String>>(key_ref: S) -> Self {
        Self {
            aead: "chacha20-poly1305".to_string(),
            key_ref: key_ref.into(),
        }
    }
}

impl FileRef {
    /// Create a new file reference
    pub fn new<S: Into<String>>(path: S, cargo: S) -> Self {
        Self {
            path: path.into(),
            cargo: cargo.into(),
            starmap: None,
        }
    }

    /// Set star map reference
    pub fn with_starmap<S: Into<String>>(mut self, starmap: S) -> Self {
        self.starmap = Some(starmap.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flight_plan_creation() {
        let source = Endpoint::filesystem("/data/source");
        let target = Endpoint::filesystem("/data/target");
        let encryption = Encryption::aes256_gcm("env:ORBIT_KEY");
        let policy = Policy::default_with_encryption(encryption);

        let flight_plan = FlightPlan::new(source.clone(), target.clone(), policy);

        assert_eq!(flight_plan.schema, FLIGHT_PLAN_SCHEMA_VERSION);
        assert!(flight_plan.job_id.starts_with("job-"));
        assert_eq!(flight_plan.source, source);
        assert_eq!(flight_plan.target, target);
        assert!(!flight_plan.is_finalized());
    }

    #[test]
    fn test_endpoint_types() {
        let fs = Endpoint::filesystem("/tmp");
        assert_eq!(fs.endpoint_type, "fs");

        let smb = Endpoint::smb("//server/share");
        assert_eq!(smb.endpoint_type, "smb");

        let s3 = Endpoint::s3("s3://bucket/prefix");
        assert_eq!(s3.endpoint_type, "s3");
    }

    #[test]
    fn test_endpoint_with_fingerprint() {
        let endpoint = Endpoint::filesystem("/data").with_fingerprint("abc123");
        assert_eq!(endpoint.fingerprint, Some("abc123".to_string()));
    }

    #[test]
    fn test_policy_builder() {
        let encryption = Encryption::aes256_gcm("env:KEY");
        let policy = Policy::default_with_encryption(encryption)
            .with_retention_days(180)
            .with_classification("OFFICIAL:Sensitive");

        assert_eq!(policy.retention_days, Some(180));
        assert_eq!(
            policy.classification,
            Some("OFFICIAL:Sensitive".to_string())
        );
        assert_eq!(policy.verify_on_arrival, Some(true));
    }

    #[test]
    fn test_file_ref_builder() {
        let file_ref = FileRef::new("data.bin", "sha256:abc123").with_starmap("sha256:def456");

        assert_eq!(file_ref.path, "data.bin");
        assert_eq!(file_ref.cargo, "sha256:abc123");
        assert_eq!(file_ref.starmap, Some("sha256:def456".to_string()));
    }

    #[test]
    fn test_finalization() {
        let source = Endpoint::filesystem("/src");
        let target = Endpoint::filesystem("/dst");
        let policy = Policy::default_with_encryption(Encryption::aes256_gcm("env:KEY"));

        let mut flight_plan = FlightPlan::new(source, target, policy);
        assert!(!flight_plan.is_finalized());

        flight_plan.finalize("sha256:final_digest".to_string());
        assert!(flight_plan.is_finalized());
        assert_eq!(
            flight_plan.job_digest,
            Some("sha256:final_digest".to_string())
        );
    }

    #[test]
    fn test_serialization() {
        let source = Endpoint::filesystem("/src");
        let target = Endpoint::filesystem("/dst");
        let policy = Policy::default_with_encryption(Encryption::aes256_gcm("env:KEY"));

        let flight_plan = FlightPlan::new(source, target, policy);

        // Serialize to JSON
        let json = serde_json::to_string(&flight_plan).unwrap();

        // Deserialize back
        let deserialized: FlightPlan = serde_json::from_str(&json).unwrap();

        assert_eq!(flight_plan.schema, deserialized.schema);
        assert_eq!(flight_plan.job_id, deserialized.job_id);
    }
}
