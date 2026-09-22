# Orbit single-file copy design

## Goal

Add the first write operation to Orbit: safely copy one regular local file to a new local destination. This is deliberately smaller than a general-purpose copy engine so its safety properties can be established before adding replacement, directories, resume, or remote storage.

## User contract

`orbit copy SOURCE DESTINATION` accepts one regular file as its source.

- The destination's parent directory must already exist.
- A missing destination is eligible for copying.
- An existing destination with identical content is a successful no-op.
- An existing destination with different content is refused without modification.
- Symbolic links, directories, special files, and remote endpoints remain unsupported.
- Hash verification is the default; size verification remains available explicitly.
- Failures use Orbit's existing diagnostics, JSON/human/quiet output conventions, and non-zero exit codes.

## Execution model

Orbit first reuses the existing planner and refuses to execute any blocked or unsupported plan. For an eligible file it then:

1. Creates a uniquely named staging file beside the destination using create-new semantics.
2. Streams the source into the staging file without modifying the source.
3. Flushes the staging file to stable storage.
4. Verifies the staged content against the source using the selected verification mode.
5. Publishes the staged file with an atomic no-replace operation.
6. Removes staging material on failure and reports its exact path if cleanup cannot be completed.

Publishing must never overwrite a path that appears after planning. Platform-specific code is acceptable for the final atomic no-replace step; unsupported filesystems fail closed rather than falling back to a partial or overwriting copy.

## Boundaries

This milestone does not add replacement, backup/rollback, recursive directory copy, resumability, parallel transfer, metadata preservation beyond ordinary file creation, cloud protocols, a daemon, or a language split. Rust remains the right implementation language for this small filesystem core; the broader full-stack question remains open for later user interfaces and services.

## Recovery and observability

The destination is either absent or complete and verified. A failed operation never exposes a partial destination under the requested name. Reports identify whether the result was copied, already identical, refused, or failed, and include stable machine-readable error codes in JSON mode. Temporary names use an Orbit-reserved pattern and are never silently treated as completed files.

## Verification

Tests cover successful copy, byte-for-byte equality, identical-destination no-op, differing-destination refusal, destination-creation races, source mutation detection, unsupported source types, missing parent directories, staging cleanup, reporting modes, and injected I/O failures. Existing planner behavior remains unchanged.
