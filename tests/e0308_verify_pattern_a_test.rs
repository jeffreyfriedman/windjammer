//! E0308 Phase 9: Verify Pattern A - Struct literal tuple float fields
//!
//! Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) } with rotation: (f32, f32, f32, f32)
//! should generate (0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32)

use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir).unwrap();
    let input = temp_dir.path().join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj_bin)
        .args([
            "build",
            input.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    let rust_file = out_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        panic!(
            "Generated .rs file not found at {:?}\nstdout: {}\nstderr: {}",
            rust_file,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
    })
}

#[test]
fn test_struct_tuple_field_f32() {
    let source = r#"
pub struct Keyframe {
    pub rotation: (f32, f32, f32, f32),
    pub scale: (f32, f32, f32),
}

pub fn default_keyframe() -> Keyframe {
    Keyframe {
        rotation: (0.0, 0.0, 0.0, 1.0),
        scale: (1.0, 1.0, 1.0),
    }
}
"#;

    let rust = compile_and_get_rust(source);
    assert!(
        rust.contains("0.0_f32") || rust.contains("0.0f32"),
        "Expected f32 literals in tuple. Got:\n{}",
        rust
    );
    assert!(
        !rust.contains("_f64"),
        "Tuple fields should infer f32 from struct. Got:\n{}",
        rust
    );
}
