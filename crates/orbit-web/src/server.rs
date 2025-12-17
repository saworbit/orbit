//! Axum server setup with Leptos integration

use crate::{api, state::AppState, ws, ServerConfig};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Row;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

// Only import these if the UI feature is enabled
#[cfg(feature = "ui")]
use tower_http::services::{ServeDir, ServeFile};

/// Request for creating a job
#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub source: String,
    pub destination: String,
    pub compress: bool,
    pub verify: bool,
    pub parallel_workers: u32,
}

/// List all jobs handler
async fn list_jobs_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<api::JobInfo>>, (axum::http::StatusCode, String)> {
    tracing::info!("Listing jobs...");

    let rows = sqlx::query(
        r#"
        SELECT id, source, destination, status, progress,
               total_chunks, completed_chunks, failed_chunks,
               created_at, updated_at
        FROM jobs
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to list jobs: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    tracing::info!("Found {} jobs", rows.len());

    let mut jobs = Vec::new();
    for row in rows {
        // Handle type conversions safely - SQLite types can vary
        let progress_val: f64 = row.try_get(4).unwrap_or(0.0);
        let created_val: i64 = row.try_get(8).unwrap_or(0);
        let updated_val: i64 = row.try_get(9).unwrap_or(0);

        jobs.push(api::JobInfo {
            id: row.get(0),
            source: row.get(1),
            destination: row.get(2),
            status: row.get(3),
            progress: progress_val as f32,
            total_chunks: row.get(5),
            completed_chunks: row.get(6),
            failed_chunks: row.get(7),
            created_at: created_val,
            updated_at: updated_val,
        });
    }

    tracing::info!("Returning {} jobs", jobs.len());
    Ok(Json(jobs))
}

/// Create job handler
async fn create_job_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<i64>, axum::http::StatusCode> {
    let now = chrono::Utc::now().timestamp();
    let result = sqlx::query(
        r#"
        INSERT INTO jobs (source, destination, compress, verify, parallel, status, progress, total_chunks, completed_chunks, failed_chunks, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, 'pending', 0.0, 0, 0, 0, ?, ?)
        "#,
    )
    .bind(&request.source)
    .bind(&request.destination)
    .bind(request.compress)
    .bind(request.verify)
    .bind(request.parallel_workers as i32)
    .bind(now)
    .bind(now)
    .execute(&state.magnetar_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create job: {}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let job_id = result.last_insert_rowid();
    tracing::info!(
        "Created job {} ({} -> {})",
        job_id,
        request.source,
        request.destination
    );

    // Wake up the reactor to process the new job
    state.reactor_notify.notify_one();
    tracing::debug!("Notified reactor about new job {}", job_id);

    Ok(Json(job_id))
}

/// Request for job actions (run, cancel, delete)
#[derive(Debug, Deserialize)]
pub struct JobActionRequest {
    pub job_id: i64,
}

/// Run job handler - starts a pending job
async fn run_job_handler(
    State(state): State<AppState>,
    Json(request): Json<JobActionRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let now = chrono::Utc::now().timestamp();
    let result = sqlx::query(
        "UPDATE jobs SET status = 'running', updated_at = ? WHERE id = ? AND status = 'pending'",
    )
    .bind(now)
    .bind(request.job_id)
    .execute(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to run job: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Job not found or not in pending state".to_string(),
        ));
    }

    tracing::info!("Started job {}", request.job_id);
    Ok(Json("Job started".to_string()))
}

/// Cancel job handler - cancels a running job
async fn cancel_job_handler(
    State(state): State<AppState>,
    Json(request): Json<JobActionRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let now = chrono::Utc::now().timestamp();
    let result = sqlx::query(
        "UPDATE jobs SET status = 'cancelled', updated_at = ? WHERE id = ? AND status IN ('pending', 'running')",
    )
    .bind(now)
    .bind(request.job_id)
    .execute(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to cancel job: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Job not found or already completed".to_string(),
        ));
    }

    tracing::info!("Cancelled job {}", request.job_id);
    Ok(Json("Job cancelled".to_string()))
}

/// Delete job handler
async fn delete_job_handler(
    State(state): State<AppState>,
    Json(request): Json<JobActionRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let result = sqlx::query("DELETE FROM jobs WHERE id = ?")
        .bind(request.job_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to delete job: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Job not found".to_string(),
        ));
    }

    tracing::info!("Deleted job {}", request.job_id);
    Ok(Json("Job deleted".to_string()))
}

