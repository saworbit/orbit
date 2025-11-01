# S3 Integration Guide

**Version:** v0.4.0
**Status:** Production Ready
**Last Updated:** November 1, 2025

---

## Overview

Orbit v0.4.0 introduces native AWS S3 support, enabling seamless file transfers between local storage and S3 buckets. The implementation is pure Rust, async-first, and designed for high-performance data transfers with built-in resilience features.

### Key Features

✅ **Pure Rust** - No external dependencies or binaries required  
✅ **Async Operations** - Built on Tokio for high concurrency  
✅ **Multipart Upload/Download** - Efficient handling of large files (>5MB)  
✅ **Resumable Transfers** - Automatic resume on interruption  
✅ **Parallel Operations** - Configurable concurrent chunk transfers  
✅ **Integrity Verification** - Built-in checksum validation  
✅ **Flexible Authentication** - Multiple credential sources  
✅ **S3-Compatible Storage** - Works with MinIO, LocalStack, and other S3-compatible services  

---

## Installation

### Enable S3 Support

Build Orbit with the `s3-native` feature flag:

```bash
cargo build --release --features s3-native
```

Or add to your `Cargo.toml` if using Orbit as a library:

```toml
[dependencies]
orbit = { version = "0.4", features = ["s3-native"] }
```

### System Requirements

- **Rust:** 1.70 or later
- **Network:** Outbound HTTPS access to S3 endpoints
- **Memory:** Recommended 512MB+ for large file transfers

---

## Quick Start

### Basic Upload

```rust
use orbit::protocol::s3::{S3Client, S3Config};
use bytes::Bytes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration
    let config = S3Config::new("my-bucket".to_string());
    
    // Create client
    let client = S3Client::new(config).await?;
    
    // Upload data
    let data = Bytes::from("Hello, S3!");
    client.upload_bytes(data, "path/to/file.txt").await?;
    
    println!("Upload complete!");
    Ok(())
}
```

### Basic Download

```rust
use orbit::protocol::s3::{S3Client, S3Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = S3Config::new("my-bucket".to_string());
    let client = S3Client::new(config).await?;
    
    // Download data
    let data = client.download_bytes("path/to/file.txt").await?;
    println!("Downloaded {} bytes", data.len());
    
    Ok(())
}
```

---

## Authentication

Orbit supports multiple authentication methods, following the standard AWS SDK credential chain.

### 1. Environment Variables (Recommended)

```bash
export AWS_ACCESS_KEY_ID="AKIAIOSFODNN7EXAMPLE"
export AWS_SECRET_ACCESS_KEY="wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
export AWS_REGION="us-east-1"
```

Then use Orbit without explicit credentials:

```rust
let config = S3Config::new("my-bucket".to_string());
let client = S3Client::new(config).await?;
```

### 2. AWS Credentials File

Create `~/.aws/credentials`:

```ini
[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
```

And `~/.aws/config`:

```ini
[default]
region = us-east-1
```

### 3. IAM Roles (EC2/ECS/Lambda)

When running on AWS infrastructure, Orbit automatically uses IAM role credentials:

```rust
// No credentials needed - automatically retrieved from instance metadata
let config = S3Config::new("my-bucket".to_string());
let client = S3Client::new(config).await?;
```

### 4. Explicit Credentials

For programmatic access:

```rust
use orbit::protocol::s3::S3ConfigBuilder;

let config = S3ConfigBuilder::new("my-bucket".to_string())
    .credentials(
        "AKIAIOSFODNN7EXAMPLE".to_string(),
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()
    )
    .region("us-east-1".to_string())
    .build()?;

let client = S3Client::new(config).await?;
```

⚠️ **Security Warning:** Never hardcode credentials in source code. Use environment variables or AWS credential files instead.

---

## Configuration

### Basic Configuration

```rust
use orbit::protocol::s3::S3Config;

let config = S3Config::new("my-bucket".to_string());
```

### Advanced Configuration with Builder

