



use std::http::*;

use std::time::*;

use std::json::*;


#[derive(Serialize, Debug)]
struct HealthStatus {
    pub status: String,
    pub timestamp: i64,
    pub version: String,
}

#[inline]
fn check(req: &Request) -> Response {
    let now = time.now_unix();
    let health_status = HealthStatus { status: "healthy", timestamp: now, version: "0.1.0" };
    http::json_response(200, health_status)
}

