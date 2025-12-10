# Dependabot Issues - Breaking Changes

The following dependency updates require manual handling due to breaking API changes. Create GitHub issues for these:

---

## Issue 1: Update bincode to 2.0 (breaking changes)

**Labels**: `dependencies`, `refactoring`, `breaking-change`

**Description**:
Dependabot has flagged bincode 1.3 → 2.0 update, but this requires significant refactoring due to breaking API changes.

### Breaking Changes
- **Old API**: `bincode::serialize()`, `bincode::deserialize()` with Serde traits
- **New API**: Custom `Encode`/`Decode` traits instead of Serde, or use `bincode::serde` module with different function signatures

### Affected Files
All in `crates/core-starmap`:
- `src/builder.rs` (serialization)
- `src/reader.rs` (deserialization)
- `src/universe.rs` (multiple serialize/deserialize calls)
- `src/universe_v3.rs` (multiple serialize/deserialize calls)

### Impact
~15+ instances of `bincode::serialize`/`deserialize` calls need updating.

### Migration Steps
1. Enable `serde` feature in bincode 2.0: `bincode = { version = "2.0", features = ["serde"] }`
2. Update all calls:
   - Old: `bincode::serialize(&data)`
   - New: `bincode::encode_to_vec(&data, bincode::config::standard())`
3. Update deserialize calls:
   - Old: `bincode::deserialize(&bytes)`
   - New: `let (data, _) = bincode::decode_from_slice(&bytes, bincode::config::standard())`
4. Comprehensive testing of all serialization paths

### References
- Bincode 2.0 migration guide: https://github.com/bincode-org/bincode
- Dependabot branch was: `dependabot/cargo/bincode-2.0.1`

---

## Issue 2: Update redb to 3.1 (breaking changes)

**Labels**: `dependencies`, `refactoring`, `breaking-change`

**Description**:
Dependabot has flagged redb 2.x → 3.1 update. The transaction API has changed significantly.

### Breaking Changes
- Transaction creation methods have changed
- `begin_read()` method signature or behavior changed
- Error handling may have changed

### Affected Files
- `crates/core-starmap/src/universe.rs`
- `crates/core-starmap/src/universe_v3.rs`
- `crates/magnetar/src/backends/redb.rs`

### Impact
Multiple database read transactions need API updates. The `magnetar` redb backend also uses these APIs extensively.

### Migration Steps
1. Review redb 3.0 changelog for API changes
2. Update transaction creation calls
3. Check if `begin_read()` was renamed or signature changed
4. Update error handling if needed
5. Test all database read/write operations thoroughly
6. Verify ACID compliance maintained

### References
- Redb 3.0 release notes: https://github.com/cberner/redb/releases
- Dependabot branch was: `dependabot/cargo/redb-3.1.0`

---

## Issue 3: Update jsonschema to 0.37 (breaking changes)

**Labels**: `dependencies`, `refactoring`, `breaking-change`

**Description**:
Dependabot has flagged jsonschema 0.22 → 0.37 update. The validation error handling API has changed.

### Breaking Changes
- `ValidationError` is no longer directly iterable
- `instance_path` changed from field to method: `instance_path()`
- Error iteration requires `.into_iter()` or different approach

### Affected Files
- `crates/core-manifest/src/validate.rs` (2 validation functions, 1 helper)

### Current Errors
```rust
// Line 22 & 44: Need to add .into_iter() or use different iteration method
errors.map(|e| ...) // Error: ValidationError doesn't implement Iterator

// Line 57: Need to call method instead of accessing field
error.instance_path  // Should be: error.instance_path()
```

### Migration Steps
1. Update error iteration:
   - Check if `errors` needs `.into_iter()` or has different iteration API
   - May need to collect errors differently
2. Update `format_validation_error()`:
   - Change `error.instance_path` to `error.instance_path()`
3. Review other ValidationError API changes
4. Test validation for both Flight Plan and Cargo Manifest schemas
5. Verify error messages are still user-friendly

### References
- jsonschema 0.37 changelog: https://github.com/Stranger6667/jsonschema-rs/releases
- Dependabot branch was: `dependabot/cargo/jsonschema-0.37.4`

