# Protocol Support in Orbit

Orbit v0.4.1 introduces a **protocol abstraction layer** that enables copying files across different storage backends using a unified interface.

---

## 🎯 Overview

The protocol system allows you to specify sources and destinations using URI syntax:

```
protocol://[credentials@]location/path
```

This enables seamless copying between:
- Local filesystems
- Network shares (SMB/CIFS)
- Cloud storage (S3 available now, Azure Blob and GCS coming soon)

---

## 📋 Supported Protocols

### ✅ Local Filesystem (Stable)

**Protocol:** `file://` or direct path

**Status:** Production-ready

**Examples:**
```bash
# Direct path (recommended for local files)
orbit -s /tmp/file.txt -d /backup/file.txt

# Explicit file:// URI
orbit -s file:///tmp/file.txt -d file:///backup/file.txt

# Cross-platform paths
orbit -s ./source/data.csv -d /mnt/external/data.csv

# Works with all Orbit features
orbit -s /home/user/docs -d /backup/docs \
  -R \
  --compress zstd:9 \
  --preserve-metadata
```

---

### 🚧 SMB/CIFS Network Shares (Experimental)

**Protocol:** `smb://` or `cifs://`

**Status:** Experimental stub implementation (v0.4.0, awaiting upstream dependency fix)

**Production-ready:** v0.4.1 (planned Q1 2026)

**URI Format:**
```
smb://[user[:password]@]server/share/path
```

**Examples:**
```bash
# Anonymous access (if share allows)
orbit -s smb://fileserver/documents/report.pdf -d ./report.pdf

# With credentials
orbit -s smb://jdoe:pass123@fileserver/documents/report.pdf -d ./report.pdf

# Copy from SMB to local
orbit -s smb://server/share/source.txt -d /local/dest.txt

# Copy from local to SMB
orbit -s /local/source.txt -d smb://server/share/dest.txt

# Recursive directory copy (when fully implemented)
orbit -s smb://server/projects/data -d /backup/data -R
```

**Security Modes:**

The SMB implementation supports three security modes:

1. **RequireEncryption** (Most Secure)
   - Forces SMB3 encryption for all traffic
   - Connection fails if server doesn't support encryption
   - Best for sensitive data over untrusted networks
   ```rust
   SmbTarget {
       security: SmbSecurity::RequireEncryption,
       // ... other fields
   }
   ```

2. **SignOnly** (Performance-Optimized)
   - Disables encryption but enforces packet signing
   - Provides integrity protection without encryption overhead
   - Use for trusted networks where performance is critical
   ```rust
   SmbTarget {
       security: SmbSecurity::SignOnly,
       // ... other fields
   }
   ```

3. **Opportunistic** (Flexible, Default)
   - Uses encryption if available, falls back to signing
   - Balances security and compatibility
   - Recommended for most use cases
   ```rust
   SmbTarget {
       security: SmbSecurity::Opportunistic,
       // ... other fields
   }
   ```

**Security Features:**
- ✅ Automatic policy enforcement - connection fails if requirements not met
- ✅ SMB3 encryption (AES-128/256-GCM, AES-128/256-CCM)
- ✅ Packet signing (HMAC-SHA256, AES-GMAC, AES-CMAC)
- ✅ Unsigned guest access disabled by default
- ✅ Credential protection (zeroed on drop)

**Current Limitations (v0.4.0/v0.4.1):**
- ⚠️ Stub implementation only - awaiting upstream dependency fix
- ⚠️ Not recommended for production use
- ⚠️ Authentication not fully implemented
- ⚠️ Large file transfers untested

**Coming in v0.4.1:**
- ✅ Full SMB protocol implementation
- ✅ Kerberos/NTLM authentication
- ✅ Domain support
- ✅ Performance optimizations
- ✅ Comprehensive testing

---

### ✅ Amazon S3 (Available in v0.4.1)

**Protocol:** `s3://`

**Status:** Production-ready (feature flag: `s3-native`)

**URI Format:**
```
s3://bucket/path/to/object
```

**Examples:**
```bash
# Upload to S3
orbit --source ./local-file.txt --dest s3://my-bucket/backups/file.txt

# Download from S3
orbit --source s3://my-bucket/data/report.pdf --dest ./report.pdf

# Sync directory to S3
orbit --source /local/photos --dest s3://my-bucket/photos/ \
  --mode sync --recursive --compress zstd:5

# Use with MinIO or S3-compatible storage
export S3_ENDPOINT=http://localhost:9000
orbit --source file.txt --dest s3://my-bucket/file.txt
```

