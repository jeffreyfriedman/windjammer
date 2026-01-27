/// TDD Test: Copy Type Parameter Inference Bug
///
/// Bug: Parameters of Copy types are incorrectly inferred as &mut
/// when they are used in method calls within the function body.
///
/// Example:
/// fn insert(self, entity: Entity, component: T) {
///     self.entities.push(entity)  // entity used here
/// }
///
/// Generated (WRONG):
/// fn insert(&mut self, entity: &mut Entity, component: T)
///
/// Expected (CORRECT):
/// fn insert(&mut self, entity: Entity, component: T)
///
/// THE WINDJAMMER WAY: Copy types should remain owned in signatures
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
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_param_not_inferred_as_mut_ref() {
    let source = r#"
@derive(Copy, Clone, Debug)
struct Entity {
    index: i64,
}

struct ComponentArray<T> {
    entities: Vec<Entity>,
    components: Vec<T>,
}

impl<T> ComponentArray<T> {
    pub fn insert(self, entity: Entity, component: T) {
        self.entities.push(entity)
        self.components.push(component)
    }
}

fn main() {
    let mut array: ComponentArray<i64> = ComponentArray {
        entities: Vec::new(),
        components: Vec::new(),
    }
    let entity = Entity { index: 1 }
    array.insert(entity, 42)
}
"#;

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Copy type parameters should stay owned, not become &mut
    assert!(
        rust_code.contains("fn insert(&mut self, entity: Entity, component: T)")
            || rust_code.contains("fn insert(mut self, entity: Entity, component: T)"),
        "Copy type parameter 'entity' should remain Entity, not &mut Entity.\nGenerated:\n{}",
        rust_code
    );

    // Should NOT generate &mut Entity for the parameter
    assert!(
        !rust_code.contains("entity: &mut Entity") && !rust_code.contains("entity: &Entity"),
        "entity parameter should NOT be inferred as &mut Entity or &Entity.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_with_clone() {
    let source = r#"
@derive(Copy, Clone, Hash, Eq, PartialEq)
struct EntityId {
    id: i64,
}

use std::collections::HashMap

struct World {
    transforms: HashMap<EntityId, i64>,
}

impl World {
    pub fn add_transform(self, entity: EntityId, transform: i64) {
        self.transforms.insert(entity.clone(), transform)
    }
}

fn main() {
    let mut world = World {
        transforms: HashMap::new(),
    }
    let entity = EntityId { id: 1 }
    world.add_transform(entity, 100)
}
"#;

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: Even when .clone() is called, Copy types should stay owned
    assert!(
        rust_code.contains("fn add_transform(&mut self, entity: EntityId, transform: i64)")
            || rust_code.contains("fn add_transform(mut self, entity: EntityId, transform: i64)"),
        "entity parameter should remain EntityId (Copy type), not &mut EntityId.\nGenerated:\n{}",
        rust_code
    );

    assert!(
        !rust_code.contains("entity: &mut EntityId") && !rust_code.contains("entity: &EntityId"),
        "entity parameter should NOT be &mut or &.\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_copy_type_passed_to_multiple_methods() {
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
    pub fn add(self, point: Point) {
        self.points.push(point)
        self.log_point(point)
    }
    
    fn log_point(&self, p: Point) {
        println!("Added point: {}, {}", p.x, p.y)
    }
}

fn main() {
    let mut canvas = Canvas { points: Vec::new() }
    let p = Point { x: 10, y: 20 }
    canvas.add(p)
}
"#;

    let rust_code = compile_code(source).expect("Compilation failed");
    println!("Generated Rust code:\n{}", rust_code);

    // Copy types should remain owned even when passed to multiple methods
    assert!(
        rust_code.contains("fn add(&mut self, point: Point)")
            || rust_code.contains("fn add(mut self, point: Point)"),
        "point parameter should remain Point (Copy type).\nGenerated:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("fn log_point(&self, p: Point)"),
        "p parameter should be Point (Copy type passed by value).\nGenerated:\n{}",
        rust_code
    );
}
