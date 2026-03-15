//! TDD Test: E0614 Entity "cannot be dereferenced" fix
//!
//! Fixes: entity::Entity cannot be dereferenced when entity comes from:
//! - for (entity, mesh, transform) in entities { render_mesh(entity, ...) }
//! - for entity in entities { process(entity) } where Entity is Copy
//!
//! Root cause: Entity (ecs/entity.wj) has @derive(Copy). When we add * for "reference coercion",
//! we wrongly dereference owned Entity. Fix: is_known_copy_type("Entity") + tuple pattern
//! local_var_types population.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn compile_wj_to_rust(source: &str) -> (String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "wj_e0614_entity_{}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        id
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let wj_file = dir.join("test.wj");
    std::fs::write(&wj_file, source).unwrap();

    let _output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            wj_file.to_str().unwrap(),
            "--output",
            dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("Failed to run wj compiler");

    let src_dir = dir.join("src");
    let main_rs = if src_dir.join("main.rs").exists() {
        src_dir.join("main.rs")
    } else {
        dir.join("test.rs")
    };

    let rs_content = std::fs::read_to_string(&main_rs).unwrap_or_default();

    let rlib_output = dir.join("test.rlib");
    let rustc = Command::new("rustc")
        .args([
            "--crate-type",
            "lib",
            "--edition",
            "2021",
            "-o",
            rlib_output.to_str().unwrap(),
        ])
        .arg(&main_rs)
        .output()
        .expect("Failed to run rustc");

    let compiles = rustc.status.success();
    if !compiles {
        eprintln!("rustc stderr:\n{}", String::from_utf8_lossy(&rustc.stderr));
    }

    (rs_content, compiles)
}

/// Entity with @derive(Copy) - like ecs/entity.wj
#[test]
fn test_entity_tuple_pattern_no_deref() {
    // for (entity, mesh, transform) in entities { render_mesh(entity, ...) }
    // entity is Entity (Copy) - must NOT generate *(entity)
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct Entity {
    pub index: i64,
    pub generation: i64,
}

pub struct Mesh {}
pub struct Transform {}

pub fn render_mesh(entity: Entity, mesh: Mesh, transform: Transform) {
}

pub fn run_rendering(entities: Vec<(Entity, Mesh, Transform)>) {
    for (entity, mesh, transform) in entities {
        render_mesh(entity, mesh, transform)
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(entity)"),
        "Should NOT add *(entity) for Copy Entity from tuple pattern. Generated:\n{}",
        rs
    );
}

/// Simple for loop with Entity
#[test]
fn test_entity_simple_loop_no_deref() {
    // for entity in entities { process(entity) } where Entity is Copy
    let source = r#"
@derive(Copy, Clone, Debug)
pub struct Entity {
    pub index: i64,
}

pub fn process(entity: Entity) {
}

pub fn process_all(entities: Vec<Entity>) {
    for entity in entities {
        process(entity)
    }
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. Generated:\n{}", rs);
    assert!(
        !rs.contains("*(entity)"),
        "Should NOT add *(entity) for Copy Entity from simple loop. Generated:\n{}",
        rs
    );
}

/// E0614: User writes *entity - compiler strips * when entity is owned Copy
#[test]
fn test_entity_explicit_deref_stripped_when_copy() {
    // result.push(*entity) where entity from for entity in vec, Entity is Copy
    // Compiler should emit result.push(entity) - * causes E0614 on owned Copy
    let source = r#"
@derive(Copy, Clone, Debug, PartialEq)
pub struct Entity {
    pub index: i64,
}

pub fn collect_all(entities: Vec<Entity>) -> Vec<Entity> {
    let mut result = Vec::new()
    for entity in entities {
        result.push(*entity)
    }
    result
}

pub fn main() {}
"#;
    let (rs, compiles) = compile_wj_to_rust(source);
    assert!(compiles, "Should compile. *entity should be stripped when Entity is Copy. Generated:\n{}", rs);
    assert!(
        !rs.contains("push(*entity)"),
        "Should NOT generate push(*entity) for owned Copy - causes E0614. Generated:\n{}",
        rs
    );
}
