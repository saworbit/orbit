/*!
 * Enhanced progress reporting with indicatif for multi-transfer support
 *
 * This module provides:
 * - Multi-progress bars with ETA and transfer speed
 * - Concurrent transfer tracking
 * - Event-driven updates integrated with existing progress system
 */

use crate::core::progress::{ProgressEvent, ProgressSubscriber};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Enhanced progress tracker using indicatif
pub struct EnhancedProgressTracker {
    multi: MultiProgress,
    bars: Arc<Mutex<HashMap<String, ProgressBarState>>>,
    show_progress: bool,
}

struct ProgressBarState {
    bar: ProgressBar,
    start_time: Instant,
    total_bytes: u64,
}

impl EnhancedProgressTracker {
    /// Create a new enhanced progress tracker
    pub fn new(show_progress: bool) -> Self {
        Self {
            multi: MultiProgress::new(),
            bars: Arc::new(Mutex::new(HashMap::new())),
            show_progress,
        }
    }

    /// Start tracking a file transfer
    pub fn start_transfer(&self, file_id: &str, source: &str, total_bytes: u64) {
        if !self.show_progress {
            return;
        }

        let bar = self.multi.add(ProgressBar::new(total_bytes));

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        bar.set_message(format!("📁 {}", source));

        let mut bars = self.bars.lock().unwrap();
        bars.insert(
            file_id.to_string(),
            ProgressBarState {
                bar,
                start_time: Instant::now(),
                total_bytes,
            },
        );
    }

    /// Update transfer progress
    pub fn update_progress(&self, file_id: &str, bytes_transferred: u64) {
        if !self.show_progress {
            return;
        }

        let bars = self.bars.lock().unwrap();
        if let Some(state) = bars.get(file_id) {
            state.bar.set_position(bytes_transferred);

            // Calculate transfer rate
            let elapsed = state.start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let rate = bytes_transferred as f64 / elapsed;
                state
                    .bar
                    .set_message(format!("📁 Transfer ({:.2} MB/s)", rate / 1_048_576.0));
            }
        }
    }

    /// Mark transfer as complete
    pub fn complete_transfer(&self, file_id: &str, success: bool) {
        if !self.show_progress {
            return;
        }

        let mut bars = self.bars.lock().unwrap();
        if let Some(state) = bars.remove(file_id) {
            let elapsed = state.start_time.elapsed();
            let throughput = if elapsed.as_secs_f64() > 0.0 {
                state.total_bytes as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };

            state.bar.finish_with_message(format!(
                "{} Complete - {} in {:.2}s ({:.2} MB/s)",
                if success { "✓" } else { "✗" },
                format_bytes(state.total_bytes),
                elapsed.as_secs_f64(),
                throughput / 1_048_576.0
            ));
        }
    }

    /// Create a subscriber handler that processes events
    pub fn handle_events(&self, subscriber: ProgressSubscriber) {
        use std::thread;

        let bars = self.bars.clone();
        let show_progress = self.show_progress;

        thread::spawn(move || {
            while let Some(event) = subscriber.recv() {
                if !show_progress {
                    continue;
                }

                match event {
                    ProgressEvent::TransferStart { .. } => {
                        // This would be handled by start_transfer
                    }
                    ProgressEvent::TransferProgress {
                        file_id,
                        bytes_transferred,
                        ..
                    } => {
                        let bars = bars.lock().unwrap();
                        if let Some(state) = bars.get(file_id.as_str()) {
                            state.bar.set_position(bytes_transferred);
                        }
                    }
                    ProgressEvent::TransferComplete {
                        file_id,
                        total_bytes,
                        duration_ms,
                        ..
                    } => {
                        let mut bars = bars.lock().unwrap();
                        if let Some(state) = bars.remove(file_id.as_str()) {
                            let throughput = if duration_ms > 0 {
                                (total_bytes as f64 / 1_048_576.0) / (duration_ms as f64 / 1000.0)
                            } else {
                                0.0
                            };
                            state.bar.finish_with_message(format!(
                                "✓ Complete ({:.2} MB/s)",
                                throughput
                            ));
                        }
                    }
                    ProgressEvent::TransferFailed { file_id, error, .. } => {
                        let mut bars = bars.lock().unwrap();
                        if let Some(state) = bars.remove(file_id.as_str()) {
                            state
                                .bar
                                .finish_with_message(format!("✗ Failed: {}", error));
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1_048_576), "1.00 MB");
    }

    #[test]
    fn test_progress_tracker_creation() {
        let tracker = EnhancedProgressTracker::new(true);
        tracker.start_transfer("test", "test.txt", 1000);
        tracker.update_progress("test", 500);
        tracker.complete_transfer("test", true);
    }
}
