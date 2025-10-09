// TaskFlow API - Rust Implementation
// A production-quality task management REST API

mod auth;
mod config;
mod db;
mod handlers;
mod models;

use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{delete, get, patch, post},
    Router,
};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing::{info, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::Config;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "taskflow_api=info,tower_http=debug,axum::rejection=trace".into()
        }))
        .init();

    info!("Starting TaskFlow API (Rust)");

    // Load configuration
    let config = Config::load();
    info!("Configuration loaded - port: {}", config.port);

    // Connect to database
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    info!("Database connected");

    // Build router
    let app = Router::new()
        // Health check
        .route("/health", get(handlers::health::check))
        // Auth endpoints
        .route("/api/v1/auth/register", post(handlers::auth::register))
        .route("/api/v1/auth/login", post(handlers::auth::login))
        .route("/api/v1/auth/me", get(handlers::auth::me))
        .route("/api/v1/auth/logout", post(handlers::auth::logout))
        // User endpoints
        .route("/api/v1/users", get(handlers::users::list))
        .route("/api/v1/users/:id", get(handlers::users::get))
        .route("/api/v1/users/:id", patch(handlers::users::update))
        .route("/api/v1/users/:id", delete(handlers::users::delete))
        // Project endpoints
        .route("/api/v1/projects", get(handlers::projects::list))
        .route("/api/v1/projects", post(handlers::projects::create))
        .route("/api/v1/projects/:id", get(handlers::projects::get))
        .route("/api/v1/projects/:id", patch(handlers::projects::update))
        .route("/api/v1/projects/:id", delete(handlers::projects::delete))
        .route(
            "/api/v1/projects/:id/members",
            post(handlers::projects::add_member),
        )
        .route(
            "/api/v1/projects/:id/members/:user_id",
            delete(handlers::projects::remove_member),
        )
        // Task endpoints
        .route(
            "/api/v1/projects/:project_id/tasks",
            get(handlers::tasks::list_by_project),
        )
        .route(
            "/api/v1/projects/:project_id/tasks",
            post(handlers::tasks::create),
        )
        .route("/api/v1/tasks/:id", get(handlers::tasks::get))
        .route("/api/v1/tasks/:id", patch(handlers::tasks::update))
        .route("/api/v1/tasks/:id", delete(handlers::tasks::delete))
        .route("/api/v1/tasks/:id/assign", post(handlers::tasks::assign))
        .route("/api/v1/tasks/search", get(handlers::tasks::search))
        // Share state
        .layer(Extension(pool))
        .layer(Extension(config.clone()));

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .expect("Invalid address");

    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}