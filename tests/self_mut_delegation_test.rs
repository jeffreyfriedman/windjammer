#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

/// TDD: Methods that delegate to &mut self callees should also be &mut self, not consuming.
///
/// Bug: `fn add_capsule(self, ...)` which calls `self.add_cylinder(...)` (an &mut self method)
/// gets generated as `pub fn add_capsule(mut self, ...)` instead of `pub fn add_capsule(&mut self, ...)`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_delegating_method_gets_mut_self() {
    let source = r#"
struct Builder {
    items: Vec<i32>,
    count: i32,
}

impl Builder {
    fn add_item(self, value: i32) {
        self.items.push(value)
        self.count = self.count + 1
    }

    fn add_pair(self, a: i32, b: i32) {
        self.add_item(a)
        self.add_item(b)
    }

    fn add_triple(self, a: i32, b: i32, c: i32) {
        self.add_pair(a, b)
        self.add_item(c)
    }
}

pub fn main() {
    let mut builder = Builder { items: Vec::new(), count: 0 }
    builder.add_triple(1, 2, 3)
    assert_eq(builder.count, 3)
}
"#;

    let output = test_utils::compile_single(source);

    // add_item directly mutates self fields → &mut self
    assert!(
        output.contains("fn add_item(&mut self"),
        "add_item should be &mut self since it mutates self.items and self.count. Got:\n{}",
        output
    );

    // add_pair calls self.add_item() which is &mut self → add_pair should also be &mut self
    assert!(
        output.contains("fn add_pair(&mut self"),
        "add_pair should be &mut self since it delegates to add_item(&mut self). Got:\n{}",
        output
    );

    // add_triple calls self.add_pair() and self.add_item() → also &mut self
    assert!(
        output.contains("fn add_triple(&mut self"),
        "add_triple should be &mut self since it delegates to add_pair(&mut self). Got:\n{}",
        output
    );

    // Caller uses &mut borrow, not move
    assert!(
        !output.contains("fn add_pair(mut self"),
        "add_pair should NOT be consuming (mut self)"
    );
    assert!(
        !output.contains("fn add_triple(mut self"),
        "add_triple should NOT be consuming (mut self)"
    );
}

#[test]
fn test_method_calling_user_mutating_method_gets_mut_self() {
    let source = r#"
struct Logger {
    messages: Vec<string>,
}

impl Logger {
    fn log(self, msg: string) {
        self.messages.push(msg)
    }

    fn log_two(self, a: string, b: string) {
        self.log(a)
        self.log(b)
    }
}

pub fn main() {
    let mut logger = Logger { messages: Vec::new() }
    logger.log_two("hello".to_string(), "world".to_string())
    assert_eq(logger.messages.len(), 2)
}
"#;

    let output = test_utils::compile_single(source);

    assert!(
        output.contains("fn log(&mut self"),
        "log should be &mut self. Got:\n{}",
        output
    );
    assert!(
        output.contains("fn log_two(&mut self"),
        "log_two should be &mut self since it calls log(&mut self). Got:\n{}",
        output
    );
}

