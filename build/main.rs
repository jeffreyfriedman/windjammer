




use axum::Router;

use axum::Json;

use axum::extract::Path;

use serde::Serialize;

use serde::Deserialize;

use tokio::*;


static mut USERS: Vec<User> = /* expression */;
static mut NEXT_ID: i64 = 1;

struct User {
    id: i64,
    name: String,
    email: String,
}

struct CreateUserRequest {
    name: String,
    email: String,
}

#[inline]
#[axum::routing::get]
fn index() -> String {
    "Welcome to Windjammer API!"
}

#[inline]
#[axum::routing::get]
fn list_users() -> Json<Vec<User>> {
    let users = {
        USERS::clone()
    };
    Json(users)
}

#[inline]
#[axum::routing::get]
fn get_user(mut id: Path<i64>) -> Result<Json<User>, StatusCode> {
    let users = {
        &USERS
    };
    match users.iter().find(move |u| u.id == *id) {
        Some(user) => Ok(Json(user.clone())),
        None => Err(StatusCode::NOT_FOUND),
    }
}

#[inline]
#[axum::routing::post]
#[timing]
fn create_user(mut req: Json<CreateUserRequest>) -> Result<Json<User>, StatusCode> {
    {
        let user = User { id: NEXT_ID, name: req.name.clone(), email: req.email.clone() };
        NEXT_ID = NEXT_ID + 1;
        USERS::push(user.clone());
        Ok(Json(user))
    }
}

#[inline]
#[axum::routing::delete]
fn delete_user(mut id: Path<i64>) -> Result<StatusCode, StatusCode> {
    {
        let original_len = USERS::len();
        USERS::retain(move |u| u.id != *id);
        if USERS::len() < original_len {
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }
}

#[inline]
#[middleware]
fn auth_middleware(mut req: Request, mut next: Next) -> Response {
    match req.headers().get("X-API-Key") {
        Some(key) if key == "secret-key" => next.run(req).await,
        _ => Response::builder().status(401).body("Unauthorized").unwrap(),
    }
}

#[tokio.main]
fn main() {
    {
        USERS::push(User { id: 1, name: "Alice", email: "alice@example.com" });
        USERS::push(User { id: 2, name: "Bob", email: "bob@example.com" });
        NEXT_ID = 3;
    };
    let app = Router::new().route("/", get(index)).route("/users", get(list_users).post(create_user)).route("/users/:id", get(get_user).delete(delete_user)).layer(auth_middleware);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000".to_string()).await.unwrap();
    println!("Server running on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap()
}