/// Get single job handler
async fn get_job_handler(
    State(state): State<AppState>,
    Json(request): Json<JobActionRequest>,
) -> Result<Json<api::JobInfo>, (axum::http::StatusCode, String)> {
    let row = sqlx::query(
        r#"
        SELECT id, source, destination, status, progress,
               total_chunks, completed_chunks, failed_chunks,
               created_at, updated_at
        FROM jobs
        WHERE id = ?
        "#,
    )
    .bind(request.job_id)
    .fetch_optional(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to get job: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    match row {
        Some(row) => {
            let progress_val: f64 = row.try_get(4).unwrap_or(0.0);
            let created_val: i64 = row.try_get(8).unwrap_or(0);
            let updated_val: i64 = row.try_get(9).unwrap_or(0);
            Ok(Json(api::JobInfo {
                id: row.get(0),
                source: row.get(1),
                destination: row.get(2),
                status: row.get(3),
                progress: progress_val as f32,
                total_chunks: row.get(5),
                completed_chunks: row.get(6),
                failed_chunks: row.get(7),
                created_at: created_val,
                updated_at: updated_val,
            }))
        }
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Job not found".to_string(),
        )),
    }
}

/// Backend info for API response (without credentials)
#[derive(Debug, serde::Serialize)]
pub struct BackendInfoResponse {
    pub id: String,
    pub name: String,
    pub backend_type: String,
    pub details: String,
    pub created_at: i64,
}

/// List backends handler
async fn list_backends_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<BackendInfoResponse>>, (axum::http::StatusCode, String)> {
    let backends = state.backends.read().await;
    let backend_list: Vec<BackendInfoResponse> = backends
        .values()
        .map(|config| {
            let (backend_type, details) = match &config.backend_type {
                crate::state::BackendType::Local { root_path } => {
                    ("Local".to_string(), format!("Path: {}", root_path))
                }
                crate::state::BackendType::S3 { bucket, region } => (
                    "S3".to_string(),
                    format!("Bucket: {}, Region: {}", bucket, region),
                ),
                crate::state::BackendType::SMB { host, share } => (
                    "SMB".to_string(),
                    format!("Host: {}, Share: {}", host, share),
                ),
            };
            BackendInfoResponse {
                id: config.id.clone(),
                name: config.name.clone(),
                backend_type,
                details,
                created_at: config.created_at,
            }
        })
        .collect();

    Ok(Json(backend_list))
}

/// Request to create a backend
#[derive(Debug, Deserialize)]
pub struct CreateBackendRequest {
    pub name: String,
    pub backend_type: String,
    // S3 fields
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    // SMB fields
    pub host: Option<String>,
    pub share: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    // Local fields
    pub path: Option<String>,
}

/// Create backend handler
async fn create_backend_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateBackendRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::{BackendConfig, BackendCredentials, BackendType};

    let (backend_type, credentials) = match request.backend_type.as_str() {
        "s3" | "S3" => {
            let bucket = request.bucket.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "S3 backend requires bucket".to_string(),
            ))?;
            let region = request.region.unwrap_or_else(|| "us-east-1".to_string());
            let access_key = request.access_key.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "S3 backend requires access_key".to_string(),
            ))?;
            let secret_key = request.secret_key.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "S3 backend requires secret_key".to_string(),
            ))?;
            (
                BackendType::S3 { bucket, region },
                BackendCredentials::S3 {
                    access_key,
                    secret_key,
                },
            )
        }
        "smb" | "SMB" => {
            let host = request.host.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "SMB backend requires host".to_string(),
            ))?;
            let share = request.share.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "SMB backend requires share".to_string(),
            ))?;
            let username = request.username.unwrap_or_default();
            let password = request.password.unwrap_or_default();
            (
                BackendType::SMB { host, share },
                if username.is_empty() {
                    BackendCredentials::None
                } else {
                    BackendCredentials::SMB { username, password }
                },
            )
        }
        "local" | "Local" => {
            let path = request.path.ok_or((
                axum::http::StatusCode::BAD_REQUEST,
                "Local backend requires path".to_string(),
            ))?;
            (
                BackendType::Local { root_path: path },
                BackendCredentials::None,
            )
        }
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Unknown backend type: {}", request.backend_type),
            ));
        }
    };

    let config = BackendConfig::new(request.name, backend_type, credentials);
    let id = config.id.clone();

    let mut backends = state.backends.write().await;
    backends.insert(id.clone(), config);

    tracing::info!("Created backend: {}", id);
    Ok(Json(id))
}

/// Delete backend request
#[derive(Debug, Deserialize)]
pub struct DeleteBackendRequest {
    pub backend_id: String,
}

/// Delete backend handler
async fn delete_backend_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteBackendRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let mut backends = state.backends.write().await;

    if backends.remove(&request.backend_id).is_some() {
        tracing::info!("Deleted backend: {}", request.backend_id);
        Ok(Json("Backend deleted".to_string()))
    } else {
        Err((
            axum::http::StatusCode::NOT_FOUND,
            "Backend not found".to_string(),
        ))
    }
}

// =============================================================================
// FILE EXPLORER API
// =============================================================================

