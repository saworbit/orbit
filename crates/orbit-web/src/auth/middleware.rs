//! JWT authentication middleware for Axum

use super::models::{Claims, Role};
use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::env;

/// JWT secret key (loaded from ORBIT_JWT_SECRET env var)
pub fn get_jwt_secret() -> String {
    env::var("ORBIT_JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("ORBIT_JWT_SECRET not set, using insecure default!");
        "INSECURE_DEFAULT_CHANGE_ME_IN_PRODUCTION".to_string()
    })
}

/// Generate JWT token from claims
pub fn generate_token(claims: &Claims) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret();
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate JWT token and extract claims
pub fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = get_jwt_secret();
    let validation = Validation::default();

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
}

/// Extract JWT from cookie jar
pub fn extract_jwt_from_cookies(jar: &CookieJar) -> Option<String> {
    jar.get("orbit_token").map(|cookie| cookie.value().to_string())
}

/// Axum middleware to require authentication
pub async fn require_auth(
    jar: CookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract JWT from cookies
    let token = extract_jwt_from_cookies(&jar)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = validate_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Check if expired
    if claims.is_expired() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Add claims to request extensions for downstream handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Axum middleware to require specific role
pub fn require_role(required_role: Role) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |mut request: Request, next: Next| {
        let required = required_role;
        Box::pin(async move {
            // Get claims from request extensions (added by require_auth)
            let claims = request
                .extensions()
                .get::<Claims>()
                .ok_or(StatusCode::UNAUTHORIZED)?
                .clone();

            // Check role permission
            if !claims.get_role().has_permission(required) {
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Auth error response
#[derive(Debug)]
pub struct AuthError {
    pub message: String,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.message
        });

        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }
}

impl From<&str> for AuthError {
    fn from(msg: &str) -> Self {
        AuthError {
            message: msg.to_string(),
        }
    }
}

impl From<String> for AuthError {
    fn from(message: String) -> Self {
        AuthError { message }
    }
}
