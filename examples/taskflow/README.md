# TaskFlow API - Production Validation Project

**Goal:** Empirically validate Windjammer's 80/20 thesis with a production-quality REST API.

## Overview

TaskFlow is a **production-grade task management REST API** built **twice**:
1. **Windjammer version** - Using Windjammer's stdlib abstractions
2. **Rust version** - Using Axum + SQLx directly

This allows us to compare:
- Lines of code (target: 30-40% reduction)
- Development time (target: 50% faster)
- Performance (target: 95-100% of Rust)
- Complexity (target: measurably lower)
- Maintainability

## Features

**Core Functionality:**
- User authentication (JWT tokens)
- User management
- Project management
- Task CRUD operations
- Task assignment
- Search and filtering

**Production Features:**
- PostgreSQL database
- Structured logging
- Error handling
- Request validation
- Docker deployment
- Comprehensive tests

## API Endpoints

### Authentication
- `POST /api/v1/auth/register` - Register new user
- `POST /api/v1/auth/login` - Login and get JWT
- `POST /api/v1/auth/logout` - Logout
- `GET /api/v1/auth/me` - Get current user

### Users
- `GET /api/v1/users` - List users
- `GET /api/v1/users/:id` - Get user
- `PATCH /api/v1/users/:id` - Update user
- `DELETE /api/v1/users/:id` - Delete user

### Projects
- `GET /api/v1/projects` - List projects
- `POST /api/v1/projects` - Create project
- `GET /api/v1/projects/:id` - Get project
- `PATCH /api/v1/projects/:id` - Update project
- `DELETE /api/v1/projects/:id` - Delete project
- `POST /api/v1/projects/:id/members` - Add member
- `DELETE /api/v1/projects/:id/members/:user_id` - Remove member

### Tasks
- `GET /api/v1/projects/:project_id/tasks` - List tasks
- `POST /api/v1/projects/:project_id/tasks` - Create task
- `GET /api/v1/tasks/:id` - Get task
- `PATCH /api/v1/tasks/:id` - Update task
- `DELETE /api/v1/tasks/:id` - Delete task
- `POST /api/v1/tasks/:id/assign` - Assign task
- `GET /api/v1/tasks/search` - Search tasks

### System
- `GET /health` - Health check

## Project Structure

```
taskflow/
├── windjammer/          # Windjammer implementation
│   ├── src/
│   │   ├── main.wj     # Entry point
│   │   ├── config.wj   # Configuration
│   │   ├── auth/       # Authentication
│   │   ├── models/     # Data models
│   │   ├── handlers/   # HTTP handlers
│   │   ├── db/         # Database layer
│   │   └── utils/      # Utilities
│   ├── migrations/     # SQL migrations
│   └── wj.toml        # Config
├── rust/                # Rust implementation (same structure)
└── benchmarks/          # Performance tests
```

## Implementation Status

### Phase 1: Foundation ✅ (In Progress)
- [x] Project structure
- [x] Database schema
- [x] Health endpoint
- [x] Configuration
- [x] JWT authentication
- [x] Password hashing
- [ ] User registration
- [ ] User login
- [ ] Rust equivalent

### Phase 2: Core Features (Pending)
- [ ] User CRUD
- [ ] Project CRUD
- [ ] Task CRUD
- [ ] Authorization

### Phase 3: Advanced Features (Pending)
- [ ] Search and filtering
- [ ] Pagination
- [ ] Validation

### Phase 4: Production Ready (Pending)
- [ ] Tests
- [ ] Docker
- [ ] Documentation

### Phase 5: Analysis (Pending)
- [ ] Metrics comparison
- [ ] Performance benchmarks
- [ ] Comparison report

## Running the Project

### Windjammer Version

```bash
cd windjammer

# Set up database
createdb taskflow
psql taskflow < migrations/001_initial_schema.sql

# Set environment variables
export DATABASE_URL="postgresql://localhost/taskflow"
export JWT_SECRET="your-secret-key"

# Run the server
wj run src/main.wj
```

### Rust Version

```bash
cd rust
cargo run
```

## Expected Results

| Metric | Windjammer | Rust | Difference |
|--------|------------|------|------------|
| Lines of Code | ~1,500-2,000 | ~2,500-3,500 | **-40%** |
| Dev Time | ~60-80 hrs | ~120-150 hrs | **-50%** |
| Performance | 95-100% | 100% | ~5% |
| Complexity | Lower | Higher | Simpler |

## Current Progress

**Completed:**
- ✅ Project structure
- ✅ Database schema
- ✅ Health endpoint
- ✅ Configuration management
- ✅ JWT token generation/verification
- ✅ Password hashing (bcrypt)
- ✅ User data models

**Next Steps:**
- Complete user registration endpoint
- Complete user login endpoint
- Build Rust equivalent for comparison
- Calculate initial LOC metrics

## Metrics Tracking

We're tracking:
- **LOC:** Total lines of code (excluding comments/blank lines)
- **Development Time:** Hours spent on each implementation
- **Complexity:** Cyclomatic complexity per function
- **Performance:** RPS, latency (p50/p95/p99), memory usage
- **Dependencies:** Count of direct dependencies

**Current Status:** Foundation phase in progress. Full metrics will be available after Phase 1 completion.

---

**See:** `docs/V160_PLAN.md` for complete implementation plan.

