# Orbit

Orbit is being rebuilt as a small, trustworthy local data copier.

The first product contract is: provide an exact source and destination, preview a deterministic plan, copy through safe staging, and verify the result. The reboot currently has no supported release.

See [the reboot design](docs/superpowers/specs/2026-09-22-orbit-reboot-design.md) for the approved scope. Historical code and documentation are preserved at `archive/pre-reboot-2026-09-22`; see [LEGACY.md](LEGACY.md).

## Current milestone

The reboot currently provides read-only local planning and one deliberately narrow write path:

```text
orbit plan SOURCE DEST [--replace] [--verify hash|size] [--ignore-unsupported] [--json|--quiet]
orbit copy SOURCE_FILE DEST_FILE [--verify hash|size] [--json|--quiet]
```

`DEST` is always the exact target path. Planning scans and hashes as required but never writes source, destination, journal, or partial data.

`copy` currently accepts one regular local file. Orbit writes a uniquely named staging file beside the destination, flushes and verifies it, then publishes it without overwriting an existing path. An identical existing file is a successful no-op; a different existing file is refused. The destination parent must already exist. Directories, links, replacement, resume, and remote storage remain intentionally out of scope.

If Orbit cannot determine the process working directory, startup fails before planning and emits a human-readable diagnostic on stderr, including when `--json` was requested.

## End-to-end vibe harness

[`scripts/vibe-harness.ps1`](scripts/vibe-harness.ps1) is a black-box PowerShell harness that drives a built `orbit` binary and asserts the observable behaviours this milestone commits to: deterministic plans, no-op idempotence (hash and mtime preserved), concurrent publish atomicity, staging-file placement and cleanup, mid-copy source-change detection, large-file round-trip hashes, well-formed `--json` NDJSON, exit-code mapping, and more.

It is complementary to `cargo test`: the Rust tests exercise the code from inside the process, the harness exercises the shipped binary from outside with real filesystem and race conditions.

```powershell
cargo build --release
pwsh -NoProfile -File scripts/vibe-harness.ps1
# or point at a specific binary
pwsh -NoProfile -File scripts/vibe-harness.ps1 -OrbitPath path\to\orbit.exe
```

Each check prints `[PASS]` or `[FAIL]` with a one-line detail. The script exits non-zero if any check fails and leaves its temp fixtures in place for inspection.
