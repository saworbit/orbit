/*!
 * URI parsing for protocol detection
 */

use std::path::PathBuf;
use crate::error::{OrbitError, Result};
use super::Protocol;

pub fn parse_uri(uri: &str) -> Result<(Protocol, PathBuf)> {
    if uri.contains("://") {
        parse_protocol_uri(uri)
    } else {
        Ok((Protocol::Local, PathBuf::from(uri)))
    }
}

fn parse_protocol_uri(uri: &str) -> Result<(Protocol, PathBuf)> {
    let parts: Vec<&str> = uri.splitn(2, "://").collect();
    if parts.len() != 2 {
        return Err(OrbitError::Config(format!("Invalid URI format: {}", uri)));
    }
    
    let protocol = parts[0];
    let rest = parts[1];
    
    match protocol {
        "smb" | "cifs" => parse_smb_uri(rest),
        "s3" => parse_s3_uri(rest),
        "file" => Ok((Protocol::Local, PathBuf::from(rest))),
        _ => Err(OrbitError::Config(format!("Unsupported protocol: {}", protocol))),
    }
}

fn parse_smb_uri(rest: &str) -> Result<(Protocol, PathBuf)> {
    let (auth, server_share_path) = if rest.contains('@') {
        let parts: Vec<&str> = rest.splitn(2, '@').collect();
        (Some(parts[0]), parts[1])
    } else {
        (None, rest)
    };
    
    let (username, password) = if let Some(auth_str) = auth {
        if auth_str.contains(':') {
            let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
            (Some(parts[0].to_string()), Some(parts[1].to_string()))
        } else {
            (Some(auth_str.to_string()), None)
        }
    } else {
        (None, None)
    };
    
    let parts: Vec<&str> = server_share_path.splitn(3, '/').collect();
    if parts.len() < 2 {
        return Err(OrbitError::Config(
            "SMB URI must include server and share: smb://server/share/path".to_string()
        ));
    }
    
    let server = parts[0].to_string();
    let share = parts[1].to_string();
    let path = if parts.len() > 2 {
        format!("/{}", parts[2])
    } else {
        "/".to_string()
    };
    
    Ok((
        Protocol::Smb {
            server,
            share,
            username,
            password,
            domain: None,
        },
        PathBuf::from(path),
    ))
}

