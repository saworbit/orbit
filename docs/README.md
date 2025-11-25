# ORBIT Documentation

This directory contains all documentation for the ORBIT project.

## Structure

```
docs/
├── guides/           # User-facing guides and tutorials
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
│   └── SYNC_MIRROR.md
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
│   ├── smb_implementation.md
│   ├── SMB_NATIVE_STATUS.md
│   ├── WEB_GUI.md
│   └── ZERO_COPY.md
│
├── release/          # Release notes and summaries
│   ├── RELEASE.md
│   └── RELEASE_SUMMARY.md
│
└── file_structure_checklist.md
```

## Quick Links

### Getting Started
- [Quickstart Guide](guides/quickstart_guide.md)
- [Backend Guide](guides/BACKEND_GUIDE.md)

### Features
- [Sync & Mirror](guides/SYNC_MIRROR.md)
- [Delta Detection](guides/DELTA_DETECTION_GUIDE.md)
- [Filter System](guides/FILTER_SYSTEM.md)
- [S3 Integration](guides/S3_USER_GUIDE.md)

### Architecture
- [Guidance System](architecture/GUIDANCE_SYSTEM.md)
- [Zero-Copy Transfers](architecture/ZERO_COPY.md)
- [Resume System](architecture/RESUME_SYSTEM.md)
- [Manifest System](architecture/MANIFEST_SYSTEM.md)
