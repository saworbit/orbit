# Orbit Reboot Design

**Status:** Approved design

**Date:** 2026-09-22

**Initial platform:** Windows on NTFS

**Initial implementation language:** Rust

## 1. Purpose

Orbit will restart as a small, trustworthy data copier rather than continue as a broad transfer platform. Its initial promise is:

> Give Orbit a source and destination. It safely copies the data, explains what it will do, survives interruption, and verifies the result.

The existing repository is treated as an experiment whose history remains valuable. The reboot keeps that history but removes the old implementation from the active product surface. There are no known users, stored manifests, or automation that require compatibility with the old CLI.

## 2. Reboot Strategy

Orbit will use a same-repository hard reset. This keeps provenance, issue history, and useful prior research without forcing the new design to coexist with the old architecture.

Before destructive repository changes:

1. Confirm the working tree is clean and record the exact pre-reboot commit.
2. Create the annotated tag `archive/pre-reboot-2026-09-22`.
3. Publish that tag as a GitHub release when authenticated GitHub access is working.
4. Do not create a separate legacy branch initially; the immutable tag is the authoritative archive and avoids another branch to maintain.
5. Perform the reboot on `codex/orbit-reboot` and merge it normally. Do not rewrite or force-push `main`.

The reset commit removes the old active source, tests, workflows, packaging, and product documentation. It retains repository-level legal and community files, including the license, `.gitignore`, and code of conduct. It adds a rewritten README, a short `LEGACY.md` pointing to the archive tag, and the approved reboot documentation. Old issues are labelled as legacy rather than silently closed or repurposed.

The first new version is `0.1.0-alpha`. It has no compatibility obligation to earlier experimental commands, configuration, manifests, or output formats.

## 3. Product Boundary

### 3.1 Included in v0.1

- Copy one regular file or a directory tree between local filesystem paths.
- Treat OS-mounted network shares as filesystem paths; Orbit does not implement their protocols.
- Scan source and relevant destination state before execution.
- Produce a deterministic, inspectable plan.
- Stage writes and finalize them safely.
- Resume interrupted operations from Orbit-owned state.
- Verify results by content hash by default, with explicit size-only verification as a performance trade-off.
- Report progress and results for both people and automation.
- Make Windows and NTFS behavior the release gate while preserving portable core semantics.

### 3.2 Excluded from v0.1

- Native SMB, SFTP, object-storage, or cloud-provider backends.
- Move, sync, mirror, destination deletion, or bidirectional reconciliation.
- Deduplication, compression, encryption, delta transfer, or semantic naming.
- Daemons, distributed workers, hosted services, schedulers, and queues.
- Configuration files, plugin systems, public libraries, FFI bindings, and remote APIs.
- User-facing concurrency, buffer, retry, or tuning controls.
- Heuristics that cannot explain their decisions.

These exclusions are product boundaries, not permanent prohibitions. A feature earns inclusion only after the local copy contract is reliable and a concrete use case demonstrates the need.

## 4. Language and Stack Strategy

v0.1 is one Rust crate, one binary, one toolchain, and one deployment unit. Rust is the default for the transfer engine because it provides strong memory safety, predictable native deployment, and suitable filesystem and concurrency control.

The product is not required to remain Rust-only. Future components may use a language with a concrete ecosystem or delivery advantage:

- TypeScript may suit a desktop or web interface.
- Rust or Go may suit a cloud control plane.
- Python may suit experiments and operational analysis.
- C or C++ may be used behind a narrow wrapper when an essential native dependency or measured limitation requires it.

No second language, IPC protocol, FFI layer, or service boundary will be introduced speculatively. Performance intuition alone is not justification for C++.

The Rust engine will nevertheless keep CLI concerns at its boundary: it accepts typed operation requests and emits typed events and results. This permits a later interface to reuse the engine without turning the current build into a premature platform.

## 5. Architecture

The initial implementation is a single crate with focused internal modules. It must not define a generic storage-backend trait while only one backend exists.

```text
CLI
 └─ Scan
     └─ Plan
         └─ Execute
             ├─ Journal
             ├─ Verify
             └─ Report
```

### 5.1 Components

- **CLI:** Parses commands into typed requests, selects human or JSON presentation, and maps final results to stable exit codes. It contains no transfer policy.
- **Scan:** Captures an immutable view of source entries and the destination entries needed for comparison. It detects supported object types and discoverable unsupported metadata.
- **Plan:** Converts scan results and explicit options into an ordered list of actions, conflicts, and warnings. Given the same snapshot and options, it produces the same plan.
- **Execute:** Performs only actions present in the accepted plan. It schedules files, stages writes, coordinates interruption, and finalizes completed entries.
- **Journal:** Persists operation identity and durable progress so execution can resume without guessing what succeeded.
- **Verify:** Applies the requested verification policy and returns explicit evidence for finalization and reporting.
- **Report:** Converts typed events and the final result into concise terminal output or a versioned JSON envelope.

