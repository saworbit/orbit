# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 0.4.x   | :white_check_mark: |
| 0.3.x   | :white_check_mark: |
| < 0.3.0 | :x:                |

**Note:** We strongly recommend always using the latest version of Orbit.

---

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Orbit, please follow these steps:

### 🔒 Private Disclosure

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security issues privately:

1. **Email:** Send details to **shaneawall@gmail.com**
2. **Subject Line:** Include "[SECURITY]" in the subject
3. **Include:**
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
   - Your contact information

### ⏱️ Response Timeline

- **Initial Response:** Within 48 hours
- **Status Update:** Within 7 days
- **Fix Timeline:** Varies by severity (see below)

### 🎯 Severity Levels

| Severity | Description | Response Time |
|----------|-------------|---------------|
| **Critical** | Remote code execution, data loss | 24-48 hours |
| **High** | Privilege escalation, authentication bypass | 7 days |
| **Medium** | Information disclosure, DoS | 14 days |
| **Low** | Minor issues with limited impact | 30 days |

---

## Security Features in Orbit

### ✅ Built-in Security

- **SHA-256 Checksums** - Verify file integrity on all transfers
- **Memory Safety** - Written in Rust with no unsafe code in core operations
- **Audit Logging** - All operations logged with timestamps and checksums
- **No Network Code in Core** - File operations isolated from network concerns

### ⚠️ Security Considerations

**Current Limitations:**

1. **No Encryption in Transit**
   - Data is not encrypted during transfer
   - **Mitigation:** Use VPN, SSH tunnels, or wait for v0.5.0 encryption support

2. **Credentials in URIs**
   - URIs with passwords appear in logs and command history
   - **Mitigation:** Use environment variables (coming in v0.4.1)

3. **Audit Logs May Contain Sensitive Paths**
   - File paths are logged in audit files
   - **Mitigation:** Restrict audit log access, sanitize before sharing

4. **SMB Protocol (v0.4.0)**
   - Experimental implementation, not security-hardened
   - **Mitigation:** Do not use in production until v0.4.1

---

## Best Practices for Secure Usage

### 1. Avoid Credentials in Commands

**❌ Bad:**
```bash
orbit -s smb://admin:password123@server/share/file.txt -d ./file.txt
```

**✅ Good (coming in v0.4.1):**
```bash
export ORBIT_SMB_USER=admin
export ORBIT_SMB_PASSWORD=password123
orbit -s smb://server/share/file.txt -d ./file.txt
```

### 2. Protect Audit Logs

```bash
# Set restrictive permissions on audit logs
chmod 600 ~/.orbit/audit.log

# Use a secure location
orbit -s source -d dest --audit-log /var/log/orbit/audit.log
```

### 3. Verify Checksums

Always enable checksum verification (default):
```bash
orbit -s source.txt -d dest.txt
# Checksum automatically verified
```

Disable only if you trust the environment:
```bash
orbit -s source.txt -d dest.txt --no-verify
```

### 4. Use Secure Protocols

For network transfers:
```bash
# Use VPN or SSH tunnel
ssh -L 445:fileserver:445 jumphost
orbit -s smb://localhost/share/file.txt -d ./file.txt
```

### 5. Review Configuration Files

```bash
# Check for sensitive data in config
cat ~/.orbit/orbit.toml

# Ensure proper permissions
chmod 600 ~/.orbit/orbit.toml
```

---

## Known Security Issues

### Current Issues

**None at this time.**

### Dependency Security Audit

We regularly run `cargo audit` to monitor security advisories in our dependency tree. Current status as of dependency update (2025-12-04):

#### ✅ Default Build: No Active Vulnerabilities

The default build configuration (`cargo build`) has **zero runtime security vulnerabilities**. All reported issues exist only in optional feature dependencies that are not compiled by default.

#### ⚠️ Advisory Warnings (Optional Features Only)

**RSA Timing Side-Channel (RUSTSEC-2023-0071) - Severity: Medium (5.9)**
- **Status:** Present in `Cargo.lock` but NOT compiled in default builds
- **Affected:** Only when building with `--features smb-native` or `--features full`
- **Dependency Chain:** `rsa 0.10.0-rc.9` ← `sspi` ← `smb` (SMB protocol support)
- **Impact:** Potential key recovery through timing side-channels during RSA operations
- **Mitigation:**
  - Default build does not include SMB support
  - No upstream fix available yet (tracked by RustSec)
  - Attack requires active MITM position during SMB authentication
- **Actual Risk:** Low (requires specific feature enablement + active exploitation)

