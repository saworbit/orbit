/*!
 * Metadata and path transformation framework
 *
 * Provides transformations for:
 * - Path renaming using regex patterns
 * - Filename encoding normalization
 * - Metadata filtering and adaptation
 * - Compression state tracking
 */

use crate::core::file_metadata::FileMetadata;
use crate::error::{OrbitError, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Transformation configuration
#[derive(Debug, Clone, Default)]
pub struct TransformConfig {
    /// Path transformation rules
    pub path_transforms: Vec<PathTransform>,

    /// Filename encoding normalization
    pub normalize_encoding: bool,

    /// Metadata filtering rules
    pub metadata_filters: Vec<MetadataFilter>,

    /// Case conversion for filenames
    pub case_transform: CaseTransform,
}

/// Path transformation using regex
#[derive(Debug, Clone)]
pub struct PathTransform {
    /// Regex pattern to match
    pub pattern: Regex,

    /// Replacement pattern (supports capture groups like $1, $2)
    pub replacement: String,

    /// Apply to basename only (false = apply to full path)
    pub basename_only: bool,
}

impl PathTransform {
    /// Create a new path transform from pattern strings
    pub fn new(pattern: &str, replacement: &str, basename_only: bool) -> Result<Self> {
        let regex = Regex::new(pattern)
            .map_err(|e| OrbitError::MetadataFailed(format!("Invalid regex pattern: {}", e)))?;

        Ok(Self {
            pattern: regex,
            replacement: replacement.to_string(),
            basename_only,
        })
    }

    /// Apply this transformation to a path
    pub fn apply(&self, path: &Path) -> PathBuf {
        if self.basename_only {
            // Transform only the filename
            if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy();
                let transformed = self.pattern.replace_all(&filename_str, &self.replacement);

                if let Some(parent) = path.parent() {
                    parent.join(transformed.as_ref())
                } else {
                    PathBuf::from(transformed.as_ref())
                }
            } else {
                path.to_path_buf()
            }
        } else {
            // Transform the full path
            let path_str = path.to_string_lossy();
            let transformed = self.pattern.replace_all(&path_str, &self.replacement);
            PathBuf::from(transformed.as_ref())
        }
    }
}

/// Case transformation for filenames
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseTransform {
    #[default]
    None,
    Lowercase,
    Uppercase,
    TitleCase,
}

impl CaseTransform {
    /// Apply case transformation to a filename
    pub fn apply(&self, filename: &str) -> String {
        match self {
            Self::None => filename.to_string(),
            Self::Lowercase => filename.to_lowercase(),
            Self::Uppercase => filename.to_uppercase(),
            Self::TitleCase => {
                // Simple title case: capitalize first letter of each word
                filename
                    .split_whitespace()
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().chain(chars).collect(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            }
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "lower" | "lowercase" => Ok(Self::Lowercase),
            "upper" | "uppercase" => Ok(Self::Uppercase),
            "title" | "titlecase" => Ok(Self::TitleCase),
            _ => Err(OrbitError::MetadataFailed(format!(
                "Unknown case transform: {}",
                s
            ))),
        }
    }
}

/// Metadata filtering rules
#[derive(Debug, Clone)]
pub enum MetadataFilter {
    /// Strip all extended attributes
    StripXattrs,

    /// Strip specific xattr by name pattern
    StripXattrPattern(Regex),

    /// Strip ownership information (UID/GID)
    StripOwnership,

    /// Strip permissions (set to default)
    StripPermissions,