/// File/directory entry for API response
#[derive(Debug, serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// Request to list directory contents
#[derive(Debug, Deserialize)]
pub struct ListDirRequest {
    pub path: Option<String>,
}

/// Query parameters for listing files
#[derive(Debug, Deserialize)]
pub struct ListFilesQuery {
    pub path: Option<String>,
}

/// List files handler (GET with query params)
async fn list_files_handler(
    axum::extract::Query(query): axum::extract::Query<ListFilesQuery>,
) -> Result<Json<Vec<FileEntry>>, (axum::http::StatusCode, String)> {
    let path = query.path.unwrap_or_else(|| {
        #[cfg(windows)]
        {
            "C:\\".to_string()
        }
        #[cfg(not(windows))]
        {
            "/".to_string()
        }
    });

    tracing::info!("Listing files: {}", path);

    let dir_path = std::path::Path::new(&path);
    if !dir_path.exists() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("Path not found: {}", path),
        ));
    }

    if !dir_path.is_dir() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Path is not a directory: {}", path),
        ));
    }

    let mut entries = Vec::new();

    // Add parent directory entry if not at root
    if let Some(parent) = dir_path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_string_lossy().to_string(),
            is_dir: true,
            size: 0,
            modified: 0,
        });
    }

    match std::fs::read_dir(dir_path) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        Err(e) => {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                format!("Cannot read directory: {}", e),
            ));
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.name == ".." {
            std::cmp::Ordering::Less
        } else if b.name == ".." {
            std::cmp::Ordering::Greater
        } else if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(Json(entries))
}

/// List directory contents handler
async fn list_dir_handler(
    Json(request): Json<ListDirRequest>,
) -> Result<Json<Vec<FileEntry>>, (axum::http::StatusCode, String)> {
    let path = request.path.unwrap_or_else(|| {
        #[cfg(windows)]
        {
            "C:\\".to_string()
        }
        #[cfg(not(windows))]
        {
            "/".to_string()
        }
    });

    tracing::info!("Listing directory: {}", path);

    let dir_path = std::path::Path::new(&path);
    if !dir_path.exists() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("Path not found: {}", path),
        ));
    }

    if !dir_path.is_dir() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Path is not a directory: {}", path),
        ));
    }

    let mut entries = Vec::new();

    // Add parent directory entry if not at root
    if let Some(parent) = dir_path.parent() {
        entries.push(FileEntry {
            name: "..".to_string(),
            path: parent.to_string_lossy().to_string(),
            is_dir: true,
            size: 0,
            modified: 0,
        });
    }

    match std::fs::read_dir(dir_path) {
        Ok(read_dir) => {
            for entry in read_dir.flatten() {
                let metadata = entry.metadata().ok();
                let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                entries.push(FileEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    is_dir,
                    size,
                    modified,
                });
            }
        }
        Err(e) => {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                format!("Cannot read directory: {}", e),
            ));
        }
    }

    // Sort: directories first, then by name
    entries.sort_by(|a, b| {
        if a.name == ".." {
            std::cmp::Ordering::Less
        } else if b.name == ".." {
            std::cmp::Ordering::Greater
        } else if a.is_dir && !b.is_dir {
            std::cmp::Ordering::Less
        } else if !a.is_dir && b.is_dir {
            std::cmp::Ordering::Greater
        } else {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
    });

    Ok(Json(entries))
}

/// Get system drives/roots
#[cfg(windows)]
async fn list_drives_handler() -> Json<Vec<FileEntry>> {
    let mut drives = Vec::new();
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = std::path::Path::new(&drive);
        if path.exists() {
            drives.push(FileEntry {
                name: drive.clone(),
                path: drive,
                is_dir: true,
                size: 0,
                modified: 0,
            });
        }
    }
    Json(drives)
}

#[cfg(not(windows))]
async fn list_drives_handler() -> Json<Vec<FileEntry>> {
    Json(vec![FileEntry {
        name: "/".to_string(),
        path: "/".to_string(),
        is_dir: true,
        size: 0,
        modified: 0,
    }])
}

// =============================================================================
// USER MANAGEMENT API
// =============================================================================

/// User info for API response
#[derive(Debug, serde::Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: i64,
}

/// List all users handler (Admin only)
async fn list_users_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<UserInfo>>, (axum::http::StatusCode, String)> {
    let rows =
        sqlx::query("SELECT id, username, role, created_at FROM users ORDER BY created_at DESC")
            .fetch_all(&state.user_pool)
            .await
            .map_err(|e| {
                let msg = format!("Failed to list users: {}", e);
                tracing::error!("{}", msg);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
            })?;

    let users: Vec<UserInfo> = rows
        .iter()
        .map(|row| UserInfo {
            id: row.get(0),
            username: row.get(1),
            role: row.get(2),
            created_at: row.get(3),
        })
        .collect();

    Ok(Json(users))
}

