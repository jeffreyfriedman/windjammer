#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

/// TDD Test: Vec::push with Copy Types
///
/// Bug: Vec::push(entity) where entity is Copy incorrectly generates
/// Vec::push(&mut entity) instead of Vec::push(entity)
///
/// THE WINDJAMMER WAY: Copy types should be passed by value, not by reference
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_push_copy_type() {
    let source = r#"
@derive(Copy, Clone, Debug)
struct Entity {
    index: i64,
}

struct Storage {
    entities: Vec<Entity>,
}

impl Storage {
    fn add(&mut self, entity: Entity) {
        self.entities.push(entity)
    }
}

fn main() {
    let mut storage = Storage { entities: Vec::new() }
    let entity = Entity { index: 1 }
    storage.add(entity)
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Copy types should be passed by value to Vec::push
    assert!(
        rust_code.contains("self.entities.push(entity)"),
        "Vec::push should receive Copy type by value, not by reference.\nGenerated:\n{}",
        rust_code
    );

    // Should NOT add &mut
    assert!(
        !rust_code.contains("self.entities.push(&mut entity)")
            && !rust_code.contains("self.entities.push(&entity)"),
        "Vec::push should NOT add & or &mut for Copy types.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_insert_copy_key() {
    let source = r#"
use std::collections::HashMap

@derive(Copy, Clone, Debug, Hash, Eq, PartialEq)
struct EntityId {
    id: i64,
}

struct Registry {
    map: HashMap<EntityId, string>,
}

impl Registry {
    fn register(&mut self, entity: EntityId, name: string) {
        self.map.insert(entity, name)
    }
}

fn main() {
    let mut registry = Registry { map: HashMap::new() }
    let id = EntityId { id: 1 }
    registry.register(id, "Test".to_string())
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: HashMap::insert with Copy key should pass key by value
    assert!(
        rust_code.contains("self.map.insert(entity, name)"),
        "HashMap::insert should receive Copy type key by value.\nGenerated:\n{}",
        rust_code
    );

    // Should NOT add &mut to the key
    assert!(
        !rust_code.contains("self.map.insert(&mut entity, name)")
            && !rust_code.contains("self.map.insert(&entity, name)"),
        "HashMap::insert should NOT add & or &mut to Copy type keys.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_call_with_copy_param() {
    let source = r#"
@derive(Copy, Clone)
struct Point {
    x: int,
    y: int,
}

struct Canvas {
    points: Vec<Point>,
}

impl Canvas {
    fn add_point(&mut self, p: Point) {
        self.points.push(p)
    }
}

fn main() {
    let mut canvas = Canvas { points: Vec::new() }
    let point = Point { x: 10, y: 20 }
    canvas.add_point(point)
}
"#;

    let rust_code = test_utils::compile_single_result(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Copy types should be passed by value, not by reference
    assert!(
        rust_code.contains("canvas.add_point(point)"),
        "Method calls with Copy types should pass by value.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("canvas.add_point(&point)")
            && !rust_code.contains("canvas.add_point(&mut point)"),
        "Method calls with Copy types should NOT add &.\nGenerated:\n{}",
        rust_code
    );

    // Inside the method, push should also pass by value
    assert!(
        rust_code.contains("self.points.push(p)"),
        "Vec::push with Copy type should pass by value inside method.\nGenerated:\n{}",
        rust_code
    );
}
