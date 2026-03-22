//! E0308 Phase 9: Verify Pattern A - Struct literal tuple float fields
//!
//! Keyframe { rotation: (0.0, 0.0, 0.0, 1.0) } with rotation: (f32, f32, f32, f32)
//! should generate (0.0_f32, 0.0_f32, 0.0_f32, 1.0_f32)

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let test_dir = std::env::temp_dir().join("wj_e0308_pattern_a");
    let _ = std::fs::create_dir_all(&test_dir);
    let input = test_dir.join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj");
    let output = Command::new(&wj_bin)
        .arg("build")
        .arg(&input)
        .current_dir(&test_dir)
        .output()
        .expect("wj build");

    let rust_file = test_dir.join("test.rs");
    std::fs::read_to_string(&rust_file).unwrap_or_else(|_| {
        String::from_utf8_lossy(&output.stderr).to_string()
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