---

## Issue 4: Update recharts to 3.5.1 (React 19 compatibility)

**Labels**: `dependencies`, `dashboard`, `typescript`, `breaking-change`

**Description**:
Dependabot has flagged recharts 3.4 → 3.5.1 update, but it has TypeScript type incompatibilities with React 19.

### Breaking Changes
React 19.2.1 type definitions are incompatible with recharts chart component types, causing TypeScript compilation errors.

### Affected Files
- `dashboard/src/components/ui/chart.tsx`

### Current Errors
```typescript
// Type errors in chart.tsx:
- Property 'payload' does not exist on type
- Property 'label' does not exist on type
- Parameter 'item' implicitly has 'any' type
- Type '"payload"' is not assignable to constraint
- Property 'length' does not exist on type '{}'
- Property 'map' does not exist on type '{}'
```

### Migration Options
1. **Wait for recharts update**: Monitor recharts for React 19 compatibility release
2. **Type fixes**: Add explicit type annotations to work around type inference issues
3. **Rewrite chart component**: Use different charting library compatible with React 19
4. **Keep React 18**: Delay React 19 upgrade until recharts is compatible

### Impact
Currently blocks React 19 upgrade. Dashboard build fails with TypeScript errors.

### Migration Steps (Option 2 - Type Fixes)
1. Add explicit types to chart component props
2. Type the `payload` and `label` properties explicitly
3. Add type annotations for callback parameters
4. May need to use type assertions in some cases
5. Verify charts render correctly after type fixes

### References
- Recharts GitHub issues for React 19: https://github.com/recharts/recharts/issues
- Dependabot branch was: `dependabot/npm_and_yarn/dashboard/recharts-3.5.1`
- Related: React 19 was successfully merged despite this blocker (other npm packages work)

---

## Issue 5: Update axum-extra to 0.12 (requires axum 0.8 upgrade)

**Labels**: `dependencies`, `web`, `breaking-change`

**Description**:
Dependabot has flagged axum-extra 0.9 → 0.12 update, but this requires upgrading axum from 0.7 to 0.8, which is a breaking change.

### Breaking Changes
- axum-extra 0.12 requires axum 0.8 as a peer dependency
- axum 0.8 has breaking API changes from 0.7
- Multiple handler trait changes and routing API updates

### Affected Files
- `crates/orbit-web/Cargo.toml`
- `crates/orbit-web/src/server.rs` (API route handlers)
- `crates/orbit-web/src/api/*.rs` (all API modules)

### Current Error
```
error[E0277]: the trait bound `fn(...) -> ... {handler}: Handler<_, _>` is not satisfied
note: there are multiple different versions of crate `axum` in the dependency graph
- axum 0.7.9 (direct dependency)
- axum 0.8.7 (via axum-extra 0.12)
```

### Migration Options
1. **Upgrade to axum 0.8**: Update all handler signatures and routing code
2. **Stay on axum 0.7**: Keep axum-extra at 0.9 (current approach)

### Impact
Blocks axum-extra upgrade until full axum 0.8 migration is completed. This affects access to new features in axum-extra like improved cookie handling and typed headers.

### Migration Steps (Option 1 - Upgrade to axum 0.8)
1. Update `Cargo.toml`: `axum = "0.8"`, `axum-extra = "0.12"`
2. Review axum 0.8 migration guide: https://github.com/tokio-rs/axum/releases/tag/axum-0.8.0
3. Update all route handler signatures (trait bounds may have changed)
4. Update State extraction patterns if changed
5. Test all API endpoints thoroughly
6. Verify WebSocket connections still work
7. Check multipart file upload functionality

### References
- Axum 0.8 release notes: https://github.com/tokio-rs/axum/releases/tag/axum-0.8.0
- Dependabot branch was: `dependabot/cargo/axum-extra-0.12.2`

---

## Summary

**Successfully Merged**: 22 dependency updates
**Require Manual Work**: 5 updates (axum-extra, bincode, redb, jsonschema, recharts)

All 34 dependabot branches have been cleaned up. The safe updates are already merged to main and building successfully.
