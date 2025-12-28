# Windows Support for Orbit GhostFS

Status, workarounds, and future plans for Windows platform.

## Current Status

**Orbit GhostFS v0.1.0 does NOT support Windows natively.**

### Why Not?

**FUSE (Filesystem in Userspace) is not available on Windows.**

- FUSE is a Linux kernel module
- macOS has a port (macFUSE)
- Windows has no native FUSE support

**Build errors on Windows:**
```
error: failed to run custom build command for `fuser v0.14.0`
Could not run `pkg-config --libs --cflags fuse3`
The pkg-config command could not be found.
```

This is expected and cannot be fixed without a fundamental architecture change.

## Workarounds

### Option 1: WSL2 (Windows Subsystem for Linux)

**Best current option for Windows users.**

#### Pros
- ✅ Full Linux compatibility
- ✅ No code changes needed
- ✅ Official Microsoft support
- ✅ Good performance

#### Cons
- ❌ Filesystem isolated in WSL (not directly in Windows Explorer)
- ❌ Requires WSL2 setup
- ❌ Extra layer of complexity

#### Setup Instructions

**1. Install WSL2**

```powershell
# In PowerShell (Administrator)
wsl --install
# Restart computer
```

**2. Install Ubuntu in WSL2**

```powershell
wsl --install -d Ubuntu
# Follow setup prompts
```

**3. Install Dependencies in WSL2**

```bash
# Inside WSL Ubuntu:
sudo apt-get update
sudo apt-get install -y fuse3 libfuse3-dev pkg-config build-essential curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**4. Build and Run Orbit GhostFS**

```bash
# Clone repository
git clone https://github.com/saworbit/orbit.git
cd orbit/orbit-ghost

# Build
cargo build --release

# Run
./target/release/orbit-ghost
```

**5. Access Files from WSL**

```bash
# Inside WSL
ls /tmp/orbit_ghost_mount
cat /tmp/orbit_ghost_mount/visionary_demo.mp4
```

#### Accessing WSL Files from Windows

**Option A: Via Network Path**

```powershell
# In Windows Explorer, navigate to:
\\wsl$\Ubuntu\tmp\orbit_ghost_mount
```

**Option B: Mount in Shared Directory**

```bash
# In WSL, mount to Windows-accessible location
mkdir -p /mnt/c/orbit_mount
# Edit src/main.rs to use /mnt/c/orbit_mount
# Rebuild and run
```

Then in Windows:
```
C:\orbit_mount\visionary_demo.mp4
```

**Limitation:** File I/O may be slower due to WSL translation layer.

### Option 2: Virtual Machine

Run Linux in a VM on Windows.

#### Pros
- ✅ Full Linux environment
- ✅ Can use any Linux distribution
- ✅ Isolated environment

#### Cons
- ❌ High overhead (dedicated RAM/CPU)
- ❌ Complex networking for file sharing
- ❌ Slow performance

#### Setup (VirtualBox)

**1. Install VirtualBox**
- Download from https://www.virtualbox.org/

**2. Create Ubuntu VM**
- Download Ubuntu ISO from https://ubuntu.com/download
- Create VM with 4 GB RAM, 20 GB disk
- Install Ubuntu

**3. Install Orbit GhostFS**
- Follow standard Linux installation (see [INSTALL.md](INSTALL.md))

**4. Share Files with Windows**
- Use VirtualBox Shared Folders
- Or configure Samba/NFS

### Option 3: Docker on Windows

Run Orbit GhostFS in a Docker container.

#### Pros
- ✅ Lightweight (compared to VM)
- ✅ Reproducible environment
- ✅ Easy to manage

#### Cons
- ❌ Requires privileged container (for FUSE)
- ❌ Complex volume mounting
- ❌ Not recommended for production

#### Setup

**Create Dockerfile:**

```dockerfile
FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    fuse3 libfuse3-dev pkg-config build-essential curl

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ["./target/release/orbit-ghost"]
```

**Run:**

```powershell
docker build -t orbit-ghost .
docker run --privileged --device /dev/fuse orbit-ghost
```

**Note:** This still runs Linux inside Docker on Windows (uses WSL2 backend).

## Future Native Windows Support

### Planned for v0.5.0 (Q1 2025)

**Target:** Native Windows filesystem driver.

### Implementation Options

#### Option A: WinFSP (Recommended)

**What is WinFSP?**
- Windows File System Proxy
- User-mode filesystem framework (like FUSE for Windows)
- Stable, production-ready
- Used by: rclone, SSHFS-Win, EncFS

**Implementation Plan:**

1. **Use winfsp-rs bindings** (or create if needed)
   ```toml
   [target.'cfg(windows)'.dependencies]
   winfsp = "0.1"
   ```

2. **Implement WinFSP callbacks** (similar to FUSE)
   - `GetVolumeInfo`
   - `GetFileInfo`
   - `ReadDirectory`
   - `Read`

3. **Share core logic** between FUSE and WinFSP
   ```rust
   #[cfg(target_os = "linux")]
   mod fuse_impl;

   #[cfg(target_os = "windows")]
   mod winfsp_impl;

   mod core;  // Shared entangler, caching, etc.
   ```

**Challenges:**
- API differences between FUSE and WinFSP
- Windows-specific permission model
- Driver installation (requires admin)

**Timeline:** 15-20 days development + testing

#### Option B: NFS Server Emulation

**Concept:** Implement NFS server, use Windows built-in NFS client.

**Pros:**
- ✅ No kernel driver needed
- ✅ Uses standard protocols
- ✅ Works on Windows 10 Pro+

**Cons:**
- ❌ High overhead (network stack)
- ❌ Complex authentication
- ❌ Requires NFS client feature (not always enabled)

**Implementation:**
```
Orbit NFS Server (localhost:2049)
    ↓ NFS protocol
