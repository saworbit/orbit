use crate::auth::models::User;
use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct UserDto {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: i64,
}

#[derive(Deserialize)]
pub struct CreateUserReq {
    pub username: String,
    pub password: String,
    pub role: String,
}

pub async fn list_users(State(state): State<AppState>) -> Json<Vec<UserDto>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, username, password_hash, role, created_at FROM users",
    )
    .fetch_all(&state.user_pool)
    .await
    .unwrap_or_default();

    Json(
        users
            .into_iter()
            .map(|u| UserDto {
                id: u.id,
                username: u.username,
                role: u.role,
                created_at: u.created_at,
            })
            .collect(),
    )
}

pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserReq>,
) -> StatusCode {
    let password_hash = match crate::auth::hash_password(&payload.password) {
        Ok(hash) => hash,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&payload.role)
    .bind(now)
    .execute(&state.user_pool)
    .await;

    match result {
        Ok(_) => StatusCode::CREATED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
