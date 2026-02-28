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
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
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
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    // Both seek and seek_weighted should have agent as &mut Agent
    assert!(
        generated.contains("fn seek_weighted(agent: &mut Agent"),
        "seek_weighted should infer &mut Agent. Generated:\n{}", generated
    );
    assert!(
        generated.contains("fn seek(agent: &mut Agent"),
        "seek should infer &mut Agent. Generated:\n{}", generated
    );
    
    // CRITICAL: seek should NOT generate &mut agent when calling seek_weighted
    // because agent is already &mut Agent
    assert!(
        !generated.contains("seek_weighted(&mut agent"),
        "seek should NOT add &mut to already-&mut param. Generated:\n{}", generated
    );
    
    // Instead, it should just pass agent directly
    assert!(
        generated.contains("seek_weighted(agent,"),
        "seek should pass agent directly (auto-reborrow). Generated:\n{}", generated
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
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
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
    let test_id = format!("wj_test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos());
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
    let generated = fs::read_to_string(&rust_file)
        .expect("Failed to read generated Rust file");
    
    println!("Generated code:\n{}", generated);
    
    // All three should infer &mut Counter
    assert!(
        generated.contains("fn do_increment(c: &mut Counter"),
        "do_increment should infer &mut Counter. Generated:\n{}", generated
    );
    
    // None should generate &mut c when passing through
    assert!(
        !generated.contains("do_increment(&mut c"),
        "wrapper should NOT add &mut to already-&mut param. Generated:\n{}", generated
    );
    assert!(
        !generated.contains("wrapper(&mut c"),
        "outer should NOT add &mut to already-&mut param. Generated:\n{}", generated
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
        panic!("Compilation failed:\n{}\n\nGenerated code:\n{}", stderr, generated);
    }
    
    fs::remove_dir_all(&test_dir).ok();
}
