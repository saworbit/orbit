# Changelog

All notable changes to Orbit will be documented in this file.

## [0.3.0] - 2025-10-12

### Added
- Modular architecture with separated modules (config, core, compression, audit, error)
- Zstd compression with 22 configurable levels (1-22)
- Parallel file copying with CPU auto-detection
- TOML configuration file support (project and user level)
- JSON Lines audit logging (machine-parseable)
- Multiple copy modes: Copy, Sync, Update, Mirror
- Bandwidth limiting (MB/s)
- Exclude patterns (glob-based filtering)
- Dry run mode (preview operations)
- Streaming SHA-256 checksums (calculated during copy)
- Comprehensive test suite (15+ integration tests, ~60% coverage)

### Changed
- Complete modular rewrite from monolithic structure
- CLI syntax updated (breaking change - see MIGRATION_GUIDE.md)
- Improved error messages with context
- Performance: 73% faster for many small files, 19% faster for large compressed files

### Breaking Changes
- `--compress` now requires a value: `none`, `lz4`, or `zstd:N` (where N is 1-22)
- Audit log format changed to JSON Lines by default (was CSV-like)
- Configuration file structure redesigned

## [0.2.0] - 2025-06-02

### Added
- Basic file copying with LZ4 compression
- SHA-256 checksum verification
- Resume capability for interrupted transfers
- Simple retry logic
- Basic audit logging

## [0.1.0] - 2025-05-01

### Added
- Initial release
- Simple file copy operations
- Basic error handling