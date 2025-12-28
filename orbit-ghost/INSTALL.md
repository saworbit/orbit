# Orbit GhostFS Installation Guide

Complete installation instructions for all supported platforms.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Linux Installation](#linux-installation)
- [macOS Installation](#macos-installation)
- [Windows Status](#windows-status)
- [Building from Source](#building-from-source)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### All Platforms

- **Rust Toolchain**: Version 1.70 or later
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustc --version  # Should show 1.70+
  ```

- **Git**: For cloning the repository
  ```bash
  git --version
  ```

## Linux Installation

### Ubuntu / Debian

#### 1. Install FUSE3 and Development Libraries

```bash
sudo apt-get update
sudo apt-get install -y fuse3 libfuse3-dev pkg-config build-essential
```

#### 2. Verify FUSE Installation

```bash
# Check FUSE device
ls -l /dev/fuse
# Should show: crw-rw-rw- 1 root root 10, 229 <date> /dev/fuse

# Check FUSE version
fusermount3 --version
# Should show: fusermount3 version: 3.x.x
```

#### 3. Add User to FUSE Group (if needed)

```bash
sudo usermod -a -G fuse $USER
# Log out and log back in for changes to take effect
```

#### 4. Build Orbit GhostFS

```bash
cd orbit-ghost
cargo build --release
```

#### 5. Test Installation

```bash
./target/release/orbit-ghost --help
# Or run the demo:
chmod +x demo_quantum.sh
./demo_quantum.sh
```

### Fedora / RHEL / CentOS

#### 1. Install FUSE3

```bash
sudo dnf install -y fuse3 fuse3-devel pkgconfig gcc
```

#### 2. Enable User Mounts

```bash
# Edit /etc/fuse.conf and uncomment:
sudo sed -i 's/#user_allow_other/user_allow_other/' /etc/fuse.conf
```

#### 3. Build and Test

```bash
cd orbit-ghost
cargo build --release
./demo_quantum.sh
```

### Arch Linux

#### 1. Install FUSE3

```bash
sudo pacman -S fuse3 pkgconf base-devel
```

#### 2. Build and Test

```bash
cd orbit-ghost
cargo build --release
./demo_quantum.sh
```

### Alpine Linux

#### 1. Install Dependencies

```bash
sudo apk add fuse3 fuse3-dev pkgconfig build-base rust cargo
```

#### 2. Build and Test

```bash
cd orbit-ghost
cargo build --release
./demo_quantum.sh
```

## macOS Installation

### Using Homebrew (Recommended)

#### 1. Install macFUSE

```bash
brew install --cask macfuse
```

**Important:** After installation, you must approve the kernel extension:

1. Open **System Preferences** → **Security & Privacy**
2. Click the **General** tab
3. Click **Allow** next to "System software from developer 'Benjamin Fleischer' was blocked"
4. Restart your Mac

#### 2. Verify macFUSE Installation

```bash
# Check if macFUSE is installed
ls /Library/Filesystems/macfuse.fs
# Should show directory contents

# Check version
/Library/Filesystems/macfuse.fs/Contents/Resources/mount_macfuse --version
```

#### 3. Install Build Tools

```bash
# Xcode Command Line Tools (if not already installed)
xcode-select --install

# pkg-config via Homebrew
brew install pkg-config
```

#### 4. Build Orbit GhostFS

```bash
cd orbit-ghost
cargo build --release
```

#### 5. Test Installation

```bash
./target/release/orbit-ghost
# In another terminal:
ls /tmp/orbit_ghost_mount
```

**Note:** macOS may prompt for additional permissions. Grant access when requested.

### Manual macFUSE Installation

If Homebrew is not available:

1. Download macFUSE from [https://osxfuse.github.io/](https://osxfuse.github.io/)
2. Open the `.dmg` file and run the installer
3. Follow kernel extension approval steps above
4. Continue with build steps

## Windows Status

**Current Status:** Not yet supported natively.

### Why FUSE Doesn't Work on Windows

Windows does not have native FUSE support. The `fuser` crate requires FUSE kernel modules that are Unix-specific.

### Future Windows Support Options

1. **WinFSP (Windows File System Proxy)**
   - Requires separate implementation using `winfsp` bindings
   - See [WINDOWS.md](WINDOWS.md) for details

2. **WSL2 (Windows Subsystem for Linux)**
   - Run Orbit GhostFS inside WSL2
   - Access mounted filesystem from Windows
   - See WSL2 instructions below

3. **NFS Emulation**
   - Use Windows built-in NFS client
   - Requires NFS server implementation

### Running in WSL2 (Workaround)

If you have WSL2 installed:

```bash
# Inside WSL2 Ubuntu:
sudo apt-get update
sudo apt-get install -y fuse3 libfuse3-dev pkg-config build-essential

cd /mnt/c/orbit/orbit-ghost
cargo build --release
./demo_quantum.sh
```

**Limitation:** Mounted filesystem will be in WSL2, not directly accessible from Windows Explorer.

## Building from Source

### Clone the Repository

```bash
git clone https://github.com/saworbit/orbit.git
cd orbit/orbit-ghost
```

### Build Options

#### Debug Build (faster compilation, slower runtime)

```bash
cargo build
./target/debug/orbit-ghost
```

#### Release Build (optimized, recommended)

```bash
cargo build --release
./target/release/orbit-ghost
```

#### With Specific Features

```bash
# Currently no optional features, but structure for future:
cargo build --release --features "advanced-prefetch,ml-predictions"
```

### Install System-Wide (Optional)

```bash
cargo install --path .
# Installs to ~/.cargo/bin/orbit-ghost

# Ensure ~/.cargo/bin is in your PATH
export PATH="$HOME/.cargo/bin:$PATH"
```

## Verification

### Check Binary

```bash
./target/release/orbit-ghost --version
# Should output: orbit-ghost 0.1.0
```

### Run Health Check

```bash
# Create test directories
mkdir -p /tmp/orbit_ghost_mount /tmp/orbit_cache

# Start the ghost filesystem (in background)
./target/release/orbit-ghost &
GHOST_PID=$!

# Wait for mount
sleep 2

# Verify mount
mount | grep orbit_ghost
# Should show: orbit_ghost on /tmp/orbit_ghost_mount type fuse.orbit_ghost ...

# Check files
ls /tmp/orbit_ghost_mount
# Should show: visionary_demo.mp4

# Read test
head -c 100 /tmp/orbit_ghost_mount/visionary_demo.mp4

# Cleanup
kill $GHOST_PID
fusermount -u /tmp/orbit_ghost_mount
```

### Run Demo Script

```bash
chmod +x demo_quantum.sh
./demo_quantum.sh
```

**Expected Output:**
```
---------------------------------------------------
   ORBIT VISIONARY DEMO: QUANTUM ENTANGLEMENT
---------------------------------------------------
[Setup] Compiling Orbit GhostFS...
[Action] Activating Flight Plan...
[Orbit] 🌌 Projecting Holographic Filesystem at /tmp/orbit_ghost_mount
[Check] checking mount point...
✅ GHOST FILE PROJECTED: visionary_demo.mp4 found.
[Magic] Attempting to read tail of ghost file...

✅ READ COMPLETE.
⏱️  Time to access random byte: 523ms
   (Note: Only the requested block was transferred)
[Cleanup] Deactivating Orbit...
```

## Troubleshooting

### Issue: "FUSE device not found"

```bash
# Check if FUSE module is loaded
lsmod | grep fuse

# Load FUSE module if missing
sudo modprobe fuse

# Verify
ls -l /dev/fuse
```

### Issue: "Permission denied" when mounting

```bash
# Add user to fuse group
sudo usermod -a -G fuse $USER

# Or edit /etc/fuse.conf
echo "user_allow_other" | sudo tee -a /etc/fuse.conf

# Log out and back in
```

### Issue: "Transport endpoint is not connected"

Previous mount was not cleaned up properly:

```bash
# Unmount forcefully
fusermount -u /tmp/orbit_ghost_mount
# Or on macOS:
umount /tmp/orbit_ghost_mount

# If that fails, force unmount
sudo umount -f /tmp/orbit_ghost_mount
```

### Issue: "Could not run pkg-config"

pkg-config is missing:

```bash
# Ubuntu/Debian
sudo apt-get install pkg-config

# macOS
brew install pkg-config

# Fedora
sudo dnf install pkgconfig
```

### Issue: Build fails with "fuser requires fuse3 >= 3.0.0"

FUSE3 development libraries not installed:

```bash
# Ubuntu/Debian
sudo apt-get install libfuse3-dev

# Fedora
sudo dnf install fuse3-devel

# macOS
brew reinstall macfuse
```

### Issue: macOS "Operation not permitted"

Kernel extension not approved:

1. Go to **System Preferences** → **Security & Privacy**
2. Click **Allow** for macFUSE
3. Restart your Mac
4. Try again

### Issue: "Bus error" or crashes

Likely platform-specific FUSE version mismatch:

```bash
# Check FUSE version
pkg-config --modversion fuse3

# Should be 3.0.0 or higher
# If lower, upgrade:
sudo apt-get install libfuse3-dev  # Ubuntu
brew upgrade macfuse                # macOS
```

### Issue: Demo script fails on macOS with "command not found: fusermount"

macOS uses `umount` instead:

```bash
# Edit demo_quantum.sh, replace:
# fusermount -u $MOUNT_POINT
# with:
umount $MOUNT_POINT
```

Or create an alias:

```bash
alias fusermount='umount'
```

## Next Steps

After successful installation:

1. Read the [User Guide](GUIDE.md) for usage instructions
2. Check the [FAQ](FAQ.md) for common questions
3. Review [ARCHITECTURE.md](ARCHITECTURE.md) for technical details
4. See [DEVELOPMENT.md](DEVELOPMENT.md) if contributing

## Getting Help

- **Issues:** [GitHub Issues](https://github.com/saworbit/orbit/issues)
- **Discussions:** [GitHub Discussions](https://github.com/saworbit/orbit/discussions)
- **Documentation:** [README.md](README.md)

## Platform Support Matrix

| Platform | Status | FUSE Library | Notes |
|----------|--------|--------------|-------|
| Ubuntu 20.04+ | ✅ Supported | libfuse3 | Recommended platform |
| Debian 11+ | ✅ Supported | libfuse3 | Fully tested |
| Fedora 35+ | ✅ Supported | fuse3 | Works well |
| RHEL 8+ | ✅ Supported | fuse3 | Enterprise ready |
| Arch Linux | ✅ Supported | fuse3 | Rolling release |
| Alpine Linux | ✅ Supported | fuse3 | Docker-friendly |
| macOS 11+ | ⚠️ Experimental | macFUSE | Requires kernel extension |
| macOS 10.15 | ⚠️ Limited | macFUSE | May have issues |
| Windows 10/11 | ❌ Not Supported | N/A | Use WSL2 or await WinFSP |
| FreeBSD | ❓ Untested | fusefs | May work with modifications |

## System Requirements

### Minimum

- CPU: 1 core
- RAM: 512 MB
- Disk: 100 MB for binary + cache space
- OS: Linux kernel 3.15+ or macOS 10.15+

### Recommended

- CPU: 2+ cores (for parallel block fetching)
- RAM: 2 GB (for cache and manifest)
- Disk: 10 GB for cache
- OS: Ubuntu 22.04 LTS or macOS 12+
- Network: 10 Mbps+ for remote backends

## License

Part of the Orbit project. See [LICENSE](../LICENSE) for details.
