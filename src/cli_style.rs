/*!
 * Orbit CLI Style System
 *
 * Unified styling utilities for consistent, beautiful CLI output.
 * Provides tables, boxes, progress indicators, and themed text formatting.
 */

use crate::core::guidance::Notice;
use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
use console::{style, StyledObject};

// ============================================================================
// THEME COLORS
// ============================================================================

/// Brand colors for consistent styling
pub struct Theme;

impl Theme {
    /// Primary accent color (cyan/blue)
    pub fn primary<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).cyan()
    }

    /// Success color (green)
    pub fn success<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).green()
    }

    /// Warning color (yellow)
    pub fn warning<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).yellow()
    }

    /// Error color (red)
    pub fn error<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).red()
    }

    /// Muted/secondary text (dim)
    pub fn muted<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).dim()
    }

    /// Bold text
    pub fn bold<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).bold()
    }

    /// Header style (bold cyan)
    pub fn header<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).cyan().bold()
    }

    /// Value/number highlight (bold white)
    pub fn value<D: std::fmt::Display>(text: D) -> StyledObject<D> {
        style(text).white().bold()
    }
}

// ============================================================================
// ICONS
// ============================================================================

/// Unicode icons for visual feedback
pub struct Icons;

impl Icons {
    // Status icons
    pub const SUCCESS: &'static str = "✓";
    pub const ERROR: &'static str = "✗";
    pub const WARNING: &'static str = "⚠";
    pub const INFO: &'static str = "ℹ";
    pub const PENDING: &'static str = "○";
    pub const RUNNING: &'static str = "◐";

    // Feature icons
    pub const ORBIT: &'static str = "🪐";
    pub const ROCKET: &'static str = "🚀";
    pub const LIGHTNING: &'static str = "⚡";
    pub const SHIELD: &'static str = "🛡";
    pub const GLOBE: &'static str = "🌐";
    pub const FOLDER: &'static str = "📁";
    pub const FILE: &'static str = "📄";
    pub const MANIFEST: &'static str = "📋";
    pub const STATS: &'static str = "📊";
    pub const GEAR: &'static str = "⚙";
    pub const LOCK: &'static str = "🔒";
    pub const CLOCK: &'static str = "⏱";
    pub const SATELLITE: &'static str = "🛰";
    pub const WRENCH: &'static str = "🔧";
    pub const SPARKLE: &'static str = "✨";

    // Arrow indicators
    pub const ARROW_RIGHT: &'static str = "→";
    pub const ARROW_DOWN: &'static str = "↓";
    pub const BULLET: &'static str = "•";
}

// ============================================================================
// BOX DRAWING
// ============================================================================

/// Draw a styled header box
pub fn header_box(title: &str, subtitle: Option<&str>) {
    let width = 56;
    let top = format!("╔{}╗", "═".repeat(width));
    let bottom = format!("╚{}╝", "═".repeat(width));

    println!("{}", Theme::primary(&top));

    // Center the title
    let title_display = format!("{} {}", Icons::ORBIT, title);
    let padding = (width - title_display.chars().count()) / 2;
    println!(
        "{}{}{}{}",
        Theme::primary("║"),
        " ".repeat(padding),
        Theme::header(&title_display),
        " ".repeat(width - padding - title_display.chars().count())
    );

    if let Some(sub) = subtitle {
        let sub_padding = (width - sub.len()) / 2;
        println!(
            "{}{}{}{}{}",
            Theme::primary("║"),
            " ".repeat(sub_padding),
            Theme::muted(sub),
            " ".repeat(width - sub_padding - sub.len()),
            Theme::primary("║")
        );
    }

    println!("{}", Theme::primary(&bottom));
}

/// Draw a section header with a line
pub fn section_header(title: &str) {
    let line_len = 50 - title.len().min(40);
    println!(
        "\n{} {}",
        Theme::header(title),
        Theme::muted("─".repeat(line_len))
    );
}

/// Draw an info box with content
pub fn info_box(title: &str, lines: &[&str]) {
    let max_len = lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(40)
        .max(title.len() + 4);
    let width = max_len + 4;

    println!(
        "┌── {} {}┐",
        Theme::header(title),
        "─".repeat(width.saturating_sub(title.len() + 6))
    );

    for line in lines {
        println!("│ {:<width$} │", line, width = width - 2);
    }

    println!("└{}┘", "─".repeat(width));
}

