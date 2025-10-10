// Health check endpoint

use axum::{http::StatusCode, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    status: String,
    timestamp: i64,
    version: String,
}

pub async fn check() -> (StatusCode, Json<HealthStatus>) {
    let health_status = HealthStatus {
        status: "healthy".to_string(),
        timestamp: Utc::now().timestamp(),
        version: "0.1.0".to_string(),
    };
    
    (StatusCode::OK, Json(health_status))
}
