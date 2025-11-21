// Task management handlers

use axum::{
    extract::{Extension, Path, Query},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::{
    db::{projects, tasks},
    models::{
        AssignTaskRequest, CreateTaskRequest, TaskPublic, TaskSearchQuery, UpdateTaskRequest,
    },
};

pub async fn list_by_project(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
) -> Result<(StatusCode, Json<Vec<TaskPublic>>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/projects/{}/tasks", project_id);
    
    // TODO: Check user has access to project
    let user_id = 1;
    
    // Verify access
    let has_access = projects::user_has_access(&pool, project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    let task_list = tasks::list_by_project(&pool, project_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;
    
    let public_tasks: Vec<TaskPublic> = task_list.into_iter().map(|t| t.to_public()).collect();
    
    Ok((StatusCode::OK, Json(public_tasks)))
}

pub async fn create(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
    Json(body): Json<CreateTaskRequest>,
) -> Result<(StatusCode, Json<TaskPublic>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/projects/{}/tasks", project_id);
    
    // Validate
    if body.title.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Task title is required"})),
        ));
    }
    
    // TODO: Check user has access to project
    let user_id = 1;
    
    // Verify access
    let has_access = projects::user_has_access(&pool, project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    let task = tasks::create(&pool, project_id, &body)
        .await
        .map_err(|e| {
            tracing::error!("Task creation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create task"})),
            )
        })?;
    
    info!("Task created successfully: {}", task.id);
    
    Ok((StatusCode::CREATED, Json(task.to_public())))
}

pub async fn get(
    Extension(pool): Extension<PgPool>,
    Path(task_id): Path<i32>,
) -> Result<(StatusCode, Json<TaskPublic>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/tasks/{}", task_id);
    
    // TODO: Check user has access
    let user_id = 1;
    
    // Get task
    let task = tasks::find_by_id(&pool, task_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Task not found"})),
            )
        })?;
    
    // Check project access
    let has_access = projects::user_has_access(&pool, task.project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    Ok((StatusCode::OK, Json(task.to_public())))
}

pub async fn update(
    Extension(pool): Extension<PgPool>,
    Path(task_id): Path<i32>,
    Json(body): Json<UpdateTaskRequest>,
) -> Result<(StatusCode, Json<TaskPublic>), (StatusCode, Json<Value>)> {
    info!("PATCH /api/v1/tasks/{}", task_id);
    
    // TODO: Check user has access
    let user_id = 1;
    
    // Get task to check project access
    let existing_task = tasks::find_by_id(&pool, task_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Task not found"})),
            )
        })?;
    
    // Check project access
    let has_access = projects::user_has_access(&pool, existing_task.project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    let task = tasks::update(
        &pool,
        task_id,
        body.title,
        body.description,
        body.status,
        body.priority,
        body.assigned_to,
    )
    .await
    .map_err(|e| {
        tracing::error!("Task update failed: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update task"})),
        )
    })?;
    
    Ok((StatusCode::OK, Json(task.to_public())))
}

pub async fn delete(
    Extension(pool): Extension<PgPool>,
    Path(task_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    info!("DELETE /api/v1/tasks/{}", task_id);
    
    // TODO: Check user has access
    let user_id = 1;
    
    // Get task to check project access
    let existing_task = tasks::find_by_id(&pool, task_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Task not found"})),
            )
        })?;
    
    // Check project access
    let has_access = projects::user_has_access(&pool, existing_task.project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    tasks::delete(&pool, task_id)
        .await
        .map_err(|e| {
            tracing::error!("Task deletion failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete task"})),
            )
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn assign(
    Extension(pool): Extension<PgPool>,
    Path(task_id): Path<i32>,
    Json(body): Json<AssignTaskRequest>,
) -> Result<(StatusCode, Json<TaskPublic>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/tasks/{}/assign", task_id);
    
    // TODO: Check user has access
    let user_id = 1;
    
    // Get task to check project access
    let existing_task = tasks::find_by_id(&pool, task_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Task not found"})),
            )
        })?;
    
    // Check project access
    let has_access = projects::user_has_access(&pool, existing_task.project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Access check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Access check failed"})),
            )
        })?;
    
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Access denied"})),
        ));
    }
    
    let task = tasks::assign(&pool, task_id, body.user_id)
        .await
        .map_err(|e| {
            tracing::error!("Task assignment failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to assign task"})),
            )
        })?;
    
    Ok((StatusCode::OK, Json(task.to_public())))
}

pub async fn search(
    Extension(pool): Extension<PgPool>,
    Query(query): Query<TaskSearchQuery>,
) -> Result<(StatusCode, Json<Vec<TaskPublic>>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/tasks/search");
    
    let task_list = tasks::search(&pool, query)
        .await
        .map_err(|e| {
            tracing::error!("Search query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Search failed"})),
            )
        })?;
    
    let public_tasks: Vec<TaskPublic> = task_list.into_iter().map(|t| t.to_public()).collect();
    
    Ok((StatusCode::OK, Json(public_tasks)))
}

