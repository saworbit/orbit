use glob::Pattern as GlobPattern;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during filter operations
#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Invalid glob pattern '{pattern}': {source}")]
    InvalidGlob {
        pattern: String,
        source: glob::PatternError,
    },

    #[error("Invalid regex pattern '{pattern}': {source}")]
    InvalidRegex {
        pattern: String,
        source: regex::Error,
    },

    #[error("Failed to read filter file '{path}': {source}")]
    FileReadError {
        path: String,
        source: std::io::Error,
    },

    #[error("Invalid filter rule syntax at line {line}: '{text}' - {reason}")]
    InvalidSyntax {
        line: usize,
        text: String,
        reason: String,
    },
}

/// Type of pattern matching to use
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterType {
    /// Glob pattern matching (e.g., "*.txt", "src/**/*.rs")
    Glob(String),
    /// Regular expression matching (e.g., "^dir/.*\.log$")
    Regex(String),
    /// Exact path matching (relative or absolute)
    Path(String),
}

/// Action to take when a filter matches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    /// Include the matching file/directory
    Include,
    /// Exclude the matching file/directory
    Exclude,
}

impl Default for FilterAction {
    fn default() -> Self {
        FilterAction::Include
    }
}

/// A single filter rule with pattern and action
#[derive(Debug, Clone)]
pub struct FilterRule {
    /// The action to take when this rule matches
    action: FilterAction,
    /// The type of pattern matching
    filter_type: FilterType,
    /// Compiled glob pattern (cached)
    glob_pattern: Option<GlobPattern>,
    /// Compiled regex (cached)
    regex_pattern: Option<Regex>,
    /// Whether this is a negation rule (inverts the action)
    negated: bool,
}

impl FilterRule {
    /// Create a new filter rule
    pub fn new(action: FilterAction, filter_type: FilterType) -> Result<Self, FilterError> {
        let mut rule = FilterRule {
            action,
            filter_type: filter_type.clone(),
            glob_pattern: None,
            regex_pattern: None,
            negated: false,
        };

        // Pre-compile patterns for performance
        match &filter_type {
            FilterType::Glob(pattern) => {
                let compiled = GlobPattern::new(pattern).map_err(|e| FilterError::InvalidGlob {
                    pattern: pattern.clone(),
                    source: e,
                })?;
                rule.glob_pattern = Some(compiled);
            }
            FilterType::Regex(pattern) => {
                let compiled = Regex::new(pattern).map_err(|e| FilterError::InvalidRegex {
                    pattern: pattern.clone(),
                    source: e,
                })?;
                rule.regex_pattern = Some(compiled);
            }
            FilterType::Path(_) => {
                // No compilation needed for exact path matching
            }
        }

        Ok(rule)
    }

    /// Create a negated rule (inverts the action)
    pub fn negated(mut self) -> Self {
        self.negated = true;
        self
    }

    /// Check if this rule matches the given path
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // Normalize path separators to forward slashes for consistent matching across platforms
        let normalized_path = path_str.replace('\\', "/");

        let base_matches = match &self.filter_type {
            FilterType::Glob(_) => self
                .glob_pattern
                .as_ref()
                .map(|p| p.matches(&normalized_path))
                .unwrap_or(false),
            FilterType::Regex(_) => self
                .regex_pattern
                .as_ref()
                .map(|r| r.is_match(&normalized_path))
                .unwrap_or(false),
            FilterType::Path(exact) => {
                // Normalize both paths for comparison
                let normalized_exact = exact.replace('\\', "/");
                normalized_path == normalized_exact
            }
        };

        base_matches
    }

    /// Get the action for this rule (considering negation)
    pub fn action(&self) -> FilterAction {
        if self.negated {
            match self.action {
                FilterAction::Include => FilterAction::Exclude,
                FilterAction::Exclude => FilterAction::Include,
            }
        } else {
            self.action
        }
    }

    /// Get a string representation of this rule
    pub fn pattern_string(&self) -> String {
        match &self.filter_type {
            FilterType::Glob(p) | FilterType::Regex(p) | FilterType::Path(p) => p.clone(),
        }
    }

    /// Get the filter type
    pub fn filter_type(&self) -> &FilterType {
        &self.filter_type
    }
}

/// Decision result for path filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterDecision {
    /// Path should be included
    Include,
    /// Path should be excluded
    Exclude,
    /// No rule matched - use default behavior
    NoMatch,
}

/// A collection of filter rules with first-match-wins semantics
#[derive(Debug, Clone, Default)]
pub struct FilterList {
    rules: Vec<FilterRule>,
    /// Default action when no rules match
    default_action: FilterAction,
}