```rust
use orbit::protocol::s3::{S3ConfigBuilder, S3StorageClass, S3ServerSideEncryption};

let config = S3ConfigBuilder::new("my-data-lake".to_string())
    .region("us-west-2".to_string())
    .storage_class(S3StorageClass::IntelligentTiering)
    .server_side_encryption(S3ServerSideEncryption::Aes256)
    .chunk_size(10 * 1024 * 1024)  // 10MB chunks
    .parallel_operations(8)         // 8 concurrent operations
    .timeout_seconds(600)           // 10 minute timeout
    .verify_checksums(true)
    .build()?;
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `bucket` | S3 bucket name (required) | - |
| `region` | AWS region | Auto-detect |
| `endpoint` | Custom endpoint for S3-compatible services | None |
| `storage_class` | Storage class for uploads | `Standard` |
| `server_side_encryption` | Encryption at rest | `None` |
| `chunk_size` | Size of multipart chunks | 5MB |
| `parallel_operations` | Concurrent operations | 4 |
| `timeout_seconds` | Request timeout | 300s |
| `max_retries` | Retry attempts for failed operations | 3 |
| `verify_checksums` | Enable integrity checking | `true` |
| `force_path_style` | Use path-style URLs (for MinIO) | `false` |

---

## Operations

### Check Object Existence

```rust
if client.exists("data/report.pdf").await? {
    println!("File exists");
}
```

### Get Object Metadata

```rust
let metadata = client.get_metadata("data/large-file.bin").await?;
println!("Size: {} bytes", metadata.size);
println!("Last modified: {:?}", metadata.last_modified);
println!("ETag: {:?}", metadata.etag);
```

### List Objects

```rust
use orbit::protocol::s3::S3Operations;

let result = client.list_objects("data/2025/").await?;

for object in result.objects {
    println!("{}: {} bytes", object.key, object.size);
}

// Handle pagination for large listings
if result.is_truncated {
    if let Some(token) = result.continuation_token {
        let next_page = client.list_objects_paginated(
            "data/2025/",
            Some(token),
            None
        ).await?;
    }
}
```

### Copy Objects

```rust
// Copy within same bucket
client.copy_object("source/file.txt", "destination/file.txt").await?;
```

### Delete Objects

```rust
client.delete("old-data/file.txt").await?;
```

---

## Large File Transfers

### Multipart Upload

Orbit automatically uses multipart uploads for files larger than the configured chunk size:

```rust
use std::path::Path;

// Automatically uses multipart for large files
client.upload_file_multipart(
    Path::new("local/large-dataset.tar.gz"),
    "backups/dataset.tar.gz",
    None  // No resume state
).await?;
```

### Resumable Upload

If an upload is interrupted, you can resume it:

```rust
use orbit::protocol::s3::ResumeState;

// First attempt
let resume_state = match client.upload_file_multipart(
    Path::new("huge-file.bin"),
    "uploads/huge-file.bin",
    None
).await {
    Ok(state) => state,
    Err(e) => {
        eprintln!("Upload failed: {}", e);
        // Save resume state for later
        return Err(e.into());
    }
};

// Resume from interruption
let final_state = client.upload_file_multipart(
    Path::new("huge-file.bin"),
    "uploads/huge-file.bin",
    Some(resume_state)  // Resume from previous attempt
).await?;
```

### Resumable Download

Download large files with automatic resume support:

```rust
use std::path::Path;

client.download_file_resumable(
    "backups/large-backup.tar",
    Path::new("local/backup.tar"),
    0  // Start from beginning (or use offset to resume)
).await?;
```

---

## Storage Classes

Optimize costs by selecting appropriate storage classes:

```rust
use orbit::protocol::s3::{S3ConfigBuilder, S3StorageClass};

// For frequently accessed data
let config = S3ConfigBuilder::new("hot-data".to_string())
    .storage_class(S3StorageClass::Standard)
    .build()?;

// For infrequently accessed data
let config = S3ConfigBuilder::new("archive".to_string())
    .storage_class(S3StorageClass::StandardIa)
    .build()?;

// For cost optimization with automatic tiering
let config = S3ConfigBuilder::new("analytics".to_string())
    .storage_class(S3StorageClass::IntelligentTiering)
    .build()?;

// For long-term archives
let config = S3ConfigBuilder::new("cold-storage".to_string())
    .storage_class(S3StorageClass::GlacierFlexibleRetrieval)
    .build()?;
```

### Available Storage Classes

- **Standard** - Frequently accessed data
- **StandardIa** - Infrequently accessed data (cheaper storage, retrieval fee)
- **OnezoneIa** - Infrequent access, single AZ (lowest cost IA)
- **IntelligentTiering** - Automatic cost optimization
- **GlacierInstantRetrieval** - Archive with millisecond retrieval
- **GlacierFlexibleRetrieval** - Archive with minutes-hours retrieval
- **GlacierDeepArchive** - Lowest cost archive (hours retrieval)

---

## Server-Side Encryption

### AES-256 Encryption

```rust
use orbit::protocol::s3::{S3ConfigBuilder, S3ServerSideEncryption};

