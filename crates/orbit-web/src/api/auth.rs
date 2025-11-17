//! Authentication API endpoints

use crate::{
    auth::{authenticate_user, LoginForm, UserInfo},
    error::WebError,
    state::AppState,
};
use axum::{extract::State, http::StatusCode, Json};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Serialize;
use sqlx::Row;

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub user: UserInfo,
    pub message: String,
}

/// Login endpoint
///
/// Validates credentials and returns JWT token as httpOnly cookie
pub async fn login_handler(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(login): Json<LoginForm>,
) -> Result<(CookieJar, Json<LoginResponse>), WebError> {
    // Authenticate user
    let (user, token) = authenticate_user(&state.user_pool, &login)
        .await
        .map_err(|e| WebError::Auth(e.to_string()))?;

    // Create httpOnly secure cookie
    let cookie = Cookie::build(("orbit_token", token))
        .path("/")
        .http_only(true)
        .secure(true) // Enable in production with HTTPS
        .max_age(time::Duration::hours(24))
        .build();

    let response = LoginResponse {
        user: UserInfo::from(user),
        message: "Login successful".to_string(),
    };

    tracing::info!("User logged in: {}", response.user.username);

    Ok((jar.add(cookie), Json(response)))
}

/// Logout endpoint
///
/// Clears authentication cookie
pub async fn logout_handler(jar: CookieJar) -> (CookieJar, StatusCode) {
    let cookie = Cookie::build(("orbit_token", ""))
        .path("/")
        .max_age(time::Duration::seconds(0))
        .build();

    (jar.add(cookie), StatusCode::OK)
}

/// Current user endpoint
///
/// Returns information about the currently authenticated user
pub async fn me_handler(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<UserInfo>, WebError> {
    // Extract JWT
    let token = crate::auth::extract_jwt_from_cookies(&jar)
        .ok_or_else(|| WebError::Auth("Not authenticated".to_string()))?;

    // Validate token
    let claims = crate::auth::validate_token(&token)
        .map_err(|_| WebError::Auth("Invalid token".to_string()))?;

    // Fetch user from database
    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash, role, created_at
        FROM users
        WHERE id = ?
        "#
    )
    .bind(&claims.sub)
    .fetch_one(&state.user_pool)
    .await
    .map_err(|_| WebError::Auth("User not found".to_string()))?;

    let user = crate::auth::User {
        id: row.get(0),
        username: row.get(1),
        password_hash: row.get(2),
        role: row.get(3),
        created_at: row.get(4),
    };

    Ok(Json(UserInfo::from(user)))
}
