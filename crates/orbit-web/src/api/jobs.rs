//! Job management API endpoints (Leptos server functions)

use crate::state::{AppState, OrbitEvent};
use leptos::*;
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Job information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobInfo {
    pub id: i64,
    pub source: String,
    pub destination: String,
    pub status: String,
    pub progress: f32,
    pub total_chunks: i64,
    pub completed_chunks: i64,
    pub failed_chunks: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Job creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateJobRequest {
    pub source: String,
    pub destination: String,
    pub compress: bool,
    pub verify: bool,
    pub parallel_workers: u32,
}

/// List all jobs (Leptos server function)
#[server(ListJobs, "/api")]
pub async fn list_jobs() -> Result<Vec<JobInfo>, ServerFnError> {
    use axum::extract::State;

    // Get app state from context
    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    // Query Magnetar database for all jobs (using runtime query for MVP)
    let rows = match sqlx::query(
        r#"
        SELECT
            id, source, destination, status, progress,
            total_chunks, completed_chunks, failed_chunks,
            created_at, updated_at
        FROM jobs
        ORDER BY created_at DESC
        LIMIT 100
        "#
    )
    .fetch_all(&state.magnetar_pool)
    .await {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let jobs: Vec<JobInfo> = rows
        .into_iter()
        .map(|row| JobInfo {
            id: row.get(0),
            source: row.get(1),
            destination: row.get(2),
            status: row.get(3),
            progress: row.get(4),
            total_chunks: row.get(5),
            completed_chunks: row.get(6),
            failed_chunks: row.get(7),
            created_at: row.get(8),
            updated_at: row.get(9),
        })
        .collect();

    Ok(jobs)
}

/// Get job statistics
#[server(GetJobStats, "/api")]
pub async fn get_job_stats(job_id: i64) -> Result<JobInfo, ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    let row = match sqlx::query(
        r#"
        SELECT
            id, source, destination, status, progress,
            total_chunks, completed_chunks, failed_chunks,
            created_at, updated_at
        FROM jobs
        WHERE id = ?
        "#
    )
    .bind(job_id)
    .fetch_one(&state.magnetar_pool)
    .await {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let job = JobInfo {
        id: row.get(0),
        source: row.get(1),
        destination: row.get(2),
        status: row.get(3),
        progress: row.get(4),
        total_chunks: row.get(5),
        completed_chunks: row.get(6),
        failed_chunks: row.get(7),
        created_at: row.get(8),
        updated_at: row.get(9),
    };

    Ok(job)
}

/// Create new job
#[server(CreateJob, "/api")]
pub async fn create_job(request: CreateJobRequest) -> Result<i64, ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    // Validate inputs
    if request.source.is_empty() || request.destination.is_empty() {
        return Err(ServerFnError::ServerError(
            "Source and destination are required".to_string(),
        ));
    }

    // Insert job into Magnetar database
    let now = chrono::Utc::now().timestamp();
    let result = match sqlx::query(
        r#"
        INSERT INTO jobs (source, destination, status, progress, total_chunks, completed_chunks, failed_chunks, created_at, updated_at)
        VALUES (?, ?, 'pending', 0.0, 0, 0, 0, ?, ?)
        "#
    )
    .bind(&request.source)
    .bind(&request.destination)
    .bind(now)
    .bind(now)
    .execute(&state.magnetar_pool)
    .await {
        Ok(r) => r,
        Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
    };

    let job_id = result.last_insert_rowid();

    // Emit job created event
    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "pending".to_string(),
        0.0,
    ));

    tracing::info!("Created job {} ({} -> {})", job_id, request.source, request.destination);

    Ok(job_id)
}

/// Delete job
#[server(DeleteJob, "/api")]
pub async fn delete_job(job_id: i64) -> Result<(), ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    match sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await {
            Ok(_) => {},
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    tracing::info!("Deleted job {}", job_id);

    Ok(())
}

/// Run/execute a job
#[server(RunJob, "/api")]
pub async fn run_job(job_id: i64) -> Result<(), ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    // Update job status to running
    let now = chrono::Utc::now().timestamp();
    match sqlx::query("UPDATE jobs SET status = 'running', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await {
            Ok(_) => {},
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    // Emit event
    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "running".to_string(),
        0.0,
    ));

    // TODO: Actually spawn job execution task
    tracing::info!("Started job {}", job_id);

    Ok(())
}

/// Cancel a running job
#[server(CancelJob, "/api")]
pub async fn cancel_job(job_id: i64) -> Result<(), ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => return Err(ServerFnError::ServerError("App state not found".to_string())),
    };

    let now = chrono::Utc::now().timestamp();
    match sqlx::query("UPDATE jobs SET status = 'cancelled', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await {
            Ok(_) => {},
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        };

    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "cancelled".to_string(),
        0.0,
    ));

    tracing::info!("Cancelled job {}", job_id);

    Ok(())
}
