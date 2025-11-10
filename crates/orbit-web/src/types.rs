//! Type definitions for the web application

use serde::{Deserialize, Serialize};

/// Job information for display
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobInfo {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub status: String,
    pub total_chunks: u64,
    pub pending: u64,
    pub processing: u64,
    pub done: u64,
    pub failed: u64,
    pub completion_percent: f64,
}

/// Configuration for creating a new job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub source: String,
    pub destination: String,
    pub compress: bool,
    pub verify: bool,
    pub parallel: Option<usize>,
}

/// Progress update for a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    pub job_id: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_mbps: f64,
    pub eta_seconds: Option<u64>,
    pub current_file: Option<String>,
}

/// Log entry for display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub job_id: Option<String>,
}