/// Match the actual pattern from windjammer-game: methods that call callee methods
/// on self which themselves only delegate (never directly modify fields).
/// Tests SINGLE-FILE compilation.
#[test]
fn test_deep_delegation_chain_gets_mut_self() {
    let source = r#"
struct Mesh {
    vertices: Vec<f32>,
    count: i32,
}

impl Mesh {
    fn add_vertex(self, x: f32, y: f32, z: f32) {
        self.vertices.push(x)
        self.vertices.push(y)
        self.vertices.push(z)
        self.count = self.count + 1
    }

    fn add_sphere(self, cx: f32, cy: f32, cz: f32, radius: f32, segments: i32) {
        let mut i = 0
        while i < segments {
            let angle = (i as f32) * 6.28 / (segments as f32)
            let x = cx + radius * angle
            let y = cy + radius * angle
            let z = cz
            self.add_vertex(x, y, z)
            i = i + 1
        }
    }

    fn add_cylinder(self, cx: f32, cy: f32, cz: f32, radius: f32, height: f32, segments: i32) {
        self.add_sphere(cx, cy + height * 0.5, cz, radius, segments)
        self.add_sphere(cx, cy - height * 0.5, cz, radius, segments)
    }

    fn add_capsule(self, cx: f32, cy: f32, cz: f32, radius: f32, half_height: f32, segments: i32) {
        self.add_cylinder(cx, cy, cz, radius, half_height * 2.0, segments)
        self.add_sphere(cx, cy + half_height, cz, radius, segments)
        self.add_sphere(cx, cy - half_height, cz, radius, segments)
    }

    fn build_body(self) {
        self.add_capsule(0.0, 1.0, 0.0, 0.5, 1.0, 8)
        self.add_sphere(0.0, 2.5, 0.0, 0.3, 8)
    }
}

pub fn main() {
    let mut mesh = Mesh { vertices: Vec::new(), count: 0 }
    mesh.build_body()
    assert(mesh.count > 0, "Should have vertices")
}
"#;

    let output = test_utils::compile_single(source);

    // add_vertex directly mutates fields → &mut self
    assert!(
        output.contains("fn add_vertex(&mut self"),
        "add_vertex should be &mut self. Got:\n{}",
        output
    );

    // add_sphere delegates to add_vertex → &mut self
    assert!(
        output.contains("fn add_sphere(&mut self"),
        "add_sphere should be &mut self. Got:\n{}",
        output
    );

    // add_cylinder delegates to add_sphere → &mut self
    assert!(
        output.contains("fn add_cylinder(&mut self"),
        "add_cylinder should be &mut self. Got:\n{}",
        output
    );

    // add_capsule delegates to add_cylinder and add_sphere → &mut self
    assert!(
        output.contains("fn add_capsule(&mut self"),
        "add_capsule should be &mut self (delegates to add_cylinder which is &mut self). Got:\n{}",
        output
    );

    // build_body delegates to add_capsule and add_sphere → &mut self
    assert!(
        output.contains("fn build_body(&mut self"),
        "build_body should be &mut self (delegates to add_capsule which is &mut self). Got:\n{}",
        output
    );
}

/// Reproduce the exact bug from windjammer-game: multifile library compilation
/// incorrectly infers `mut self` (consuming) instead of `&mut self` for methods
/// that delegate to other field-mutating methods.
#[test]
fn test_multifile_deep_delegation_chain_gets_mut_self() {
    let files = &[(
        "mesh_builder.wj",
        r#"
struct MeshBuilder {
    vertices: Vec<f32>,
    indices: Vec<i32>,
    count: i32,
}

impl MeshBuilder {
    pub fn new() -> MeshBuilder {
        MeshBuilder {
            vertices: Vec::new(),
            indices: Vec::new(),
            count: 0,
        }
    }

    pub fn add_vertex(self, x: f32, y: f32, z: f32) {
        self.vertices.push(x)
        self.vertices.push(y)
        self.vertices.push(z)
        self.count = self.count + 1
    }

    pub fn add_sphere(self, cx: f32, cy: f32, cz: f32, r: f32) {
        self.add_vertex(cx + r, cy, cz)
        self.add_vertex(cx - r, cy, cz)
        self.add_vertex(cx, cy + r, cz)
    }

    pub fn add_cylinder(self, x: f32, y: f32, z: f32, h: f32, r: f32) {
        self.add_sphere(x, y, z, r)
        self.add_sphere(x, y + h, z, r)
    }

    pub fn add_capsule(self, x: f32, y: f32, z: f32, h: f32, r: f32) {
        self.add_cylinder(x, y, z, h, r)
        self.add_sphere(x, y + h, z, r)
    }

    pub fn build_body(self) {
        self.add_capsule(0.0, 0.0, 0.0, 2.0, 0.5)
        self.add_sphere(0.0, 3.0, 0.0, 0.4)
    }
}
"#,
    )];

    let results = test_utils::compile_project(files);
    let output = results
        .get("mesh_builder.rs")
        .expect("mesh_builder.rs should exist");

    assert!(
        output.contains("fn add_vertex(&mut self"),
        "add_vertex should be &mut self (directly mutates fields). Got:\n{}",
        output
    );
    assert!(
        output.contains("fn add_sphere(&mut self"),
        "add_sphere should be &mut self (delegates to add_vertex). Got:\n{}",
        output
    );
    assert!(
        output.contains("fn add_cylinder(&mut self"),
        "add_cylinder should be &mut self (delegates to add_sphere). Got:\n{}",
        output
    );
    assert!(
        output.contains("fn add_capsule(&mut self"),
        "add_capsule should be &mut self (delegates to add_cylinder). Got:\n{}",
        output
    );
    assert!(
        output.contains("fn build_body(&mut self"),
        "build_body should be &mut self (delegates to add_capsule). Got:\n{}",
        output
    );
}
