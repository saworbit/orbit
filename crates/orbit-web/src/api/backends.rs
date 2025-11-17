//! Backend configuration API endpoints

use crate::state::{AppState, BackendConfig, BackendType};
use leptos::*;
use serde::{Deserialize, Serialize};

/// Backend info for API responses (without sensitive credentials)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[server(ListBackends, "/api")]
pub async fn list_backends() -> Result<Vec<BackendInfo>, ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => {
            return Err(ServerFnError::ServerError(
                "App state not found".to_string(),
            ))
        }
    };

    let backends = state.backends.read().await;
    let backend_list: Vec<BackendInfo> = backends
        .values()
        .map(|config| BackendInfo::from(config.clone()))
        .collect();

    Ok(backend_list)
}

/// Get backend details
#[server(GetBackend, "/api")]
pub async fn get_backend(backend_id: String) -> Result<BackendInfo, ServerFnError> {
    use axum::extract::State;

    let State(state): State<AppState> = match use_context::<State<AppState>>() {
        Some(s) => s,
        None => {
            return Err(ServerFnError::ServerError(
                "App state not found".to_string(),
            ))
        }
    };

    let backends = state.backends.read().await;
    let config = match backends.get(&backend_id) {
        Some(c) => c,
        None => return Err(ServerFnError::ServerError("Backend not found".to_string())),
    };

    Ok(BackendInfo::from(config.clone()))
}

// Note: Full CRUD (create, update, delete) will be added in post-MVP phases
// For MVP, we focus on listing existing backends
