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

/// TDD: Compound assignment on integer fields should NOT cast RHS to float.
///
/// Bug: `self.current_frame = self.current_frame + 1` where current_frame is i32
/// generates `self.current_frame += 1_i32 as f32;` which fails to compile
/// because you can't `i32 += f32`.
///
/// Fix: Guard the compound assignment float cast to only apply when the target
/// type is actually a float, not when float inference incorrectly suggests one.
use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("test.wj");
    std::fs::write(&src_path, source).unwrap();

    let wj = std::env::var("WJ_BINARY").unwrap_or_else(|_| {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest
            .join("target/release/wj")
            .to_string_lossy()
            .to_string()
    });

    let out_dir = dir.path().join("out");
    let output = Command::new(&wj)
        .arg("build")
        .arg(src_path.to_str().unwrap())
        .arg("--no-cargo")
        .arg("-o")
        .arg(out_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute wj");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let rs_path = out_dir.join("test.rs");
    let generated = std::fs::read_to_string(&rs_path).unwrap_or_default();

    (
        output.status.success(),
        generated,
        format!("{}\n{}", stdout, stderr),
    )
}

#[test]
fn test_i32_compound_add_no_float_cast() {
    let source = r#"
pub struct AnimatedSprite {
    pub current_frame: i32,
    pub elapsed: f32,
    pub frame_time: f32,
    pub is_playing: bool,
}

impl AnimatedSprite {
    pub fn update(self, delta: f32) {
        if !self.is_playing { return }
        self.elapsed = self.elapsed + delta
        if self.elapsed >= self.frame_time {
            self.elapsed = self.elapsed - self.frame_time
            self.current_frame = self.current_frame + 1
        }
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // The generated code should NOT contain `as f32` on the current_frame increment
    // It should be: self.current_frame += 1; (or self.current_frame += 1_i32;)
    // NOT: self.current_frame += 1_i32 as f32;
    assert!(
        !generated.contains("current_frame += 1_i32 as f32"),
        "Generated code should not cast i32 RHS to f32 for i32 target.\nGenerated:\n{}",
        generated
    );
    assert!(
        !generated.contains("current_frame += 1 as f32"),
        "Generated code should not cast RHS to f32 for i32 target.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_f32_compound_add_does_cast() {
    let source = r#"
pub struct Counter {
    pub value: f32,
}

impl Counter {
    pub fn increment(self) {
        self.value = self.value + 1
    }
}
"#;

    let (success, generated, output) = compile_wj_to_rs(source);
    assert!(success, "WJ compilation should succeed: {}", output);

    // For f32 target, int RHS SHOULD be cast to f32
    // self.value += 1 as f32; or self.value += 1_i32 as f32;
    let has_proper_cast = generated.contains("as f32") || generated.contains("1.0");
    assert!(
        has_proper_cast,
        "f32 target should cast int RHS or use float literal.\nGenerated:\n{}",
        generated
    );
}
