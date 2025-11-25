# Orbit Performance Guide

This guide provides information on optimizing Orbit performance and understanding how Orbit manages system resources.

## Concurrency Detection

Orbit attempts to automatically detect the number of available CPU cores to optimize parallel transfer threads.

### Auto-detection Behavior

- **Auto-detection:** Uses `std::thread::available_parallelism` to detect available CPU cores
- **Fallback:** If detection fails (e.g., in strict `cgroup` environments or restricted containers), Orbit defaults to **1 core** and logs a warning to stderr
- **Override:** You can manually set concurrency using the `--concurrency` (or `-j`) flag to bypass detection

### Why Single-Threaded Fallback?

When CPU detection fails, it typically indicates a restricted or hostile environment (e.g., containers with limited syscall access, strict cgroup configurations). In such cases:

- **Safety First:** Defaulting to 1 thread prevents resource exhaustion and OOM kills
- **Predictable Behavior:** Single-threaded mode is the safest fallback, guaranteeing minimal system pressure
- **User Awareness:** A warning is logged to stderr so operators know detection failed

### Manual Concurrency Override

If you need to override the detected concurrency level:

```bash
# Use 8 concurrent operations
orbit sync --concurrency 8 /source s3://bucket/destination

# Single-threaded mode
orbit sync --concurrency 1 /source s3://bucket/destination
```

### Optimal Concurrency

For I/O-bound operations like file transfers, Orbit uses `2 × CPU_count` concurrent operations, capped at 16. This allows better utilization during network I/O operations where threads spend time waiting.

## Performance Tips

1. **Network Transfers:** Higher concurrency helps saturate network bandwidth
2. **Local Operations:** Lower concurrency may be better for disk-bound operations
3. **Resource-Constrained Environments:** Use `--concurrency 1` or `--concurrency 2` to minimize resource usage
4. **High-Performance Servers:** Manually set higher concurrency if auto-detection caps too low

## Monitoring Performance

Watch for the CPU detection warning:

```
WARN: Orbit failed to detect available parallelism: <error>.
Defaulting to 1 concurrent operation to prevent resource exhaustion.
```

If you see this warning:
- Check your container/cgroup configuration
- Verify syscall permissions
- Consider manually setting `--concurrency` to an appropriate value for your environment
