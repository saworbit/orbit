/*!
 * Statistics analysis from audit logs
 */

use crate::audit::{read_audit_log, AuditEntry};
use crate::config::AuditFormat;
use crate::error::Result;
use std::path::Path;

/// Statistics summary
#[derive(Debug, Clone)]
pub struct TransferStats {
    pub total_operations: usize,
    pub successful: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total_bytes: u64,
    pub total_duration_ms: u64,
    pub with_compression: usize,
    pub total_compression_ratio: f64,
    pub fastest_speed_bps: f64,
    pub slowest_speed_bps: f64,
    pub most_recent: Option<AuditEntry>,
}

impl TransferStats {
    /// Calculate statistics from audit log
    pub fn from_audit_log(log_path: &Path, format: AuditFormat) -> Result<Self> {
        let entries = read_audit_log(log_path, format)?;

        if entries.is_empty() {
            return Ok(Self::default());
        }

        let mut stats = Self::default();
        stats.total_operations = entries.len();

        let mut fastest = 0.0_f64;
        let mut slowest = f64::MAX;
        let mut compression_count = 0;
        let mut compression_sum = 0.0;

        for entry in &entries {
            // Count status
            match entry.status.as_str() {
                "success" => stats.successful += 1,
                "failed" => stats.failed += 1,
                "skipped" => stats.skipped += 1,
                _ => {}
            }

            // Sum bytes and duration
            stats.total_bytes += entry.bytes_copied;
            stats.total_duration_ms += entry.duration_ms;

            // Calculate speed (bytes per second)
            if entry.duration_ms > 0 {
                let speed = (entry.bytes_copied as f64 / entry.duration_ms as f64) * 1000.0;
                if speed > fastest {
                    fastest = speed;
                }
                if speed < slowest && speed > 0.0 {
                    slowest = speed;
                }
            }

            // Compression stats
            if let Some(ratio) = entry.compression_ratio {
                compression_count += 1;
                compression_sum += ratio;
            }
        }

        stats.with_compression = compression_count;
        if compression_count > 0 {
            stats.total_compression_ratio = compression_sum / compression_count as f64;
        }

        stats.fastest_speed_bps = fastest;
        stats.slowest_speed_bps = if slowest < f64::MAX { slowest } else { 0.0 };

        // Most recent entry
        stats.most_recent = entries.last().cloned();

        Ok(stats)
    }

    /// Print formatted statistics
    pub fn print(&self) {
        println!("📊 Orbit Transfer Statistics");
        println!("============================\n");

        println!("Total Operations: {}", self.total_operations);
        println!("✅ Successful: {}", self.successful);
        println!("❌ Failed: {}", self.failed);
        println!("⏭️  Skipped: {}\n", self.skipped);

        println!("📦 Data Transferred:");
        println!("   Total: {}", format_bytes(self.total_bytes));
        if self.total_operations > 0 {
            let avg = self.total_bytes / self.total_operations as u64;
            println!("   Average per operation: {}\n", format_bytes(avg));
        }

        println!("⏱️  Performance:");
        println!(
            "   Total duration: {}",
            format_duration(self.total_duration_ms)
        );
        if self.total_duration_ms > 0 {
            let avg_speed = (self.total_bytes as f64 / self.total_duration_ms as f64) * 1000.0;
            println!("   Average speed: {}/s", format_bytes(avg_speed as u64));
        }
        if self.fastest_speed_bps > 0.0 {
            println!(
                "   Fastest transfer: {}/s",
                format_bytes(self.fastest_speed_bps as u64)
            );
        }
        if self.slowest_speed_bps > 0.0 {
            println!(
                "   Slowest transfer: {}/s\n",
                format_bytes(self.slowest_speed_bps as u64)
            );
        }

        if self.with_compression > 0 {
            println!("🗜️  Compression:");
            let pct = (self.with_compression as f64 / self.total_operations as f64) * 100.0;
            println!(
                "   Operations with compression: {} ({:.1}%)",
                self.with_compression, pct
            );
            println!(
                "   Average compression ratio: {:.1}%",
                self.total_compression_ratio
            );

            // Calculate space saved
            let original_size =
                self.total_bytes as f64 / (1.0 - self.total_compression_ratio / 100.0);
            let saved = original_size - self.total_bytes as f64;
            if saved > 0.0 {
                println!("   Space saved: {}\n", format_bytes(saved as u64));
            }
        }

        if let Some(ref recent) = self.most_recent {
            println!("📅 Most Recent Transfer:");
            println!("   {}", recent.timestamp);
            println!("   {:?} -> {:?}", recent.source, recent.destination);
            println!(
                "   {} in {}",
                format_bytes(recent.bytes_copied),
                format_duration(recent.duration_ms)
            );
            if let Some(ref checksum) = recent.checksum {
                println!(
                    "   Checksum: {}...{}",
                    &checksum[..8],
                    &checksum[checksum.len() - 8..]
                );
            }
        }
    }
}

impl Default for TransferStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful: 0,
            failed: 0,
            skipped: 0,
            total_bytes: 0,
            total_duration_ms: 0,
            with_compression: 0,
            total_compression_ratio: 0.0,
            fastest_speed_bps: 0.0,
            slowest_speed_bps: 0.0,
            most_recent: None,
        }
    }
}

/// Format bytes into human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

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

/// Format duration into human-readable format
fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        return format!("{}ms", ms);
    }

    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes % 60, seconds % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else {
        format!("{}s", seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1048576), "1.00 MB");
        assert_eq!(format_bytes(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1000), "1s");
        assert_eq!(format_duration(60000), "1m 0s");
        assert_eq!(format_duration(3661000), "1h 1m 1s");
    }
}
