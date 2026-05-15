//! Vec<Copy> index codegen: never emit `*(vec[i])` (E0614).
//!
//! Windjammer lowers many `Vec<T>` parameters to `&Vec<T>` in Rust, but `vec[i]` is still `T` for
//! `T: Copy` without an extra dereference.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_param_index_no_star_compiles() {
    let src = r#"
pub fn process(values: Vec<f32>) -> f32 {
    values[0]
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(src);
    assert!(
        !rs.contains("*(values[") && !rs.contains("* (values["),
        "must not emit *(values[…]) (E0614). Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile with rustc:\n{rs}");
}

#[test]
fn test_vec_local_index_no_star_compiles() {
    let src = r#"
pub fn sample() -> f32 {
    let mut values = vec![1.0, 2.0]
    values[1]
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(src);
    assert!(
        !rs.contains("*(values[") && !rs.contains("* (values["),
        "local Vec indexing must not use explicit deref. Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile:\n{rs}");
}

#[test]
fn test_vec_copy_struct_field_index_no_star() {
    let src = r#"
pub struct S {
    pub data: Vec<f32>,
}

pub fn first(s: S) -> f32 {
    s.data[0]
}
"#;
    let (rs, compiles) = test_utils::compile_single_check(src);
    assert!(
        !rs.contains("*(s.data[") && !rs.contains("* (s.data["),
        "field Vec<Copy> index must not use *. Generated:\n{rs}"
    );
    assert!(compiles, "generated Rust should compile:\n{rs}");
}
