// Authentication handlers

use axum::{
    extract::Extension,
    http::StatusCode,
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::{
    auth::{generate_token, hash_password, verify_password, extract_user_id_from_token},
    config::Config,
    db::users,
    models::{AuthResponse, LoginRequest, RegisterRequest},
};

pub async fn register(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/auth/register");
    
    // Validate input
    if body.username.is_empty() || body.email.is_empty() || body.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Username, email, and password are required"})),
        ));
    }
    
    if body.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Password must be at least 8 characters"})),
        ));
    }
    
    // Check if username already exists
    let username_taken = users::username_exists(&pool, &body.username)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;
    
    if username_taken {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Username already taken"})),
        ));
    }
    
    // Check if email already exists
    let email_taken = users::email_exists(&pool, &body.email)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;
    
    if email_taken {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Email already registered"})),
        ));
    }
    
    // Hash password
    let password_hash = hash_password(&body.password).map_err(|e| {
        tracing::error!("Password hashing failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Password hashing failed"})),
        )
    })?;
    
    // Create user
    let user = users::create(&pool, &body, &password_hash)
        .await
        .map_err(|e| {
            tracing::error!("User creation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create user"})),
            )
        })?;
    
    // Generate JWT token
    let token = generate_token(user.id, user.username.clone(), user.role.clone(), &config.jwt_secret)
        .map_err(|e| {
            tracing::error!("Token generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to generate token"})),
            )
        })?;
    
    info!("User registered successfully: {}", user.id);
    
    let response = AuthResponse {
        user: user.to_public(),
        token,
    };
    
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn login(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/auth/login");
    
    // Validate input
    if body.username.is_empty() || body.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Username and password are required"})),
        ));
    }
    
    // Find user by username
    let user_with_password = users::find_by_username(&pool, &body.username)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            warn!("Login failed: user not found - {}", body.username);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid username or password"})),
            )
        })?;
    
    // Verify password
    let password_valid = verify_password(&body.password, &user_with_password.password_hash)
        .map_err(|e| {
            tracing::error!("Password verification failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Password verification failed"})),
            )
        })?;
    
    if !password_valid {
        warn!("Login failed: invalid password - {}", body.username);
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Invalid username or password"})),
        ));
    }
    
    let user = user_with_password.to_user();
    
    // Generate JWT token
    let token = generate_token(user.id, user.username.clone(), user.role.clone(), &config.jwt_secret)
        .map_err(|e| {
            tracing::error!("Token generation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to generate token"})),
            )
        })?;
    
    info!("User logged in successfully: {}", user.id);
    
    let response = AuthResponse {
        user: user.to_public(),
        token,
    };
    
    Ok((StatusCode::OK, Json(response)))
}

pub async fn me(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/auth/me");
    
    // Verify token and extract user ID
    let user_id = extract_user_id_from_token(auth.token(), &config.jwt_secret)
        .map_err(|e| {
            warn!("Token validation failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired token"})),
            )
        })?;
    
    // Find user
    let user = users::find_by_id(&pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "User not found"})),
            )
        })?;
    
    Ok((StatusCode::OK, Json(json!(user.to_public()))))
}

pub async fn logout() -> StatusCode {
    info!("POST /api/v1/auth/logout");
    // In a stateless JWT system, logout is handled client-side
    StatusCode::NO_CONTENT
}
