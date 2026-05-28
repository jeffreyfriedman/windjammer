use std::process::Command;

fn compile_wj_to_rs(source: &str) -> (bool, String, String) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let input = dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test.wj");
    let output = dir.path().join("output");
    std::fs::create_dir_all(&output).expect("create output dir");

    let result = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", input.to_str().unwrap(), "--no-cargo", "-o"])
        .arg(output.to_str().unwrap())
        .output()
        .expect("run wj");

    let success = result.status.success();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();

    let generated = if success {
        let rs_path = output.join("test.rs");
        std::fs::read_to_string(&rs_path).unwrap_or_default()
    } else {
        String::new()
    };

    (success, generated, stderr)
}

/// Bug: `.clamp(0.0, 1.0)` on an f32 receiver generates `clamp(0.0_f64, 1.0_f64)`
/// because the float inference engine only constrains `min` and `max` arguments
/// to match the receiver type, not `clamp` arguments.
///
/// Fix: Extend the MustMatch constraint to cover `clamp` (both arguments must
/// match receiver type), plus other Self-returning f32 methods that take Self args.
#[test]
fn test_clamp_arguments_infer_f32_from_receiver() {
    let source = r#"
pub struct SoundSystem {
    listener_x: f32,
    listener_y: f32,
    listener_z: f32,
}

impl SoundSystem {
    pub fn play_spatial(self, distance: f32) -> f32 {
        let attenuation = (1.0 / (1.0 + distance * 0.1)).clamp(0.0, 1.0)
        return attenuation
    }
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "wj build failed:\n{}", stderr);

    assert!(
        !generated.contains("_f64"),
        "Generated code contains f64 suffix but all values should be f32:\n{}",
        generated
    );

    assert!(
        generated.contains("0.0_f32") && generated.contains("1.0_f32"),
        "clamp arguments should be f32:\n{}",
        generated
    );
}

#[test]
fn test_min_max_arguments_infer_f32_from_receiver() {
    let source = r#"
pub fn clamp_manual(x: f32) -> f32 {
    let y = x.max(0.0)
    let z = y.min(1.0)
    return z
}
"#;

    let (success, generated, stderr) = compile_wj_to_rs(source);
    assert!(success, "wj build failed:\n{}", stderr);

    assert!(
        !generated.contains("_f64"),
        "Generated code contains f64 suffix but all values should be f32:\n{}",
        generated
    );
}
