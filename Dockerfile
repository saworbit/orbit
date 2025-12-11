# ---------------------------------------------------
# Stage 1: Build Dashboard (Frontend)
# ---------------------------------------------------
FROM node:20-slim AS frontend-builder
WORKDIR /app/dashboard
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ .
# Builds static files to /app/dashboard/dist
RUN npm run build

# ---------------------------------------------------
# Stage 2: Build Orbit Server (Backend)
# ---------------------------------------------------
FROM rust:1.81-slim-bookworm AS backend-builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files to cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY src/ src/
COPY .cargo/ .cargo/

# Build the server specifically
RUN cargo build --release --package orbit-server

# ---------------------------------------------------
# Stage 3: Runtime
# ---------------------------------------------------
FROM debian:bookworm-slim
WORKDIR /app

# Install necessary runtime libs (OpenSSL, ca-certificates)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy Binary
COPY --from=backend-builder /app/target/release/orbit-server /usr/local/bin/orbit-server

# Copy Frontend Assets
# Assuming orbit-server is configured to serve static files from ./public or similar
# You may need to adjust the destination based on your orbit-server config
COPY --from=frontend-builder /app/dashboard/dist /app/static

ENV ORBIT_HOST=0.0.0.0
ENV ORBIT_PORT=3000
ENV ORBIT_STATIC_DIR=/app/static

EXPOSE 3000

CMD ["orbit-server"]
