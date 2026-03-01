# ORBIT Documentation

This directory contains all documentation for the ORBIT project.

## Structure

```
docs/
├── guides/           # User-facing guides and tutorials
│   ├── RUNNING_V2.md
│   ├── quickstart_guide.md
│   ├── BACKEND_GUIDE.md
│   ├── DELTA_DETECTION_GUIDE.md
│   ├── DELTA_QUICKSTART.md
│   ├── FILTER_SYSTEM.md
│   ├── migration_guide.md
│   ├── PROTOCOL_GUIDE.md
│   ├── RELEASE_QUICKSTART.md
│   ├── S3_USER_GUIDE.md
│   ├── SMB_TESTING.md
│   ├── SYNC_MIRROR.md
│   └── UNIVERSE_V3_MIGRATION.md
│
├── architecture/     # Internal implementation details
│   ├── DELTA_IMPLEMENTATION_SUMMARY.md
│   ├── DELTA_INTEGRATION_CHANGES.md
│   ├── DISK_GUARDIAN.md
│   ├── ERROR_HANDLING_IMPLEMENTATION.md
│   ├── GUIDANCE_SYSTEM.md
│   ├── GUI_INTEGRATION.md
│   ├── IMPLEMENTATION_SUMMARY.md
│   ├── MANIFEST_SYSTEM.md
│   ├── ORBIT_WEB_IMPLEMENTATION_SUMMARY.md
│   ├── PROGRESS_AND_CONCURRENCY.md
│   ├── PROGRESS_SYSTEM.md
│   ├── RESUME_SYSTEM.md
│   ├── SCALABILITY_SPEC.md
│   ├── smb_implementation.md
│   ├── SMB_NATIVE_STATUS.md
│   ├── WEB_GUI.md
│   ├── ZERO_COPY.md
│   └── ADVANCED_TRANSFER.md
│
├── specs/            # Technical specifications
│   └── RETRY_OPTIMIZATION_SPEC.md
│
├── release/          # Release notes and summaries
│   ├── RELEASE.md
│   ├── RELEASE_SUMMARY.md
│   └── V2.1_RELEASE_NOTES.md
│
└── file_structure_checklist.md
```

## Quick Links

### Getting Started
- **[Running Orbit v2.2 Web Platform](guides/RUNNING_V2.md)** ⭐ Start here for the dashboard!
- [Quickstart Guide](guides/quickstart_guide.md) - CLI tool usage
- [Backend Guide](guides/BACKEND_GUIDE.md)

### Features
- [Sync & Mirror](guides/SYNC_MIRROR.md)
- [Delta Detection](guides/DELTA_DETECTION_GUIDE.md)
- [Filter System](guides/FILTER_SYSTEM.md)
- [S3 Integration](guides/S3_USER_GUIDE.md)

### Architecture
- [V2 Architecture (CDC, Semantic, Universe)](architecture/ORBIT_V2_ARCHITECTURE.md)
- [Guidance System](architecture/GUIDANCE_SYSTEM.md)
- [Zero-Copy Transfers](architecture/ZERO_COPY.md)
- [Resume System](architecture/RESUME_SYSTEM.md)
- [Manifest System](architecture/MANIFEST_SYSTEM.md)
- [Error Handling & Retries](architecture/ERROR_HANDLING_IMPLEMENTATION.md)
- [Observability & Telemetry](architecture/OBSERVABILITY_IMPLEMENTATION.md)
- [Scalability (Universe V3)](architecture/SCALABILITY_SPEC.md)
- [Advanced Transfer Features](architecture/ADVANCED_TRANSFER.md)

### Technical Specifications
- [Retry Logic Optimization](specs/RETRY_OPTIMIZATION_SPEC.md)