    /// Normalize timestamps to a specific value
    NormalizeTimestamps,
}

impl MetadataFilter {
    /// Apply this filter to metadata
    pub fn apply(&self, metadata: &mut FileMetadata) {
        match self {
            Self::StripXattrs => {
                metadata.xattrs = None;
            }
            Self::StripXattrPattern(pattern) => {
                if let Some(xattrs) = &mut metadata.xattrs {
                    xattrs.retain(|name, _| !pattern.is_match(name));
                    if xattrs.is_empty() {
                        metadata.xattrs = None;
                    }
                }
            }
            Self::StripOwnership => {
                metadata.owner_uid = None;
                metadata.owner_gid = None;
            }
            Self::StripPermissions => {
                metadata.permissions = None;
            }
            Self::NormalizeTimestamps => {
                // Set all timestamps to epoch for reproducible builds
                let epoch = std::time::UNIX_EPOCH;
                metadata.modified = Some(epoch);
                metadata.accessed = Some(epoch);
                metadata.created = Some(epoch);
            }
        }
    }
}

/// Parse transformation configuration from CLI string
/// Format: "rename:pattern=replacement,case:lower,strip:xattrs"
pub fn parse_transform_string(s: &str) -> Result<TransformConfig> {
    let mut config = TransformConfig::default();

    for part in s.split(',') {
        let part = part.trim();

        if let Some((transform_type, value)) = part.split_once(':') {
            match transform_type.trim() {
                "rename" => {
                    // Format: rename:s/old/new/ or rename:pattern=replacement
                    let transform = parse_rename_transform(value)?;
                    config.path_transforms.push(transform);
                }
                "case" => {
                    config.case_transform = CaseTransform::from_str(value.trim())?;
                }
                "strip" => match value.trim() {
                    "xattrs" => config.metadata_filters.push(MetadataFilter::StripXattrs),
                    "ownership" => config.metadata_filters.push(MetadataFilter::StripOwnership),
                    "permissions" => config
                        .metadata_filters
                        .push(MetadataFilter::StripPermissions),
                    _ => {
                        return Err(OrbitError::MetadataFailed(format!(
                            "Unknown strip target: {}",
                            value
                        )))
                    }
                },
                "normalize" => {
                    if value.trim() == "timestamps" {
                        config
                            .metadata_filters
                            .push(MetadataFilter::NormalizeTimestamps);
                    }
                }
                "encoding" => {
                    if value.trim() == "normalize" {
                        config.normalize_encoding = true;
                    }
                }
                _ => {
                    return Err(OrbitError::MetadataFailed(format!(
                        "Unknown transform type: {}",
                        transform_type
                    )))
                }
            }
        }
    }

    Ok(config)
}

/// Parse rename transformation from various formats
fn parse_rename_transform(s: &str) -> Result<PathTransform> {
    // Support sed-like syntax: s/pattern/replacement/ or s/pattern/replacement/g
    if s.starts_with("s/") {
        let parts: Vec<&str> = s[2..].split('/').collect();
        if parts.len() >= 2 {
            return PathTransform::new(parts[0], parts[1], false);
        }
    }

    // Support pattern=replacement syntax
    if let Some((pattern, replacement)) = s.split_once('=') {
        return PathTransform::new(pattern.trim(), replacement.trim(), false);
    }

    Err(OrbitError::MetadataFailed(format!(
        "Invalid rename pattern: {}",
        s
    )))
}

/// Apply all transformations to a path
pub fn transform_path(path: &Path, config: &TransformConfig) -> PathBuf {
    let mut result = path.to_path_buf();

    // Apply path transforms
    for transform in &config.path_transforms {
        result = transform.apply(&result);
    }

    // Apply case transformation to filename
    if config.case_transform != CaseTransform::None {
        if let Some(filename) = result.file_name() {
            let filename_str = filename.to_string_lossy();
            let transformed = config.case_transform.apply(&filename_str);

            if let Some(parent) = result.parent() {
                result = parent.join(transformed);
            } else {
                result = PathBuf::from(transformed);
            }
        }
    }

    // Normalize encoding if requested
    if config.normalize_encoding {
        result = normalize_path_encoding(&result);
    }

    result
}

/// Apply all transformations to metadata
pub fn transform_metadata(metadata: &mut FileMetadata, config: &TransformConfig) {
    for filter in &config.metadata_filters {
        filter.apply(metadata);
    }
}

/// Normalize path encoding (UTF-8 NFC normalization)
fn normalize_path_encoding(path: &Path) -> PathBuf {
    // Use Unicode NFC normalization for consistent representation
    // This ensures that é (single character) and é (e + combining accent) are the same
    #[cfg(feature = "unicode-normalization")]
    {
        use unicode_normalization::UnicodeNormalization;
        let path_str = path.to_string_lossy();
        let normalized: String = path_str.nfc().collect();
        PathBuf::from(normalized)
    }

    #[cfg(not(feature = "unicode-normalization"))]
    {
        // Without normalization feature, just return as-is
        path.to_path_buf()
    }
}

/// Builder for transformation configuration
pub struct TransformBuilder {
    config: TransformConfig,
}

impl TransformBuilder {
    pub fn new() -> Self {
        Self {
            config: TransformConfig::default(),
        }
    }

