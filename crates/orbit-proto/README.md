# orbit-proto

gRPC protocol definitions for the Orbit Grid.

This crate provides the protocol buffer definitions and generated Rust code for communication between the Nucleus (Hub) and Stars (Agents) in the Orbit distributed data fabric.

## Overview

The `orbit.proto` schema defines four core RPC methods:

1. **Handshake** - Authentication and session establishment
2. **ScanDirectory** - Streaming directory enumeration
3. **ReadHeader** - Magic number detection for semantic analysis
4. **CalculateHash** - Remote content-defined chunking and hashing

## Building

This crate requires the Protocol Buffers compiler (`protoc`) to build.

### Installing protoc

**Windows:**
```bash
# Download and extract protoc
curl -LO https://github.com/protocolbuffers/protobuf/releases/download/v28.3/protoc-28.3-win64.zip
unzip protoc-28.3-win64.zip -d protoc-tools

# Set environment variable for Cargo
export PROTOC="$(pwd)/protoc-tools/bin/protoc.exe"
```

**macOS:**
```bash
brew install protobuf
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install -y protobuf-compiler

# Fedora
sudo dnf install protobuf-compiler

# Arch
sudo pacman -S protobuf
```

### Building the crate

```bash
cargo build -p orbit-proto
```

## Usage

```rust
use orbit_proto::{
    star_service_client::StarServiceClient,
    HandshakeRequest,
};
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://localhost:50051")
        .connect()
        .await?;

    let mut client = StarServiceClient::new(channel);

    let request = tonic::Request::new(HandshakeRequest {
        star_token: "secret-token".to_string(),
        version: "0.6.0".to_string(),
        capabilities: vec!["zstd".to_string()],
    });

    let response = client.handshake(request).await?;
    println!("Handshake: {:?}", response);

    Ok(())
}
```

## Protocol Schema

See [`proto/orbit.proto`](proto/orbit.proto) for the full protocol definition.

## License

MIT
