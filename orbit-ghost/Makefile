# Orbit GhostFS Makefile
# Convenience commands for development and CI

.PHONY: help build test clean fmt lint audit check-all install run demo

# Default target
help:
	@echo "Orbit GhostFS - Available targets:"
	@echo ""
	@echo "  build        - Build the project (debug mode)"
	@echo "  release      - Build optimized release binary"
	@echo "  test         - Run all tests"
	@echo "  clean        - Remove build artifacts"
	@echo "  fmt          - Format code with rustfmt"
	@echo "  fmt-check    - Check code formatting"
	@echo "  lint         - Run clippy linter"
	@echo "  audit        - Check for security vulnerabilities"
	@echo "  deny         - Run cargo-deny checks"
	@echo "  outdated     - Check for outdated dependencies"
	@echo "  check-all    - Run all checks (fmt, lint, test, audit)"
	@echo "  install      - Install binary to ~/.cargo/bin"
	@echo "  run          - Run the ghost filesystem"
	@echo "  demo         - Run the demo script"
	@echo "  docs         - Build and open documentation"
	@echo "  coverage     - Generate code coverage report"
	@echo "  bloat        - Analyze binary size"
	@echo ""

# Build targets
build:
	cargo build

release:
	cargo build --release

# Testing
test:
	cargo test --verbose

test-all:
	cargo test --verbose --all-features

# Code quality
fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings

# Security and dependency checks
audit:
	cargo audit

deny:
	cargo deny check

outdated:
	cargo outdated

# Comprehensive check
check-all: fmt-check lint test audit
	@echo "✅ All checks passed!"

# Installation
install:
	cargo install --path .

# Running
run:
	cargo run --release

demo:
	./demo_quantum.sh

# Documentation
docs:
	cargo doc --open --no-deps

docs-all:
	cargo doc --open --document-private-items

# Code coverage
coverage:
	cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html
	@echo "Coverage report generated: tarpaulin-report.html"

# Binary size analysis
bloat:
	cargo bloat --release --crates

bloat-full:
	cargo bloat --release

# Benchmarking (future)
bench:
	cargo bench

# Cleaning
clean:
	cargo clean
	rm -rf target/
	rm -f Cargo.lock

clean-cache:
	rm -rf /tmp/orbit_cache/*
	rm -rf /tmp/orbit_ghost_mount

# Development helpers
watch:
	cargo watch -x check -x test

watch-run:
	cargo watch -x run

# Pre-commit checks
pre-commit: fmt lint test
	@echo "✅ Pre-commit checks passed!"

# CI simulation (runs what CI will run)
ci: fmt-check lint test audit
	@echo "✅ CI checks passed!"

# Release checklist
pre-release: clean check-all
	@echo "✅ Release checks passed!"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Update version in Cargo.toml"
	@echo "  2. Update CHANGELOG.md"
	@echo "  3. git tag -a vX.Y.Z -m 'Release vX.Y.Z'"
	@echo "  4. git push origin vX.Y.Z"
	@echo "  5. cargo publish (when ready)"
