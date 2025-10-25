# SMB Native Implementation Testing Guide

## Overview

The SMB native implementation includes both unit tests and integration tests.

- **Unit tests**: Run without external dependencies
- **Integration tests**: Require a real SMB server

---

## Running Unit Tests

Unit tests validate the implementation logic without requiring an SMB server:
```bash
# Run all unit tests
cargo test --features smb-native

# Run only SMB unit tests
cargo test --features smb-native smb::
```

---

## Setting Up Integration Tests

Integration tests require a real SMB server. You have several options:

### Option 1: Local Samba Server (Linux/macOS)

#### Install Samba

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install samba
```

**macOS:**
```bash
brew install samba
```

#### Configure Samba

Edit `/etc/samba/smb.conf`:
```ini
[global]
   workgroup = WORKGROUP
   security = user
   map to guest = bad user
   
[test]
   path = /tmp/smb_test
   browseable = yes
   writable = yes
   guest ok = yes
   read only = no
   force user = nobody
```

#### Create test directory and start Samba
```bash
sudo mkdir -p /tmp/smb_test
sudo chmod 777 /tmp/smb_test
sudo smbpasswd -a testuser  # Set password when prompted
sudo systemctl restart smbd
```

### Option 2: Docker Samba Container
```bash
docker run -d \
  --name samba-test \
  -p 445:445 \
  -e "USERID=1000" \
  -e "GROUPID=1000" \
  dperson/samba \
  -u "testuser;testpass" \
  -s "test;/share;yes;no;no;testuser"
```

### Option 3: Windows SMB Share

1. Create a shared folder in Windows
2. Right-click → Properties → Sharing → Share
3. Set permissions for a test user
4. Note the share path: `\\YOUR_PC_NAME\ShareName`

---

## Running Integration Tests

### Set Environment Variables
```bash
export SMB_TEST_HOST=localhost          # or your server IP
export SMB_TEST_SHARE=test              # your share name
export SMB_TEST_USER=testuser           # username
export SMB_TEST_PASS=testpass           # password
export SMB_TEST_ENABLED=1               # enable integration tests
```

### Run Integration Tests
```bash
# Run all integration tests (they're marked as #[ignore] by default)
cargo test --features smb-native -- --ignored

# Run specific integration test
cargo test --features smb-native test_real_connection -- --ignored

# Run with verbose output
cargo test --features smb-native -- --ignored --nocapture
```

---

## Test Coverage

### Unit Tests

- ✅ Target validation
- ✅ Path traversal prevention
- ✅ Authentication type creation
- ✅ Error type handling
- ✅ Security level configuration
- ✅ Secret redaction in debug output
- ✅ Metadata structure
- ✅ Capability flags

### Integration Tests

- ✅ Real server connection
- ✅ Directory listing
- ✅ File write and read
- ✅ Range reads (partial file)
- ✅ Metadata queries
- ✅ Directory creation
- ✅ Large file transfers (5MB+)
- ⏳ File deletion (pending full implementation)
- ⏳ File rename (pending full implementation)

---

## Troubleshooting

### Connection Refused
```
Error: Connection(Failed to connect: ...)
```

**Solution:**
- Check if SMB server is running: `systemctl status smbd`
- Verify port 445 is open: `netstat -an | grep 445`
- Check firewall rules

### Authentication Failed
```
Error: Auth
```

**Solution:**
- Verify username/password are correct
- Check Samba user exists: `sudo pdbedit -L`
- Try anonymous authentication first (if enabled)

### Permission Denied
```
Error: Permission(...)
```

**Solution:**
- Check share permissions in `smb.conf`
- Verify file system permissions on the server
- Ensure `writable = yes` in share configuration

### Path Not Found
```
Error: NotFound(...)
```

**Solution:**
- Verify the share exists
- Check the subpath is correct
- Try listing the root directory first

---

## Continuous Integration

### GitHub Actions Example
```yaml
name: SMB Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    services:
      samba:
        image: dperson/samba
        ports:
          - 445:445
        env:
          USERID: 1000
          GROUPID: 1000
        options: >-
          -u "testuser;testpass"
          -s "test;/share;yes;no;no;testuser"
    
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run unit tests
        run: cargo test --features smb-native
      
      - name: Run integration tests
        env:
          SMB_TEST_HOST: localhost
          SMB_TEST_SHARE: test
          SMB_TEST_USER: testuser
          SMB_TEST_PASS: testpass
          SMB_TEST_ENABLED: 1
        run: cargo test --features smb-native -- --ignored
```

---

## Performance Testing

For performance benchmarks with large files:
```bash
# Create a large test file (100MB)
dd if=/dev/urandom of=/tmp/test_100mb.bin bs=1M count=100

# Benchmark with hyperfine
hyperfine --warmup 3 \
  'orbit cp /tmp/test_100mb.bin smb://testuser:testpass@localhost/test/output.bin --features smb-native'
```

---

## Security Testing

### Test Encryption
```bash
export SMB_TEST_REQUIRE_ENCRYPTION=1

cargo test --features smb-native test_real_connection -- --ignored
```

### Test Authentication Methods
```bash
# Test anonymous
export SMB_TEST_ANONYMOUS=1

# Test NTLM
export SMB_TEST_USE_NTLM=1

# Test Kerberos (when implemented)
export SMB_TEST_USE_KERBEROS=1
```

---

## Contributing Tests

When adding new tests:

1. Add unit tests to `src/protocols/smb/tests.rs`
2. Add integration tests to the `integration_tests` module
3. Mark integration tests with `#[ignore]` attribute
4. Update this documentation
5. Ensure tests pass locally before submitting PR

---

## Known Limitations

- Kerberos authentication not yet fully implemented
- File rename operation pending full smb crate support
- Detailed metadata (timestamps) pending implementation
- Multi-channel support not yet enabled
- DFS referrals not yet implemented

---

## Getting Help

- Check the [SMB crate documentation](https://docs.rs/smb)
- Review the [Orbit issues](https://github.com/saworbit/orbit/issues)
- Consult Samba documentation for server configuration