/// Create user request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: String,
}

/// Create user handler (Admin only)
async fn create_user_handler(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    // Validate role
    let valid_roles = ["admin", "operator", "viewer"];
    if !valid_roles.contains(&request.role.to_lowercase().as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid role. Must be one of: {:?}", valid_roles),
        ));
    }

    // Hash password
    let password_hash = crate::auth::hash_password(&request.password)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let now = chrono::Utc::now().timestamp();

    // Insert user
    sqlx::query(
        "INSERT INTO users (username, password_hash, role, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(&request.username)
    .bind(&password_hash)
    .bind(request.role.to_lowercase())
    .bind(now)
    .execute(&state.user_pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            (
                axum::http::StatusCode::CONFLICT,
                "Username already exists".to_string(),
            )
        } else {
            let msg = format!("Failed to create user: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        }
    })?;

    tracing::info!(
        "Created user: {} with role: {}",
        request.username,
        request.role
    );
    Ok(Json("User created".to_string()))
}

/// Update user request
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub user_id: i64,
    pub password: Option<String>,
    pub role: Option<String>,
}

/// Update user handler (Admin only)
async fn update_user_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateUserRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    // Update password if provided
    if let Some(password) = &request.password {
        let password_hash = crate::auth::hash_password(password)
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
            .bind(&password_hash)
            .bind(request.user_id)
            .execute(&state.user_pool)
            .await
            .map_err(|e| {
                let msg = format!("Failed to update password: {}", e);
                tracing::error!("{}", msg);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
            })?;
    }

    // Update role if provided
    if let Some(role) = &request.role {
        let valid_roles = ["admin", "operator", "viewer"];
        if !valid_roles.contains(&role.to_lowercase().as_str()) {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid role. Must be one of: {:?}", valid_roles),
            ));
        }

        sqlx::query("UPDATE users SET role = ? WHERE id = ?")
            .bind(role.to_lowercase())
            .bind(request.user_id)
            .execute(&state.user_pool)
            .await
            .map_err(|e| {
                let msg = format!("Failed to update role: {}", e);
                tracing::error!("{}", msg);
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
            })?;
    }

    tracing::info!("Updated user {}", request.user_id);
    Ok(Json("User updated".to_string()))
}

/// Delete user request
#[derive(Debug, Deserialize)]
pub struct DeleteUserRequest {
    pub user_id: i64,
}

/// Delete user handler (Admin only)
async fn delete_user_handler(
    State(state): State<AppState>,
    Json(request): Json<DeleteUserRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    // Prevent deleting the last admin
    let admin_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role = 'admin'")
        .fetch_one(&state.user_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to check admin count: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    // Check if user is admin
    let user_role: Option<String> = sqlx::query_scalar("SELECT role FROM users WHERE id = ?")
        .bind(request.user_id)
        .fetch_optional(&state.user_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to get user role: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    if let Some(role) = user_role {
        if role == "admin" && admin_count <= 1 {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                "Cannot delete the last admin user".to_string(),
            ));
        }
    } else {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "User not found".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(request.user_id)
        .execute(&state.user_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to delete user: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "User not found".to_string(),
        ));
    }

    tracing::info!("Deleted user {}", request.user_id);
    Ok(Json("User deleted".to_string()))
}

// =============================================================================
// FILE UPLOAD API
// =============================================================================

/// Upload file handler
async fn upload_file_handler(
    mut multipart: axum::extract::Multipart,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let mut target_path: Option<String> = None;
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Multipart error: {}", e),
        )
    })? {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "path" => {
                target_path = Some(field.text().await.map_err(|e| {
                    (
                        axum::http::StatusCode::BAD_REQUEST,
                        format!("Failed to read path: {}", e),
                    )
                })?);
            }
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                file_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| {
                            (
                                axum::http::StatusCode::BAD_REQUEST,
                                format!("Failed to read file: {}", e),
                            )
                        })?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let target_dir = target_path.ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "Missing target path".to_string(),
    ))?;

    let file_name = file_name.ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "Missing file name".to_string(),
    ))?;

    let file_data = file_data.ok_or((
        axum::http::StatusCode::BAD_REQUEST,
        "Missing file data".to_string(),
    ))?;

    let full_path = std::path::Path::new(&target_dir).join(&file_name);

    // Ensure target directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create directory: {}", e),
            )
        })?;
    }

    // Write file
    std::fs::write(&full_path, &file_data).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to write file: {}", e),
        )
    })?;

    tracing::info!(
        "Uploaded file: {} ({} bytes)",
        full_path.display(),
        file_data.len()
    );
    Ok(Json(format!("Uploaded: {}", full_path.display())))
}