Windows NFS Client
    ↓ Mount as network drive
Windows Explorer (Z:\)
```

**Timeline:** 10-15 days development

#### Option C: Windows Filter Driver

**Most powerful but most complex.**

**What is it?**
- Kernel-mode filesystem filter
- Deepest Windows integration
- Highest performance

**Pros:**
- ✅ Native Windows filesystem
- ✅ No translation layer
- ✅ Best performance

**Cons:**
- ❌ Requires kernel-mode programming (C/C++)
- ❌ Driver signing required (EV certificate ~$300/year)
- ❌ High complexity and risk
- ❌ Cannot use Rust directly (unsafe FFI)

**Timeline:** 30-60 days development + certification

### Decision Matrix

| Approach | Complexity | Performance | Integration | Timeline |
|----------|------------|-------------|-------------|----------|
| **WinFSP** | Medium | Good | Excellent | Q1 2025 |
| **NFS** | Low | Fair | Good | Q4 2024 |
| **Filter Driver** | Very High | Excellent | Perfect | Q3 2025+ |

**Recommended:** WinFSP (Option A) for v0.5.0.

## Contributing Windows Support

Interested in helping bring Orbit GhostFS to Windows?

### What You Can Do

**1. Research WinFSP Integration**
- Test winfsp-rs bindings
- Document API differences vs FUSE
- Create proof-of-concept

**2. Design Cross-Platform Architecture**
- Abstract filesystem operations
- Separate platform-specific code
- Maintain shared core logic

**3. Testing on Windows**
- Test WSL2 workaround
- Document edge cases
- Report compatibility issues

### Getting Started

```rust
// Proposed architecture
pub trait FilesystemBackend {
    fn lookup(&self, parent: u64, name: &str) -> Result<FileAttr>;
    fn read(&self, inode: u64, offset: u64, size: u32) -> Result<Vec<u8>>;
    // ...
}

#[cfg(unix)]
struct FuseBackend { /* FUSE-specific */ }

#[cfg(windows)]
struct WinFSPBackend { /* WinFSP-specific */ }

