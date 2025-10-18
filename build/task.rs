use std::time::*;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<i64>,
    pub created_by: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub due_date: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub priority: String,
    pub due_date: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AssignTaskRequest {
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskResponse {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<i64>,
    pub created_by: i64,
    pub created_at: i64,
    pub due_date: Option<i64>,
}

impl Task {
#[inline]
fn to_response(self) -> TaskResponse {
        TaskResponse { id: self.id, project_id: self.project_id, title: self.title, description: self.description, status: self.status, priority: self.priority, assigned_to: self.assigned_to, created_by: self.created_by, created_at: self.created_at, due_date: self.due_date }
}
}