**Features:**
- ✅ Multipart upload/download for large files
- ✅ Resumable transfers with checkpoints
- ✅ Parallel chunk transfers
- ✅ S3-compatible services (MinIO, LocalStack, DigitalOcean Spaces)
- ✅ Flexible authentication (env vars, credentials file, IAM roles)
- ✅ Server-side encryption support

**Configuration:** See [`docs/S3_USER_GUIDE.md`](docs/S3_USER_GUIDE.md) for complete setup guide.

---

### 🔮 Cloud Protocols (Planned)

#### Azure Blob Storage (v0.5.0)
```bash
orbit --source azure://account/container/blob --dest ./file.txt
orbit --source ./file.txt --dest azure://account/container/blob
```

#### Google Cloud Storage (v0.5.0)
```bash
orbit --source gs://bucket/object --dest ./file.txt
orbit --source ./file.txt --dest gs://bucket/object
```

---

## 🏗️ Architecture

### How It Works

```
┌─────────────────────────────────────────┐
│         Orbit CLI / Library API         │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│        Protocol URI Parser              │
│  (parses smb://server/share/file)       │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│       Protocol Enum & Router            │
│  (selects appropriate backend)          │
└──────────────────┬──────────────────────┘
                   │
        ┌──────────┼──────────┐
        ▼          ▼          ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │ Local  │ │  SMB   │ │  S3    │
   │Backend │ │Backend │ │Backend │
   └────────┘ └────────┘ └────────┘
```

### StorageBackend Trait

All protocols implement the `StorageBackend` trait:

```rust
pub trait StorageBackend: Send + Sync {
    fn open_read(&self, path: &Path) -> Result<Box<dyn Read + Send>>;
    fn open_write(&self, path: &Path, append: bool) -> Result<Box<dyn Write + Send>>;
    fn metadata(&self, path: &Path) -> Result<FileMetadata>;
    fn exists(&self, path: &Path) -> Result<bool>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;
    fn remove_file(&self, path: &Path) -> Result<()>;
    fn sync(&self, path: &Path) -> Result<()>;
    fn protocol_name(&self) -> &'static str;
}
```

This ensures all protocols have consistent behavior.

---

## 💻 Using Protocols in Code

### Basic Usage

```rust
use orbit::protocol::Protocol;
use orbit::config::CopyConfig;
use orbit::core::copy_file;

fn main() -> orbit::error::Result<()> {
    // Parse source URI
    let (src_protocol, src_path) = Protocol::from_uri("smb://server/share/file.txt")?;
    
    // Parse destination URI
    let (dest_protocol, dest_path) = Protocol::from_uri("/local/file.txt")?;
    
    // Create backends
    let src_backend = src_protocol.create_backend()?;
    let dest_backend = dest_protocol.create_backend()?;
    
    // Use with copy operations (future API)
    let config = CopyConfig::default();
    copy_file(&src_path, &dest_path, &config)?;
    
    Ok(())
}
```

### Advanced: Custom Backend

You can implement your own storage backend:

```rust
use orbit::protocol::StorageBackend;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

struct MyCustomBackend {
    // Your implementation
}

impl StorageBackend for MyCustomBackend {
    fn open_read(&self, path: &Path) -> orbit::error::Result<Box<dyn Read + Send>> {
        // Your implementation
        todo!()
    }
    
    fn open_write(&self, path: &Path, append: bool) -> orbit::error::Result<Box<dyn Write + Send>> {
        // Your implementation
        todo!()
    }
    
    // Implement other required methods...
    
    fn protocol_name(&self) -> &'static str {
        "custom"
    }
}
```

---

## 🔐 Security Considerations

### Credentials in URIs

**⚠️ Warning:** Putting passwords in URIs is convenient but insecure:

```bash
# ❌ BAD: Password visible in command history
orbit -s smb://user:password@server/share/file.txt -d ./file.txt
```

**Better approaches (coming in v0.4.1):**

```bash
# ✅ GOOD: Use environment variables
export SMB_USERNAME=jdoe
export SMB_PASSWORD=secret
orbit -s smb://server/share/file.txt -d ./file.txt

# ✅ GOOD: Interactive password prompt
orbit -s smb://jdoe@server/share/file.txt -d ./file.txt
# Password: [hidden input]

# ✅ GOOD: Credential file
orbit -s smb://server/share/file.txt -d ./file.txt --credentials ~/.orbit/creds.toml
```