/// Draw a guidance notice box (for the Guidance System)
pub fn guidance_box(notices: &[Notice]) {
    if notices.is_empty() {
        return;
    }

    let rendered: Vec<String> = notices.iter().map(|notice| notice.to_string()).collect();
    let max_len = rendered
        .iter()
        .map(|n| strip_ansi(n).len())
        .max()
        .unwrap_or(40);
    let width = max_len.max(45) + 2;

    println!(
        "??? {} Orbit Guidance System {}?",
        Icons::SATELLITE,
        "?".repeat(width.saturating_sub(28))
    );

    for notice in &rendered {
        let notice_len = strip_ansi(notice).len();
        let padding = width.saturating_sub(notice_len + 1);
        println!("? {}{} ?", notice, " ".repeat(padding));
    }

    println!("?{}?", "?".repeat(width + 2));
    println!();
}

// ============================================================================
// TABLES
// ============================================================================

/// Create a styled data table
pub fn create_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Create a minimal table (no outer borders)
pub fn create_minimal_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(presets::UTF8_NO_BORDERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Create a key-value table for stats
pub fn stats_table(items: &[(&str, String)]) -> Table {
    let mut table = create_minimal_table();

    for (key, value) in items {
        table.add_row(vec![
            Cell::new(key).fg(Color::Cyan),
            Cell::new(value)
                .fg(Color::White)
                .add_attribute(Attribute::Bold),
        ]);
    }

    table
}

/// Create a feature capability table
pub fn capability_table(items: &[(&str, bool, &str)]) -> Table {
    let mut table = create_table();
    table.set_header(vec![
        Cell::new("Feature")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Status")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Details")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
    ]);

    for (feature, available, details) in items {
        let status = if *available {
            Cell::new(format!("{} Available", Icons::SUCCESS)).fg(Color::Green)
        } else {
            Cell::new(format!("{} Not Available", Icons::ERROR)).fg(Color::Red)
        };

        table.add_row(vec![
            Cell::new(feature),
            status,
            Cell::new(details).fg(Color::DarkGrey),
        ]);
    }

    table
}

/// Create a preset comparison table
pub fn preset_table(presets: &[PresetInfo]) -> Table {
    let mut table = create_table();
    table.set_header(vec![
        Cell::new("Preset")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Checksum")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Resume")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Compression")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Zero-Copy")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new("Best For")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
    ]);

    for preset in presets {
        table.add_row(vec![
            Cell::new(format!("{} {}", preset.icon, preset.name))
                .fg(Color::White)
                .add_attribute(Attribute::Bold),
            bool_cell(preset.checksum),
            bool_cell(preset.resume),
            Cell::new(preset.compression.as_str()),
            bool_cell(preset.zero_copy),
            Cell::new(preset.best_for.as_str()).fg(Color::DarkGrey),
        ]);
    }

    table
}

/// Create a transfer summary table
pub fn transfer_summary_table(stats: &TransferSummary) -> Table {
    let mut table = create_table();
    table.set_header(vec![
        Cell::new("Transfer Summary")
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
        Cell::new(""),
    ]);

    table.add_row(vec![
        Cell::new("Files Copied"),
        Cell::new(stats.files_copied.to_string())
            .fg(Color::Green)
            .add_attribute(Attribute::Bold),
    ]);

    if stats.files_skipped > 0 {
        table.add_row(vec![
            Cell::new("Files Skipped"),
            Cell::new(stats.files_skipped.to_string()).fg(Color::Yellow),
        ]);
    }

    if stats.files_failed > 0 {
        table.add_row(vec![
            Cell::new("Files Failed"),
            Cell::new(stats.files_failed.to_string())
                .fg(Color::Red)
                .add_attribute(Attribute::Bold),
        ]);
    }

    table.add_row(vec![
        Cell::new("Total Size"),
        Cell::new(stats.total_size.as_str())
            .fg(Color::White)
            .add_attribute(Attribute::Bold),
    ]);

    table.add_row(vec![
        Cell::new("Duration"),
        Cell::new(stats.duration.as_str()).fg(Color::White),
    ]);

    table.add_row(vec![
        Cell::new("Speed"),
        Cell::new(stats.speed.as_str())
            .fg(Color::Cyan)
            .add_attribute(Attribute::Bold),
    ]);

    if let Some(ref checksum) = stats.checksum {
        table.add_row(vec![
            Cell::new("Checksum"),
            Cell::new(checksum).fg(Color::DarkGrey),
        ]);
    }

    if let Some(ref ratio) = stats.compression_ratio {
        table.add_row(vec![
            Cell::new("Compression"),
            Cell::new(ratio).fg(Color::Magenta),
        ]);
    }

    table
}

