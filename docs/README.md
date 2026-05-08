# Orbit Documentation

This directory contains all documentation for the Orbit project.

## Quick Links

### Getting Started
- **[Getting Started Guide](GETTING_STARTED.md)** - Installation, setup, and first transfers
- [Quickstart Guide](guides/quickstart_guide.md) - Extended CLI usage examples
- [Init Wizard Guide](guides/INIT_WIZARD_GUIDE.md) - Deep dive into `orbit init`

### User Guides
- [Backend Guide](guides/BACKEND_GUIDE.md) - All storage backends
- [S3 User Guide](guides/S3_USER_GUIDE.md) - Complete S3 operations
- [GCS User Guide](guides/GCS_USER_GUIDE.md) - Google Cloud Storage
- [Protocol Guide](guides/PROTOCOL_GUIDE.md) - Protocol-specific details
- [Filter System](guides/FILTER_SYSTEM.md) - Glob, regex, and path filters
- [Delta Detection](guides/DELTA_DETECTION_GUIDE.md) - Efficient transfer algorithms
- [Delta Quickstart](guides/DELTA_QUICKSTART.md) - Quick delta setup
- [Disk Space Guide](guides/DISK_SPACE_GUIDE.md) - Managing disk space
- [Logging Guide](guides/LOGGING_GUIDE.md) - Configuring log output
- [Performance Guide](guides/PERFORMANCE.md) - Tuning for speed
- [Production Deployment](guides/PRODUCTION_DEPLOYMENT.md) - Deployment best practices
- [SMB Testing](guides/SMB_TESTING.md) - Testing SMB connections
- [Backend Streaming](guides/BACKEND_STREAMING_GUIDE.md) - Streaming I/O patterns
- [Active Guidance](guides/ACTIVE_GUIDANCE_GUIDE.md) - Config Optimizer usage
- [Testing Scripts](guides/TESTING_SCRIPTS_GUIDE.md) - Test automation

### Architecture
- [Config Optimizer](architecture/GUIDANCE_SYSTEM.md) - Configuration validation system
- [Zero-Copy Transfers](architecture/ZERO_COPY.md) - Platform-specific optimizations
- [Resume System](architecture/RESUME_SYSTEM.md) - Checkpoint and recovery
- [Manifest System](architecture/MANIFEST_SYSTEM.md) - File tracking
- [Error Handling](architecture/ERROR_HANDLING_IMPLEMENTATION.md) - Retry and error classification
- [Disk Guardian](architecture/DISK_GUARDIAN.md) - Pre-flight safety checks
- [Progress System](architecture/PROGRESS_SYSTEM.md) - Progress reporting
- [Advanced Transfer](architecture/ADVANCED_TRANSFER.md) - Advanced transfer features
- [V2 Architecture](architecture/ORBIT_V2_ARCHITECTURE.md) - CDC + Semantic (alpha)
- [SMB Implementation](architecture/smb_implementation.md) - Native SMB details
- [SMB Native Status](architecture/SMB_NATIVE_STATUS.md) - SMB backend status
- [Scalability](architecture/SCALABILITY_SPEC.md) - Scaling characteristics
- [Observability](architecture/OBSERVABILITY_IMPLEMENTATION.md) - Telemetry system
- [Delta Implementation](architecture/DELTA_IMPLEMENTATION_SUMMARY.md) - Delta internals

### Specifications
- [Backend Refactoring Plan](specs/BACKEND_REFACTORING_PLAN.md) - Backend trait evolution
- [Phase 1 Abstraction](specs/PHASE_1_ABSTRACTION_SPEC.md) - OrbitSystem design
- [Retry Optimization](specs/RETRY_OPTIMIZATION_SPEC.md) - Retry logic design

### Migration Guides
- [Migration Guide](guides/migration_guide.md) - Version migration
- [V2 Migration](guides/MIGRATION_V2.md) - Migrating to V2
- [Universe V3 Migration](guides/UNIVERSE_V3_MIGRATION.md) - Universe index upgrade

### Release Notes
- [Release Process](release/RELEASE.md) - How releases are made
- [Release Summary](release/RELEASE_SUMMARY.md) - Latest release overview
- [V2.1 Release Notes](release/V2.1_RELEASE_NOTES.md) - V2.1 changelog

### Project Status
- [Project Status Overview](project-status/README.md) - Current status
- [Safety First](project-status/SAFETY_FIRST.md) - Safety philosophy
- [Testing Status](project-status/TESTING.md) - Test coverage
- [Azure Status](project-status/AZURE_IMPLEMENTATION_STATUS.md) - Azure backend

### Other
- [Observability V3](observability-v3.md) - V3 observability design
- [Dependabot Issues](DEPENDABOT_ISSUES.md) - Dependency advisories
- [Wormhole Design](wormhole/wormhole_design_doc.md) - FEC module design

## Directory Structure

```
docs/
├── GETTING_STARTED.md        # Start here
├── guides/                   # User-facing guides and tutorials
├── architecture/             # Internal implementation details
├── specs/                    # Technical specifications
├── release/                  # Release notes and process
├── project-status/           # Current project status
├── archive/                  # Historical/deprecated docs
├── manifest/                 # Manifest integration docs
└── wormhole/                 # Wormhole FEC module
```