**Unmaintained Dependency: `paste` (RUSTSEC-2024-0436)**
- **Status:** Compile-time macro crate only
- **Dependency Chain:** `paste` ← `rmp` ← `rmp-serde` ← `polars` (analytics feature)
- **Impact:** No runtime security risk (macros only used during compilation)
- **Mitigation:** Monitoring for replacement or upstream maintenance resumption
- **Actual Risk:** Minimal (no runtime code execution)

#### 🎯 Security Posture by Feature Set

| Build Configuration | Runtime Vulnerabilities | Notes |
|---------------------|------------------------|-------|
| Default (`cargo build`) | **None** | Recommended for production |
| `--features api` | **None** | Web API uses SQLite only (MySQL disabled) |
| `--features smb-native` | ⚠️ RSA timing (medium) | SMB connections only, opt-in |
| `--features full` | ⚠️ RSA timing (medium) | Full test suite, not for production |

#### 📋 Verification

To verify the default build has no active vulnerabilities:

```bash
# Check RSA is not in dependency tree
cargo tree -p rsa
# Expected: "nothing to print"

# Check SQLite-only (no MySQL)
cargo tree -p sqlx-mysql
# Expected: "package ID specification did not match any packages"

# Run full audit scan
cargo audit
# Note: Shows Cargo.lock entries, not active dependencies
```

#### 🔄 Maintenance

- **Audit Frequency:** Weekly automated checks via Dependabot
- **Update Policy:** Security updates applied within 7 days
- **Feature Defaults:** Minimal attack surface (zero-copy only)
- **Upstream Tracking:** Monitoring RustSec advisories for fixes

### Resolved Issues

| Issue | Version Affected | Fixed In | Severity |
|-------|------------------|----------|----------|
| *(none yet)* | - | - | - |

---

## Security Updates

Security patches are released as:
- **Patch versions** (0.4.1, 0.4.2) for minor issues
- **Minor versions** (0.5.0) for more significant changes
- **Out-of-band releases** for critical vulnerabilities

### Stay Informed

- **Watch** this repository for security advisories
- **Subscribe** to releases on GitHub
- **Follow** project announcements

---

## Disclosure Policy

When a security issue is reported:

1. **Acknowledgment:** We confirm receipt within 48 hours
2. **Investigation:** We assess severity and develop a fix
3. **Fix Development:** We create and test a patch
4. **Coordinated Disclosure:**
   - We notify the reporter when fix is ready
   - We publish security advisory
   - We release patched version
   - Reporter receives credit (unless requested otherwise)

### Public Disclosure Timeline

- **Critical/High:** 30 days after fix release
- **Medium:** 60 days after fix release
- **Low:** 90 days after fix release

**Exception:** If a vulnerability is already public or actively exploited, we accelerate disclosure.

---

## Security Hall of Fame

We appreciate security researchers who help make Orbit safer:

*(No reports yet - be the first!)*

---

## Compliance & Certifications

**Current Status:**
- No formal certifications yet
- Suitable for internal use and non-regulated data
- **Not yet certified for:**
  - HIPAA (healthcare data)
  - PCI-DSS (payment card data)
  - FedRAMP (US government)

**Future Plans (v1.0.0+):**
- SOC 2 Type II preparation
- Security audit by third-party firm
- Penetration testing

---

## Security Roadmap

### v0.4.1 (Q1 2026)
- [ ] Environment variable support for credentials
- [ ] Credential file encryption
- [ ] SMB security hardening

### v0.5.0 (Q2 2026)
- [ ] End-to-end encryption (AES-256)
- [ ] TLS for network protocols
- [ ] Cryptographic signing of binaries

### v1.0.0 (Q3 2026)
- [ ] Security audit
- [ ] Penetration testing
- [ ] Security documentation suite

---

## Questions?

If you have security-related questions that are **not** vulnerabilities:

- **General security questions:** Open a GitHub Discussion
- **Security features:** Open a Feature Request issue
- **Best practices:** Check documentation first, then ask in Discussions

For **actual vulnerabilities**, always email: shaneawall@gmail.com

---

## Legal

By reporting security vulnerabilities to this project, you agree:

1. To provide reasonable time for us to fix the issue before public disclosure
2. Not to exploit the vulnerability beyond what is necessary to demonstrate it
3. To act in good faith and not cause harm

We commit to:

1. Respond to your report promptly
2. Keep you informed of our progress
3. Credit you appropriately (unless you prefer anonymity)
4. Not take legal action against good-faith security research

---

**Thank you for helping keep Orbit secure!** 🔒