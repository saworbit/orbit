//! Authentication models for Nebula web interface
//!
//! Implements JWT + Argon2 authentication with RBAC (Role-Based Access Control)

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// User roles for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum Role {
    /// Full system access - manage users, backends, all operations
    Admin,
    /// Create/run/cancel jobs, browse files
    Operator,
    /// Read-only access
    Viewer,
}

impl Role {
    /// Check if role has permission for an operation
    pub fn has_permission(&self, required: Role) -> bool {
        matches!(
            (self, required),
            (Role::Admin, _)
                | (Role::Operator, Role::Operator | Role::Viewer)
                | (Role::Viewer, Role::Viewer)
        )
    }

    /// Convert role to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "Admin",
            Role::Operator => "Operator",
            Role::Viewer => "Viewer",
        }
    }
}

impl std::str::FromStr for Role {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Admin" => Ok(Role::Admin),
            "Operator" => Ok(Role::Operator),
            "Viewer" => Ok(Role::Viewer),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}

/// User account
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String, // UUID as string for SQLite compatibility
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,    // Stored as string, converted to Role enum
    pub created_at: i64, // Unix timestamp
}

impl User {
    /// Get user role as enum
    pub fn get_role(&self) -> Role {
        self.role.parse().unwrap_or(Role::Viewer)
    }

    /// Create new user with hashed password
    pub fn new(
        username: String,
        password: &str,
        role: Role,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2,
        };

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(hash) => hash.to_string(),
            Err(e) => return Err(format!("Password hashing failed: {}", e).into()),
        };

        Ok(User {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash,
            role: role.as_str().to_string(),
            created_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Verify password against stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        use argon2::{
            password_hash::{PasswordHash, PasswordVerifier},
            Argon2,
        };

        let parsed_hash = match PasswordHash::new(&self.password_hash) {
            Ok(h) => h,
            Err(_) => return false,
        };

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok()
    }
}

/// Login form data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// JWT claims for authentication tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // Subject (user ID)
    pub username: String, // Username for display
    pub role: String,     // User role
    pub exp: usize,       // Expiration timestamp
}

impl Claims {
    /// Create new claims with 24-hour expiration
    pub fn new(user: &User) -> Self {
        let exp = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp() as usize;

        Claims {
            sub: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            exp,
        }
    }

    /// Get role as enum
    pub fn get_role(&self) -> Role {
        self.role.parse().unwrap_or(Role::Viewer)
    }

    /// Check if token has expired
    pub fn is_expired(&self) -> bool {
        let now = chrono::Utc::now().timestamp() as usize;
        now >= self.exp
    }
}

/// Safe user info for API responses (no password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: i64,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        UserInfo {
            id: user.id,
            username: user.username,
            role: user.role,
            created_at: user.created_at,
        }
    }
}
