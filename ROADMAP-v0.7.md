# Orbit v0.7 Roadmap

Tracking the v0.7 stabilization plan: ship a rock-solid core, gate experimental V3 features behind a clear promotion path, and tighten the observability + testing surface.

Status taxonomy: **Planned** → **In progress** → **Done**. Each item has a one-line acceptance criterion so we know when it's shippable.

---

## Theme 1 — Core transfer (mostly done, see audit)

Already solid: zero-copy, resume/checkpoint, parallel workers, LZ4/Zstd, BLAKE3+SHA-256, Disk Guardian, error classification. No work tracked here unless a regression appears.

### 1.1 Progress throughput / ETA polish — Planned
- **Why:** `indicatif` is wired but throughput and ETA aren't surfaced consistently across copy paths.
- **Accept:** `orbit cp <big-file>` shows current MB/s and ETA that update at least once per second, on all backends.

### 1.2 `print_error` "did you mean" suggestions — Planned
- **Why:** Error formatting exists but lacks pattern-based hints (typo'd flag, common misconfigs).
- **Accept:** At least 10 common failure modes map to a one-line suggestion; covered by a regression test.

### 1.3 `parse_uri` prefix duplication cleanup — Planned
- **Why:** `parse_uri` returns `(BackendConfig, PathBuf)` where for `s3://`/`azblob://`/`gs://`/`smb://` URIs the path segment is stored twice — as `prefix` inside the config *and* as the returned `PathBuf`. Anything that hands the returned path back to `Backend::list` double-prefixes silently. `orbit doctor` already works around this with `probe_path_for` (see `src/commands/doctor.rs`), but every future consumer of `parse_uri` will hit the same trap.
- **Accept:** `parse_uri` returns a config that uniquely owns the prefix; the returned `PathBuf` is either empty or represents a sub-path relative to that prefix (decide which, document it). All existing call sites audited and updated; doctor's `probe_path_for` shim becomes a one-liner or is removed. Regression test in the `backend::config` test module pins the chosen contract.

---

## Theme 2 — Promote or cut experimental V3 features

The V3 crates (`core-cdc`, `core-semantic`) compile but are not exposed via any CLI flag. They are dead weight until promoted or removed.

### 2.1 `--smart` flag + alpha labelling — Planned
- **Why:** Users currently have no way to opt into CDC + semantic prioritization. Without a gate, the crates are unreachable; without a label, users will assume they're stable.
- **Accept:** `orbit cp --smart <src> <dst>` enables CDC + semantic prioritization end-to-end; `--help` text marks the flag `[preview]`; doc note explains the alpha contract.
- **Alternative:** if the feature isn't promotable in this cycle, remove the crates from the default build and gate them behind a `experimental` cargo feature.

### 2.2 Global dedup — Deferred
- Reference counting + chunk store are not implemented. Track as a separate v0.8 effort, not v0.7.

### 2.3 Delta manifests (rusqlite) — Already gated
- Behind `delta-manifest` feature. No work needed unless promoted to default.

### Promotion criteria for any V3 feature

Before a `--smart`-class feature is moved out of `[preview]`, it must satisfy all of:

1. **Correctness:** round-trip test on at least three real-world corpora (mixed text, large binaries, deeply nested trees) shows zero content divergence.
2. **Performance:** no regression vs. the non-smart path on a copy of ≥ 10 GB at the 95th percentile.
3. **Stability:** at least one minor release with the flag exposed and zero open correctness issues against it.
4. **Docs:** user-facing `--help` text and a section in the README explaining when to use it and when not to.

---

## Theme 3 — S3 / Remote UX

### 3.1 `orbit s3 sync` — Planned
- **Why:** CLI has cp/ls/du/rm/mv/mb/rb but no `sync`. Users wanting `aws s3 sync` parity have to script multipart resume themselves.
- **Accept:** `orbit s3 sync <src> <dst>` mirrors a local tree to S3 (and the reverse) with: ETag-based change detection, multipart resume, `--delete` flag for orphan removal, dry-run via `--dry-run`. Built on `object_store` (the `s3-native` path) to keep it dep-light.

### 3.2 `orbit doctor` `--strict` mode — Planned (after 4.1 lands)
- **Why:** v0.7 ships doctor live probes as informational only. A `--strict` flag makes it usable in CI.
- **Accept:** `orbit doctor --strict` exits non-zero if any probe fails; documented in the doctor help text.

---

## Theme 4 — `orbit doctor` (Theme 4.1 is in progress)

### 4.1 Live backend connectivity checks — In progress
- **Why:** Current doctor reports static state (platform, features, hardware) but doesn't tell users whether their actual backend config works. Debugging is blind.
- **Accept:** `orbit doctor --target <uri>` probes any configured backend with a real list call; `ORBIT_BACKEND_TYPE`-style env vars are auto-probed; failures map to a one-line actionable suggestion.

---

## Theme 5 — Observability

### 5.1 Prometheus `/metrics` endpoint — Planned
- **Why:** `metrics_port` config field already exists but no exporter is implemented. This is the lightweight default that pairs with OTel being opt-in.
- **Accept:** When `metrics_port` is set, Orbit serves OpenMetrics on `/metrics` with at minimum: bytes transferred, files completed, errors by category, current throughput. No new heavy deps (use `prometheus` or hand-rolled text format).

### 5.2 OTel — Already gated
- Behind `opentelemetry` feature. No work unless we hear demand for default-on.

---

## Theme 6 — Testing & CI

### 6.1 proptest scaffolding — Planned
- **Why:** No property-based tests for path handling, filter system, or resume logic — all of which have combinatorial input spaces.
- **Accept:** `proptest` in dev-deps; at least three property tests covering: (a) path normalization roundtrips, (b) filter inclusion/exclusion under random pattern sets, (c) resume manifest survives random checkpoint truncations.

### 6.2 localstack / mock S3 integration tests — Planned
- **Why:** Current S3 integration tests rely on real AWS or skip — protocol semantics are untested in CI.
- **Accept:** New integration test layer spins up localstack (or `aws-smithy-mocks`) in CI and runs: upload, multipart upload, multipart resume, list, delete, sync (once 3.1 lands).

### 6.3 Fuzz targets for CDC — Planned
- **Why:** Gear-hash boundary logic has subtle correctness properties (see `cdc-details.md` in memory). No fuzzing exists.
- **Accept:** `cargo-fuzz` target in `fuzz/` that asserts: chunk boundaries are deterministic for a given input; concatenating chunks reproduces the input byte-for-byte; chunks respect min/max size bounds.

### 6.4 CI — Already in good shape
- `cargo deny`, `cargo audit`, `cargo fmt`, `clippy -D warnings`, multi-OS, multi-feature matrix all green. No work unless something breaks.

---

## Out of scope for v0.7

- Global dedup (Theme 2.2) — v0.8.
- Mandatory OTel — opt-in is correct for v0.7.
- Rewriting the manifest schema — stable enough; only touch on correctness fixes.

---

## How to use this doc

- Pick a theme. Move its item from **Planned** → **In progress** in the same PR that starts work.
- Close out an item by moving to **Done** with the PR/commit reference.
- New gaps surfaced during v0.7 work get appended under the appropriate theme — don't start a separate doc.
