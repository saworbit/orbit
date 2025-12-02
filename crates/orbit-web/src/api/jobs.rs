//! Job management API endpoints

use crate::error::{WebError, WebResult};
use crate::state::{AppState, OrbitEvent};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

/// Job information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
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
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct CreateJobRequest {
    pub source: String,
    pub destination: String,
    pub compress: bool,
    pub verify: bool,
    pub parallel_workers: u32,
}

/// List all jobs
#[cfg_attr(feature = "utoipa", utoipa::path(
    get,
    path = "/api/jobs",
    responses(
        (status = 200, description = "List of jobs", body = Vec<JobInfo>)
    )
))]
pub async fn list_jobs(State(state): State<AppState>) -> WebResult<Json<Vec<JobInfo>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            id, source, destination, status, progress,
            total_chunks, completed_chunks, failed_chunks,
            created_at, updated_at
        FROM jobs
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.magnetar_pool)
    .await
    .map_err(|e| WebError::Database(e.to_string()))?;

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

    Ok(Json(jobs))
}

/// Get job statistics
#[cfg_attr(feature = "utoipa", utoipa::path(
    get,
    path = "/api/jobs/{job_id}",
    responses(
        (status = 200, description = "Job details", body = JobInfo),
        (status = 404, description = "Job not found")
    )
))]
pub async fn get_job_stats(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> WebResult<Json<JobInfo>> {
    let row = sqlx::query(
        r#"
        SELECT
            id, source, destination, status, progress,
            total_chunks, completed_chunks, failed_chunks,
            created_at, updated_at
        FROM jobs
        WHERE id = ?
        "#,
    )
    .bind(job_id)
    .fetch_one(&state.magnetar_pool)
    .await
    .map_err(|_| WebError::NotFound(format!("Job {} not found", job_id)))?;

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

    Ok(Json(job))
}

/// Create new job
#[cfg_attr(feature = "utoipa", utoipa::path(
    post,
    path = "/api/jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created", body = i64),
        (status = 400, description = "Invalid request")
    )
))]
pub async fn create_job(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> WebResult<Json<i64>> {
    // Validate inputs
    if request.source.is_empty() || request.destination.is_empty() {
        return Err(WebError::BadRequest(
            "Source and destination are required".to_string(),
        ));
    }

    // Insert job into Magnetar database
    let now = chrono::Utc::now().timestamp();
    let result = sqlx::query(
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
    .await
    .map_err(|e| WebError::Database(e.to_string()))?;

    let job_id = result.last_insert_rowid();

    // Emit job created event
    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "pending".to_string(),
        0.0,
    ));

    tracing::info!(
        "Created job {} ({} -> {})",
        job_id,
        request.source,
        request.destination
    );

    Ok(Json(job_id))
}

/// Delete job
#[cfg_attr(feature = "utoipa", utoipa::path(
    delete,
    path = "/api/jobs/{job_id}",
    responses(
        (status = 200, description = "Job deleted"),
        (status = 404, description = "Job not found")
    )
))]
pub async fn delete_job(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> WebResult<Json<()>> {
    sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| WebError::Database(e.to_string()))?;

    tracing::info!("Deleted job {}", job_id);

    Ok(Json(()))
}

/// Run/execute a job
#[cfg_attr(feature = "utoipa", utoipa::path(
    post,
    path = "/api/jobs/{job_id}/run",
    responses(
        (status = 200, description = "Job started"),
        (status = 404, description = "Job not found")
    )
))]
pub async fn run_job(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> WebResult<Json<()>> {
    // Update job status to running
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE jobs SET status = 'running', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| WebError::Database(e.to_string()))?;

    // Emit event
    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "running".to_string(),
        0.0,
    ));

    // TODO: Actually spawn job execution task
    tracing::info!("Started job {}", job_id);

    Ok(Json(()))
}

/// Cancel a running job
#[cfg_attr(feature = "utoipa", utoipa::path(
    post,
    path = "/api/jobs/{job_id}/cancel",
    responses(
        (status = 200, description = "Job cancelled"),
        (status = 404, description = "Job not found")
    )
))]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> WebResult<Json<()>> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE jobs SET status = 'cancelled', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(job_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| WebError::Database(e.to_string()))?;

    state.emit_event(OrbitEvent::job_updated(
        job_id.to_string(),
        "cancelled".to_string(),
        0.0,
    ));

    tracing::info!("Cancelled job {}", job_id);

    Ok(Json(()))
}
