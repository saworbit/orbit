/*!
 * CLI progress renderer for interactive terminal display
 *
 * Subscribes to progress events and renders them as formatted console output
 * with progress bars, transfer rates, and ETAs.
 */

use crate::core::progress::{ProgressEvent, ProgressSubscriber};
use std::collections::HashMap;
use std::io::{self, Write};
use std::thread;
use std::time::Instant;

/// Transfer state for tracking progress
struct TransferState {
    _source: String,
    _dest: String,
    total_bytes: u64,
    bytes_transferred: u64,
    start_time: Instant,
    last_update: Instant,
}

impl TransferState {
    fn progress_pct(&self) -> f64 {
        if self.total_bytes > 0 {
            (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
        } else {
            0.0
        }
    }

    fn transfer_rate_mbps(&self) -> f64 {
        let elapsed = self.last_update.duration_since(self.start_time).as_secs_f64();
        if elapsed > 0.0 {
            (self.bytes_transferred as f64 / 1_048_576.0) / elapsed
        } else {
            0.0
        }
    }

    fn eta_seconds(&self) -> Option<u64> {
        let elapsed = self.last_update.duration_since(self.start_time).as_secs_f64();
        if elapsed > 0.0 && self.bytes_transferred > 0 {
            let rate = self.bytes_transferred as f64 / elapsed;
            let remaining_bytes = self.total_bytes.saturating_sub(self.bytes_transferred);
            Some((remaining_bytes as f64 / rate) as u64)
        } else {
            None
        }
    }
}

/// CLI progress renderer
pub struct CliProgressRenderer {
    subscriber: ProgressSubscriber,
    active_transfers: HashMap<String, TransferState>,
    verbose: bool,
}

impl CliProgressRenderer {
    /// Create a new CLI progress renderer
    pub fn new(subscriber: ProgressSubscriber, verbose: bool) -> Self {
        Self {
            subscriber,
            active_transfers: HashMap::new(),
            verbose,
        }
    }

    /// Run the progress renderer in the current thread
    pub fn run(mut self) -> io::Result<()> {
        while let Some(event) = self.subscriber.recv() {
            self.handle_event(event)?;
        }
        Ok(())
    }

    /// Spawn the renderer in a background thread
    pub fn spawn(self) -> thread::JoinHandle<io::Result<()>> {
        thread::spawn(move || self.run())
    }