// =============================================================================
// PIPELINE API
// =============================================================================

/// Pipeline info for API response
#[derive(Debug, serde::Serialize)]
pub struct PipelineInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Full pipeline response with nodes and edges
#[derive(Debug, serde::Serialize)]
pub struct PipelineDetailResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<crate::state::PipelineNode>,
    pub edges: Vec<crate::state::PipelineEdge>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// List all pipelines handler
async fn list_pipelines_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<PipelineInfo>>, (axum::http::StatusCode, String)> {
    let pipelines = state.pipelines.read().await;
    let pipeline_list: Vec<PipelineInfo> = pipelines
        .values()
        .map(|p| PipelineInfo {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            node_count: p.nodes.len(),
            edge_count: p.edges.len(),
            status: p.status.to_string(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        })
        .collect();

    Ok(Json(pipeline_list))
}

/// Request to get a specific pipeline
#[derive(Debug, Deserialize)]
pub struct GetPipelineRequest {
    pub pipeline_id: String,
}

/// Get single pipeline with full details
async fn get_pipeline_handler(
    State(state): State<AppState>,
    Json(request): Json<GetPipelineRequest>,
) -> Result<Json<PipelineDetailResponse>, (axum::http::StatusCode, String)> {
    let pipelines = state.pipelines.read().await;

    match pipelines.get(&request.pipeline_id) {
        Some(p) => Ok(Json(PipelineDetailResponse {
            id: p.id.clone(),
            name: p.name.clone(),
            description: p.description.clone(),
            nodes: p.nodes.clone(),
            edges: p.edges.clone(),
            status: p.status.to_string(),
            created_at: p.created_at,
            updated_at: p.updated_at,
        })),
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            "Pipeline not found".to_string(),
        )),
    }
}

/// Request to create a pipeline
#[derive(Debug, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub description: Option<String>,
}

/// Create pipeline handler
async fn create_pipeline_handler(
    State(state): State<AppState>,
    Json(request): Json<CreatePipelineRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::Pipeline;

    let pipeline = Pipeline::new(request.name, request.description.unwrap_or_default());
    let id = pipeline.id.clone();

    // Save to database
    let nodes_json = serde_json::to_string(&pipeline.nodes).unwrap_or_else(|_| "[]".to_string());
    let edges_json = serde_json::to_string(&pipeline.edges).unwrap_or_else(|_| "[]".to_string());

    sqlx::query(
        r#"
        INSERT INTO pipelines (id, name, description, nodes_json, edges_json, status, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&pipeline.id)
    .bind(&pipeline.name)
    .bind(&pipeline.description)
    .bind(&nodes_json)
    .bind(&edges_json)
    .bind(pipeline.status.to_string())
    .bind(pipeline.created_at)
    .bind(pipeline.updated_at)
    .execute(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to create pipeline: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    // Add to in-memory state
    let mut pipelines = state.pipelines.write().await;
    pipelines.insert(id.clone(), pipeline);

    tracing::info!("Created pipeline: {}", id);
    Ok(Json(id))
}

/// Request to update pipeline metadata
#[derive(Debug, Deserialize)]
pub struct UpdatePipelineRequest {
    pub pipeline_id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
}

/// Update pipeline handler
async fn update_pipeline_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdatePipelineRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::PipelineStatus;

    let mut pipelines = state.pipelines.write().await;

    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    if let Some(name) = request.name {
        pipeline.name = name;
    }
    if let Some(description) = request.description {
        pipeline.description = description;
    }
    if let Some(status) = request.status {
        pipeline.status = match status.as_str() {
            "draft" => PipelineStatus::Draft,
            "ready" => PipelineStatus::Ready,
            "running" => PipelineStatus::Running,
            "completed" => PipelineStatus::Completed,
            "failed" => PipelineStatus::Failed,
            "paused" => PipelineStatus::Paused,
            _ => {
                return Err((
                    axum::http::StatusCode::BAD_REQUEST,
                    format!("Invalid status: {}", status),
                ))
            }
        };
    }
    pipeline.updated_at = chrono::Utc::now().timestamp();

    // Save to database
    sqlx::query(
        "UPDATE pipelines SET name = ?, description = ?, status = ?, updated_at = ? WHERE id = ?",
    )
    .bind(&pipeline.name)
    .bind(&pipeline.description)
    .bind(pipeline.status.to_string())
    .bind(pipeline.updated_at)
    .bind(&request.pipeline_id)
    .execute(&state.magnetar_pool)
    .await
    .map_err(|e| {
        let msg = format!("Failed to update pipeline: {}", e);
        tracing::error!("{}", msg);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
    })?;

    tracing::info!("Updated pipeline: {}", request.pipeline_id);
    Ok(Json("Pipeline updated".to_string()))
}

/// Request to delete a pipeline
#[derive(Debug, Deserialize)]
pub struct DeletePipelineRequest {
    pub pipeline_id: String,
}

/// Delete pipeline handler
async fn delete_pipeline_handler(
    State(state): State<AppState>,
    Json(request): Json<DeletePipelineRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    // Remove from database
    let result = sqlx::query("DELETE FROM pipelines WHERE id = ?")
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to delete pipeline: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    if result.rows_affected() == 0 {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Pipeline not found".to_string(),
        ));
    }

    // Remove from in-memory state
    let mut pipelines = state.pipelines.write().await;
    pipelines.remove(&request.pipeline_id);

    tracing::info!("Deleted pipeline: {}", request.pipeline_id);
    Ok(Json("Pipeline deleted".to_string()))
}

