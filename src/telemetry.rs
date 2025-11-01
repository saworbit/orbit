/*!
 * Telemetry subscriber for JSON event logging
 *
 * Subscribes to progress events and outputs structured JSON logs
 * for monitoring, audit trails, and integration with external systems.
 */

use crate::core::progress::{ProgressEvent, ProgressSubscriber};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{self, Write, BufWriter};
use std::path::Path;
use std::thread;

/// Telemetry event for JSON serialization
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TelemetryEvent {
    TransferStart {
        file_id: String,
        source: String,
        dest: String,
        total_bytes: u64,
        timestamp: u64,
    },
    TransferProgress {
        file_id: String,
        bytes_transferred: u64,
        total_bytes: u64,
        progress_pct: f64,
        timestamp: u64,
    },
    TransferComplete {
        file_id: String,
        total_bytes: u64,
        duration_ms: u64,
        throughput_mbps: f64,
        checksum: Option<String>,
        timestamp: u64,
    },
    TransferFailed {
        file_id: String,
        error: String,
        bytes_transferred: u64,
        timestamp: u64,
    },
    DirectoryScanStart {
        path: String,
        timestamp: u64,
    },
    DirectoryScanProgress {
        files_found: u64,
        dirs_found: u64,
        timestamp: u64,
    },
    DirectoryScanComplete {
        total_files: u64,
        total_dirs: u64,
        timestamp: u64,
    },
    BatchComplete {
        files_succeeded: u64,
        files_failed: u64,
        total_bytes: u64,
        duration_ms: u64,
        avg_throughput_mbps: f64,
        timestamp: u64,
    },
}

impl TelemetryEvent {
    /// Convert ProgressEvent to TelemetryEvent
    pub fn from_progress_event(event: ProgressEvent) -> Self {
        match event {
            ProgressEvent::TransferStart { file_id, source, dest, total_bytes, timestamp } => {
                TelemetryEvent::TransferStart {
                    file_id: file_id.as_str().to_string(),
                    source: source.display().to_string(),
                    dest: dest.display().to_string(),
                    total_bytes,
                    timestamp,
                }
            }
            ProgressEvent::TransferProgress { file_id, bytes_transferred, total_bytes, timestamp } => {
                let progress_pct = if total_bytes > 0 {
                    (bytes_transferred as f64 / total_bytes as f64) * 100.0
                } else {
                    0.0
                };
                TelemetryEvent::TransferProgress {
                    file_id: file_id.as_str().to_string(),
                    bytes_transferred,
                    total_bytes,
                    progress_pct,
                    timestamp,
                }
            }
            ProgressEvent::TransferComplete { file_id, total_bytes, duration_ms, checksum, timestamp } => {
                let throughput_mbps = if duration_ms > 0 {
                    (total_bytes as f64 / 1_048_576.0) / (duration_ms as f64 / 1000.0)
                } else {
                    0.0
                };
                TelemetryEvent::TransferComplete {
                    file_id: file_id.as_str().to_string(),
                    total_bytes,
                    duration_ms,
                    throughput_mbps,
                    checksum,
                    timestamp,
                }
            }
            ProgressEvent::TransferFailed { file_id, error, bytes_transferred, timestamp } => {
                TelemetryEvent::TransferFailed {
                    file_id: file_id.as_str().to_string(),
                    error,
                    bytes_transferred,
                    timestamp,
                }
            }
            ProgressEvent::DirectoryScanStart { path, timestamp } => {
                TelemetryEvent::DirectoryScanStart {
                    path: path.display().to_string(),
                    timestamp,
                }
            }
            ProgressEvent::DirectoryScanProgress { files_found, dirs_found, timestamp } => {
                TelemetryEvent::DirectoryScanProgress {
                    files_found,
                    dirs_found,
                    timestamp,
                }
            }
            ProgressEvent::DirectoryScanComplete { total_files, total_dirs, timestamp } => {
                TelemetryEvent::DirectoryScanComplete {
                    total_files,
                    total_dirs,
                    timestamp,
                }
            }
            ProgressEvent::BatchComplete { files_succeeded, files_failed, total_bytes, duration_ms, timestamp } => {
                let avg_throughput_mbps = if duration_ms > 0 {
                    (total_bytes as f64 / 1_048_576.0) / (duration_ms as f64 / 1000.0)
                } else {
                    0.0
                };
                TelemetryEvent::BatchComplete {
                    files_succeeded,
                    files_failed,
                    total_bytes,
                    duration_ms,
                    avg_throughput_mbps,
                    timestamp,
                }
            }
        }
    }
}

/// Output destination for telemetry events
pub enum TelemetryOutput {
    /// Write to stdout
    Stdout,
    /// Write to stderr
    Stderr,
    /// Write to a file
    File(BufWriter<File>),
}

impl TelemetryOutput {
    /// Create file output
    pub fn file(path: &Path) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(TelemetryOutput::File(BufWriter::new(file)))
    }

    /// Write a telemetry event as JSON line
    fn write_event(&mut self, event: &TelemetryEvent) -> io::Result<()> {
        let json = serde_json::to_string(event)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        match self {
            TelemetryOutput::Stdout => {
                println!("{}", json);
                io::stdout().flush()
            }
            TelemetryOutput::Stderr => {
                eprintln!("{}", json);
                io::stderr().flush()
            }
            TelemetryOutput::File(writer) => {
                writeln!(writer, "{}", json)?;
                writer.flush()
            }
        }
    }
}

/// Telemetry logger that subscribes to progress events
pub struct TelemetryLogger {
    subscriber: ProgressSubscriber,
    output: TelemetryOutput,
}

impl TelemetryLogger {
    /// Create a new telemetry logger
    pub fn new(subscriber: ProgressSubscriber, output: TelemetryOutput) -> Self {
        Self { subscriber, output }
    }

    /// Run the telemetry logger in the current thread
    ///
    /// This will block until the event stream is closed.
    pub fn run(mut self) -> io::Result<()> {
        for event in self.subscriber.receiver().iter() {
            let telemetry_event = TelemetryEvent::from_progress_event(event);
            self.output.write_event(&telemetry_event)?;
        }
        Ok(())
    }

    /// Spawn the telemetry logger in a background thread
    ///
    /// Returns a join handle that can be used to wait for completion.
    pub fn spawn(self) -> thread::JoinHandle<io::Result<()>> {
        thread::spawn(move || self.run())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::progress::{ProgressPublisher, FileId};
    use std::path::PathBuf;

    #[test]
    fn test_telemetry_event_serialization() {
        let event = TelemetryEvent::TransferStart {
            file_id: "test -> dest".to_string(),
            source: "/source/file.txt".to_string(),
            dest: "/dest/file.txt".to_string(),
            total_bytes: 1024,
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("transfer_start"));
        assert!(json.contains("\"total_bytes\":1024"));
    }

    #[test]
    fn test_telemetry_event_conversion() {
        let file_id = FileId::new(
            &PathBuf::from("/source/test.txt"),
            &PathBuf::from("/dest/test.txt"),
        );

        let progress_event = ProgressEvent::TransferProgress {
            file_id,
            bytes_transferred: 512,
            total_bytes: 1024,
            timestamp: 1234567890,
        };

        let telemetry_event = TelemetryEvent::from_progress_event(progress_event);

        match telemetry_event {
            TelemetryEvent::TransferProgress { progress_pct, .. } => {
                assert!((progress_pct - 50.0).abs() < 0.01);
            }
            _ => panic!("Expected TransferProgress event"),
        }
    }
}
