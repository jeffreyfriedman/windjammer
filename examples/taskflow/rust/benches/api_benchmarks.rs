// Criterion microbenchmarks for TaskFlow API
//
// Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use serde_json::json;
use taskflow_api::models::{RegisterRequest, LoginRequest, CreateProjectRequest, CreateTaskRequest};

fn json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("JSON Serialization");
    
    // Benchmark user registration request
    group.bench_function("RegisterRequest", |b| {
        b.iter(|| {
            let req = RegisterRequest {
                username: black_box("testuser".to_string()),
                email: black_box("test@example.com".to_string()),
                password: black_box("password123".to_string()),
                full_name: Some(black_box("Test User".to_string())),
            };
            serde_json::to_string(&req).unwrap()
        });
    });
    
    // Benchmark project creation request
    group.bench_function("CreateProjectRequest", |b| {
        b.iter(|| {
            let req = CreateProjectRequest {
                name: black_box("Test Project".to_string()),
                description: Some(black_box("A test project".to_string())),
            };
            serde_json::to_string(&req).unwrap()
        });
    });
    
    group.finish();
}

fn json_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("JSON Deserialization");
    
    // Benchmark login request deserialization
    group.bench_function("LoginRequest", |b| {
        let json_str = r#"{"username":"testuser","password":"password123"}"#;
        b.iter(|| {
            serde_json::from_str::<LoginRequest>(black_box(json_str)).unwrap()
        });
    });
    
    // Benchmark task creation deserialization
    group.bench_function("CreateTaskRequest", |b| {
        let json_str = r#"{"title":"Test Task","description":"Description","status":"todo","priority":"medium","assigned_to":null}"#;
        b.iter(|| {
            serde_json::from_str::<CreateTaskRequest>(black_box(json_str)).unwrap()
        });
    });
    
    group.finish();
}

fn password_hashing(c: &mut Criterion) {
    use taskflow_api::auth::password::hash_password;
    
    c.bench_function("bcrypt_hash", |b| {
        b.iter(|| {
            hash_password(black_box("password123")).unwrap()
        });
    });
}

fn jwt_operations(c: &mut Criterion) {
    use taskflow_api::auth::jwt::{generate_token, verify_token};
    
    let mut group = c.benchmark_group("JWT Operations");
    
    group.bench_function("generate", |b| {
        b.iter(|| {
            generate_token(
                black_box(1),
                black_box("testuser".to_string()),
                black_box("user".to_string()),
                black_box("secret-key"),
            )
            .unwrap()
        });
    });
    
    let token = generate_token(1, "testuser".to_string(), "user".to_string(), "secret-key").unwrap();
    
    group.bench_function("verify", |b| {
        b.iter(|| {
            verify_token(black_box(&token), black_box("secret-key")).unwrap()
        });
    });
    
    group.finish();
}

// Database query building (without actual DB connection)
fn query_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("Query Building");
    
    group.bench_function("simple_select", |b| {
        b.iter(|| {
            format!(
                "SELECT id, name, email FROM users WHERE id = {}",
                black_box(123)
            )
        });
    });
    
    group.bench_function("complex_join", |b| {
        b.iter(|| {
            format!(
                "SELECT DISTINCT p.id, p.name FROM projects p \
                 LEFT JOIN project_members pm ON p.id = pm.project_id \
                 WHERE p.owner_id = {} OR pm.user_id = {}",
                black_box(123),
                black_box(123)
            )
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    json_serialization,
    json_deserialization,
    password_hashing,
    jwt_operations,
    query_building
);
criterion_main!(benches);