let config = S3ConfigBuilder::new("encrypted-bucket".to_string())
    .server_side_encryption(S3ServerSideEncryption::Aes256)
    .build()?;
```

### AWS KMS Encryption

```rust
let config = S3ConfigBuilder::new("kms-bucket".to_string())
    .server_side_encryption(S3ServerSideEncryption::AwsKms {
        key_id: Some("arn:aws:kms:us-east-1:123456789:key/12345".to_string())
    })
    .build()?;
```

---

## S3-Compatible Storage

Orbit works with any S3-compatible storage service.

### MinIO

```rust
use orbit::protocol::s3::S3ConfigBuilder;

let config = S3ConfigBuilder::new("my-bucket".to_string())
    .endpoint("http://localhost:9000".to_string())
    .region("us-east-1".to_string())  // Required even for MinIO
    .credentials("minioadmin".to_string(), "minioadmin".to_string())
    .force_path_style(true)  // Required for MinIO
    .build()?;

let client = S3Client::new(config).await?;
```

### LocalStack

```rust
let config = S3ConfigBuilder::new("test-bucket".to_string())
    .endpoint("http://localhost:4566".to_string())
    .region("us-east-1".to_string())
    .credentials("test".to_string(), "test".to_string())
    .force_path_style(true)
    .build()?;
```

### DigitalOcean Spaces

```rust
let config = S3ConfigBuilder::new("my-space".to_string())
    .endpoint("https://nyc3.digitaloceanspaces.com".to_string())
    .region("nyc3".to_string())
    .credentials(
        std::env::var("SPACES_KEY")?,
        std::env::var("SPACES_SECRET")?
    )
    .build()?;
```

---

## Error Handling

```rust
use orbit::protocol::s3::{S3Client, S3Error};

