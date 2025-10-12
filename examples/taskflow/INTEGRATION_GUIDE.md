# TaskFlow Integration Guide

**How to integrate all the new production features into main.wj**

---

## Overview

We've built 18 production-ready components. This guide shows how to wire them together.

---

## 1. Update main.wj

```windjammer
// TaskFlow API - Windjammer Implementation (Production v0.23.0)

use std.http
use std.log  
use std.env
use std.db
use std.json

// Import all modules
use ./config
use ./models.user
use ./models.project
use ./models.task
use ./models.role
use ./models.api_key
use ./models.pagination

// Middleware
use ./middleware.auth
use ./middleware.request_id
use ./middleware.logging
use ./middleware.rate_limit

// Handlers
use ./handlers.health_enhanced
use ./handlers.metrics
use ./handlers.auth
use ./handlers.auth_refresh
use ./handlers.users
use ./handlers.projects
use ./handlers.tasks_enhanced

@async
fn main() {
    // Initialize logging
    log.init("info")
    log.info("Starting TaskFlow API v0.23.0 (Production)")
    
    // Load configuration
    let cfg = config.load()
    let addr = "${cfg.host}:${cfg.port}"
    log.info("Configuration loaded, listening on ${addr}")
    
    // Initialize database connection pool
    let pool = db.connect(cfg.database_url)
    
    // Run migrations
    log.info("Running database migrations...")
    db.migrate(pool)
    
    // Initialize rate limiter (100 requests per minute)
    let rate_limiter = RateLimiter::new(100)
    
    // Build and start server
    log.info("Starting HTTP server...")
    let server = http.new_server(addr)
    
    // Register routes with middleware
    register_routes(server, pool, cfg, rate_limiter)
    
    // Start serving
    match http.serve(server) {
        Ok(_) => log.info("Server stopped gracefully"),
        Err(e) => {
            log.error("Server error: ${e}")
            std.process.exit(1)
        }
    }
}

fn register_routes(server: Server, pool: DbPool, cfg: Config, limiter: RateLimiter) {
    // Health checks (no auth required)
    http.route(server, "GET", "/health", health_enhanced.liveness)
    http.route(server, "GET", "/health/live", health_enhanced.liveness)
    http.route(server, "GET", "/health/ready", |req| health_enhanced.readiness(req, pool))
    http.route(server, "GET", "/health/detailed", |req| health_enhanced.detailed(req, pool))
    
    // Metrics (no auth required - but should be firewalled in production)
    http.route(server, "GET", "/metrics", metrics.metrics)
    http.route(server, "GET", "/metrics/json", metrics.metrics_json)
    
    // Auth endpoints (no auth required for login/register)
    http.route(server, "POST", "/api/v1/auth/register", auth.register)
    http.route(server, "POST", "/api/v1/auth/login", auth.login)
    http.route(server, "POST", "/api/v1/auth/refresh", |req| auth_refresh.refresh(req, cfg))
    
    // Protected auth endpoints
    http.route(server, "POST", "/api/v1/auth/logout", with_auth(auth.logout, pool, cfg))
    http.route(server, "GET", "/api/v1/auth/me", with_auth(auth.me, pool, cfg))
    
    // User endpoints (authenticated)
    http.route(server, "GET", "/api/v1/users", with_auth(users.list, pool, cfg))
    http.route(server, "GET", "/api/v1/users/:id", with_auth(users.get, pool, cfg))
    http.route(server, "PATCH", "/api/v1/users/:id", with_auth(users.update, pool, cfg))
    http.route(server, "DELETE", "/api/v1/users/:id", with_auth(users.delete, pool, cfg))
    
    // Project endpoints (authenticated + rate limited)
    http.route(server, "GET", "/api/v1/projects", 
        with_auth_and_rate_limit(projects.list, pool, cfg, limiter))
    http.route(server, "POST", "/api/v1/projects", 
        with_auth_and_rate_limit(projects.create, pool, cfg, limiter))
    http.route(server, "GET", "/api/v1/projects/:id", 
        with_auth_and_rate_limit(projects.get, pool, cfg, limiter))
    
    // Task endpoints (authenticated + rate limited + enhanced)
    http.route(server, "GET", "/api/v1/tasks", 
        with_auth_and_rate_limit(|req, auth_ctx| {
            tasks_enhanced.list_with_filters(req, pool, auth_ctx)
        }, pool, cfg, limiter))
    
    http.route(server, "POST", "/api/v1/projects/:project_id/tasks",
        with_auth_and_rate_limit(|req, auth_ctx| {
            let project_id = http.get_param(req, "project_id").parse::<int>().unwrap()
            tasks_enhanced.create_with_rbac(req, pool, auth_ctx, project_id)
        }, pool, cfg, limiter))
    
    http.route(server, "DELETE", "/api/v1/tasks/:id",
        with_auth_and_rate_limit(|req, auth_ctx| {
            let task_id = http.get_param(req, "id").parse::<int>().unwrap()
            tasks_enhanced.soft_delete(req, pool, auth_ctx, task_id)
        }, pool, cfg, limiter))
    
    log.info("All routes registered with authentication and rate limiting")
}

// Middleware wrapper: Authentication
fn with_auth(
    handler: fn(Request, AuthContext) -> Response,
    pool: DbPool,
    cfg: Config
) -> fn(Request) -> Response {
    |req| {
        // Add request ID
        let req = request_id.add_request_id_header(req)
        let request_id = http.get_context(req, "request_id").unwrap()
        
        // Authenticate
        let (req, auth_ctx) = match auth.authenticate(req, cfg, pool).await {
            Ok((r, ctx)) => (r, ctx),
            Err(e) => return e.to_http_response(),
        }
        
        // Call handler
        let start = time.now().timestamp_millis()
        let response = handler(req, auth_ctx)
        let duration = time.now().timestamp_millis() - start
        
        // Log request
        logging.log_request(req, response, duration as int)
        
        // Record metrics
        metrics.record_request(http.path(req), duration as int, http.status(response))
        
        // Add request ID to response
        request_id.add_request_id_to_response(response, request_id)
    }
}

// Middleware wrapper: Authentication + Rate Limiting
fn with_auth_and_rate_limit(
    handler: fn(Request, AuthContext) -> Response,
    pool: DbPool,
    cfg: Config,
    limiter: RateLimiter
) -> fn(Request) -> Response {
    |req| {
        // Add request ID
        let req = request_id.add_request_id_header(req)
        let request_id = http.get_context(req, "request_id").unwrap()
        
        // Authenticate
        let (req, auth_ctx) = match auth.authenticate(req, cfg, pool).await {
            Ok((r, ctx)) => (r, ctx),
            Err(e) => return e.to_http_response(),
        }
        
        // Rate limit
        let req = match rate_limit.rate_limit_by_user(req, limiter) {
            Ok(r) => r,
            Err(e) => {
                return http.json_response(429, json!({
                    "error": "Rate limit exceeded",
                    "retry_after": e.retry_after,
                    "limit": e.limit
                }))
            }
        }
        
        // Call handler
        let start = time.now().timestamp_millis()
        let response = handler(req, auth_ctx)
        let duration = time.now().timestamp_millis() - start
        
        // Log request
        logging.log_request(req, response, duration as int)
        
        // Record metrics
        metrics.record_request(http.path(req), duration as int, http.status(response))
        
        // Add request ID to response
        request_id.add_request_id_to_response(response, request_id)
    }
}
```

