use std::time::*;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateUserRequest {
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: i64,
}

impl User {
#[inline]
fn to_response(self) -> UserResponse {
        UserResponse { id: self.id, username: self.username, email: self.email, created_at: self.created_at }
}
}

