//! Server functions for job management
//!
//! These functions run on the server and are called from the client via Leptos server functions

use crate::types::{CreateJobRequest, JobInfo};
use leptos::*;

#[cfg(feature = "ssr")]
use magnetar::{JobStats, JobStore};

/// List all jobs with their current status
#[server(ListJobs, "/api")]
pub async fn list_jobs() -> Result<Vec<JobInfo>, ServerFnError> {
    use magnetar::JobStatus;

    let db_path = std::env::var("ORBIT_WEB_DB").unwrap_or_else(|_| "orbit-web.db".to_string());
    let store = magnetar::open(&db_path)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // For MVP, we'll track jobs 1-100 (in a real implementation, we'd have a job registry)
    let mut jobs = Vec::new();
    for job_id in 1..=100i64 {
        if let Ok(stats) = store.get_stats(job_id).await {
            if stats.total_chunks > 0 {
                let status = if stats.is_complete() {
                    "completed"
                } else if stats.has_failures() {
                    "failed"
                } else if stats.processing > 0 {
                    "processing"
                } else {
                    "pending"
                };

                jobs.push(JobInfo {
                    id: job_id.to_string(),
                    source: format!("Job {}", job_id), // TODO: Store job metadata
                    destination: format!("Dest {}", job_id),
                    status: status.to_string(),
                    total_chunks: stats.total_chunks,
                    pending: stats.pending,
                    processing: stats.processing,
                    done: stats.done,
                    failed: stats.failed,
                    completion_percent: stats.completion_percent(),
                });
            }
        }
    }

    Ok(jobs)
}

/// Get statistics for a specific job
#[server(GetJobStats, "/api")]
pub async fn get_job_stats(job_id: String) -> Result<JobInfo, ServerFnError> {
    let db_path = std::env::var("ORBIT_WEB_DB").unwrap_or_else(|_| "orbit-web.db".to_string());
    let store = magnetar::open(&db_path)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let job_id_i64: i64 = job_id
        .parse()
        .map_err(|e| ServerFnError::new(format!("Invalid job ID: {}", e)))?;

    let stats = store
        .get_stats(job_id_i64)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let status = if stats.is_complete() {
        "completed"
    } else if stats.has_failures() {
        "failed"
    } else if stats.processing > 0 {
        "processing"
    } else {
        "pending"
    };

    Ok(JobInfo {
        id: job_id,
        source: format!("Job {}", job_id_i64),
        destination: format!("Dest {}", job_id_i64),
        status: status.to_string(),
        total_chunks: stats.total_chunks,
        pending: stats.pending,
        processing: stats.processing,
        done: stats.done,
        failed: stats.failed,
        completion_percent: stats.completion_percent(),
    })
}

/// Create a new job with a numeric ID
#[server(CreateJob, "/api")]
pub async fn create_job(request: CreateJobRequest) -> Result<String, ServerFnError> {
    tracing::info!(
        "Creating job: {} -> {} (compress: {}, verify: {})",
        request.source,
        request.destination,
        request.compress,
        request.verify
    );

    let db_path = std::env::var("ORBIT_WEB_DB").unwrap_or_else(|_| "orbit-web.db".to_string());
    let mut store = magnetar::open(&db_path).await.map_err(|e| ServerFnError::new(e.to_string()))?;

    // Create a new job and get the auto-generated numeric ID
    let job_id = store
        .new_job(
            request.source.clone(),
            request.destination.clone(),
            request.compress,
            request.verify,
            request.parallel,
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Create a minimal manifest with a single placeholder chunk
    // In a full implementation, this would scan the source and create a proper manifest
    let manifest = toml::Value::Table(toml::toml! {
        [[chunks]]
        id = 0
        checksum = "pending"
    });

    // Initialize the job in Magnetar
    store
        .init_from_manifest(job_id, &manifest)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    tracing::info!("Created job with ID: {}", job_id);

    // Return the numeric job ID as a string
    Ok(job_id.to_string())
}

/// Delete a job
#[server(DeleteJob, "/api")]
pub async fn delete_job(job_id: String) -> Result<(), ServerFnError> {
    let db_path = std::env::var("ORBIT_WEB_DB").unwrap_or_else(|_| "orbit-web.db".to_string());
    let mut store = magnetar::open(&db_path)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let job_id_i64: i64 = job_id
        .parse()
        .map_err(|e| ServerFnError::new(format!("Invalid job ID: {}", e)))?;

    store
        .delete_job(job_id_i64)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