    /// Add a path transformation
    pub fn add_rename(
        mut self,
        pattern: &str,
        replacement: &str,
        basename_only: bool,
    ) -> Result<Self> {
        let transform = PathTransform::new(pattern, replacement, basename_only)?;
        self.config.path_transforms.push(transform);
        Ok(self)
    }

    /// Set case transformation
    pub fn case_transform(mut self, transform: CaseTransform) -> Self {
        self.config.case_transform = transform;
        self
    }

    /// Add metadata filter
    pub fn add_filter(mut self, filter: MetadataFilter) -> Self {
        self.config.metadata_filters.push(filter);
        self
    }

    /// Enable encoding normalization
    pub fn normalize_encoding(mut self, enable: bool) -> Self {
        self.config.normalize_encoding = enable;
        self
    }

    /// Build the configuration
    pub fn build(self) -> TransformConfig {
        self.config
    }
}

impl Default for TransformBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_path_transform_basename() {
        let transform = PathTransform::new(r"\.txt$", ".md", true).unwrap();
        let path = Path::new("/path/to/file.txt");
        let result = transform.apply(path);
        assert_eq!(result, PathBuf::from("/path/to/file.md"));
    }

    #[test]
    fn test_path_transform_full_path() {
        let transform = PathTransform::new(r"/old/", "/new/", false).unwrap();
        let path = Path::new("/old/path/file.txt");
        let result = transform.apply(path);
        assert_eq!(result, PathBuf::from("/new/path/file.txt"));
    }

    #[test]
    fn test_case_transform() {
        assert_eq!(CaseTransform::Lowercase.apply("FILE.TXT"), "file.txt");
        assert_eq!(CaseTransform::Uppercase.apply("file.txt"), "FILE.TXT");
        assert_eq!(CaseTransform::None.apply("MiXeD.txt"), "MiXeD.txt");
    }

    #[test]
    fn test_metadata_filter_strip_xattrs() {
        let mut metadata = FileMetadata::default();
        metadata.xattrs = Some(HashMap::new());

        MetadataFilter::StripXattrs.apply(&mut metadata);
        assert!(metadata.xattrs.is_none());
    }

    #[test]
    fn test_metadata_filter_strip_ownership() {
        let mut metadata = FileMetadata::default();
        metadata.owner_uid = Some(1000);
        metadata.owner_gid = Some(1000);

        MetadataFilter::StripOwnership.apply(&mut metadata);
        assert!(metadata.owner_uid.is_none());
        assert!(metadata.owner_gid.is_none());
    }

    #[test]
    fn test_parse_transform_string() {
        let config = parse_transform_string("rename:s/old/new/,case:lower,strip:xattrs").unwrap();

        assert_eq!(config.path_transforms.len(), 1);
        assert_eq!(config.case_transform, CaseTransform::Lowercase);
        assert_eq!(config.metadata_filters.len(), 1);
        assert!(matches!(
            config.metadata_filters[0],
            MetadataFilter::StripXattrs
        ));
    }

    #[test]
    fn test_transform_builder() {
        let config = TransformBuilder::new()
            .add_rename(r"\.txt$", ".md", true)
            .unwrap()
            .case_transform(CaseTransform::Lowercase)
            .add_filter(MetadataFilter::StripXattrs)
            .build();

        assert_eq!(config.path_transforms.len(), 1);
        assert_eq!(config.case_transform, CaseTransform::Lowercase);
        assert_eq!(config.metadata_filters.len(), 1);
    }

    #[test]
    fn test_complete_transformation() {
        let config = TransformBuilder::new()
            .add_rename(r"\.txt$", ".md", true)
            .unwrap()
            .case_transform(CaseTransform::Lowercase)
            .build();

        let path = Path::new("/path/to/FILE.txt");
        let result = transform_path(path, &config);
        assert_eq!(result, PathBuf::from("/path/to/file.md"));
    }
}