Modules communicate using domain types rather than formatted strings. Formatting and process exit occur only at the CLI boundary.

### 5.2 Plan Model

Plans use normalized relative paths and stable ordering. At minimum, an entry has one of these dispositions:

- `copy`: destination entry does not exist.
- `skip_identical`: source and destination contents have been proven identical.
- `replace`: an existing regular file will be replaced and `--replace` was supplied.
- `conflict`: execution is prohibited without a policy change.
- `unsupported`: the source cannot be copied within the fidelity contract.

Equal size alone never produces `skip_identical`; same-size candidates are hashed before that decision. Directory operations merge planned source entries into the exact destination root and never delete unrelated destination entries. File-versus-directory and other object-type conflicts fail even with `--replace` in v0.1.

## 6. Operation Flow

1. Parse and normalize the exact source and destination paths.
2. Reject invalid relationships, including a destination contained within its source tree.
3. Discover compatible Orbit-owned journal state for this source, destination, command, and relevant options.
4. Scan the source and required destination state.
5. Produce and validate a deterministic plan.
6. For `plan`, report the plan and stop without writing.
7. For `copy`, create or update the durable journal and execute planned entries.
8. Write each file into an Orbit-owned temporary sibling on the destination filesystem.
9. Flush the staged file, verify it according to policy, and finalize it using a platform operation that preserves the last known-good destination.
10. Persist completion before reporting an entry as complete.
11. Emit a final report and exit status. The operation succeeds only when every required action, finalization, and verification succeeds.

For a new destination file, finalization is an atomic rename when the filesystem supports it. For replacement, Orbit uses a platform-supported atomic replacement operation. If the destination filesystem cannot provide the required safety semantics, preflight fails rather than silently degrading to delete-then-rename.

## 7. Command and Interaction Model

The initial interface is:

```text
orbit copy SOURCE DEST
orbit plan SOURCE DEST
```

The complete initial option set is:

```text
--replace
--verify hash|size
--ignore-unsupported
--json
--quiet
```

Rules:

- `--verify hash` is the default. Hash verification reads the finalized destination and compares it with the source digest computed during copying. The initial digest algorithm is BLAKE3.
- `--verify size` explicitly accepts weaker verification and still requires successful write, flush, finalization, and source-stability checks.
- `plan` performs all available scanning, hashing, and validation but makes no filesystem changes. It replaces a separate `--dry-run` mode.
- `copy` creates the same plan and executes it without an interactive confirmation.
- Progress is enabled automatically on a terminal and omitted when output is redirected. `--quiet` suppresses non-error human output.
- `--json` emits newline-delimited, versioned JSON events followed by one final result event. Human progress never contaminates this stream.
- The program never prompts, ensuring unattended commands cannot hang.

`DEST` is always the exact target path. Orbit does not reinterpret an existing directory to mean “place the source beneath this directory using its basename.” Examples:

```text
orbit copy report.pdf D:\archive\report.pdf
orbit copy photos D:\archive\photos
```

## 8. Fidelity Contract

### 8.1 Preserved in v0.1

- Exact regular-file contents.
- Directory structure, including empty directories.
- Last-modified timestamps.
- Basic read-only state.
- Symbolic links as links, where the platform permits creating them.

Symbolic links are never silently followed. The planner records the link target and execution recreates that link rather than copying the target contents.

### 8.2 Not Preserved in v0.1

- ACLs, ownership, and auditing metadata.
- Alternate data streams and extended attributes.
- Hard-link relationships.
- Sparse-file allocation.
- Arbitrary Windows reparse points and junctions.
- Device files and other special filesystem objects.

Orbit must not silently omit unsupported objects or semantically significant metadata. When the platform exposes their presence, the planner reports the affected paths and fails before execution by default. `--ignore-unsupported` permits a deliberately lossy copy: unsupported objects are skipped, supported file content is copied without unsupported metadata, and every omission or degradation appears in the final report.

Routine platform-managed state is treated separately so the default remains usable. On Windows, inherited access control, ownership assigned by the destination, and the ordinary archive bit are summarized as known fidelity limitations but do not make every file unsupported. Explicit non-inherited access-control or auditing entries, alternate data streams, extended attributes, sparse allocation, and non-symlink reparse data are blocking when Orbit can detect them. This distinction is part of the Windows test contract and must not be expanded implicitly.

If the platform cannot reliably detect a particular unsupported metadata class, the documentation and plan summary state that limitation. Orbit must not claim fidelity it cannot inspect.

## 9. Safety, Failure, and Recovery

### 9.1 Invariants

