# orbit-star

Orbit Grid Star Agent - Remote execution server for distributed data operations.

The Star agent is a lightweight, stateless gRPC server that exposes local filesystem and CPU resources to the Orbit Grid. It runs on remote NAS devices or file servers, allowing the Nucleus (Hub) to orchestrate distributed deduplication and data operations.

## Features

- **Secure Path Jail**: Sandboxed filesystem access limited to allowed directories
- **Streaming Directory Scans**: Efficient enumeration of massive directories (1M+ files)
- **Remote Hashing**: Offload CPU-intensive BLAKE3 hashing to edge nodes
- **Session-based Authentication**: Token-based handshake with session management

### Lifecycle Hooks 🆕

Formalized state machine for Star agent lifecycle transitions, preventing orphan jobs when a Star disappears mid-transfer:

```
         register()
              │
     ┌────────▼────────┐
     │   Registered     │  Star is known but not receiving work
     └────────┬─────────┘
              │ schedule()
     ┌────────▼────────┐
     │   Scheduled      │  Actively receiving and processing work
     └────────┬─────────┘
              │ drain()
     ┌────────▼────────┐
     │   Draining       │  Finishing current work, no new assignments
     └────────┬─────────┘
              │ shutdown()
     ┌────────▼────────┐
     │   Shutdown        │  Clean exit
     └──────────────────┘
```

- **Graceful Drain**: Tracks active tasks; `is_drained()` only returns true when task count reaches zero
- **Force Shutdown**: Can skip directly from any state to Shutdown for emergencies
- **Event History**: All transitions recorded with timestamps for audit and diagnostics
- **Invalid Transition Safety**: Returns `None` instead of panicking on illegal state changes

**Key types**: `StarLifecycle`, `LifecycleState`, `LifecycleEvent`

**Status**: 🔴 Alpha — 20 tests passing

## Installation

### Prerequisites

- Rust 1.70+ with `cargo`
- Protocol Buffers compiler (`protoc`) - see [orbit-proto README](../orbit-proto/README.md)

### Building

```bash
# Set PROTOC environment variable (Windows)
export PROTOC="C:\\path\\to\\protoc.exe"

# Build the binary
cargo build --release -p orbit-star

# Binary location
./target/release/orbit-star.exe  # Windows
./target/release/orbit-star       # Linux/macOS
```

## Usage

### Basic Startup

```bash
orbit-star \
  --port 50051 \
  --token "your-secret-token" \
  --allow /mnt/data \
  --allow /backups
```

### CLI Options

```
Options:
  -p, --port <PORT>                Port to listen on [default: 50051]
  -t, --token <TOKEN>              Authentication token (or set ORBIT_STAR_TOKEN env var)
  -a, --allow <ALLOW_PATHS>        Allowed root directories (can be repeated)
  -d, --debug                      Enable debug logging
  -b, --bind <BIND>                Bind address [default: 0.0.0.0]
  -h, --help                       Print help
  -V, --version                    Print version
```

### Environment Variables

- `ORBIT_STAR_TOKEN`: Authentication token (alternative to `--token`)
- `RUST_LOG`: Logging level (`info`, `debug`, `trace`)

## Security

### Path Jail

The Star agent implements a security sandbox that restricts filesystem access to explicitly allowed directories. This prevents:

- Directory traversal attacks (`../../etc/passwd`)
- Symlink escapes
- Unauthorized access to system files

Example:
```bash
# Only allow access to /data and /backups
orbit-star --allow /data --allow /backups --token SECRET
```

Requests for paths outside these directories will be rejected with a permission denied error.

### Authentication Flow

1. Client sends `Handshake` request with `star_token`
2. Star validates token and generates a `session_id`
3. All subsequent requests must include `session-id` in gRPC metadata

## Testing with grpcurl

Install `grpcurl`:
```bash
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest
```

### Handshake

```bash
grpcurl -plaintext \
  -d '{"star_token": "your-secret-token", "version": "0.6.0"}' \
  localhost:50051 orbit.v1.StarService/Handshake
```

### Calculate Hash

```bash
grpcurl -plaintext \
  -H "session-id: YOUR-SESSION-ID" \
  -d '{"path": "/data/file.bin", "offset": 0, "length": 1048576}' \
  localhost:50051 orbit.v1.StarService/CalculateHash
```

### Scan Directory

```bash
grpcurl -plaintext \
  -H "session-id: YOUR-SESSION-ID" \
  -d '{"path": "/data"}' \
  localhost:50051 orbit.v1.StarService/ScanDirectory
```

## Architecture

The Star agent implements the `StarService` gRPC interface defined in [orbit-proto](../orbit-proto):

```
┌─────────────┐         gRPC          ┌─────────────┐
│   Nucleus   │ ◄──────────────────► │    Star     │
│    (Hub)    │                       │   (Agent)   │
└─────────────┘                       └─────────────┘
                                            │
                                            ▼
                                      ┌──────────┐
                                      │ Local FS │
                                      │   CPU    │
                                      └──────────┘
```

## Development

### Running Tests

```bash
cargo test -p orbit-star
```

### Running with Debug Logging

```bash
RUST_LOG=debug orbit-star --debug --allow /tmp --token TEST
```

## See Also

- [Phase 2 Specification](../../docs/specs/PHASE_2_STAR_PROTO_SPEC.md)
- [orbit-proto](../orbit-proto)
- [Orbit Grid Architecture](../../docs/ORBIT_GRID_ARCHITECTURE.md)

## License

MIT
