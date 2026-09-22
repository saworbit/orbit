# Orbit

Orbit is being rebuilt as a small, trustworthy local data copier.

The first product contract is: provide an exact source and destination, preview a deterministic plan, copy through safe staging, resume interruption, and verify the result. The reboot currently has no supported release.

See [the reboot design](docs/superpowers/specs/2026-09-22-orbit-reboot-design.md) for the approved scope. Historical code and documentation are preserved at `archive/pre-reboot-2026-09-22`; see [LEGACY.md](LEGACY.md).

## Current milestone

The reboot currently provides read-only local planning:

```text
orbit plan SOURCE DEST [--replace] [--verify hash|size] [--ignore-unsupported] [--json|--quiet]
```

`DEST` is the exact target path. Planning scans and hashes as required but never writes source, destination, journal, or partial data. The `copy` command will appear only when safe staging and verification are implemented.

If Orbit cannot determine the process working directory, startup fails before planning and emits a human-readable diagnostic on stderr, including when `--json` was requested.