/// Request to add a node to a pipeline
#[derive(Debug, Deserialize)]
pub struct AddNodeRequest {
    pub pipeline_id: String,
    pub node_type: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub config: Option<crate::state::NodeConfig>,
}

/// Add node to pipeline handler
async fn add_node_handler(
    State(state): State<AppState>,
    Json(request): Json<AddNodeRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::{PipelineNode, PipelineNodeType};

    let node_type = match request.node_type.as_str() {
        "source" => PipelineNodeType::Source,
        "destination" => PipelineNodeType::Destination,
        "transfer" => PipelineNodeType::Transfer,
        "transform" => PipelineNodeType::Transform,
        "filter" => PipelineNodeType::Filter,
        "merge" => PipelineNodeType::Merge,
        "split" => PipelineNodeType::Split,
        "conditional" => PipelineNodeType::Conditional,
        _ => {
            return Err((
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid node type: {}", request.node_type),
            ))
        }
    };

    let mut node = PipelineNode::new(node_type, request.name, request.x, request.y);
    if let Some(config) = request.config {
        node.config = config;
    }
    let node_id = node.id.clone();

    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    pipeline.add_node(node);

    // Save to database
    let nodes_json = serde_json::to_string(&pipeline.nodes).unwrap_or_else(|_| "[]".to_string());
    sqlx::query("UPDATE pipelines SET nodes_json = ?, updated_at = ? WHERE id = ?")
        .bind(&nodes_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to save node: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!("Added node {} to pipeline {}", node_id, request.pipeline_id);
    Ok(Json(node_id))
}

/// Request to update a node
#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub pipeline_id: String,
    pub node_id: String,
    pub name: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub config: Option<crate::state::NodeConfig>,
}

/// Update node handler
async fn update_node_handler(
    State(state): State<AppState>,
    Json(request): Json<UpdateNodeRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    let node = pipeline
        .nodes
        .iter_mut()
        .find(|n| n.id == request.node_id)
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Node not found".to_string(),
        ))?;

    if let Some(name) = request.name {
        node.name = name;
    }
    if let Some(x) = request.x {
        node.position.x = x;
    }
    if let Some(y) = request.y {
        node.position.y = y;
    }
    if let Some(config) = request.config {
        node.config = config;
    }
    pipeline.updated_at = chrono::Utc::now().timestamp();

    // Save to database
    let nodes_json = serde_json::to_string(&pipeline.nodes).unwrap_or_else(|_| "[]".to_string());
    sqlx::query("UPDATE pipelines SET nodes_json = ?, updated_at = ? WHERE id = ?")
        .bind(&nodes_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to update node: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!(
        "Updated node {} in pipeline {}",
        request.node_id,
        request.pipeline_id
    );
    Ok(Json("Node updated".to_string()))
}

/// Request to remove a node
#[derive(Debug, Deserialize)]
pub struct RemoveNodeRequest {
    pub pipeline_id: String,
    pub node_id: String,
}

/// Remove node from pipeline handler
async fn remove_node_handler(
    State(state): State<AppState>,
    Json(request): Json<RemoveNodeRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    let initial_count = pipeline.nodes.len();
    pipeline.remove_node(&request.node_id);

    if pipeline.nodes.len() == initial_count {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Node not found".to_string(),
        ));
    }

    // Save to database
    let nodes_json = serde_json::to_string(&pipeline.nodes).unwrap_or_else(|_| "[]".to_string());
    let edges_json = serde_json::to_string(&pipeline.edges).unwrap_or_else(|_| "[]".to_string());
    sqlx::query("UPDATE pipelines SET nodes_json = ?, edges_json = ?, updated_at = ? WHERE id = ?")
        .bind(&nodes_json)
        .bind(&edges_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to remove node: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!(
        "Removed node {} from pipeline {}",
        request.node_id,
        request.pipeline_id
    );
    Ok(Json("Node removed".to_string()))
}

/// Request to add an edge
#[derive(Debug, Deserialize)]
pub struct AddEdgeRequest {
    pub pipeline_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub source_port: Option<String>,
    pub target_port: Option<String>,
}

