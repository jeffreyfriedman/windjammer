/// Regression guard: struct field `Vec<Copy>` indices must not use explicit `*` (E0614).
///
/// Rust already yields `f32` / `i32` for `Copy` elements; `*(node.params[0])` is invalid.
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
fn test_struct_field_vec_f32_index_no_spurious_star() {
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

    assert!(
        !output.contains("*(node.params[0])") && !output.contains("* (node.params[0])"),
        "must NOT deref Vec<Copy> index (E0614), got:\n{}",
        output
    );
}
