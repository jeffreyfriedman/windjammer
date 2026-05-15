/// Regression guard: struct field `Vec<Copy>` indices must not use explicit `*` (E0614).
///
/// Rust already yields `f32` / `i32` for `Copy` elements; `*(node.params[0])` is invalid.
#[path = "../common/test_utils.rs"]
mod test_utils;

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

    let output = test_utils::compile_single(source);

    assert!(
        !output.contains("*(node.params[0])") && !output.contains("* (node.params[0])"),
        "must NOT deref Vec<Copy> index (E0614), got:\n{}",
        output
    );
}
