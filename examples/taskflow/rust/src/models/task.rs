// Task data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: i32,
    pub project_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct AssignTaskRequest {
    pub user_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct TaskSearchQuery {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub assigned_to: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TaskPublic {
    pub id: i32,
    pub project_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

impl Task {
    pub fn to_public(self) -> TaskPublic {
        TaskPublic {
            id: self.id,
            project_id: self.project_id,
            title: self.title,
            description: self.description,
            status: self.status,
            priority: self.priority,
            assigned_to: self.assigned_to,
            created_at: self.created_at.to_rfc3339(),
            updated_at: self.updated_at.to_rfc3339(),
        }
    }
}

