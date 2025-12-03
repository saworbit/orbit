use crate::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct SystemHealth {
    pub active_jobs: usize,
    pub total_bandwidth_mbps: f64,
    pub system_load: f32,
    pub storage_health: String,
}

pub async fn get_system_health(State(_state): State<AppState>) -> Json<SystemHealth> {
    // In production: Query Magnetar + System Resources
    Json(SystemHealth {
        active_jobs: 3,
        total_bandwidth_mbps: 245.0,
        system_load: 0.45,
        storage_health: "Healthy".to_string(),
    })
}
