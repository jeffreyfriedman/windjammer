/// TDD Test: Vec::push with Copy Types
///
/// Bug: Vec::push(entity) where entity is Copy incorrectly generates
/// Vec::push(&mut entity) instead of Vec::push(entity)
///
/// THE WINDJAMMER WAY: Copy types should be passed by value, not by reference
use std::path::PathBuf;
use tempfile::TempDir;

fn compile_code(code: &str) -> Result<String, String> {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.wj");
    std::fs::write(&test_file, code).unwrap();

    let output_dir = temp_dir.path().join("output");
    std::fs::create_dir(&output_dir).unwrap();

    let compiler_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wj_binary = compiler_path.join("target/release/wj");

    let output = std::process::Command::new(&wj_binary)
        .arg("build")
        .arg(&test_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to execute wj compiler");

    if !output.status.success() {
        return Err(format!(
            "Compilation failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let generated_file = output_dir.join("test.rs");
    let generated_code =
        std::fs::read_to_string(&generated_file).expect("Failed to read generated code");

    Ok(generated_code)
}

#[test]
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

    let rust_code = compile_code(source).expect("Compilation failed");
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

    let rust_code = compile_code(source).expect("Compilation failed");
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

    let rust_code = compile_code(source).expect("Compilation failed");
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
