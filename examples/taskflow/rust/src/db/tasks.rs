// Database operations for tasks

use anyhow::Result;
use sqlx::{PgPool, Row, QueryBuilder, Postgres};
use tracing::{debug, instrument};

use crate::models::{Task, CreateTaskRequest, TaskSearchQuery};

#[instrument(skip(pool))]
pub async fn create(
    pool: &PgPool,
    project_id: i32,
    req: &CreateTaskRequest,
) -> Result<Task> {
    debug!("Creating task: {}", req.title);
    
    let task = sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (project_id, title, description, status, priority, assigned_to)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, project_id, title, description, status, priority, assigned_to,
                  created_at, updated_at
        "#,
    )
    .bind(project_id)
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.status.as_ref().unwrap_or(&"todo".to_string()))
    .bind(req.priority.as_ref().unwrap_or(&"medium".to_string()))
    .bind(&req.assigned_to)
    .fetch_one(pool)
    .await?;
    
    Ok(task)
}

#[instrument(skip(pool))]
pub async fn find_by_id(pool: &PgPool, task_id: i32) -> Result<Option<Task>> {
    debug!("Finding task by ID: {}", task_id);
    
    let task = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, project_id, title, description, status, priority, assigned_to,
               created_at, updated_at
        FROM tasks
        WHERE id = $1
        "#,
    )
    .bind(task_id)
    .fetch_optional(pool)
    .await?;
    
    Ok(task)
}

#[instrument(skip(pool))]
pub async fn list_by_project(pool: &PgPool, project_id: i32) -> Result<Vec<Task>> {
    debug!("Listing tasks for project: {}", project_id);
    
    let tasks = sqlx::query_as::<_, Task>(
        r#"
        SELECT id, project_id, title, description, status, priority, assigned_to,
               created_at, updated_at
        FROM tasks
        WHERE project_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    
    Ok(tasks)
}

#[instrument(skip(pool))]
pub async fn update(
    pool: &PgPool,
    task_id: i32,
    title: Option<String>,
    description: Option<String>,
    status: Option<String>,
    priority: Option<String>,
    assigned_to: Option<i32>,
) -> Result<Task> {
    debug!("Updating task: {}", task_id);
    
    let task = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET title = COALESCE($2, title),
            description = COALESCE($3, description),
            status = COALESCE($4, status),
            priority = COALESCE($5, priority),
            assigned_to = COALESCE($6, assigned_to),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, project_id, title, description, status, priority, assigned_to,
                  created_at, updated_at
        "#,
    )
    .bind(task_id)
    .bind(title)
    .bind(description)
    .bind(status)
    .bind(priority)
    .bind(assigned_to)
    .fetch_one(pool)
    .await?;
    
    Ok(task)
}

#[instrument(skip(pool))]
pub async fn delete(pool: &PgPool, task_id: i32) -> Result<()> {
    debug!("Deleting task: {}", task_id);
    
    sqlx::query("DELETE FROM tasks WHERE id = $1")
        .bind(task_id)
        .execute(pool)
        .await?;
    
    Ok(())
}

#[instrument(skip(pool))]
pub async fn assign(pool: &PgPool, task_id: i32, user_id: i32) -> Result<Task> {
    debug!("Assigning task: {}", task_id);
    
    let task = sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET assigned_to = $2,
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, project_id, title, description, status, priority, assigned_to,
                  created_at, updated_at
        "#,
    )
    .bind(task_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    
    Ok(task)
}

#[instrument(skip(pool))]
pub async fn search(pool: &PgPool, query: TaskSearchQuery) -> Result<Vec<Task>> {
    debug!("Searching tasks with filters");
    
    let mut sql_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT id, project_id, title, description, status, priority, assigned_to, created_at, updated_at FROM tasks WHERE 1=1"
    );
    
    if let Some(status) = query.status {
        sql_builder.push(" AND status = ");
        sql_builder.push_bind(status);
    }
    
    if let Some(priority) = query.priority {
        sql_builder.push(" AND priority = ");
        sql_builder.push_bind(priority);
    }
    
    if let Some(assigned_to) = query.assigned_to {
        sql_builder.push(" AND assigned_to = ");
        sql_builder.push_bind(assigned_to);
    }
    
    sql_builder.push(" ORDER BY created_at DESC");
    
    let tasks = sql_builder
        .build_query_as::<Task>()
        .fetch_all(pool)
        .await?;
    
    Ok(tasks)
}

