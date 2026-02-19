use std::io::Write;
/// TDD Test: Compiler should NOT emit .clone() on Copy types
///
/// Bug: When a variable is assigned from a static method call that returns f32/i32,
/// and the variable is used multiple times, the compiler incorrectly adds .clone()
/// because it can't infer the return type of the function call.
///
/// Example in Windjammer:
///   let u = MyMath::fade(xf)
///   let a = MyMath::lerp(u, x1, x2)   // u.clone() should NOT be emitted
///   let b = MyMath::lerp(u, x3, x4)   // u is Copy (f32), just copy it
///
/// Expected: generated Rust should use `u` directly, not `u.clone()`
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().expect("Failed to create temp dir");
    let wj_path = dir.path().join("test.wj");
    let out_dir = dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();

    let mut file = std::fs::File::create(&wj_path).unwrap();
    file.write_all(source.as_bytes()).unwrap();

    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "--",
            "build",
            wj_path.to_str().unwrap(),
            "--no-cargo",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to run wj");

    let rs_path = out_dir.join("test.rs");
    if rs_path.exists() {
        std::fs::read_to_string(&rs_path).unwrap()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "No output file generated.\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }
}

#[test]
fn test_no_clone_on_f32_from_static_method() {
    let source = r#"
pub struct MathHelper {
    pub dummy: i32,
}

impl MathHelper {
    fn fade(t: f32) -> f32 {
        t * t * t
    }

    fn lerp(t: f32, a: f32, b: f32) -> f32 {
        a + t * (b - a)
    }

    pub fn compute(&self, x: f32) -> f32 {
        let u = MathHelper::fade(x)
        let r1 = MathHelper::lerp(u, 1.0, 2.0)
        let r2 = MathHelper::lerp(u, 3.0, 4.0)
        r1 + r2
    }
}
"#;

    let generated = compile_wj(source);
    // u is f32 (Copy type) - should NOT have .clone()
    assert!(
        !generated.contains("u.clone()"),
        "Generated code should not clone f32 variable 'u'.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_i32_parameter_used_twice() {
    let source = r#"
pub struct Foo {
    pub items: Vec<i32>,
}

impl Foo {
    pub fn get_two(&self, idx: i32) -> i32 {
        let a = self.items[idx as usize]
        let b = self.items[(idx + 1) as usize]
        a + b
    }
}
"#;

    let generated = compile_wj(source);
    // idx is i32 (Copy type) - should NOT have .clone()
    assert!(
        !generated.contains("idx.clone()"),
        "Generated code should not clone i32 parameter 'idx'.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_bool_variable() {
    let source = r#"
pub struct Game {
    pub running: bool,
    pub score: i32,
}

impl Game {
    pub fn check(&self) -> i32 {
        let active = self.running
        let a = if active { 1 } else { 0 }
        let b = if active { self.score } else { 0 }
        a + b
    }
}
"#;

    let generated = compile_wj(source);
    // active is bool (Copy type) - should NOT have .clone()
    assert!(
        !generated.contains("active.clone()"),
        "Generated code should not clone bool variable 'active'.\nGenerated:\n{}",
        generated
    );
}
