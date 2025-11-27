# Unified Backend Abstraction Guide

The Orbit backend abstraction provides a modular layer for handling diverse data sources and destinations, enabling seamless integration of local filesystems, remote protocols (SSH/SFTP), and cloud storage providers (S3, Google Cloud Storage, etc.).

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Quick Start](#quick-start)
- [Backend Types](#backend-types)
- [Configuration](#configuration)
- [Advanced Usage](#advanced-usage)
- [Extending with Custom Backends](#extending-with-custom-backends)
- [API Reference](#api-reference)

## Overview

The backend abstraction provides a unified `Backend` trait that all storage implementations must conform to. This enables:

- **Protocol independence**: Write code once, run on any backend
- **Async-first design**: All operations use async/await with Tokio
- **Type safety**: Strong typing with comprehensive error handling
- **Extensibility**: Plugin system for custom backends

## Features

### Core Operations

All backends support these operations:

- `stat()` - Get file/directory metadata
- `list()` - List directory contents (with recursive support)
- `read()` - Read file as async stream
- `write()` - Write data to file
- `delete()` - Delete files/directories
- `mkdir()` - Create directories
- `rename()` - Rename/move files
- `exists()` - Check if path exists

### Metadata Operations (v0.4.1+)

**NEW!** Enhanced metadata operations for comprehensive attribute handling:

- `set_permissions()` - Set Unix permissions (mode bits)
- `set_timestamps()` - Set access and modification times
- `get_xattrs()` - Read extended attributes
- `set_xattrs()` - Write extended attributes
- `set_ownership()` - Set owner and group (UID/GID)

These operations have default implementations that return `Unsupported` for backends that don't implement them, ensuring graceful degradation.

### Security

- Secure credential handling using the `secrecy` crate
- Least-privilege access patterns
- Support for SSH keys, AWS IAM roles, etc.

### Performance

- Streaming I/O for memory efficiency
- Async operations with Tokio runtime
- Support for concurrent transfers
- Batching operations where possible (S3 multi-object delete)

## Quick Start

### Enable Backend Features

Add to your `Cargo.toml`:

```toml
[dependencies]
orbit = { version = "0.4", features = ["backend-abstraction"] }

# Optional: Enable specific backends
# orbit = { version = "0.4", features = ["backend-abstraction", "ssh-backend", "s3-native"] }
```

### Basic Example

```rust
use orbit::backend::{Backend, LocalBackend};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a local filesystem backend
    let backend = LocalBackend::new();

    // Get file metadata
    let metadata = backend.stat(Path::new("Cargo.toml")).await?;
    println!("File size: {} bytes", metadata.size);

    // List directory contents
    let entries = backend.list(Path::new("."), Default::default()).await?;
    for entry in entries {
        println!("{}: {} bytes", entry.path.display(), entry.metadata.size);
    }

    Ok(())
}
```

## Backend Types

### Local Filesystem

Access local files and directories:

```rust
use orbit::backend::{Backend, LocalBackend};

// Unrestricted access
let backend = LocalBackend::new();

// Rooted at specific directory
let backend = LocalBackend::with_root("/data");
```

**URI Format**: `file:///path/to/dir` or just `/path/to/dir`

### SSH/SFTP (Feature: `ssh-backend`) ✅ Production-Ready

Access remote filesystems over SSH with full async support and multiple authentication methods:

```rust
use orbit::backend::{Backend, SshBackend, SshConfig, SshAuth};
use secrecy::SecretString;

// SSH Agent (Recommended - most secure)
let config = SshConfig::new(
    "example.com",
    "username",
    SshAuth::Agent
)
.with_port(22)
.with_timeout(30)
.with_compression(); // Optional: enable SSH compression

let backend = SshBackend::connect(config).await?;

// Key-based authentication with passphrase
let config = SshConfig::new(
    "example.com",
    "username",
    SshAuth::KeyFile {
        key_path: "/home/user/.ssh/id_rsa".into(),
        passphrase: Some(SecretString::new("keypass".into())),
    }
);

let backend = SshBackend::connect(config).await?;

// Password authentication (least secure - use only when necessary)
let config = SshConfig::new(
    "example.com",
    "username",
    SshAuth::Password(SecretString::new("password".into()))
);

let backend = SshBackend::connect(config).await?;

// Perform operations
let metadata = backend.stat(Path::new("/remote/file.txt")).await?;
let entries = backend.list(Path::new("/remote/dir"), Default::default()).await?;
```

**Features:**
- ✅ Full async I/O with `tokio::task::spawn_blocking`
- ✅ Three authentication methods (Agent, KeyFile, Password)
- ✅ Secure credential handling with `secrecy` crate
- ✅ Connection timeout configuration
- ✅ Optional SSH compression
- ✅ All Backend trait operations supported
- ✅ Recursive directory operations

**URI Format**: `ssh://user@host:port/path?key=/path/to/key` or `sftp://user@host/path?agent=true`

**Authentication Priority**: SSH Agent → Private Key → Password

### S3-Compatible Storage (Feature: `s3-native`)

Access AWS S3 and compatible services (MinIO, LocalStack, etc.):

```rust
use orbit::backend::{Backend, S3Backend};
use orbit::protocol::s3::S3Config;

// AWS S3
let config = S3Config {
    bucket: "my-bucket".to_string(),
    region: Some("us-east-1".to_string()),
    ..Default::default()
};

let backend = S3Backend::new(config).await?;

// MinIO or S3-compatible
let config = S3Config {
    bucket: "my-bucket".to_string(),
    endpoint: Some("http://localhost:9000".to_string()),
    region: Some("us-east-1".to_string()),
    access_key: Some("minioadmin".to_string()),
    secret_key: Some("minioadmin".to_string()),
    force_path_style: true,
    ..Default::default()
};

let backend = S3Backend::new(config).await?;
```

**URI Format**: `s3://bucket/prefix?region=us-east-1&endpoint=http://localhost:9000`

## Configuration

### URI-Based Configuration

Parse backends from URI strings:

```rust
use orbit::backend::{parse_uri, BackendConfig};

// Local filesystem
let (config, path) = parse_uri("/tmp/data")?;
let (config, path) = parse_uri("file:///tmp/data")?;

// SSH
let (config, path) = parse_uri("ssh://user@host:22/remote/path?key=/path/to/key")?;

// S3
let (config, path) = parse_uri("s3://bucket/prefix?region=us-west-2")?;
```

### Environment Variables

Configure backends from environment:

```rust
use orbit::backend::config::from_env;

// Set environment variables:
// ORBIT_BACKEND_TYPE=ssh
// ORBIT_SSH_HOST=example.com
// ORBIT_SSH_USER=admin
// ORBIT_SSH_KEY=/path/to/key

let config = from_env()?;
```

### Backend Registry

Use the global registry to create backends from configuration:

```rust
use orbit::backend::{global_registry, BackendConfig};

let registry = global_registry();

// Create from config
let config = BackendConfig::local();
let backend = registry.create(&config).await?;

// Create from URI
let (backend, path) = registry.create_from_uri("s3://my-bucket/data").await?;
```

## Advanced Usage

### Listing with Options

Control how directories are listed:

```rust
use orbit::backend::{Backend, ListOptions};

let backend = LocalBackend::new();

// Recursive listing
let entries = backend.list(
    Path::new("/data"),
    ListOptions::recursive()
        .with_max_depth(3)
        .include_hidden()
).await?;

// Shallow listing (direct children only)
let entries = backend.list(
    Path::new("/data"),
    ListOptions::shallow()
).await?;
```

### Writing with Options

Customize write behavior:

```rust
use orbit::backend::{Backend, WriteOptions};
use bytes::Bytes;

let backend = LocalBackend::new();
let data = Bytes::from("Hello, World!");

// Write with custom options
let written = backend.write(
    Path::new("output.txt"),
    data,
    WriteOptions::new()
        .with_content_type("text/plain".to_string())
        .with_permissions(0o644)
        .no_overwrite()
).await?;
```

### Streaming Reads

Efficiently read large files as streams:

```rust
use orbit::backend::Backend;
use futures::StreamExt;

let backend = LocalBackend::new();
let mut stream = backend.read(Path::new("large-file.bin")).await?;

while let Some(chunk) = stream.next().await {
    let bytes = chunk?;
    // Process chunk
    println!("Read {} bytes", bytes.len());
}
```

### Cross-Backend Transfers

Transfer data between different backends:

```rust
use orbit::backend::{Backend, LocalBackend, S3Backend};

async fn copy_to_s3(
    local: &LocalBackend,
    s3: &S3Backend,
    local_path: &Path,
    s3_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read from local
    let mut stream = local.read(local_path).await?;

    // Collect data (for small files)
    let mut data = Vec::new();
    while let Some(chunk) = stream.next().await {
        data.extend_from_slice(&chunk?);
    }

    // Write to S3
    s3.write(s3_path, Bytes::from(data), Default::default()).await?;

    Ok(())
}
```

## Extending with Custom Backends

### Implementing the Backend Trait

Create a custom backend by implementing the `Backend` trait:

```rust
use orbit::backend::{Backend, BackendResult, Metadata, DirEntry, ListOptions, WriteOptions, ReadStream};
use async_trait::async_trait;
use std::path::Path;
use bytes::Bytes;

struct MyCustomBackend {
    // Your backend state
}

#[async_trait]
impl Backend for MyCustomBackend {
    async fn stat(&self, path: &Path) -> BackendResult<Metadata> {
        // Implementation
        todo!()
    }

    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>> {
        // Implementation
        todo!()
    }

    async fn read(&self, path: &Path) -> BackendResult<ReadStream> {
        // Implementation
        todo!()
    }

    async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64> {
        // Implementation
        todo!()
    }

    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        // Implementation
        todo!()
    }

    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()> {
        // Implementation
        todo!()
    }

    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()> {
        // Implementation
        todo!()
    }

    fn backend_name(&self) -> &str {
        "my-custom-backend"
    }
}
```

### Registering Custom Backends

Register your backend with the global registry:

```rust
use orbit::backend::{global_registry, BackendConfig};
use std::sync::Arc;

let registry = global_registry();

registry.register("custom", Arc::new(|config| {
    Box::pin(async move {
        let backend = MyCustomBackend::new(config).await?;
        Ok(Box::new(backend) as Box<dyn Backend>)
    })
}));

// Now you can create it
let config = /* your custom config */;
let backend = registry.create(&config).await?;
```

## API Reference

### Backend Trait

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    async fn stat(&self, path: &Path) -> BackendResult<Metadata>;
    async fn list(&self, path: &Path, options: ListOptions) -> BackendResult<Vec<DirEntry>>;
    async fn read(&self, path: &Path) -> BackendResult<ReadStream>;
    async fn write(&self, path: &Path, data: Bytes, options: WriteOptions) -> BackendResult<u64>;
    async fn delete(&self, path: &Path, recursive: bool) -> BackendResult<()>;
    async fn mkdir(&self, path: &Path, recursive: bool) -> BackendResult<()>;
    async fn rename(&self, src: &Path, dest: &Path) -> BackendResult<()>;
    async fn exists(&self, path: &Path) -> BackendResult<bool> { /* default impl */ }
    fn backend_name(&self) -> &str;
    fn supports(&self, operation: &str) -> bool { /* default impl */ }
}
```

### Metadata

```rust
pub struct Metadata {
    pub size: u64,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub permissions: Option<u32>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub custom_metadata: Option<HashMap<String, String>>,
}
```

### ListOptions

```rust
pub struct ListOptions {
    pub recursive: bool,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub max_entries: Option<usize>,
}
```

### WriteOptions

```rust
pub struct WriteOptions {
    pub create_parents: bool,
    pub overwrite: bool,
    pub content_type: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub permissions: Option<u32>,
}
```

### BackendError

```rust
pub enum BackendError {
    Io(io::Error),
    NotFound { path: PathBuf, backend: String },
    PermissionDenied { path: PathBuf, message: String },
    AuthenticationFailed { backend: String, message: String },
    ConnectionFailed { backend: String, endpoint: String, source: Option<Box<dyn Error>> },
    Timeout { operation: String, duration_secs: u64 },
    InvalidConfig { backend: String, message: String },
    Unsupported { backend: String, operation: String },
    // ... and more
}
```

## Testing

Run backend tests:

```bash
# Test all backends
cargo test --features backend-abstraction --lib backend

# Test specific backend
cargo test --features backend-abstraction,ssh-backend --lib backend::ssh

# Test with S3
cargo test --features backend-abstraction,s3-native --lib backend::s3
```

## Performance Considerations

1. **Streaming**: Use streaming for large files to avoid loading everything into memory
2. **Batching**: Use batch operations when available (e.g., S3 batch delete)
3. **Async Concurrency**: Run multiple operations concurrently using `tokio::spawn`
4. **Connection Pooling**: Reuse backend instances instead of creating new ones

## Security Best Practices

1. **Credentials**: Always use the `secrecy` crate for sensitive data
2. **Least Privilege**: Grant minimal required permissions
3. **Encryption**: Use encrypted connections (SSH, HTTPS for S3)
4. **Key Management**: Store SSH keys and AWS credentials securely
5. **Validation**: Always validate user input before using in paths

## Troubleshooting

### SSH Connection Issues

- Verify SSH host key is in `known_hosts`
- Check firewall rules allow port 22 (or custom port)
- Ensure SSH service is running on remote host
- Verify username and authentication method

### S3 Access Issues

- Check AWS credentials are valid
- Verify IAM permissions for bucket operations
- Ensure bucket exists and region is correct
- For MinIO/LocalStack, use `force_path_style: true`

### Performance Issues

- Use streaming for large files
- Enable compression for SSH transfers
- Use multipart upload for large S3 objects
- Consider batch operations where possible

## Examples

See the `examples/` directory for complete working examples:

- `local_backend.rs` - Local filesystem operations
- `ssh_backend.rs` - SSH/SFTP file transfers
- `s3_backend.rs` - S3 upload/download
- `cross_backend_transfer.rs` - Transfer between backends
- `custom_backend.rs` - Implementing custom backends

## License

Apache-2.0

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