- Orbit never modifies or deletes the source.
- Orbit never modifies an existing destination file without `--replace`.
- Failed or interrupted work never masquerades as a complete final file.
- Replacement preserves the last known-good destination until verified staged data can be finalized safely.
- Successful completion requires execution, finalization, required verification, and durable journal state.
- Repeating a completed operation is safe and idempotent.

### 9.2 Source Stability

Orbit records source identity and relevant metadata before reading, calculates the content digest while reading, and checks identity and metadata again afterward. A detected change rejects that file. A later run rescans and creates a plan for the new source state.

### 9.3 Journal and Partial Data

Orbit keeps destination-side state so recovery does not depend on the original machine. State lives in a reserved Orbit-owned directory adjacent to the destination root, with an operation identifier derived from normalized endpoint identity and behavior-affecting options. If the reserved path collides with user data, planning fails.

The journal is schema-versioned and updated atomically. It records the source snapshot required for validation, planned actions, staged-file identity, verified progress boundaries, and completed finalizations. Resume revalidates the source and destination before trusting any recorded progress. A mismatch does not delete or reuse questionable state; Orbit refuses the resume and explains the recovery choices.

Orbit does not automatically delete abandoned partial data in v0.1. Cleanup will be a later explicit, narrowly scoped command.

### 9.4 Interruption and Errors

On `Ctrl+C`, Orbit stops scheduling new entries, moves active work to a resumable boundary where possible, persists the journal, and exits nonzero. A directory operation may continue after independent file failures to expose all problems, but any failed item makes the overall result unsuccessful.

Errors have stable machine-readable codes and retain causal context. At minimum, Orbit distinguishes usage errors, plan rejection, conflicts, unsupported fidelity, permission failures, insufficient space, source mutation, I/O failure, unsafe finalization, journal incompatibility, interruption, and verification failure.

Process exit codes are grouped as follows:

- `0`: complete success.
- `1`: unexpected internal failure.
- `2`: command-line usage error.
- `3`: plan rejected, including conflicts or unsupported fidelity.
- `4`: execution incomplete but potentially resumable, including interruption and ordinary I/O failures.
- `5`: integrity or verification failure.

## 10. Testing Strategy

Tests assert observable guarantees rather than private implementation structure.

- **Unit tests:** path validation, scan normalization, deterministic planning, conflict policy, journal transitions, verification decisions, and error classification.
- **Filesystem integration tests:** real temporary trees covering empty and large files, nested and empty directories, Unicode names, long paths, timestamps, read-only files, symbolic links, existing destinations, and type conflicts.
- **Fault-injection tests:** short writes, disk-full conditions, permission changes, source mutation, interruption at journal boundaries, malformed or incompatible journals, failed flushes, failed finalization, and hash mismatches.
- **Property tests:** generated directory trees proving that successful execution creates exactly the planned source projection, preserves unrelated destination entries, does not alter the source, and is idempotent.
- **CLI contract tests:** command parsing, exit codes, human diagnostics, redirected-output behavior, and versioned JSON events.
- **Platform tests:** the entire Windows/NTFS suite is mandatory. Linux and macOS run the portable subset without changing the Windows-first release boundary.
- **Performance tests:** record throughput, memory bounds, and resume overhead to catch regressions. They do not substitute for correctness and do not create marketing claims in v0.1.

### 10.1 Release Gates

A v0.1 candidate cannot ship until end-to-end tests demonstrate all of the following:

1. A new file and a new directory tree copy exactly.
2. An identical destination produces no content writes.
3. A conflict fails without modifying the destination.
4. `--replace` replaces through verified staging while preserving the previous file until finalization.
5. An interrupted large file resumes and produces the correct final hash.
6. Source mutation during copying is detected and rejected.
7. Discoverable unsupported metadata is reported before execution.
8. Any partial failure returns nonzero and reports recoverable state.
9. Repeating a completed operation is safe and produces no unnecessary writes.

## 11. Delivery Sequence

Implementation will proceed in independently reviewable slices:

1. Archive and repository reset.
2. Typed CLI requests, domain errors, and report envelope.
3. Local scan and deterministic plan with no writes.
4. New-file staged copy and hash verification.
5. Safe replacement.
6. Durable journal and interruption handling.
7. Resume with state revalidation.
8. Metadata fidelity and unsupported-feature detection.
9. Windows release suite, packaging, and `0.1.0-alpha` documentation.

The archival reset is intentionally documentation-only and therefore has no product build. Beginning with slice 2, every slice must leave the repository buildable and pass all tests introduced by earlier slices. Cloud, service, UI, and second-language work begins only after the v0.1 release gates pass and a separately approved design establishes a real second component.

## 12. Success Criteria

The reboot succeeds when a user can understand the two-command interface without specialist knowledge, preview the exact work, safely copy local data, interrupt and resume it, and receive trustworthy proof of the outcome. The codebase succeeds when those behaviors are expressed through a small number of explicit domain types and modules, with no speculative backend framework or hidden policy.
