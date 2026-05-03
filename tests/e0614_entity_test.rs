//! TDD Test: E0614 Entity "cannot be dereferenced" fix
//!
//! Fixes: entity::Entity cannot be dereferenced when entity comes from:
//! - for (entity, mesh, transform) in entities { render_mesh(entity, ...) }
//! - for entity in entities { process(entity) } where Entity is Copy
//!
//! Root cause: Entity (ecs/entity.wj) has @derive(Copy). When we add * for "reference coercion",
//! we wrongly dereference owned Entity. Fix: is_known_copy_type("Entity") + tuple pattern
//! local_var_types population.

#[path = "test_utils.rs"]
mod test_utils;

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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
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
    let (rs, compiles) = test_utils::compile_single_check(source);
    assert!(
        compiles,
        "Should compile. *entity should be stripped when Entity is Copy. Generated:\n{}",
        rs
    );
    assert!(
        !rs.contains("push(*entity)"),
        "Should NOT generate push(*entity) for owned Copy - causes E0614. Generated:\n{}",
        rs
    );
}