/// Add edge to pipeline handler
async fn add_edge_handler(
    State(state): State<AppState>,
    Json(request): Json<AddEdgeRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::PipelineEdge;

    let edge = if request.source_port.is_some() || request.target_port.is_some() {
        PipelineEdge::with_ports(
            request.source_node_id,
            request.target_node_id,
            request.source_port.unwrap_or_else(|| "out".to_string()),
            request.target_port.unwrap_or_else(|| "in".to_string()),
        )
    } else {
        PipelineEdge::new(request.source_node_id, request.target_node_id)
    };
    let edge_id = edge.id.clone();

    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    pipeline
        .add_edge(edge)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    // Save to database
    let edges_json = serde_json::to_string(&pipeline.edges).unwrap_or_else(|_| "[]".to_string());
    sqlx::query("UPDATE pipelines SET edges_json = ?, updated_at = ? WHERE id = ?")
        .bind(&edges_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to save edge: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!("Added edge {} to pipeline {}", edge_id, request.pipeline_id);
    Ok(Json(edge_id))
}

/// Request to remove an edge
#[derive(Debug, Deserialize)]
pub struct RemoveEdgeRequest {
    pub pipeline_id: String,
    pub edge_id: String,
}

/// Remove edge from pipeline handler
async fn remove_edge_handler(
    State(state): State<AppState>,
    Json(request): Json<RemoveEdgeRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    let initial_count = pipeline.edges.len();
    pipeline.remove_edge(&request.edge_id);

    if pipeline.edges.len() == initial_count {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Edge not found".to_string(),
        ));
    }

    // Save to database
    let edges_json = serde_json::to_string(&pipeline.edges).unwrap_or_else(|_| "[]".to_string());
    sqlx::query("UPDATE pipelines SET edges_json = ?, updated_at = ? WHERE id = ?")
        .bind(&edges_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to remove edge: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!(
        "Removed edge {} from pipeline {}",
        request.edge_id,
        request.pipeline_id
    );
    Ok(Json("Edge removed".to_string()))
}

/// Request to sync pipeline with bulk nodes/edges update
#[derive(Debug, Deserialize)]
pub struct SyncPipelineRequest {
    pub pipeline_id: String,
    pub nodes_json: String,
    pub edges_json: String,
}

/// Sync pipeline handler - bulk update for visual editor
async fn sync_pipeline_handler(
    State(state): State<AppState>,
    Json(request): Json<SyncPipelineRequest>,
) -> Result<Json<String>, (axum::http::StatusCode, String)> {
    use crate::state::PipelineEdge;
    use crate::state::PipelineNode;

    // Validate JSON structure
    let nodes: Vec<PipelineNode> = serde_json::from_str(&request.nodes_json).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid nodes JSON: {}", e),
        )
    })?;

    let edges: Vec<PipelineEdge> = serde_json::from_str(&request.edges_json).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid edges JSON: {}", e),
        )
    })?;

    // Update in-memory state
    let mut pipelines = state.pipelines.write().await;
    let pipeline = pipelines.get_mut(&request.pipeline_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "Pipeline not found".to_string(),
    ))?;

    pipeline.nodes = nodes;
    pipeline.edges = edges;
    pipeline.updated_at = chrono::Utc::now().timestamp();

    // Update database
    sqlx::query("UPDATE pipelines SET nodes_json = ?, edges_json = ?, updated_at = ? WHERE id = ?")
        .bind(&request.nodes_json)
        .bind(&request.edges_json)
        .bind(pipeline.updated_at)
        .bind(&request.pipeline_id)
        .execute(&state.magnetar_pool)
        .await
        .map_err(|e| {
            let msg = format!("Failed to sync pipeline: {}", e);
            tracing::error!("{}", msg);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, msg)
        })?;

    tracing::info!(
        "Synced pipeline {}: {} nodes, {} edges",
        request.pipeline_id,
        pipeline.nodes.len(),
        pipeline.edges.len()
    );
    Ok(Json("Pipeline synced".to_string()))
}

