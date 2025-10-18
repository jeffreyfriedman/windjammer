use std::time::*;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct Project {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub owner_id: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateProjectRequest {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectResponse {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub owner_id: i64,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectMember {
    pub project_id: i64,
    pub user_id: i64,
    pub role: String,
    pub added_at: i64,
}

impl Project {
#[inline]
fn to_response(self) -> ProjectResponse {
        ProjectResponse { id: self.id, name: self.name, description: self.description, owner_id: self.owner_id, created_at: self.created_at }
}
}

