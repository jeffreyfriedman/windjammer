// Project management handlers

use axum::{
    extract::{Extension, Path},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tracing::{info, warn};

use crate::{
    config::Config,
    db::projects,
    models::{AddMemberRequest, CreateProjectRequest, ProjectPublic, UpdateProjectRequest},
};

pub async fn list(
    Extension(pool): Extension<PgPool>,
) -> Result<(StatusCode, Json<Vec<ProjectPublic>>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/projects");
    
    // TODO: Extract user ID from JWT token
    let user_id = 1;
    
    let project_list = projects::list_for_user(&pool, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Database query failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Database error"})),
            )
        })?;
    
    let public_projects: Vec<ProjectPublic> = project_list
        .into_iter()
        .map(|p| p.to_public())
        .collect();
    
    Ok((StatusCode::OK, Json(public_projects)))
}

pub async fn create(
    Extension(pool): Extension<PgPool>,
    Extension(config): Extension<Config>,
    Json(body): Json<CreateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectPublic>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/projects");
    
    // Validate
    if body.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Project name is required"})),
        ));
    }
    
    // TODO: Extract user ID from JWT token
    let owner_id = 1;
    
    let project = projects::create(&pool, &body, owner_id)
        .await
        .map_err(|e| {
            tracing::error!("Project creation failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create project"})),
            )
        })?;
    
    info!("Project created successfully: {}", project.id);
    
    Ok((StatusCode::CREATED, Json(project.to_public())))
}

pub async fn get(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
) -> Result<(StatusCode, Json<ProjectPublic>), (StatusCode, Json<Value>)> {
    info!("GET /api/v1/projects/{}", project_id);
    
    // TODO: Extract user ID from JWT and check access
    let user_id = 1;
    
    // Check access
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
    
    // Get project
    let project = projects::find_by_id(&pool, project_id)
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
                Json(json!({"error": "Project not found"})),
            )
        })?;
    
    Ok((StatusCode::OK, Json(project.to_public())))
}

pub async fn update(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
    Json(body): Json<UpdateProjectRequest>,
) -> Result<(StatusCode, Json<ProjectPublic>), (StatusCode, Json<Value>)> {
    info!("PATCH /api/v1/projects/{}", project_id);
    
    // TODO: Extract user ID from JWT and check if owner
    let user_id = 1;
    
    // Check if user is owner
    let is_owner = projects::is_owner(&pool, project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Owner check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Owner check failed"})),
            )
        })?;
    
    if !is_owner {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only project owner can update"})),
        ));
    }
    
    let project = projects::update(&pool, project_id, body.name, body.description)
        .await
        .map_err(|e| {
            tracing::error!("Project update failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update project"})),
            )
        })?;
    
    Ok((StatusCode::OK, Json(project.to_public())))
}

pub async fn delete(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    info!("DELETE /api/v1/projects/{}", project_id);
    
    // TODO: Extract user ID from JWT and check if owner
    let user_id = 1;
    
    // Check if user is owner
    let is_owner = projects::is_owner(&pool, project_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!("Owner check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Owner check failed"})),
            )
        })?;
    
    if !is_owner {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only project owner can delete"})),
        ));
    }
    
    projects::delete(&pool, project_id)
        .await
        .map_err(|e| {
            tracing::error!("Project deletion failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete project"})),
            )
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_member(
    Extension(pool): Extension<PgPool>,
    Path(project_id): Path<i32>,
    Json(body): Json<AddMemberRequest>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    info!("POST /api/v1/projects/{}/members", project_id);
    
    // TODO: Extract user ID from JWT and check if owner
    let owner_id = 1;
    
    // Check if user is owner
    let is_owner = projects::is_owner(&pool, project_id, owner_id)
        .await
        .map_err(|e| {
            tracing::error!("Owner check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Owner check failed"})),
            )
        })?;
    
    if !is_owner {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only project owner can add members"})),
        ));
    }
    
    let member = projects::add_member(&pool, project_id, body.user_id, body.role)
        .await
        .map_err(|e| {
            tracing::error!("Add member failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add member"})),
            )
        })?;
    
    Ok((StatusCode::CREATED, Json(json!(member))))
}

pub async fn remove_member(
    Extension(pool): Extension<PgPool>,
    Path((project_id, member_user_id)): Path<(i32, i32)>,
) -> Result<StatusCode, (StatusCode, Json<Value>)> {
    info!(
        "DELETE /api/v1/projects/{}/members/{}",
        project_id, member_user_id
    );
    
    // TODO: Extract user ID from JWT and check if owner
    let owner_id = 1;
    
    // Check if user is owner
    let is_owner = projects::is_owner(&pool, project_id, owner_id)
        .await
        .map_err(|e| {
            tracing::error!("Owner check failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Owner check failed"})),
            )
        })?;
    
    if !is_owner {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({"error": "Only project owner can remove members"})),
        ));
    }
    
    projects::remove_member(&pool, project_id, member_user_id)
        .await
        .map_err(|e| {
            tracing::error!("Remove member failed: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to remove member"})),
            )
        })?;
    
    Ok(StatusCode::NO_CONTENT)
}

