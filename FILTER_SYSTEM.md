# Inclusion/Exclusion Filter System

## Overview

Orbit now includes a comprehensive inclusion/exclusion filter system that allows selective processing of files and directories during transfer operations. This system is inspired by rsync/rclone filter capabilities and supports glob patterns, regular expressions, and exact path matching.

## Features

### Core Capabilities

- **Multiple Filter Types**: Glob patterns, regular expressions, and exact path matching
- **Include/Exclude Rules**: Support for both inclusion and exclusion patterns
- **First-Match-Wins Semantics**: Rules are evaluated in order, with the first matching rule determining the action
- **Filter Files**: Load filter rules from external files for reusability
- **Negation Support**: Invert filter actions with the `!` prefix
- **Cross-Platform**: Consistent path matching across Windows, macOS, and Linux
- **Dry-Run Visibility**: Filtered items are shown in dry-run mode
- **Early Directory Pruning**: Skip entire directory trees efficiently

### Integration

- Seamlessly integrated into directory walking (`walkdir`)
- Works with all copy modes: Copy, Sync, Update, Mirror
- Respects filters during mirror mode deletions
- Backward compatible with existing `--exclude` patterns

## Usage

### Command-Line Interface

```bash
# Exclude patterns (glob by default)
orbit -s /source -d /dest --recursive \
  --exclude="*.tmp" \
  --exclude="target/**" \
  --exclude="node_modules/**"

# Include patterns (override excludes with higher priority)
orbit -s /source -d /dest --recursive \
  --include="*.rs" \
  --include="Cargo.toml" \
  --exclude="target/**"

# Explicit pattern types
orbit -s /source -d /dest --recursive \
  --include="glob:*.rs" \
  --exclude="regex:^build/.*\.o$" \
  --include="path:README.md"

# Load filters from file
orbit -s /source -d /dest --recursive \
  --filter-from=filters.orbitfilter

# Combine all approaches
orbit -s /source -d /dest --recursive \
  --include="important.log" \
  --exclude="*.log" \
  --filter-from=project.filters \
  --dry-run
```

### Filter File Format

Filter files use a simple, human-readable syntax:

```text
# Comments start with #

# Include rules (+ or include keyword)
+ *.rs
include **/*.toml

# Exclude rules (- or exclude keyword)
- *.log
exclude target/**

# Explicit pattern types
+ glob: src/**/*.rs
- regex: ^tests/.*_test\.rs$
include path: Cargo.lock

# Negation (inverts the action)
! - *.backup
```

### Pattern Types

#### 1. Glob Patterns (Default)

Standard glob syntax with recursive wildcards:

```text
*.txt           # All .txt files
src/**/*.rs     # All .rs files under src/
data/*          # All files directly in data/
**/.git/**      # All .git directories and their contents
```

#### 2. Regular Expressions

Use `regex:` prefix for advanced pattern matching:

```text
regex: ^src/.*\.rs$           # .rs files in src/ (not subdirs)
regex: .*_(test|spec)\.rs$    # Test files
regex: ^(build|dist)/.*       # Files in build or dist
```

#### 3. Exact Path Matching

Use `path:` prefix for exact matches:

```text
path: Cargo.toml              # Only the root Cargo.toml
path: src/main.rs             # Specific file
```

## Semantics

### First-Match-Wins

Rules are evaluated in order from first to last. The first rule that matches a path determines whether it's included or excluded:

```text
# Include Rust files first
+ **/*.rs

# Then exclude test files
- **/*_test.rs

# Result: main.rs included, main_test.rs excluded (first rule matched)
```

### Priority Order

1. **Include patterns** specified via `--include` (highest priority)
2. **Exclude patterns** specified via `--exclude`
3. **Rules from filter file** (in file order)
4. **Default action**: Include (if no rules match)

### Example with Priority

```bash
orbit --include="important.log" --exclude="*.log" --filter-from=filters.txt
```

Evaluation order:
1. Check `important.log` include pattern (matches → include)
2. Check `*.log` exclude pattern (skipped, already matched)
3. Check filter file rules (skipped, already matched)

## Examples

### Example 1: Rust Project

Copy only source files, exclude build artifacts:

```bash
orbit -s /my-project -d /backup --recursive \
  --include="**/*.rs" \
  --include="**/*.toml" \
  --include="**/*.md" \
  --exclude="target/**" \
  --exclude="**/*.lock"
```

### Example 2: Selective Backup

Create filter file `backup.orbitfilter`:

```text
# Include source code
+ **/*.rs
+ **/*.toml
+ **/*.md

# Include docs
+ docs/**

# Exclude build artifacts
- target/**
- build/**
- dist/**

# Exclude logs and temp files
- *.log
- *.tmp
- *.temp

# Exclude version control
- .git/**
- .svn/**

# Exclude dependencies
- node_modules/**
- vendor/**
```

Use it:

```bash
orbit -s /project -d /backup --recursive --filter-from=backup.orbitfilter
```

### Example 3: Mirror with Filters

Mirror a directory but preserve certain excluded paths at destination:

```bash
orbit -s /source -d /dest --mode=mirror --recursive \
  --exclude=".env" \
  --exclude="secrets/**"
```

Files matching the exclude patterns:
- Won't be copied from source
- Won't be deleted from destination (even if not in source)

### Example 4: Regex for Complex Patterns

```bash
orbit -s /logs -d /backup --recursive \
  --include="regex:^app-20(24|25)-.*\.log$" \
  --exclude="*.log"
```

Includes only logs from 2024 or 2025, excludes all other `.log` files.

### Example 5: Dry-Run to Preview

Test filters before actual copy:

```bash
orbit -s /source -d /dest --recursive \
  --exclude="*.tmp" \
  --exclude="build/**" \
  --dry-run
```

Output shows which files would be copied and which would be filtered out.

## Implementation Details

### Architecture

The filter system consists of:

1. **[src/core/filter.rs](src/core/filter.rs)**: Core filter module
   - `FilterRule`: Single include/exclude rule
   - `FilterList`: Ordered collection of rules
   - `FilterType`: Enum for Glob/Regex/Path patterns
   - `FilterAction`: Include or Exclude action
   - `FilterDecision`: Evaluation result (Include/Exclude/NoMatch)

2. **CLI Integration**: [src/main.rs](src/main.rs)
   - `--include` argument (repeatable)
   - `--exclude` argument (repeatable)
   - `--filter-from` argument (file path)

3. **Config Integration**: [src/config.rs](src/config.rs)
   - `include_patterns: Vec<String>`
   - `exclude_patterns: Vec<String>`
   - `filter_from: Option<PathBuf>`

4. **Directory Walking**: [src/core/directory.rs](src/core/directory.rs)
   - Filter evaluation during tree traversal
   - Early directory pruning with `walker.skip_current_dir()`
   - Filter application in mirror mode deletions

### Performance Optimizations

- **Pre-compiled Patterns**: Glob and regex patterns are compiled once during FilterList construction
- **Early Directory Pruning**: Entire directory trees are skipped without recursing when excluded
- **First-Match-Wins**: Evaluation stops at the first matching rule
- **Normalized Paths**: Paths are normalized once per evaluation for cross-platform consistency

### Cross-Platform Compatibility

- Path separators (`\` on Windows, `/` on Unix) are normalized to forward slashes
- Patterns work consistently across all platforms
- Filter files use Unix-style line endings but accept any format

## Testing

### Unit Tests

[src/core/filter.rs](src/core/filter.rs) includes comprehensive unit tests:
- Glob pattern matching (simple and recursive)
- Regex pattern matching
- Exact path matching
- Negation rules
- First-match-wins semantics
- Filter list construction
- Rule parsing from text
- Error handling for invalid patterns

### Integration Tests

[tests/filter_integration_test.rs](tests/filter_integration_test.rs) includes 8 integration tests:
- Basic exclude patterns
- Include patterns override excludes
- Regex patterns
- Nested directory filtering
- Filter loading from file
- Dry-run visibility
- Mirror mode with filters
- Exact path matching

All tests pass on Windows, macOS, and Linux.

## File Reference

### New Files Created

1. **[src/core/filter.rs](src/core/filter.rs)** (460 lines)
   - Core filter implementation with comprehensive tests

2. **[examples/filters/example.orbitfilter](examples/filters/example.orbitfilter)** (130 lines)
   - Example filter file with common patterns and documentation

3. **[tests/filter_integration_test.rs](tests/filter_integration_test.rs)** (300+ lines)
   - Integration tests covering all filter scenarios

4. **FILTER_SYSTEM.md** (this file)
   - Comprehensive documentation

### Modified Files

1. **[src/main.rs](src/main.rs)**
   - Added `--include`, `--exclude`, `--filter-from` CLI arguments
   - Pass filter config to copy operations

2. **[src/config.rs](src/config.rs)**
   - Added `include_patterns`, `exclude_patterns`, `filter_from` fields

3. **[src/core/mod.rs](src/core/mod.rs)**
   - Exported `filter` module

4. **[src/core/directory.rs](src/core/directory.rs)**
   - Integrated FilterList into directory walking
   - Apply filters during tree traversal
   - Respect filters in mirror mode deletions
   - Show filtered items in dry-run mode

5. **[Cargo.toml](Cargo.toml)**
   - Dependencies `glob` and `regex` were already present

## Backward Compatibility

The filter system is fully backward compatible:

- Existing `--exclude` patterns continue to work
- Old configurations without filters use the legacy exclude logic
- FilterList is only used when include patterns or filter files are specified
- All existing tests pass without modification

## Error Handling

The filter system provides helpful error messages for:

- **Invalid glob patterns**: Shows the pattern and specific glob error
- **Invalid regex**: Shows the pattern and regex compilation error
- **File read errors**: Shows the file path and I/O error
- **Invalid syntax**: Shows the line number and problematic text

Example error messages:

```text
Error: Invalid filter configuration: Invalid glob pattern '[invalid': Pattern syntax error near position 0

Error: Invalid filter configuration: Invalid regex pattern '(unclosed': regex parse error:
    (unclosed
    ^
    error: unclosed group

Error: Invalid filter configuration: Failed to read filter file 'missing.txt': No such file or directory

Error: Invalid filter configuration: Invalid filter rule syntax at line 5: 'invalid syntax' - Expected '+', '-', 'include', or 'exclude' prefix
```

## Future Enhancements

Potential improvements for future versions:

1. **Performance**: Benchmark and optimize for very large directory trees
2. **Filter Statistics**: Show how many files matched each filter rule
3. **Filter Testing Tool**: Command to test filters without copying
4. **Advanced Features**:
   - Size-based filtering (`--max-size`, `--min-size`)
   - Date-based filtering (`--newer-than`, `--older-than`)
   - Attribute-based filtering (hidden, read-only, etc.)
5. **Better Error Messages**: Suggest corrections for common pattern mistakes

## Conclusion

The inclusion/exclusion filter system provides powerful, flexible, and efficient file selection capabilities for Orbit transfers. With support for multiple pattern types, first-match-wins semantics, and cross-platform compatibility, it enables precise control over which files are processed during transfer operations.

For questions or issues, please refer to the test files for examples or file an issue on GitHub.