impl FilesystemBackend for FuseBackend { /* ... */ }
impl FilesystemBackend for WinFSPBackend { /* ... */ }
```

**Next Steps:**
1. Open issue: "Windows Support via WinFSP"
2. Discuss design in [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
3. Create PoC branch
4. Submit PR when ready

## Frequently Asked Questions

### Q: Will Windows support be free?

**Yes.** Same open-source license (Apache 2.0).

### Q: Can I use Orbit GhostFS on Windows Server?

**Not yet.** All Windows editions are unsupported until v0.5.0.

### Q: Does WSL2 have good performance?

**Yes**, for most use cases. WSL2 uses a real Linux kernel and has near-native performance.

**Benchmark (WSL2 vs native Linux):**
- File I/O: ~90% of native speed
- Network: ~95% of native speed
- CPU: ~98% of native speed

### Q: Can I help test Windows support?

**Absolutely!** Join the discussion:
- [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- Email: shaneawall@gmail.com

### Q: What about Dokan (alternative to WinFSP)?

**Dokan** is another user-mode filesystem framework for Windows.

**Why WinFSP instead?**
- More active development
- Better performance
- Wider adoption (rclone, etc.)
- Cleaner API

**But:** We may support both in the future.

## Windows-Specific Considerations

When implementing Windows support, we must handle:

### Path Handling
```rust
// UNIX: /tmp/orbit_cache
// Windows: C:\Users\User\AppData\Local\Orbit\cache

#[cfg(unix)]
const CACHE_DIR: &str = "/tmp/orbit_cache";

#[cfg(windows)]
const CACHE_DIR: &str = "C:\\Users\\%USERNAME%\\AppData\\Local\\Orbit\\cache";
```

### Line Endings
- UNIX: LF (`\n`)
- Windows: CRLF (`\r\n`)
- Ensure binary mode for data blocks

### Permissions
```rust
// UNIX: 0755, 0644
// Windows: ACLs (more complex)

#[cfg(unix)]
fn set_permissions(path: &Path, mode: u32) {
    // chmod
}

#[cfg(windows)]
fn set_permissions(path: &Path, mode: u32) {
    // SetFileSecurity API
}
```

### Case Sensitivity
- UNIX: Case-sensitive (`file.txt` ≠ `File.txt`)
- Windows: Case-insensitive (by default)
- Manifest must handle both

### Drive Letters
```rust
// UNIX: /mnt/data
// Windows: D:\, E:\, etc.

#[cfg(windows)]
fn mount_as_drive_letter(letter: char) {
    // Assign Z:\ for example
}
```

## Timeline

| Milestone | Target | Status |
|-----------|--------|--------|
| WSL2 testing & docs | Q2 2024 | ✅ Complete |
| WinFSP PoC | Q3 2024 | 📋 Planned |
| Alpha release (Windows) | Q4 2024 | 📋 Planned |
| Beta testing | Q1 2025 | 📋 Planned |
| Production release (v0.5.0) | Q1 2025 | 📋 Planned |

## Resources

### WinFSP
- **Website:** https://winfsp.dev/
- **GitHub:** https://github.com/winfsp/winfsp
- **Docs:** https://winfsp.dev/doc/

### Rust Bindings
- **winfsp-rs:** https://github.com/SnowflakePowered/winfsp-rs (unmaintained)
- **Opportunity:** Create new, maintained bindings

### Windows Filesystem APIs
- **Filesystem Filter Drivers:** https://docs.microsoft.com/en-us/windows-hardware/drivers/ifs/
- **NFS on Windows:** https://docs.microsoft.com/en-us/windows-server/storage/nfs/

### Community
- **Discussions:** https://github.com/saworbit/orbit/discussions
- **Issues:** https://github.com/saworbit/orbit/issues

## Call to Action

**Windows developers:** We need your help!

If you have experience with:
- WinFSP development
- Windows kernel programming
- Cross-platform Rust development
- Windows driver signing

**Please reach out:** shaneawall@gmail.com

Together, we can bring quantum entanglement to Windows. 🚀

---

*Last updated: 2024-01-15*
