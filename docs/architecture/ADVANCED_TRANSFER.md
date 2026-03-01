# Advanced Transfer Features

Six features inspired by rsync, reimplemented on Orbit's CDC + Star Map architecture.

| Feature | Module | CLI | rsync | Orbit Improvement |
|---------|--------|-----|-------|-------------------|
| Sparse files | `core/sparse.rs` | `--sparse {auto\|always\|never}` | `--sparse` | Zero-cost detection during CDC; combinable with `--inplace` |
| Hardlinks | `core/hardlink.rs` | `--preserve-hardlinks` | `-H` | Cross-platform (Unix inode + Windows FFI) |
| In-place | `core/inplace.rs` | `--inplace --inplace-safety {reflink\|journaled\|unsafe}` | `--inplace` | Three safety tiers vs none |
| Rename detect | `core/rename_detector.rs` | `--detect-renames --rename-threshold 0.8` | `--fuzzy` | Content-aware chunk overlap vs filename similarity |
| link-dest++ | `core/link_dest.rs` | `--link-dest DIR` | `--link-dest` | Chunk-level partial reuse vs all-or-nothing |
| Batch mode | `core/batch.rs` | `--write-batch FILE --read-batch FILE` | `--write-batch` | Content-addressed journal, portable across destinations |

---

## Sparse Files

Detects all-zero chunks **during CDC** (`chunk.data.iter().all(|&b| b == 0)` — SIMD-vectorized) and writes filesystem holes via `seek()`. `Auto` mode only activates for files >= 64KB. Tracks logical vs physical bytes in `SparseWriteStats`.  
**Note:** Sparse + in-place is not yet supported in the CLI; they are mutually exclusive for now.

## Hardlink Preservation

`HardlinkTracker` maps `(device, inode)` to first-seen path. Only tracks files with `nlink > 1`. Unix uses `MetadataExt`; Windows uses raw FFI to `GetFileInformationByHandle` for `volume_serial + file_index`.

## In-Place Updates

`InplaceWriter` modifies files directly via `write_at(offset, data)`:

- **Reflink**: `ioctl(FICLONE)` (Linux) / `clonefile(2)` (macOS) — O(1) CoW snapshot before first write. Unsupported filesystems degrade gracefully.
- **Journaled**: Records `(offset, original_bytes)` in `UndoJournal` before each overwrite.
- **Unsafe**: Direct overwrite, no recovery.

`recover_from_journal()` restores from reflink snapshot first, then journal entries.

## Rename Detection

For each new source file: sample first/middle/last chunk hashes → query Star Map for candidates → compute full overlap ratio → use as delta basis if >= threshold (default 80%). Complexity: O(sample_size * files_per_chunk) per file.  
**Current CLI:** Uses full-file hashing for exact-match rename detection only. Partial-overlap delta basis is planned.

## Link-Dest++

`LinkDestResolver` checks reference directories in priority order. Returns `Hardlink` (100% match + same count), `DeltaBasis` (partial match >= 30% threshold), or `FullTransfer`. `quick_match()` pre-filters by size + mtime.  
**Current CLI:** Hardlinks exact matches only; partial-chunk delta basis is planned.

## Batch Mode

`TransferJournal` records `CreateFile|UpdateFile|DeleteFile|CreateDir|CreateHardlink|SetMetadata` entries with CDC chunk hashes. Binary format: `ORBITBTC` magic + version + bincode payload. `DeltaOp::CopyChunk` (reuse by hash) / `WriteChunk` (new data). Replay is portable because entries reference content, not positions.  
**Current CLI:** `--write-batch` emits full-file `CreateFile` entries (no delta ops yet) and requires `--mode copy`.

---

## Test Coverage

| Module | Tests | Key areas |
|--------|-------|-----------|
| `sparse.rs` | 10 | Holes, auto threshold, empty files, CDC conversion |
| `hardlink.rs` | 9 | Creation, tracker groups, three-way links, error paths |
| `inplace.rs` | 12 | All safety tiers, crash recovery, reflink fallback |
| `rename_detector.rs` | 11 | Exact/partial/no match, custom threshold, same-path |
| `link_dest.rs` | 12 | Hardlink/delta/full, thresholds, quick_match |
| `batch.rs` | 12 | Save/load, all replay ops, version mismatch |
| **Total** | **66** | |
