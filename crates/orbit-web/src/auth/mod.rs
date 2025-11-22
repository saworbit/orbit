//! Authentication and authorization module for Nebula
//!
//! Provides JWT-based authentication with Argon2 password hashing
//! and role-based access control (RBAC).

pub mod middleware;
pub mod models;

pub use middleware::{
    extract_jwt_from_cookies, generate_token, get_jwt_secret, require_auth, require_role,
    validate_token, AuthError,
};
pub use models::{Claims, LoginForm, Role, User, UserInfo};

use sqlx::{Row, SqlitePool};

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| format!("Password hash error: {}", e))?
        .to_string();

    Ok(password_hash)
}

/// Initialize the user database schema
pub async fn init_user_db(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("User database schema initialized");
    Ok(())
}

/// Create default admin user if no users exist
pub async fn ensure_default_admin(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if any users exist
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;

    if count.0 == 0 {
        // Create default admin user
        let admin = User::new("admin".to_string(), "orbit2025", Role::Admin)?;

        sqlx::query(
            r#"
            INSERT INTO users (id, username, password_hash, role, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&admin.id)
        .bind(&admin.username)
        .bind(&admin.password_hash)
        .bind(&admin.role)
        .bind(admin.created_at)
        .execute(pool)
        .await?;

        tracing::warn!(
            "Created default admin user (username: admin, password: orbit2025) - CHANGE THIS PASSWORD!"
        );
    }

    Ok(())
}

/// Authenticate user and return JWT token
pub async fn authenticate_user(
    pool: &SqlitePool,
    login: &LoginForm,
) -> Result<(User, String), Box<dyn std::error::Error>> {
    // Fetch user from database
    let row = sqlx::query(
        r#"
        SELECT id, username, password_hash, role, created_at
        FROM users
        WHERE username = ?
        "#,
    )
    .bind(&login.username)
    .fetch_one(pool)
    .await
    .map_err(|_| "Invalid username or password")?;

    let user = User {
        id: row.get(0),
        username: row.get(1),
        password_hash: row.get(2),
        role: row.get(3),
        created_at: row.get(4),
    };

    // Verify password
    if !user.verify_password(&login.password) {
        return Err("Invalid username or password".into());
    }

    // Generate JWT
    let claims = Claims::new(&user);
    let token = generate_token(&claims)?;

    Ok((user, token))
}
