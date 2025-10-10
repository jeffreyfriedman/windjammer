// Database operations for projects

use anyhow::Result;
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};

use crate::models::{Project, CreateProjectRequest, ProjectMember};

#[instrument(skip(pool))]
pub async fn create(
    pool: &PgPool,
    req: &CreateProjectRequest,
    owner_id: i32,
) -> Result<Project> {
    debug!("Creating project: {}", req.name);
    
    let project = sqlx::query_as::<_, Project>(
        r#"
        INSERT INTO projects (owner_id, name, description)
        VALUES ($1, $2, $3)
        RETURNING id, owner_id, name, description, created_at, updated_at
        "#,
    )
    .bind(owner_id)
    .bind(&req.name)
    .bind(&req.description)
    .fetch_one(pool)
    .await?;
    
    Ok(project)
}

#[instrument(skip(pool))]
pub async fn find_by_id(pool: &PgPool, project_id: i32) -> Result<Option<Project>> {
    debug!("Finding project by ID: {}", project_id);
    
    let project = sqlx::query_as::<_, Project>(
        r#"
        SELECT id, owner_id, name, description, created_at, updated_at
        FROM projects
        WHERE id = $1
        "#,
    )
    .bind(project_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(project)
}

#[instrument(skip(pool))]
pub async fn list_for_user(pool: &PgPool, user_id: i32) -> Result<Vec<Project>> {
    debug!("Listing projects for user: {}", user_id);
    
    let projects = sqlx::query_as::<_, Project>(
        r#"
        SELECT DISTINCT p.id, p.owner_id, p.name, p.description, p.created_at, p.updated_at
        FROM projects p
        LEFT JOIN project_members pm ON p.id = pm.project_id
        WHERE p.owner_id = $1 OR pm.user_id = $1
        ORDER BY p.updated_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    
    Ok(projects)
}

#[instrument(skip(pool))]
pub async fn update(
    pool: &PgPool,
    project_id: i32,
    name: Option<String>,
    description: Option<String>,
) -> Result<Project> {
    debug!("Updating project: {}", project_id);
    
    let project = sqlx::query_as::<_, Project>(
        r#"
        UPDATE projects
        SET name = COALESCE($2, name),
            description = COALESCE($3, description),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, owner_id, name, description, created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(name)
    .bind(description)
    .fetch_one(pool)
    .await?;
    
    Ok(project)
}

#[instrument(skip(pool))]
pub async fn delete(pool: &PgPool, project_id: i32) -> Result<()> {
    debug!("Deleting project: {}", project_id);
    
    sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

#[instrument(skip(pool))]
pub async fn user_has_access(pool: &PgPool, project_id: i32, user_id: i32) -> Result<bool> {
    let row = sqlx::query(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM projects WHERE id = $1 AND owner_id = $2
            UNION
            SELECT 1 FROM project_members WHERE project_id = $1 AND user_id = $2
        ) as has_access
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    let has_access: bool = row.try_get("has_access")?;
    Ok(has_access)
}

#[instrument(skip(pool))]
pub async fn is_owner(pool: &PgPool, project_id: i32, user_id: i32) -> Result<bool> {
    let row = sqlx::query(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id = $1 AND owner_id = $2) as is_owner",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    let is_owner: bool = row.try_get("is_owner")?;
    Ok(is_owner)
}

#[instrument(skip(pool))]
pub async fn add_member(
    pool: &PgPool,
    project_id: i32,
    user_id: i32,
    role: String,
) -> Result<ProjectMember> {
    debug!("Adding member to project: {}", project_id);
    
    let member = sqlx::query_as::<_, ProjectMember>(
        r#"
        INSERT INTO project_members (project_id, user_id, role)
        VALUES ($1, $2, $3)
        RETURNING project_id, user_id, role, created_at
        "#,
    )
    .bind(project_id)
    .bind(user_id)
    .bind(role)
    .fetch_one(pool)
    .await?;
    
    Ok(member)
}

#[instrument(skip(pool))]
pub async fn remove_member(pool: &PgPool, project_id: i32, user_id: i32) -> Result<()> {
    debug!("Removing member from project: {}", project_id);
    
    sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND user_id = $2")
        .bind(project_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

#[instrument(skip(pool))]
pub async fn list_members(pool: &PgPool, project_id: i32) -> Result<Vec<ProjectMember>> {
    debug!("Listing project members: {}", project_id);
    
    let members = sqlx::query_as::<_, ProjectMember>(
        r#"
        SELECT project_id, user_id, role, created_at
        FROM project_members
        WHERE project_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    
    Ok(members)
}

