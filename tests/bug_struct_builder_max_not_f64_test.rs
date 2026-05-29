/// TDD regression test: Struct builder methods named `max`/`min` must NOT
/// trigger float inference conflicts with `f64::max`/`f32::max`.
///
/// Bug: `Slider::new().min(0.0).max(100.0)` caused the float inference engine
/// to match `.max(100.0)` against `f64::max(self, other)` because:
///   1. `determine_method_return_type` blindly returned `f32` for any
///      MethodCall-on-MethodCall where the method name was in F32_METHODS
///   2. The function-signature lookup matched `f64::max` by basename + arity
///
/// Root cause: No receiver-type check — struct builder methods like
/// `Slider::max` collided with numeric methods `f64::max`.
use std::process::Command;

fn compile_wj(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("test.wj");
    std::fs::write(&src, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", src.to_str().unwrap(), "--target", "rust"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    format!("{}\n{}", stdout, stderr)
}

fn compile_wj_to_rust(source: &str) -> String {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("test.wj");
    std::fs::write(&src, source).unwrap();
    let out_dir = dir.path().join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--target",
            "rust",
            "-o",
            out_dir.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stderr.is_empty() {
        eprintln!("Compiler stderr:\n{}", stderr);
    }
    // The compiler outputs to a directory; the .rs file name matches the source.
    let rs_file = out_dir.join("test.rs");
    std::fs::read_to_string(&rs_file).unwrap_or_else(|_| "FILE NOT GENERATED".to_string())
}

#[test]
fn test_struct_builder_max_no_float_inference_conflict() {
    let source = r#"
pub struct Slider {
    min: float,
    max: float,
    step: float,
    value: float,
}

impl Slider {
    pub fn new() -> Slider {
        Slider { min: 0.0, max: 100.0, step: 1.0, value: 50.0 }
    }

    pub fn min(self, min: float) -> Slider {
        self.min = min
        self
    }

    pub fn max(self, max: float) -> Slider {
        self.max = max
        self
    }

    pub fn value(self, value: float) -> Slider {
        self.value = value
        self
    }
}

fn main() {
    let slider = Slider::new().min(0.0).max(100.0).value(75.0)
}
"#;

    let combined = compile_wj(source);
    assert!(
        !combined.contains("Float inference error"),
        "Float inference should NOT produce errors for struct builder methods!\nOutput:\n{}",
        combined
    );
}

#[test]
fn test_struct_builder_min_no_float_inference_conflict() {
    let source = r#"
pub struct Config {
    min: float,
    max: float,
}

impl Config {
    pub fn new() -> Config {
        Config { min: 0.0, max: 1.0 }
    }

    pub fn min(self, val: float) -> Config {
        self.min = val
        self
    }

    pub fn max(self, val: float) -> Config {
        self.max = val
        self
    }
}

fn main() {
    let c = Config::new().min(0.5).max(1.5)
}
"#;

    let combined = compile_wj(source);
    assert!(
        !combined.contains("Float inference error"),
        "Float inference should NOT produce errors for struct builder .min()/.max()!\nOutput:\n{}",
        combined
    );
}

#[test]
fn test_real_float_max_still_works() {
    let source = r#"
fn main() {
    let x: f32 = 3.0
    let y: f32 = 5.0
    let z = x.max(y)
}
"#;

    let rust = compile_wj_to_rust(source);
    assert!(
        !rust.contains("Float inference error") && rust != "FILE NOT GENERATED",
        "Real f32::max should still work!\nOutput:\n{}",
        rust
    );
}
