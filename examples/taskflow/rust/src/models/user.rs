// User data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub full_name: Option<String>,
    pub role: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub full_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub user: UserPublic,
    pub token: String,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserPublic {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub full_name: Option<String>,
    pub role: String,
    pub created_at: String,
}

impl User {
    pub fn to_public(self) -> UserPublic {
        UserPublic {
            id: self.id,
            username: self.username,
            email: self.email,
            full_name: self.full_name,
            role: self.role,
            created_at: self.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserWithPassword {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub full_name: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserWithPassword {
    pub fn to_user(self) -> User {
        User {
            id: self.id,
            username: self.username,
            email: self.email,
            full_name: self.full_name,
            role: self.role,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