impl FilterList {
    /// Create a new empty filter list
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_action: FilterAction::Include, // Include by default
        }
    }

    /// Create a filter list with a specific default action
    pub fn with_default(default_action: FilterAction) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
        }
    }

    /// Add a rule to the filter list
    pub fn add_rule(&mut self, rule: FilterRule) {
        self.rules.push(rule);
    }

    /// Add an include glob pattern
    pub fn include_glob(&mut self, pattern: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(FilterAction::Include, FilterType::Glob(pattern.to_string()))?;
        self.add_rule(rule);
        Ok(())
    }

    /// Add an exclude glob pattern
    pub fn exclude_glob(&mut self, pattern: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(FilterAction::Exclude, FilterType::Glob(pattern.to_string()))?;
        self.add_rule(rule);
        Ok(())
    }

    /// Add an include regex pattern
    pub fn include_regex(&mut self, pattern: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(
            FilterAction::Include,
            FilterType::Regex(pattern.to_string()),
        )?;
        self.add_rule(rule);
        Ok(())
    }

    /// Add an exclude regex pattern
    pub fn exclude_regex(&mut self, pattern: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(
            FilterAction::Exclude,
            FilterType::Regex(pattern.to_string()),
        )?;
        self.add_rule(rule);
        Ok(())
    }

    /// Add an include path
    pub fn include_path(&mut self, path: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(FilterAction::Include, FilterType::Path(path.to_string()))?;
        self.add_rule(rule);
        Ok(())
    }

    /// Add an exclude path
    pub fn exclude_path(&mut self, path: &str) -> Result<(), FilterError> {
        let rule = FilterRule::new(FilterAction::Exclude, FilterType::Path(path.to_string()))?;
        self.add_rule(rule);
        Ok(())
    }

    /// Evaluate the filter list against a path using first-match-wins semantics
    pub fn evaluate(&self, path: &Path) -> FilterDecision {
        for rule in &self.rules {
            if rule.matches(path) {
                return match rule.action() {
                    FilterAction::Include => FilterDecision::Include,
                    FilterAction::Exclude => FilterDecision::Exclude,
                };
            }
        }

        // No rules matched - use default
        FilterDecision::NoMatch
    }

    /// Check if a path should be included (considering default action)
    pub fn should_include(&self, path: &Path) -> bool {
        match self.evaluate(path) {
            FilterDecision::Include => true,
            FilterDecision::Exclude => false,
            FilterDecision::NoMatch => self.default_action == FilterAction::Include,
        }
    }

    /// Check if a path should be excluded (considering default action)
    pub fn should_exclude(&self, path: &Path) -> bool {
        !self.should_include(path)
    }

    /// Load filter rules from a file
    ///
    /// File format supports:
    /// - `+ pattern` or `include pattern` - include rule
    /// - `- pattern` or `exclude pattern` - exclude rule
    /// - `! pattern` - negation prefix
    /// - `glob: pattern` - explicit glob
    /// - `regex: pattern` - explicit regex
    /// - `path: pattern` - explicit path
    /// - `# comment` or empty lines - ignored
    ///
    /// Examples:
    /// ```text
    /// # Include all Rust files
    /// + **/*.rs
    ///
    /// # Exclude target directory
    /// - target/**
    ///
    /// # Include specific file
    /// include Cargo.toml
    ///
    /// # Exclude using regex
    /// - regex: ^build/.*\.o$
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), FilterError> {
        let path_ref = path.as_ref();
        let file = File::open(path_ref).map_err(|e| FilterError::FileReadError {
            path: path_ref.to_string_lossy().to_string(),
            source: e,
        })?;

        let reader = BufReader::new(file);

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result.map_err(|e| FilterError::FileReadError {
                path: path_ref.to_string_lossy().to_string(),
                source: e,
            })?;

            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse the line into a rule
            let rule = self.parse_rule_line(line, line_num + 1)?;
            self.add_rule(rule);
        }

        Ok(())
    }

    /// Parse a single filter rule line
    fn parse_rule_line(&self, line: &str, line_num: usize) -> Result<FilterRule, FilterError> {
        let line = line.trim();

        // Check for negation prefix
        let (negated, line) = if line.starts_with('!') {
            (true, line[1..].trim())
        } else {
            (false, line)
        };

        // Parse action prefix
        let (action, pattern) = if line.starts_with("+ ") {
            (FilterAction::Include, &line[2..])
        } else if line.starts_with("- ") {
            (FilterAction::Exclude, &line[2..])
        } else if let Some(rest) = line.strip_prefix("include ") {
            (FilterAction::Include, rest)
        } else if let Some(rest) = line.strip_prefix("exclude ") {
            (FilterAction::Exclude, rest)
        } else {
            return Err(FilterError::InvalidSyntax {
                line: line_num,
                text: line.to_string(),
                reason: "Expected '+', '-', 'include', or 'exclude' prefix".to_string(),
            });
        };

        let pattern = pattern.trim();

        // Parse filter type prefix
        let filter_type = if let Some(rest) = pattern.strip_prefix("glob:") {
            FilterType::Glob(rest.trim().to_string())
        } else if let Some(rest) = pattern.strip_prefix("regex:") {
            FilterType::Regex(rest.trim().to_string())
        } else if let Some(rest) = pattern.strip_prefix("path:") {
            FilterType::Path(rest.trim().to_string())
        } else {
            // Default to glob if no prefix specified
            FilterType::Glob(pattern.to_string())
        };

        let mut rule = FilterRule::new(action, filter_type)?;
        if negated {
            rule = rule.negated();
        }

        Ok(rule)
    }

    /// Get the number of rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the filter list is empty
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Get an iterator over the rules
    pub fn rules(&self) -> impl Iterator<Item = &FilterRule> {
        self.rules.iter()
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Build a filter list from configuration
    ///
    /// Processes include/exclude patterns and loads from file if specified.
    /// Pattern format supports:
    /// - Plain glob: "*.txt", "target/**"
    /// - Explicit glob: "glob:*.rs"
    /// - Regex: "regex:^src/.*"
    /// - Path: "path:src/main.rs"
    ///
    /// Note: Include patterns are added first to give them higher priority
    /// with first-match-wins semantics.
    pub fn from_config(
        include_patterns: &[String],
        exclude_patterns: &[String],
        filter_from: Option<&Path>,
    ) -> Result<Self, FilterError> {
        let mut filter_list = FilterList::new();

        // Add include patterns first (higher priority due to first-match-wins)
        for pattern in include_patterns {
            add_pattern_to_filter(&mut filter_list, pattern, FilterAction::Include)?;
        }

        // Add exclude patterns (lower priority - checked after includes)
        for pattern in exclude_patterns {
            add_pattern_to_filter(&mut filter_list, pattern, FilterAction::Exclude)?;
        }

        // Load from file if specified (appended, so earlier rules take precedence)
        if let Some(path) = filter_from {
            filter_list.load_from_file(path)?;
        }

        Ok(filter_list)
    }
}