    /// Handle a single progress event
    fn handle_event(&mut self, event: ProgressEvent) -> io::Result<()> {
        match event {
            ProgressEvent::TransferStart { file_id, source, dest, total_bytes, .. } => {
                let state = TransferState {
                    _source: source.display().to_string(),
                    _dest: dest.display().to_string(),
                    total_bytes,
                    bytes_transferred: 0,
                    start_time: Instant::now(),
                    last_update: Instant::now(),
                };

                println!("\n📁 Transferring: {}", source.display());
                if self.verbose {
                    println!("   Destination: {}", dest.display());
                    println!("   Size: {}", format_bytes(total_bytes));
                }

                self.active_transfers.insert(file_id.as_str().to_string(), state);
            }

            ProgressEvent::TransferProgress { file_id, bytes_transferred, .. } => {
                if let Some(state) = self.active_transfers.get_mut(file_id.as_str()) {
                    state.bytes_transferred = bytes_transferred;
                    state.last_update = Instant::now();

                    // Print progress bar
                    print!("\r   ");
                    print_progress_bar(state.progress_pct(), 40);
                    print!(" {:>6.1}%  ", state.progress_pct());
                    print!("{:>10}/s  ", format_bytes((state.transfer_rate_mbps() * 1_048_576.0) as u64));

                    if let Some(eta) = state.eta_seconds() {
                        print!("ETA: {}", format_duration(eta));
                    }

                    io::stdout().flush()?;
                }
            }

            ProgressEvent::TransferComplete { file_id, total_bytes, duration_ms, checksum, .. } => {
                if let Some(_state) = self.active_transfers.remove(file_id.as_str()) {
                    let throughput_mbps = if duration_ms > 0 {
                        (total_bytes as f64 / 1_048_576.0) / (duration_ms as f64 / 1000.0)
                    } else {
                        0.0
                    };

                    println!("\r   ✓ Complete: {} in {}ms ({:.2} MB/s)",
                        format_bytes(total_bytes),
                        duration_ms,
                        throughput_mbps
                    );

                    if self.verbose {
                        if let Some(hash) = checksum {
                            println!("   Checksum: {}", hash);
                        }
                    }
                }
            }

            ProgressEvent::TransferFailed { file_id, error, bytes_transferred, .. } => {
                if let Some(_state) = self.active_transfers.remove(file_id.as_str()) {
                    println!("\r   ✗ Failed: {} after {} - {}",
                        file_id.as_str(),
                        format_bytes(bytes_transferred),
                        error
                    );
                }
            }

            ProgressEvent::DirectoryScanStart { path, .. } => {
                println!("\n📂 Scanning directory: {}", path.display());
            }

            ProgressEvent::DirectoryScanProgress { files_found, dirs_found, .. } => {
                if self.verbose {
                    print!("\r   Found: {} files, {} directories", files_found, dirs_found);
                    io::stdout().flush()?;
                }
            }

            ProgressEvent::DirectoryScanComplete { total_files, total_dirs, .. } => {
                println!("\r   ✓ Scan complete: {} files, {} directories", total_files, total_dirs);
            }

            ProgressEvent::BatchComplete { files_succeeded, files_failed, total_bytes, duration_ms, .. } => {
                let throughput_mbps = if duration_ms > 0 {
                    (total_bytes as f64 / 1_048_576.0) / (duration_ms as f64 / 1000.0)
                } else {
                    0.0
                };

                println!("\n📊 Batch Summary:");
                println!("   Succeeded: {} files", files_succeeded);
                if files_failed > 0 {
                    println!("   Failed: {} files", files_failed);
                }
                println!("   Total: {} in {}ms ({:.2} MB/s)",
                    format_bytes(total_bytes),
                    duration_ms,
                    throughput_mbps
                );
            }

            ProgressEvent::ResumeDecision { file_id, decision, from_offset, verified_chunks, reason, .. } => {
                if self.verbose {
                    println!("\n🔄 Resume Decision for {}: {}", file_id.as_str(), decision);
                    if from_offset > 0 {
                        println!("   Offset: {} ({} chunks verified)",
                            format_bytes(from_offset), verified_chunks);
                    }
                    if let Some(r) = reason {
                        println!("   Reason: {}", r);
                    }
                }
            }

            ProgressEvent::ChunkVerification {  chunk_id, chunk_size, .. } => {
                if self.verbose {
                    print!("\r   Verifying chunk {}: {}", chunk_id, format_bytes(chunk_size));
                    io::stdout().flush()?;
                }
            }

            ProgressEvent::ChunkVerified {  chunk_id, digest, .. } => {
                if self.verbose {
                    println!("\r   ✓ Chunk {} verified: {}", chunk_id, &digest[..16]);
                }
            }
        }

        Ok(())
    }
}

/// Print a text-based progress bar
fn print_progress_bar(percentage: f64, width: usize) {
    let filled = ((percentage / 100.0) * width as f64) as usize;
    let empty = width.saturating_sub(filled);

    print!("[");
    for _ in 0..filled {
        print!("█");
    }
    for _ in 0..empty {
        print!("░");
    }
    print!("]");
}

/// Format bytes in human-readable format
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

/// Format duration in human-readable format
fn format_duration(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
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
        assert_eq!(format_bytes(1_073_741_824), "1.00 GB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(90), "1m 30s");
        assert_eq!(format_duration(3661), "1h 1m");
    }

    #[test]
    fn test_transfer_state_progress() {
        let state = TransferState {
            _source: "/source".to_string(),
            _dest: "/dest".to_string(),
            total_bytes: 1000,
            bytes_transferred: 500,
            start_time: Instant::now(),
            last_update: Instant::now(),
        };

        assert!((state.progress_pct() - 50.0).abs() < 0.01);
    }
}
