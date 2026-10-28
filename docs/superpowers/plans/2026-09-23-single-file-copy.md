# Single-file copy implementation plan

> Execute in this session with strict test-first changes. Scope is only `orbit copy` for one regular local file.

**Goal:** Add a staged, verified, no-clobber single-file copy command while preserving the read-only planner.

**Architecture:** Parse `plan` and `copy` into distinct request variants. Reuse endpoint resolution, scanning, and planning for preflight. A new executor writes a sibling staging file, syncs and verifies it, then atomically publishes it with create-if-absent hard-link semantics and removes the staging name. Reporting emits a concise copy result through the existing human/JSON/quiet conventions.

**Tech stack:** Rust 2024, standard library filesystem APIs, BLAKE3, clap, serde, existing integration-test stack.

---

## Task 1: Define the command contract

**Files:** `src/request.rs`, `src/cli.rs`, `src/main.rs`, `tests/cli_contract.rs`

1. Add failing parser tests for `orbit copy SOURCE DESTINATION`, hash verification by default, `--verify size`, JSON/quiet exclusion, and rejection of plan-only flags such as `--replace`.
2. Run `cargo test --test cli_contract copy` and confirm the tests fail because `copy` is unknown.
3. Add `CopyRequest` and a top-level request enum; extend clap parsing and dispatch without changing `plan` behavior.
4. Run the focused tests and commit.

## Task 2: Build the staged executor

**Files:** `src/copy.rs`, `src/lib.rs`, `src/error.rs`

1. Add failing unit tests for copying to a missing destination, hash/size verification, identical destination no-op, differing destination refusal, directory/symlink refusal, missing parent, destination race, and staging cleanup.
2. Run `cargo test copy::tests` and confirm failures before implementation.
3. Implement preflight through the existing resolver/scanner/planner, require exactly one regular-file entry, create a unique sibling stage with `create_new`, stream and `sync_all`, verify against the planned digest or size, publish with `hard_link(stage, destination)`, and remove the staging link.
4. Fail closed on unsupported publication or races. Add stable domain errors for non-file sources, existing different destinations, verification mismatch, missing destination parent, and incomplete staging cleanup.
5. Run the focused unit tests and commit.

## Task 3: Add copy reporting and application flow

**Files:** `src/app.rs`, `src/report.rs`, `src/main.rs`, `tests/cli_contract.rs`

1. Add failing CLI tests for human success, JSON result, quiet success, visible quiet errors, and identical no-op.
2. Run those tests and confirm failure.
3. Add `run_copy` and `CopyResult` reporting. Human output identifies copied/no-op status; JSON emits one `copy_result` event with schema version 1; quiet emits nothing on success. Preserve exit code 3 for domain refusal and 1 for report/internal failures.
4. Run focused CLI tests and commit.

## Task 4: Prove safety boundaries

**Files:** `tests/copy_integration.rs`, `README.md`

1. Add integration tests proving a different existing destination is byte-for-byte unchanged, a racing destination is never overwritten, no partial destination appears after injected/observable failure paths, directory and link sources are rejected, and no `.orbit-*.partial` files remain after ordinary failures.
2. Run `cargo test --test copy_integration` and confirm each new assertion exercises the implementation.
3. Document the exact copy boundary and deferred replacement behavior in the README.
4. Run formatting, linting, the full suite, and an optimized build:
   `cargo fmt --check`
   `cargo clippy --all-targets --all-features -- -D warnings`
   `cargo test --all-targets --all-features`
   `cargo build --release`
5. Review the final diff for scope and commit.