### Network Security

- Always use encrypted protocols when available
- Consider VPN/SSH tunneling for sensitive data
- Audit logs may contain file paths - review before sharing

---

## 🧪 Testing Protocol Support

### Test SMB (v0.4.0/v0.4.1 stub)

```bash
# This will connect but not actually transfer
orbit -s smb://testserver/testshare/file.txt -d ./test.txt

# Check verbose output to see protocol detection
orbit -s smb://server/share/file.txt -d ./output.txt --verbose
```

### Test Local Protocol

```bash
# These are equivalent
orbit -s file:///tmp/test.txt -d /backup/test.txt
orbit -s /tmp/test.txt -d /backup/test.txt
```

---

## 📊 Performance by Protocol

| Protocol | Relative Speed | Best For |
|----------|----------------|----------|
| Local | 100% (baseline) | Same-machine copies |
| SMB (LAN) | ~60-80% | Local network shares |
| SMB (WAN) | ~5-30% | Remote networks |
| S3 | Varies | Cloud storage, CDN |

**Tip:** Use compression for network protocols to reduce transfer time:
```bash
orbit -s smb://server/share/large.dat -d ./large.dat --compress zstd:3
```

---

## 🐛 Troubleshooting

### "Protocol not supported"
```
Error: Unsupported protocol: ftp
```
**Solution:** Check the list of supported protocols above. FTP is not yet supported.

### SMB connection failures (v0.4.0/v0.4.1)
```
Error: SMB connection failed
```
**Expected:** SMB implementation is blocked by upstream dependency conflict in v0.4.0/v0.4.1. See [docs/SMB_NATIVE_STATUS.md](docs/SMB_NATIVE_STATUS.md) for details. Use local filesystem or S3 protocol instead.

### URI parsing errors
```
Error: Invalid URI format: server/share/file
```
**Solution:** Include the protocol: `smb://server/share/file`

---

## 🚀 Roadmap

### v0.4.1 (Q1 2026)
- Complete SMB/CIFS implementation
- S3 protocol support
- Azure Blob support
- Credential management system

### v0.5.0 (Q2 2026)
- Google Cloud Storage
- SFTP protocol
- FTP/FTPS protocols
- Protocol multiplexing (parallel connections)

### v1.0.0 (Q3 2026)
- Plugin system for custom protocols
- Protocol auto-detection
- Performance optimizations
- Production hardening

---

## 💡 Best Practices

### 1. Use Direct Paths for Local Files
```bash
# ✅ Preferred
orbit -s /tmp/file.txt -d /backup/file.txt

# ⚠️ Unnecessary
orbit -s file:///tmp/file.txt -d file:///backup/file.txt
```

### 2. Combine Protocols with Compression
```bash
# Reduce network transfer time
orbit -s smb://server/share/large.iso -d ./large.iso --compress zstd:9
```

### 3. Use Resume for Network Protocols
```bash
# Enable resume for unreliable connections
orbit -s smb://server/share/bigfile.dat -d ./bigfile.dat --resume --retry-attempts 10
```

### 4. Test with Dry Run First
```bash
# Preview what will be copied
orbit -s smb://server/share/dir -d /backup -R --dry-run
```

---

## 📚 Related Documentation

- [Quick Start Guide](quickstart_guide.md) - Get started with Orbit
- [Configuration Guide](orbit.toml) - Configure defaults
- [Migration Guide](migration_guide.md) - Upgrade from previous versions
- [API Documentation](https://docs.rs/orbit) - Library API reference

---

## ❓ FAQ

**Q: Can I mix protocols in one command?**  
A: Yes! Source and destination can use different protocols:
```bash
orbit -s smb://server/share/file.txt -d /local/file.txt
```

**Q: Are credentials encrypted in transit?**  
A: Depends on the protocol. SMB uses NTLM/Kerberos encryption. Always use secure protocols.

**Q: What happens if I lose connection during SMB transfer?**  
A: Use `--resume` flag. Orbit will checkpoint progress and resume from where it left off.

**Q: Can I use wildcards with URIs?**  
A: Not yet. Use `-R` for recursive copying instead.

**Q: How do I list files on an SMB share?**  
A: Not yet supported. Coming in v0.4.1 with `orbit ls smb://server/share/`.

---

**Need help?** Open an issue on [GitHub](https://github.com/saworbit/orbit/issues)