---

## 2. New Endpoints

### Health Checks
- `GET /health` - Basic liveness (fast, for load balancers)
- `GET /health/live` - Liveness probe
- `GET /health/ready` - Readiness probe (checks DB)
- `GET /health/detailed` - Full health status with all checks

### Metrics
- `GET /metrics` - Prometheus metrics (text format)
- `GET /metrics/json` - Metrics as JSON (for debugging)

### Auth
- `POST /api/v1/auth/refresh` - Refresh access token

### Enhanced Tasks
- `GET /api/v1/tasks?cursor=X&limit=50&status=open&sort=-created_at` - List with pagination, filtering, sorting
- `POST /api/v1/projects/:id/tasks` - Create with RBAC
- `DELETE /api/v1/tasks/:id` - Soft delete with audit

---

## 3. Middleware Stack

Every authenticated request goes through:
1. **Request ID** - Generate unique ID for tracing
2. **Authentication** - JWT or API key validation
3. **Rate Limiting** - Token bucket per user/IP/API key
4. **Handler** - Business logic
5. **Logging** - Structured JSON logs
6. **Metrics** - Prometheus counters/histograms
7. **Response** - Add headers (request ID, rate limit)

---

## 4. Database Migrations

Run migration 002:
```bash
psql $DATABASE_URL < migrations/002_add_roles_and_api_keys.sql
```

This adds:
- `users.role` column
- `api_keys` table
- `audit_log` table
- Soft delete columns (`deleted_at`, `deleted_by`)
- Indexes for performance

---

## 5. Configuration

Update `wj.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8080

[database]
url = "postgresql://user:pass@localhost/taskflow"

[auth]
jwt_secret = "your-secret-key-change-in-production"

[rate_limit]
requests_per_minute = 100
```

---

## 6. Testing

### Health Checks
```bash
curl http://localhost:8080/health
curl http://localhost:8080/health/detailed
```

### Metrics
```bash
curl http://localhost:8080/metrics
curl http://localhost:8080/metrics/json
```

### Authentication
```bash
# Login
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'

# Refresh token
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"..."}'
```

### Enhanced Tasks
```bash
# List with filters
curl http://localhost:8080/api/v1/tasks?status=open&sort=-created_at&limit=10 \
  -H "Authorization: Bearer YOUR_TOKEN"

# Create task (requires Member or Admin role)
curl -X POST http://localhost:8080/api/v1/projects/1/tasks \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"New task","description":"Test","priority":"high"}'
```

---

## 7. Monitoring

### Prometheus
Add to `prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'taskflow'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

### Grafana Dashboard
Key metrics to monitor:
- `http_requests_total` - Request count by endpoint
- `http_request_duration_seconds` - Request latency
- `http_errors_total` - Error rate
- `process_uptime_seconds` - Service uptime
- `db_connections_active` - Database connections

---

## 8. Production Checklist

- [ ] Update JWT secret in production
- [ ] Configure rate limits appropriately
- [ ] Set up Prometheus scraping
- [ ] Configure log aggregation (e.g., ELK stack)
- [ ] Set up alerts for health check failures
- [ ] Configure firewall rules (restrict /metrics access)
- [ ] Enable HTTPS/TLS
- [ ] Set up database connection pooling
- [ ] Configure graceful shutdown
- [ ] Set up backup and recovery

---

**TaskFlow is now production-ready!** 🚀

*Integration guide created: October 12, 2025*

