//! Backend configuration API endpoints

use crate::error::WebResult;
use crate::state::{AppState, BackendConfig, BackendType};
use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

/// Backend info for API responses (without sensitive credentials)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct BackendInfo {
    pub id: String,
    pub name: String,
    pub backend_type: String,
    pub created_at: i64,
}

impl From<BackendConfig> for BackendInfo {
    fn from(config: BackendConfig) -> Self {
        let backend_type = match config.backend_type {
            BackendType::Local { .. } => "Local",
            BackendType::S3 { .. } => "S3",
            BackendType::SMB { .. } => "SMB",
        }
        .to_string();

        BackendInfo {
            id: config.id,
            name: config.name,
            backend_type,
            created_at: config.created_at,
        }
    }
}

/// List all configured backends
#[cfg_attr(feature = "utoipa", utoipa::path(
    get,
    path = "/api/backends",
    responses(
        (status = 200, description = "List of backends", body = Vec<BackendInfo>)
    )
))]
pub async fn list_backends(State(state): State<AppState>) -> WebResult<Json<Vec<BackendInfo>>> {
    let backends = state.backends.read().await;
    let backend_list: Vec<BackendInfo> = backends
        .values()
        .map(|config| BackendInfo::from(config.clone()))
        .collect();

    Ok(Json(backend_list))
}

/// Get backend details
#[cfg_attr(feature = "utoipa", utoipa::path(
    get,
    path = "/api/backends/{backend_id}",
    responses(
        (status = 200, description = "Backend details", body = BackendInfo),
        (status = 404, description = "Backend not found")
    )
))]
pub async fn get_backend(
    State(state): State<AppState>,
    Path(backend_id): Path<String>,
) -> WebResult<Json<BackendInfo>> {
    let backends = state.backends.read().await;
    let config = backends
        .get(&backend_id)
        .ok_or_else(|| crate::error::WebError::NotFound("Backend not found".to_string()))?;

    Ok(Json(BackendInfo::from(config.clone())))
}

// Note: Full CRUD (create, update, delete) will be added in post-MVP phases
// For MVP, we focus on listing existing backends
