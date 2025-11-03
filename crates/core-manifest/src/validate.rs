//! Validation functions for manifest schemas
//!
//! This module provides JSON Schema validation for Flight Plan and Cargo Manifest
//! documents to ensure they conform to the expected structure.

use crate::error::{Error, Result};
use crate::{FlightPlan, CargoManifest};
use jsonschema::{Validator, ValidationError};
use serde_json::{json, Value};

/// Validate a Flight Plan against its JSON Schema
pub fn validate_flight_plan(flight_plan: &FlightPlan) -> Result<()> {
    let schema = get_flight_plan_schema();
    let compiled = Validator::new(&schema)
        .map_err(|e| Error::validation(format!("Failed to compile schema: {}", e)))?;

    // Convert FlightPlan to JSON Value for validation
    let value = serde_json::to_value(flight_plan)?;

    // Perform validation
    if let Err(errors) = compiled.validate(&value) {
        let error_messages: Vec<String> = errors
            .map(|e| format_validation_error(&e))
            .collect();
        
        return Err(Error::validation(format!(
            "Flight Plan validation failed:\n  - {}",
            error_messages.join("\n  - ")
        )));
    }

    Ok(())
}

/// Validate a Cargo Manifest against its JSON Schema
pub fn validate_cargo_manifest(cargo: &CargoManifest) -> Result<()> {
    let schema = get_cargo_manifest_schema();
    let compiled = Validator::new(&schema)
        .map_err(|e| Error::validation(format!("Failed to compile schema: {}", e)))?;

    // Convert CargoManifest to JSON Value for validation
    let value = serde_json::to_value(cargo)?;

    // Perform validation
    if let Err(errors) = compiled.validate(&value) {
        let error_messages: Vec<String> = errors
            .map(|e| format_validation_error(&e))
            .collect();
        
        return Err(Error::validation(format!(
            "Cargo Manifest validation failed:\n  - {}",
            error_messages.join("\n  - ")
        )));
    }

    Ok(())
}

/// Format a validation error into a readable string
fn format_validation_error(error: &ValidationError) -> String {
    format!("{}: {}", error.instance_path, error)
}

/// Get the Flight Plan JSON Schema
fn get_flight_plan_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["schema", "job_id", "created_utc", "source", "target", "policy", "files"],
        "properties": {
            "schema": {
                "type": "string",
                "const": "orbit.flightplan.v1"
            },
            "job_id": {
                "type": "string",
                "minLength": 1
            },
            "created_utc": {
                "type": "string",
                "format": "date-time"
            },
            "source": {
                "$ref": "#/$defs/endpoint"
            },
            "target": {
                "$ref": "#/$defs/endpoint"
            },
            "policy": {
                "$ref": "#/$defs/policy"
            },
            "capacity_vector": {
                "$ref": "#/$defs/capacityVector"
            },
            "files": {
                "type": "array",
                "items": {
                    "$ref": "#/$defs/fileRef"
                },
                "minItems": 0
            },
            "job_digest": {
                "type": ["string", "null"]
            }
        },
        "$defs": {
            "endpoint": {
                "type": "object",
                "required": ["type", "root"],
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["fs", "smb", "cifs", "s3", "custom"]
                    },
                    "root": {
                        "type": "string",
                        "minLength": 1
                    },
                    "fingerprint": {
                        "type": ["string", "null"]
                    }
                }
            },
            "policy": {
                "type": "object",
                "required": ["encryption"],
                "properties": {
                    "encryption": {
                        "type": "object",
                        "required": ["aead", "key_ref"],
                        "properties": {
                            "aead": {
                                "type": "string",
                                "minLength": 1
                            },
                            "key_ref": {
                                "type": "string",
                                "minLength": 1
                            }
                        }
                    },
                    "retention_days": {
                        "type": ["integer", "null"],
                        "minimum": 0
                    },
                    "redaction_profile": {
                        "type": ["string", "null"]
                    },
                    "verify_on_arrival": {
                        "type": ["boolean", "null"]
                    },
                    "classification": {
                        "type": ["string", "null"]
                    }
                }
            },
            "capacityVector": {
                "type": "object",
                "required": ["bytes_total", "bytes_unique", "est_overhead_pct"],
                "properties": {
                    "bytes_total": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "bytes_unique": {
                        "type": "integer",
                        "minimum": 0
                    },
                    "est_overhead_pct": {
                        "type": "number",
                        "minimum": 0
                    },
                    "eta_minutes": {
                        "type": ["object", "null"],
                        "properties": {
                            "clean": {
                                "type": ["integer", "null"],
                                "minimum": 0
                            },
                            "moderate": {
                                "type": ["integer", "null"],
                                "minimum": 0
                            },
                            "rough": {
                                "type": ["integer", "null"],
                                "minimum": 0
                            }
                        }
                    }
                }
            },
            "fileRef": {
                "type": "object",
                "required": ["path", "cargo"],
                "properties": {
                    "path": {
                        "type": "string",
                        "minLength": 1
                    },
                    "cargo": {
                        "type": "string",
                        "minLength": 1
                    },
                    "starmap": {
                        "type": ["string", "null"]
                    }
                }
            }
        }
    })
}

