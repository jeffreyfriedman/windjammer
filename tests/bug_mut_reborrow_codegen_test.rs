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
use std::process::Command;

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

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // THE WINDJAMMER WAY (v0.45.0 fix): User writes `agent: Agent` (owned),
    // compiler preserves as owned even when mutated, since explicit intent is respected.
    // seek_weighted mutates agent → inferred as `mut agent: Agent`
    // seek passes through → stays `agent: Agent` (moves to seek_weighted)
    //
    // OLD BEHAVIOR (pre-v0.45.0): Would infer `&mut Agent` for efficiency
    // NEW BEHAVIOR (v0.45.0+): Respect explicit owned, linter warns about inefficiency
    assert!(
        generated.contains("fn seek_weighted(mut agent: Agent"),
        "seek_weighted should preserve owned as `mut T` (respect explicit intent). Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn seek(agent: Agent")
            && !generated.contains("fn seek(mut agent: Agent"),
        "seek should preserve owned (moves to seek_weighted). Generated:\n{}",
        generated
    );

    // With owned parameters, seek just moves agent to seek_weighted
    // No need for explicit &mut (it's a move, not a borrow)
    assert!(
        generated.contains("seek_weighted(agent,"),
        "seek should move agent to seek_weighted. Generated:\n{}",
        generated
    );

    // Compile with rustc to verify correctness
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
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

    fs::remove_dir_all(&test_dir).ok();
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

    let temp_dir = std::env::temp_dir();
    let test_id = format!(
        "wj_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let test_dir = temp_dir.join(&test_id);
    fs::create_dir_all(&test_dir).unwrap();

    let wj_file = test_dir.join("test.wj");
    fs::write(&wj_file, source).unwrap();

    let out_dir = test_dir.join("out");

    let _output = Command::new("wj")
        .arg("build")
        .arg(&wj_file)
        .arg("--target")
        .arg("rust")
        .arg("--output")
        .arg(&out_dir)
        .output()
        .expect("Failed to run wj compiler");

    let rust_file = out_dir.join("test.rs");
    let generated = fs::read_to_string(&rust_file).expect("Failed to read generated Rust file");

    println!("Generated code:\n{}", generated);

    // THE WINDJAMMER WAY (v0.45.0 fix): User writes `c: Counter` (owned),
    // compiler preserves as owned throughout the call chain.
    // do_increment mutates c → inferred as `mut c: Counter`
    // wrapper/outer pass through → stay `c: Counter` (move semantics)
    //
    // OLD BEHAVIOR (pre-v0.45.0): Would infer `&mut Counter` for all three
    // NEW BEHAVIOR (v0.45.0+): Respect explicit owned, moves through chain
    assert!(
        generated.contains("fn do_increment(mut c: Counter"),
        "do_increment should preserve owned as `mut T` (respect explicit intent). Generated:\n{}",
        generated
    );

    // wrapper and outer should just move (no mut needed for pass-through)
    assert!(
        generated.contains("fn wrapper(c: Counter")
            && !generated.contains("fn wrapper(mut c: Counter"),
        "wrapper should preserve owned without mut (just moves). Generated:\n{}",
        generated
    );
    assert!(
        generated.contains("fn outer(c: Counter") && !generated.contains("fn outer(mut c: Counter"),
        "outer should preserve owned without mut (just moves). Generated:\n{}",
        generated
    );

    // Compile with rustc
    let rustc_output = Command::new("rustc")
        .arg(&rust_file)
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

    fs::remove_dir_all(&test_dir).ok();
}
