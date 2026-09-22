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