/// Get the Cargo Manifest JSON Schema
fn get_cargo_manifest_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "required": ["schema", "path", "size", "chunking", "windows"],
        "properties": {
            "schema": {
                "type": "string",
                "const": "orbit.cargo.v1"
            },
            "path": {
                "type": "string",
                "minLength": 1
            },
            "size": {
                "type": "integer",
                "minimum": 0
            },
            "chunking": {
                "type": "object",
                "required": ["type"],
                "properties": {
                    "type": {
                        "type": "string",
                        "enum": ["cdc", "fixed"]
                    },
                    "avg_kib": {
                        "type": ["integer", "null"],
                        "minimum": 1
                    },
                    "algo": {
                        "type": ["string", "null"]
                    },
                    "fixed_kib": {
                        "type": ["integer", "null"],
                        "minimum": 1
                    }
                }
            },
            "digests": {
                "type": ["object", "null"],
                "properties": {
                    "blake3": {
                        "type": ["string", "null"],
                        "pattern": "^[0-9a-f]{64}$"
                    },
                    "sha256": {
                        "type": ["string", "null"],
                        "pattern": "^[0-9a-f]{64}$"
                    }
                }
            },
            "windows": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "first_chunk", "count", "merkle_root"],
                    "properties": {
                        "id": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "first_chunk": {
                            "type": "integer",
                            "minimum": 0
                        },
                        "count": {
                            "type": "integer",
                            "minimum": 1
                        },
                        "merkle_root": {
                            "type": "string",
                            "minLength": 1
                        },
                        "overlap": {
                            "type": ["integer", "null"],
                            "minimum": 0
                        }
                    }
                },
                "minItems": 1
            },
            "xattrs": {
                "type": ["object", "null"]
            },
            "file_digest": {
                "type": ["string", "null"]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Endpoint, Policy, Encryption, Chunking, WindowMeta};

    #[test]
    fn test_valid_flight_plan() {
        let source = Endpoint::filesystem("/data/source");
        let target = Endpoint::filesystem("/data/target");
        let policy = Policy::default_with_encryption(
            Encryption::aes256_gcm("env:ORBIT_KEY")
        );

        let flight_plan = FlightPlan::new(source, target, policy);
        
        let result = validate_flight_plan(&flight_plan);
        assert!(result.is_ok(), "Validation should pass: {:?}", result.err());
    }

    #[test]
    fn test_invalid_flight_plan_missing_source() {
        // Create a manually constructed invalid JSON
        let invalid_json = json!({
            "schema": "orbit.flightplan.v1",
            "job_id": "job-test",
            "created_utc": "2025-10-18T12:00:00Z",
            "target": {
                "type": "fs",
                "root": "/target"
            },
            "policy": {
                "encryption": {
                    "aead": "aes256-gcm",
                    "key_ref": "env:KEY"
                }
            },
            "files": []
        });

        let schema = get_flight_plan_schema();
        let compiled = Validator::new(&schema).unwrap();

        let result = compiled.validate(&invalid_json);
        assert!(result.is_err(), "Should fail validation without source");
    }

    #[test]
    fn test_valid_cargo_manifest() {
        let chunking = Chunking::cdc(256, "gear");
        let mut cargo = CargoManifest::new("file.bin", 1024000, chunking);
        cargo.add_window(WindowMeta::new(0, 0, 64, "a".repeat(64)));

        let result = validate_cargo_manifest(&cargo);
        assert!(result.is_ok(), "Validation should pass: {:?}", result.err());
    }

    #[test]
    fn test_invalid_cargo_manifest_no_windows() {
        let chunking = Chunking::fixed(1024);
        let cargo = CargoManifest::new("file.bin", 1024000, chunking);
        // No windows added

        let result = validate_cargo_manifest(&cargo);
        assert!(result.is_err(), "Should fail validation without windows");
        
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("windows") || err_msg.contains("minItems"));
    }

    #[test]
    fn test_invalid_cargo_manifest_empty_path() {
        let chunking = Chunking::fixed(1024);
        let cargo = CargoManifest::new("", 1024000, chunking);

        let result = validate_cargo_manifest(&cargo);
        assert!(result.is_err(), "Should fail validation with empty path");
    }

    #[test]
    fn test_schema_compilation() {
        let flight_schema = get_flight_plan_schema();
        let result = Validator::new(&flight_schema);
        assert!(result.is_ok(), "Flight Plan schema should compile");

        let cargo_schema = get_cargo_manifest_schema();
        let result = Validator::new(&cargo_schema);
        assert!(result.is_ok(), "Cargo Manifest schema should compile");
    }

    #[test]
    fn test_endpoint_type_validation() {
        let valid_types = vec!["fs", "smb", "cifs", "s3", "custom"];
        
        for endpoint_type in valid_types {
            let endpoint_json = json!({
                "type": endpoint_type,
                "root": "/path"
            });

            let schema = json!({
                "type": "object",
                "required": ["type", "root"],
                "properties": {
                    "type": {
                        "enum": ["fs", "smb", "cifs", "s3", "custom"]
                    },
                    "root": {
                        "type": "string"
                    }
                }
            });

            let compiled = Validator::new(&schema).unwrap();
            let result = compiled.validate(&endpoint_json);
            assert!(result.is_ok(), "Type '{}' should be valid", endpoint_type);
        }
    }

    #[test]
    fn test_chunking_type_validation() {
        // Valid CDC
        let cdc_json = json!({
            "type": "cdc",
            "avg_kib": 256,
            "algo": "gear"
        });

        let schema = json!({
            "type": "object",
            "required": ["type"],
            "properties": {
                "type": {
                    "enum": ["cdc", "fixed"]
                }
            }
        });

        let compiled = Validator::new(&schema).unwrap();
        assert!(compiled.validate(&cdc_json).is_ok());

        // Valid Fixed
        let fixed_json = json!({
            "type": "fixed",
            "fixed_kib": 1024
        });

        assert!(compiled.validate(&fixed_json).is_ok());

        // Invalid type
        let invalid_json = json!({
            "type": "invalid"
        });

        assert!(compiled.validate(&invalid_json).is_err());
    }
}