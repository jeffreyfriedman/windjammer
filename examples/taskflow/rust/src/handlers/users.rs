// User management handlers

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::info;

use crate::db::users;

pub async fn list(Extension(pool): Extension<PgPool>) -> (StatusCode, Json<Value>) {
    info!("GET /api/v1/users");
    
    // For now, return empty list (would need proper admin auth check)
    (StatusCode::OK, Json(json!([])))
}

pub async fn get(
    Extension(pool): Extension<PgPool>,
    Path(user_id): Path<i32>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/users/{}", user_id);
    
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

pub async fn update(
    Extension(pool): Extension<PgPool>,
    Path(user_id): Path<i32>,
) -> (StatusCode, Json<Value>) {
    info!("PATCH /api/v1/users/{}", user_id);
    
    // TODO: Implement update logic
    (
        StatusCode::OK,
        Json(json!({"message": "Update not yet implemented"})),
    )
}

pub async fn delete(
    Extension(pool): Extension<PgPool>,
    Path(user_id): Path<i32>,
) -> (StatusCode, Json<Value>) {
    info!("DELETE /api/v1/users/{}", user_id);
    
    // Admin-only operation, not implemented
    (
        StatusCode::OK,
        Json(json!({"message": "Delete not yet implemented (admin only)"})),
    )
}

