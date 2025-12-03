use crate::state::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct JobEstimate {
    pub estimated_duration_ms: u64,
    pub confidence_score: f32, // 0.0 to 1.0
    pub historical_speed_mbps: f64,
}

/// Predicts job duration based on source size and historical throughput
pub async fn get_estimate(
    State(_state): State<AppState>,
    Path(_source_path): Path<String>, // In real impl, use this to scan size
) -> Json<JobEstimate> {
    // Mock intelligence logic for Beta 1:
    // 1. In production, query Magnetar for avg speed of specific backend
    // 2. Scan source directory for total bytes
    // 3. Calculate duration

    // Returning a hardcoded "Smart" prediction for UI testing
    Json(JobEstimate {
        estimated_duration_ms: 45000, // 45 seconds
        confidence_score: 0.88,
        historical_speed_mbps: 125.5,
    })
}