// ============================================================================
// HELPER STRUCTURES
// ============================================================================

/// Preset configuration info
pub struct PresetInfo {
    pub icon: &'static str,
    pub name: &'static str,
    pub checksum: bool,
    pub resume: bool,
    pub compression: String,
    pub zero_copy: bool,
    pub best_for: String,
}

/// Transfer summary data
pub struct TransferSummary {
    pub files_copied: u64,
    pub files_skipped: u64,
    pub files_failed: u64,
    pub total_size: String,
    pub duration: String,
    pub speed: String,
    pub checksum: Option<String>,
    pub compression_ratio: Option<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a boolean status cell
fn bool_cell(value: bool) -> Cell {
    if value {
        Cell::new(format!("{} Yes", Icons::SUCCESS)).fg(Color::Green)
    } else {
        Cell::new(format!("{} No", Icons::ERROR)).fg(Color::DarkGrey)
    }
}

/// Strip ANSI escape codes from a string (for length calculation)
fn strip_ansi(s: &str) -> String {
    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap());
    RE.replace_all(s, "").to_string()
}

/// Format bytes into human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let base = 1024.0_f64;
    let exp = (bytes_f.ln() / base.ln()).floor() as usize;
    let exp = exp.min(UNITS.len() - 1);

    let value = bytes_f / base.powi(exp as i32);

    if exp == 0 {
        format!("{} {}", bytes, UNITS[exp])
    } else {
        format!("{:.2} {}", value, UNITS[exp])
    }
}

/// Format duration into human-readable string
pub fn format_duration(secs: f64) -> String {
    if secs < 1.0 {
        format!("{:.0}ms", secs * 1000.0)
    } else if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        let mins = (secs / 60.0).floor();
        let remaining = secs % 60.0;
        format!("{}m {:.0}s", mins, remaining)
    } else {
        let hours = (secs / 3600.0).floor();
        let mins = ((secs % 3600.0) / 60.0).floor();
        format!("{}h {}m", hours, mins)
    }
}

/// Print a styled error message with optional suggestion
pub fn print_error(message: &str, suggestion: Option<&str>) {
    eprintln!(
        "\n{} {}",
        Theme::error(format!("{} Error:", Icons::ERROR)),
        message
    );

    if let Some(hint) = suggestion {
        eprintln!(
            "  {} {}",
            Theme::muted(Icons::ARROW_RIGHT),
            Theme::muted(hint)
        );
    }
    eprintln!();
}

/// Print a styled warning message
pub fn print_warning(message: &str) {
    eprintln!(
        "{} {}",
        Theme::warning(Icons::WARNING.to_string()),
        Theme::warning(message)
    );
}

/// Print a styled success message
pub fn print_success(message: &str) {
    println!(
        "{} {}",
        Theme::success(Icons::SUCCESS.to_string()),
        Theme::success(message)
    );
}

/// Print a styled info message
pub fn print_info(message: &str) {
    println!("{} {}", Theme::primary(Icons::INFO.to_string()), message);
}

// ============================================================================
// BANNER
// ============================================================================

/// Print the Orbit welcome banner
pub fn print_banner() {
    let version = env!("CARGO_PKG_VERSION");

    println!();
    println!(
        "{}",
        Theme::primary("  ╭─────────────────────────────────────────────────╮")
    );
    println!(
        "{}        {}         {}",
        Theme::primary("  │"),
        Theme::header("🪐 O R B I T"),
        Theme::primary("│")
    );
    println!(
        "{}   {}   {}",
        Theme::primary("  │"),
        Theme::muted("Intelligent File Transfer System"),
        Theme::primary("│")
    );
    println!(
        "{}                  {}                   {}",
        Theme::primary("  │"),
        Theme::muted(format!("v{}", version)),
        Theme::primary("│")
    );
    println!(
        "{}",
        Theme::primary("  ╰─────────────────────────────────────────────────╯")
    );
    println!();
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.5), "500ms");
        assert_eq!(format_duration(1.0), "1.0s");
        assert_eq!(format_duration(65.0), "1m 5s");
        assert_eq!(format_duration(3665.0), "1h 1m");
    }
}
