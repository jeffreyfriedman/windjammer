// Database operations for users

use anyhow::Result;
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};

use crate::models::{User, UserWithPassword, RegisterRequest};

#[instrument(skip(pool, password_hash))]
pub async fn create(
    pool: &PgPool,
    req: &RegisterRequest,
    password_hash: &str,
) -> Result<User> {
    debug!("Creating user: {}", req.username);
    
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, email, password_hash, full_name, role)
        VALUES ($1, $2, $3, $4, 'user')
        RETURNING id, username, email, full_name, role, 
                  created_at, updated_at
        "#,
    )
    .bind(&req.username)
    .bind(&req.email)
    .bind(password_hash)
    .bind(&req.full_name)
    .fetch_one(pool)
    .await?;
    
    Ok(user)
}

#[instrument(skip(pool))]
pub async fn find_by_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<UserWithPassword>> {
    debug!("Finding user by username: {}", username);
    
    let user = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, username, email, password_hash, full_name, role,
               created_at, updated_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

#[instrument(skip(pool))]
pub async fn find_by_id(pool: &PgPool, user_id: i32) -> Result<Option<User>> {
    debug!("Finding user by ID: {}", user_id);
    
    let user = sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, email, full_name, role,
               created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(user)
}

#[instrument(skip(pool))]
pub async fn username_exists(pool: &PgPool, username: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE username = $1")
        .bind(username)
        .fetch_one(pool)
        .await?;
    
    let count: i64 = row.try_get("count")?;
    Ok(count > 0)
}

#[instrument(skip(pool))]
pub async fn email_exists(pool: &PgPool, email: &str) -> Result<bool> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await?;
    
    let count: i64 = row.try_get("count")?;
    Ok(count > 0)
}