/// Parse S3 URI: s3://bucket/key/path?region=us-west-2&endpoint=http://localhost:9000
fn parse_s3_uri(rest: &str) -> Result<(Protocol, PathBuf)> {
    // Split off query parameters if present
    let (bucket_and_path, query_params) = if rest.contains('?') {
        let parts: Vec<&str> = rest.splitn(2, '?').collect();
        (parts[0], Some(parts[1]))
    } else {
        (rest, None)
    };
    
    // Extract credentials from auth section if present (access_key:secret_key@bucket/path)
    let (auth, bucket_path_str) = if bucket_and_path.contains('@') {
        let parts: Vec<&str> = bucket_and_path.splitn(2, '@').collect();
        (Some(parts[0]), parts[1])
    } else {
        (None, bucket_and_path)
    };
    
    let (access_key, secret_key) = if let Some(auth_str) = auth {
        if auth_str.contains(':') {
            let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
            (Some(parts[0].to_string()), Some(parts[1].to_string()))
        } else {
            (Some(auth_str.to_string()), None)
        }
    } else {
        (None, None)
    };
    
    // Split bucket from path
    let parts: Vec<&str> = bucket_path_str.splitn(2, '/').collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(OrbitError::Config(
            "S3 URI must include bucket name: s3://bucket/key".to_string()
        ));
    }
    
    let bucket = parts[0].to_string();
    let path = if parts.len() > 1 && !parts[1].is_empty() {
        format!("/{}", parts[1])
    } else {
        "/".to_string()
    };
    
    // Parse query parameters
    let mut region = None;
    let mut endpoint = None;
    
    if let Some(query_str) = query_params {
        for param in query_str.split('&') {
            if let Some((key, value)) = param.split_once('=') {
                match key {
                    "region" => region = Some(value.to_string()),
                    "endpoint" => endpoint = Some(value.to_string()),
                    _ => {
                        // Ignore unknown query parameters
                    }
                }
            }
        }
    }
    
    Ok((
        Protocol::S3 {
            bucket,
            region,
            endpoint,
            access_key,
            secret_key,
        },
        PathBuf::from(path),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local_path() {
        let (protocol, path) = parse_uri("/tmp/file.txt").unwrap();
        assert!(matches!(protocol, Protocol::Local));
        assert_eq!(path, PathBuf::from("/tmp/file.txt"));
    }

    #[test]
    fn test_parse_smb_simple() {
        let (protocol, path) = parse_uri("smb://server/share/path/file.txt").unwrap();
        
        match protocol {
            Protocol::Smb { server, share, username, password, .. } => {
                assert_eq!(server, "server");
                assert_eq!(share, "share");
                assert!(username.is_none());
                assert!(password.is_none());
            }
            _ => panic!("Expected SMB protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/path/file.txt"));
    }

    #[test]
    fn test_parse_smb_with_auth() {
        let (protocol, path) = parse_uri("smb://user:pass@server/share/file.txt").unwrap();
        
        match protocol {
            Protocol::Smb { server, share, username, password, .. } => {
                assert_eq!(server, "server");
                assert_eq!(share, "share");
                assert_eq!(username, Some("user".to_string()));
                assert_eq!(password, Some("pass".to_string()));
            }
            _ => panic!("Expected SMB protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/file.txt"));
    }

    #[test]
    fn test_parse_invalid_smb() {
        let result = parse_uri("smb://server");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_s3_simple() {
        let (protocol, path) = parse_uri("s3://my-bucket/path/to/file.txt").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, region, endpoint, access_key, secret_key } => {
                assert_eq!(bucket, "my-bucket");
                assert!(region.is_none());
                assert!(endpoint.is_none());
                assert!(access_key.is_none());
                assert!(secret_key.is_none());
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/path/to/file.txt"));
    }

    #[test]
    fn test_parse_s3_with_region() {
        let (protocol, path) = parse_uri("s3://my-bucket/file.txt?region=us-west-2").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, region, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(region, Some("us-west-2".to_string()));
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/file.txt"));
    }

    #[test]
    fn test_parse_s3_with_endpoint() {
        let (protocol, path) = parse_uri("s3://my-bucket/data.json?endpoint=http://localhost:9000").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, endpoint, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(endpoint, Some("http://localhost:9000".to_string()));
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/data.json"));
    }

    #[test]
    fn test_parse_s3_with_region_and_endpoint() {
        let (protocol, path) = parse_uri("s3://my-bucket/dir/file?region=eu-west-1&endpoint=http://minio:9000").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, region, endpoint, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(region, Some("eu-west-1".to_string()));
                assert_eq!(endpoint, Some("http://minio:9000".to_string()));
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/dir/file"));
    }

    #[test]
    fn test_parse_s3_with_credentials() {
        let (protocol, path) = parse_uri("s3://AKIAIOSFODNN7EXAMPLE:wJalrXUtnFEMI/K7MDENG@my-bucket/data").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, access_key, secret_key, .. } => {
                assert_eq!(bucket, "my-bucket");
                assert_eq!(access_key, Some("AKIAIOSFODNN7EXAMPLE".to_string()));
                assert_eq!(secret_key, Some("wJalrXUtnFEMI/K7MDENG".to_string()));
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/data"));
    }

    #[test]
    fn test_parse_s3_bucket_only() {
        let (protocol, path) = parse_uri("s3://my-bucket").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, .. } => {
                assert_eq!(bucket, "my-bucket");
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/"));
    }

    #[test]
    fn test_parse_s3_bucket_with_trailing_slash() {
        let (protocol, path) = parse_uri("s3://my-bucket/").unwrap();
        
        match protocol {
            Protocol::S3 { bucket, .. } => {
                assert_eq!(bucket, "my-bucket");
            }
            _ => panic!("Expected S3 protocol"),
        }
        
        assert_eq!(path, PathBuf::from("/"));
    }

    #[test]
    fn test_parse_s3_invalid_empty_bucket() {
        let result = parse_uri("s3:///path");
        assert!(result.is_err());
    }
}