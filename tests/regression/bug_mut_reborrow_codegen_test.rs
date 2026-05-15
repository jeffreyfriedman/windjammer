#![cfg(not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
)))]

/// TDD Test: Prevent double &mut in codegen for inferred &mut parameters
///
/// Bug: When a parameter is inferred as &mut through ownership analysis,
/// the codegen generates `&mut param` when passing it to another function
/// that also expects &mut. But since `param` is already `&mut T` in the
/// generated Rust, this creates an illegal `&mut &mut T`.
///
/// Example:
/// ```windjammer
/// fn inner(agent: SteeringAgent) {
///     agent.velocity.x = 1.0  // → inferred as &mut SteeringAgent
/// }
/// fn outer(agent: SteeringAgent) {
///     inner(agent)  // agent is inferred as &mut, inner expects &mut
///                    // Generated: inner(&mut agent) ← WRONG, should be: inner(agent)
/// }
/// ```
///
/// Root cause: `is_already_mut_ref` check only looks at AST param.type_ for
/// Type::MutableReference, but inferred &mut params still have their original
/// type in the AST. Need to also check inferred ownership.
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Parallel-safe wj invocation (unique temp dir; skip nested cargo — rustc checks output).
fn transpile_wj_to_rust(source: &str) -> (TempDir, String) {
    let temp_dir = TempDir::new().expect("failed to create temp dir for wj test");
    let test_dir = temp_dir.path();
    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();
    let out_dir = test_dir.join("out");
    fs::create_dir_all(&out_dir).unwrap();

    let wj_binary = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_binary)
        .current_dir(test_dir)
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .arg("--no-cargo")
        .output()
        .expect("Failed to run wj compiler");

    assert!(
        output.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");
    (temp_dir, generated)
}

fn assert_rustc_ok(test_dir: &Path, rust_file: &Path, generated: &str) {
    let rustc_output = Command::new("rustc")
        .arg(rust_file)
        .arg("--crate-type")
        .arg("bin")
        .arg("--edition")
        .arg("2021")
        .arg("-o")
        .arg(test_dir.join("test_bin"))
        .output()
        .expect("Failed to run rustc");

    if !rustc_output.status.success() {
        let stderr = String::from_utf8_lossy(&rustc_output.stderr);
        panic!(
            "Compilation failed:\n{}\n\nGenerated code:\n{}",
            stderr, generated
        );
    }
}

#[test]
fn test_inferred_mut_param_passthrough() {
    // Non-Copy struct (has Vec field) to trigger passthrough inference
    let source = r#"
struct Agent {
    pub x: f32,
    pub y: f32,
    pub forces: Vec<f32>,
}

impl Agent {
    pub fn new() -> Agent {
        Agent { x: 0.0, y: 0.0, forces: Vec::new() }
    }
    pub fn apply_force(self, fx: f32, fy: f32) {
        self.x = self.x + fx
        self.y = self.y + fy
        self.forces.push(fx)
    }
}

fn seek_weighted(agent: Agent, tx: f32, ty: f32, weight: f32) {
    let sx = (tx - agent.x) * weight
    let sy = (ty - agent.y) * weight
    agent.apply_force(sx, sy)
}

fn seek(agent: Agent, tx: f32, ty: f32) {
    seek_weighted(agent, tx, ty, 1.0)
}

fn main() {
    let mut a = Agent::new()
    seek(a, 5.0, 3.0)
}
"#;

    let (tmp, generated) = transpile_wj_to_rust(source);
    let test_dir = tmp.path();
    let rust_file = test_dir.join("out").join("test.rs");

    println!("Generated code:\n{}", generated);

    // THE WINDJAMMER WAY: Automatic ownership inference!
    // User writes `agent: Agent` (no & or &mut)
    // Compiler infers `&mut Agent` because agent.apply_force() mutates
    // Both seek_weighted and seek are inferred as &mut
    assert!(
        generated.contains("fn seek_weighted(agent: &mut Agent"),
        "seek_weighted should infer &mut for mutated parameter. Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn seek(agent: &mut Agent"),
        "seek should infer &mut (passes through to seek_weighted). Generated:\n{}",
        generated
    );

    // CRITICAL: When passing &mut param to function expecting &mut,
    // should NOT add another &mut (would create illegal &mut &mut T)
    // Just pass the parameter directly: seek_weighted(agent, ...)
    assert!(
        generated.contains("seek_weighted(agent,"),
        "seek should pass agent directly (reborrow, not &mut). Generated:\n{}",
        generated
    );

    assert_rustc_ok(test_dir, &rust_file, &generated);
}

#[test]
fn test_chained_mut_passthrough() {
    // Non-Copy struct (has Vec field) with 3 levels of pass-through
    let source = r#"
struct Counter {
    pub value: i32,
    pub history: Vec<i32>,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { value: 0, history: Vec::new() }
    }
    pub fn increment(self) {
        self.value = self.value + 1
        self.history.push(self.value)
    }
}

fn do_increment(c: Counter) {
    c.increment()
}

fn wrapper(c: Counter) {
    do_increment(c)
}

fn outer(c: Counter) {
    wrapper(c)
}

fn main() {
    let mut c = Counter::new()
    outer(c)
}
"#;

    let (tmp, generated) = transpile_wj_to_rust(source);
    let test_dir = tmp.path();
    let rust_file = test_dir.join("out").join("test.rs");

    println!("Generated code:\n{}", generated);

    // THE WINDJAMMER WAY: Automatic ownership inference!
    // User writes `c: Counter` (no & or &mut)
    // Compiler infers `&mut Counter` throughout the chain because c.increment() mutates
    // All three functions infer &mut
    assert!(
        generated.contains("fn do_increment(c: &mut Counter)"),
        "do_increment should infer &mut for mutated parameter. Generated:\n{}",
        generated
    );

    // wrapper and outer also infer &mut (pass through to mutating functions)
    assert!(
        generated.contains("fn wrapper(c: &mut Counter)"),
        "wrapper should infer &mut (passes through to do_increment). Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn outer(c: &mut Counter)"),
        "outer should infer &mut (passes through to wrapper). Generated:\n{}",
        generated
    );

    assert_rustc_ok(test_dir, &rust_file, &generated);
}
