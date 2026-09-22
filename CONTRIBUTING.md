# Contributing

Orbit is in a ground-up reboot. Keep changes within the approved design and the active implementation plan.

Before submitting a change, run:

```text
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Do not add a protocol, backend abstraction, service, plugin system, or compatibility layer without an approved design.