/// Helper function to parse and add a pattern to filter list
fn add_pattern_to_filter(
    filter_list: &mut FilterList,
    pattern: &str,
    action: FilterAction,
) -> Result<(), FilterError> {
    let pattern = pattern.trim();

    // Parse filter type prefix
    let filter_type = if let Some(rest) = pattern.strip_prefix("glob:") {
        FilterType::Glob(rest.trim().to_string())
    } else if let Some(rest) = pattern.strip_prefix("regex:") {
        FilterType::Regex(rest.trim().to_string())
    } else if let Some(rest) = pattern.strip_prefix("path:") {
        FilterType::Path(rest.trim().to_string())
    } else {
        // Default to glob if no prefix specified
        FilterType::Glob(pattern.to_string())
    };

    let rule = FilterRule::new(action, filter_type)?;
    filter_list.add_rule(rule);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_matching() {
        let rule =
            FilterRule::new(FilterAction::Include, FilterType::Glob("*.txt".to_string())).unwrap();

        assert!(rule.matches(Path::new("file.txt")));
        assert!(rule.matches(Path::new("test.txt")));
        assert!(!rule.matches(Path::new("file.rs")));
    }

    #[test]
    fn test_glob_recursive() {
        let rule = FilterRule::new(
            FilterAction::Exclude,
            FilterType::Glob("target/**".to_string()),
        )
        .unwrap();

        assert!(rule.matches(Path::new("target/debug")));
        assert!(rule.matches(Path::new("target/release/build")));
        assert!(!rule.matches(Path::new("src/main.rs")));
    }

    #[test]
    fn test_regex_matching() {
        let rule = FilterRule::new(
            FilterAction::Include,
            FilterType::Regex(r"^src/.*\.rs$".to_string()),
        )
        .unwrap();

        assert!(rule.matches(Path::new("src/main.rs")));
        assert!(rule.matches(Path::new("src/lib.rs")));
        assert!(!rule.matches(Path::new("tests/test.rs")));
    }

    #[test]
    fn test_path_matching() {
        let rule = FilterRule::new(
            FilterAction::Include,
            FilterType::Path("src/main.rs".to_string()),
        )
        .unwrap();

        assert!(rule.matches(Path::new("src/main.rs")));
        assert!(!rule.matches(Path::new("src/lib.rs")));
    }

    #[test]
    fn test_negation() {
        let rule = FilterRule::new(FilterAction::Include, FilterType::Glob("*.txt".to_string()))
            .unwrap()
            .negated();

        assert!(rule.matches(Path::new("file.txt")));
        assert_eq!(rule.action(), FilterAction::Exclude);
    }

    #[test]
    fn test_first_match_wins() {
        let mut filter = FilterList::new();
        filter.include_glob("*.rs").unwrap();
        filter.exclude_glob("src/**").unwrap();

        // First rule matches, so included even though second rule would exclude
        assert_eq!(
            filter.evaluate(Path::new("src/main.rs")),
            FilterDecision::Include
        );
    }

    #[test]
    fn test_exclude_then_include() {
        let mut filter = FilterList::new();
        filter.exclude_glob("target/**").unwrap();
        filter.include_glob("*.rs").unwrap();

        // First rule matches and excludes
        assert_eq!(
            filter.evaluate(Path::new("target/debug")),
            FilterDecision::Exclude
        );

        // Second rule matches and includes
        assert_eq!(
            filter.evaluate(Path::new("main.rs")),
            FilterDecision::Include
        );

        // No match
        assert_eq!(
            filter.evaluate(Path::new("README.md")),
            FilterDecision::NoMatch
        );
    }

    #[test]
    fn test_rsync_like_patterns() {
        // Typical rsync pattern: exclude target, but include specific subdirs
        let mut filter = FilterList::new();
        filter.exclude_glob("*.tmp").unwrap();
        filter.exclude_glob("*.log").unwrap();
        filter.include_glob("**/*.rs").unwrap();
        filter.exclude_glob("target/**").unwrap();

        assert_eq!(
            filter.evaluate(Path::new("test.tmp")),
            FilterDecision::Exclude
        );
        assert_eq!(
            filter.evaluate(Path::new("debug.log")),
            FilterDecision::Exclude
        );
        assert_eq!(
            filter.evaluate(Path::new("src/main.rs")),
            FilterDecision::Include
        );
        assert_eq!(
            filter.evaluate(Path::new("target/debug/build")),
            FilterDecision::Exclude
        );
    }

    #[test]
    fn test_invalid_glob() {
        let result = FilterRule::new(
            FilterAction::Include,
            FilterType::Glob("[invalid".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_regex() {
        let result = FilterRule::new(
            FilterAction::Include,
            FilterType::Regex("(invalid".to_string()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_list_should_include() {
        let mut filter = FilterList::new();
        filter.include_glob("*.rs").unwrap();
        filter.exclude_glob("*_test.rs").unwrap();

        assert!(filter.should_include(Path::new("main.rs")));
        assert!(filter.should_include(Path::new("lib.rs")));
        // No match - default is include
        assert!(filter.should_include(Path::new("README.md")));
    }

    #[test]
    fn test_filter_list_with_default_exclude() {
        let mut filter = FilterList::with_default(FilterAction::Exclude);
        filter.include_glob("*.rs").unwrap();

        assert!(filter.should_include(Path::new("main.rs")));
        // No match - default is exclude
        assert!(!filter.should_include(Path::new("README.md")));
    }

    #[test]
    fn test_parse_rule_line() {
        let filter = FilterList::new();

        // Test include with + prefix
        let rule = filter.parse_rule_line("+ *.txt", 1).unwrap();
        assert_eq!(rule.action(), FilterAction::Include);
        assert!(matches!(rule.filter_type(), FilterType::Glob(_)));

        // Test exclude with - prefix
        let rule = filter.parse_rule_line("- target/**", 1).unwrap();
        assert_eq!(rule.action(), FilterAction::Exclude);

        // Test include keyword
        let rule = filter.parse_rule_line("include *.rs", 1).unwrap();
        assert_eq!(rule.action(), FilterAction::Include);

        // Test exclude keyword
        let rule = filter.parse_rule_line("exclude build/", 1).unwrap();
        assert_eq!(rule.action(), FilterAction::Exclude);

        // Test explicit regex
        let rule = filter.parse_rule_line("+ regex: ^src/.*", 1).unwrap();
        assert!(matches!(rule.filter_type(), FilterType::Regex(_)));

        // Test negation
        let rule = filter.parse_rule_line("! + *.log", 1).unwrap();
        assert_eq!(rule.action(), FilterAction::Exclude); // Negated include = exclude
    }
}
