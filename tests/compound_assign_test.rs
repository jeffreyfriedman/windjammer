/// TDD Test: Compound assignment optimization for field access patterns
///
/// Bug: `self.x = self.x + dt` generates as-is instead of `self.x += dt`.
/// The compound assignment optimization only handled simple identifiers (x = x + y),
/// not field access patterns (self.x = self.x + y) or index patterns (arr[i] = arr[i] + 1).
///
/// Fix: Extended the pattern matcher to detect FieldAccess and Index targets.
use std::io::Write;
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
fn test_field_compound_add() {
    let source = r#"
pub struct Timer {
    pub elapsed: f32,
}

impl Timer {
    pub fn tick(&mut self, dt: f32) {
        self.elapsed = self.elapsed + dt
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.elapsed += dt"),
        "self.x = self.x + y should become self.x += y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_field_compound_sub() {
    let source = r#"
pub struct Health {
    pub hp: i32,
}

impl Health {
    pub fn damage(&mut self, amount: i32) {
        self.hp = self.hp - amount
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.hp -= amount"),
        "self.x = self.x - y should become self.x -= y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_field_compound_mul() {
    let source = r#"
pub struct Transform {
    pub scale: f32,
}

impl Transform {
    pub fn scale_by(&mut self, factor: f32) {
        self.scale = self.scale * factor
    }
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("self.scale *= factor"),
        "self.x = self.x * y should become self.x *= y.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_simple_var_compound_still_works() {
    let source = r#"
pub fn accumulate(n: i32) -> i32 {
    let mut total = 0
    let mut i = 0
    total = total + n
    total
}
"#;

    let generated = compile_wj(source);
    println!("Generated:\n{}", generated);

    assert!(
        generated.contains("total += n"),
        "Simple x = x + y should still be converted to x += y.\nGenerated:\n{}",
        generated
    );
}
