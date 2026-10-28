# Changelog

All notable changes to Orbit are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `orbit copy` and `orbit plan` now refuse a destination that ends with a
  path separator (exit code 3, `destination_has_trailing_separator`). A
  trailing separator is a directory signal, so silently normalizing it to a
  single-file destination was surprising. Fixes #110.
- Windows error messages render paths with a consistent ordinary spelling.
  Reversible `\\?\C:\` and `\\?\UNC\` prefixes are now stripped before
  display so similar errors read the same way. Fixes #111.