match client.download_bytes("missing-file.txt").await {
    Ok(data) => println!("Downloaded {} bytes", data.len()),
    Err(S3Error::NotFound { bucket, key }) => {
        eprintln!("File not found: {}/{}", bucket, key);
    }
    Err(S3Error::AccessDenied(msg)) => {
        eprintln!("Access denied: {}", msg);
    }
    Err(S3Error::Network(msg)) => {
        eprintln!("Network error (retryable): {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}

// Check if error is retryable
if let Err(e) = client.upload_bytes(data, "key").await {
    if e.is_retryable() {
        println!("Temporary error, safe to retry");
    }
}
```

---

## Performance Tuning

### Optimize for Throughput

```rust
let config = S3ConfigBuilder::new("my-bucket".to_string())
    .chunk_size(20 * 1024 * 1024)  // 20MB chunks for high-bandwidth
    .parallel_operations(16)        // More concurrent operations
    .build()?;
```

### Optimize for Latency

```rust
let config = S3ConfigBuilder::new("my-bucket".to_string())
    .chunk_size(5 * 1024 * 1024)   // Smaller chunks
    .parallel_operations(4)         // Fewer concurrent operations
    .timeout_seconds(60)            // Shorter timeout
    .build()?;
```

### Benchmarking

```bash
# Run performance tests
cargo test --release --features s3-native -- --ignored --test-threads=1
```

---

## Testing

### Unit Tests

```bash
# Run S3 unit tests (no S3 connection required)
cargo test --features s3-native --lib protocol::s3
```

### Integration Tests

Requires a running S3 or S3-compatible service:

```bash
# Set up environment
export S3_TESTS_ENABLED=1
export S3_TEST_BUCKET=orbit-test
export S3_TEST_ENDPOINT=http://localhost:9000  # For MinIO
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin

# Run integration tests
cargo test --features s3-native -- --ignored
```

### Testing with MinIO (Docker)

```bash
# Start MinIO
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  --name minio \
  minio/minio server /data --console-address ":9001"

# Create test bucket
docker exec minio mc mb /data/orbit-test

# Run tests
export S3_TESTS_ENABLED=1
export S3_TEST_BUCKET=orbit-test
export S3_TEST_ENDPOINT=http://localhost:9000
export AWS_ACCESS_KEY_ID=minioadmin
export AWS_SECRET_ACCESS_KEY=minioadmin

cargo test --features s3-native -- --ignored
```

---

## Common Use Cases

### Data Lake Ingestion

```rust
// Upload analytics data to S3 data lake
let config = S3ConfigBuilder::new("data-lake".to_string())
    .region("us-west-2".to_string())
    .storage_class(S3StorageClass::IntelligentTiering)
    .build()?;

let client = S3Client::new(config).await?;

// Upload Parquet files
client.upload_file_multipart(
    Path::new("analytics/sales_2025.parquet"),
    "raw/sales/2025/sales_2025.parquet",
    None
).await?;
```

### Backup and Archive

```rust
// Archive backups to Glacier
let config = S3ConfigBuilder::new("backups".to_string())
    .storage_class(S3StorageClass::GlacierFlexibleRetrieval)
    .server_side_encryption(S3ServerSideEncryption::Aes256)
    .build()?;

let client = S3Client::new(config).await?;
client.upload_file_multipart(
    Path::new("/backups/db_backup_2025-10-25.tar.gz"),
    "database/2025/10/db_backup_2025-10-25.tar.gz",
    None
).await?;
```

### Cloud Migration

```rust
// Migrate local storage to S3
use walkdir::WalkDir;

let config = S3Config::new("migration-bucket".to_string());
let client = S3Client::new(config).await?;

for entry in WalkDir::new("/data/to/migrate") {
    let entry = entry?;
    if entry.file_type().is_file() {
        let relative_path = entry.path().strip_prefix("/data/to/migrate")?;
        let s3_key = relative_path.to_str().unwrap();
        
        client.upload_file_multipart(
            entry.path(),
            s3_key,
            None
        ).await?;
        
        println!("Migrated: {}", s3_key);
    }
}
```

---

## Troubleshooting

### Connection Issues

**Problem:** Cannot connect to S3

**Solutions:**
- Verify network connectivity: `ping s3.amazonaws.com`
- Check firewall rules (allow outbound HTTPS/443)
- Verify endpoint URL if using S3-compatible storage
- Test credentials with AWS CLI: `aws s3 ls s3://my-bucket/`

### Authentication Failures

**Problem:** Access Denied errors

**Solutions:**
- Verify credentials are correct
- Check IAM permissions (requires `s3:GetObject`, `s3:PutObject`, etc.)
- Ensure bucket policy allows access
- Verify bucket exists and is in the correct region

### Slow Performance

**Problem:** Transfers are slower than expected

**Solutions:**
- Increase `chunk_size` for high-bandwidth connections
- Increase `parallel_operations` for better throughput
- Check network bandwidth: `iperf3` or similar tools
- Consider using S3 Transfer Acceleration for global transfers

### Multipart Upload Failures

**Problem:** Large file uploads fail midway

**Solutions:**
- Implement retry logic with resume state
- Increase `timeout_seconds` for slow connections
- Reduce `chunk_size` for unreliable networks
- Check available disk space and memory

---

## Best Practices

### Security
- ✅ Use IAM roles when running on AWS infrastructure
- ✅ Store credentials in AWS credentials file or environment variables
- ✅ Enable server-side encryption for sensitive data
- ✅ Use least-privilege IAM policies
- ❌ Never hardcode credentials in source code
- ❌ Don't commit credentials to version control

### Performance
- ✅ Use multipart uploads for files >100MB
- ✅ Enable parallel operations for better throughput
- ✅ Choose appropriate chunk sizes based on network conditions
- ✅ Implement retry logic for transient failures
- ✅ Monitor transfer speeds and adjust configuration

### Cost Optimization
- ✅ Use Intelligent Tiering for unpredictable access patterns
- ✅ Use Glacier for long-term archival
- ✅ Enable lifecycle policies to transition old data
- ✅ Monitor S3 costs with AWS Cost Explorer
- ✅ Delete unnecessary data and failed multipart uploads

---

## Roadmap

### Planned Features (v0.4.1+)

- [ ] Object versioning support
- [ ] S3 Select queries
- [ ] Batch operations
- [ ] Progress callbacks for UI integration
- [ ] Bandwidth throttling
- [ ] Advanced retry strategies
- [ ] CloudWatch metrics integration

---

## Support

### Documentation
- **Orbit README:** [README.md](../README.md)
- **Protocol Guide:** [PROTOCOL_GUIDE.md](PROTOCOL_GUIDE.md)
- **API Documentation:** Run `cargo doc --features s3-native --open`

### Issues
Report bugs or request features: https://github.com/saworbit/orbit/issues

### Community
- Discussions: https://github.com/saworbit/orbit/discussions

---

## License

This feature is part of Orbit and follows the same license as the main project.