/// Run the Axum Control Plane server
pub async fn run_server(
    config: ServerConfig,
    reactor_notify: std::sync::Arc<tokio::sync::Notify>,
) -> Result<(), Box<dyn std::error::Error + Send>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,orbit_web=debug".into()),
        )
        .json()
        .init();

    tracing::info!("Starting Orbit Nebula web server v1.0.0-rc.1");
    tracing::info!("Magnetar DB: {}", config.magnetar_db);
    tracing::info!("User DB: {}", config.user_db);

    // Initialize application state
    let state = AppState::new(&config.magnetar_db, &config.user_db, reactor_notify.clone()).await?;

    tracing::info!("Application state initialized");

    // Start reactor in background for job execution
    tracing::info!("☢️  Starting Orbit Reactor (job execution engine)...");
    let reactor_pool =
        sqlx::SqlitePool::connect(&format!("sqlite:{}?mode=rwc", config.magnetar_db))
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    let reactor = crate::reactor::Reactor::new(reactor_pool, reactor_notify);
    tokio::spawn(async move {
        reactor.run().await;
    });
    tracing::info!("Reactor spawned successfully");

    // Build Axum router
    let app = Router::new()
        // WebSocket endpoints
        .route("/ws/events", get(ws::ws_handler))
        .route("/ws/events/:job_id", get(ws::ws_handler))
        // Auth endpoints
        .route("/api/auth/login", post(api::login_handler))
        .route("/api/auth/logout", post(api::logout_handler))
        .route("/api/auth/me", get(api::me_handler))
        // Job endpoints
        .route("/api/list_jobs", post(list_jobs_handler))
        .route("/api/create_job", post(create_job_handler))
        .route("/api/get_job", post(get_job_handler))
        .route("/api/run_job", post(run_job_handler))
        .route("/api/cancel_job", post(cancel_job_handler))
        .route("/api/delete_job", post(delete_job_handler))
        // Backend endpoints
        .route("/api/list_backends", post(list_backends_handler))
        .route("/api/create_backend", post(create_backend_handler))
        .route("/api/delete_backend", post(delete_backend_handler))
        // File explorer endpoints
        .route("/api/files/list", get(api::list_files))
        .route("/api/list_dir", post(list_dir_handler))
        .route("/api/list_drives", get(api::list_drives))
        .route("/api/upload_file", post(upload_file_handler))
        // User management endpoints (Admin only) - v2.2.0-alpha
        .route("/api/list_users", post(list_users_handler))
        .route("/api/create_user", post(create_user_handler))
        .route("/api/update_user", post(update_user_handler))
        .route("/api/delete_user", post(delete_user_handler))
        // Admin API - v2.2.0-beta.1
        .route(
            "/api/admin/users",
            get(api::list_users).post(api::create_user),
        )
        // Intelligence API - v2.2.0-beta.1
        .route("/api/estimates/:path", get(api::get_estimate))
        // System Stats API - v2.2.0-beta.1
        .route("/api/stats/health", get(api::get_system_health))
        // Pipeline endpoints
        .route("/api/list_pipelines", post(list_pipelines_handler))
        .route("/api/get_pipeline", post(get_pipeline_handler))
        .route("/api/create_pipeline", post(create_pipeline_handler))
        .route("/api/update_pipeline", post(update_pipeline_handler))
        .route("/api/delete_pipeline", post(delete_pipeline_handler))
        .route("/api/add_node", post(add_node_handler))
        .route("/api/update_node", post(update_node_handler))
        .route("/api/remove_node", post(remove_node_handler))
        .route("/api/add_edge", post(add_edge_handler))
        .route("/api/remove_edge", post(remove_edge_handler))
        .route("/api/sync_pipeline", post(sync_pipeline_handler))
        // Health check
        .route(
            "/api/health",
            get(|| async {
                axum::Json(serde_json::json!({
                    "status": "ok",
                    "service": "orbit-web",
                    "version": "1.0.0-rc.1"
                }))
            }),
        );

    // ---------------------------------------------------------
    // CONDITIONAL UI COMPILATION
    // ---------------------------------------------------------
    #[cfg(feature = "ui")]
    let app = {
        tracing::info!("🎨 UI Feature Enabled: Serving embedded dashboard from dashboard/dist");
        app.nest_service("/", ServeDir::new("dashboard/dist"))
            .fallback_service(ServeFile::new("dashboard/dist/index.html"))
    };

    #[cfg(not(feature = "ui"))]
    let app = {
        tracing::info!("⚙️ Headless Mode: Dashboard not included, API-only server");
        app.fallback(get(|| async {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Orbit Control Plane (Headless Mode) - API available at /api/*",
            )
        }))
    };
    // ---------------------------------------------------------

    let app = app
        .layer(
            CorsLayer::new()
                .allow_origin([
                    "http://localhost:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                    "http://127.0.0.1:5173"
                        .parse::<axum::http::HeaderValue>()
                        .unwrap(),
                ])
                .allow_methods([
                    axum::http::Method::GET,
                    axum::http::Method::POST,
                    axum::http::Method::PUT,
                    axum::http::Method::DELETE,
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::AUTHORIZATION,
                ])
                .allow_credentials(true),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    tracing::info!("Nebula listening on http://{}", addr);
    tracing::info!("   Dashboard: http://{}/", addr);
    tracing::info!("   Login: http://{}/login", addr);
    tracing::info!("   Health: http://{}/api/health", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;
    axum::serve(listener, app)
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send>)?;

    Ok(())
}
