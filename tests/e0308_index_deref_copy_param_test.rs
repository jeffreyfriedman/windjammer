/// TDD: E0308 Fix - Index expressions produce &T in Rust.
/// When passing vec[i] or arr[i] to function/method expecting Copy type (f32, u32),
/// add * to dereference.
///
/// Bug: sdf_sphere(x, y, z, node.params[0], ...) - params[0] gives &f32, expects f32
/// Fix: Generate *(node.params[0])

use std::path::PathBuf;
use std::process::Command;

fn compile_and_get_rust(source: &str) -> String {
    let test_dir = std::env::temp_dir().join("wj_e0308_test");
    let _ = std::fs::create_dir_all(&test_dir);
    let input = test_dir.join("test.wj");
    std::fs::write(&input, source).expect("write test file");

    let wj_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/wj");
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
fn test_index_deref_for_f32_param() {
    let source = r#"
pub struct Node {
    pub params: Vec<f32>,
}

pub fn sdf_sphere(p_x: f32, p_y: f32, p_z: f32, cx: f32, cy: f32, cz: f32, radius: f32) -> f32 {
    0.0
}

pub fn eval(node: Node, x: f32, y: f32, z: f32) -> f32 {
    sdf_sphere(x, y, z, node.params[0], node.params[1], node.params[2], node.params[3])
}
"#;

    let output = compile_and_get_rust(source);

    // Should generate *(node.params[0]) etc. for Copy params
    assert!(
        output.contains("*(node.params[0])") || output.contains("* (node.params[0])"),
        "Expected deref for Index when param expects f32, got:\n{}",
        output
    );
}
