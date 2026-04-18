// TDD: Integer literals in vec.len() +/- literal must infer as usize (not i32).
//
// Bug: items.len() - 1 → 1_i32 → E0277 cannot subtract i32 from usize

use std::fs;
use std::process::Command;

fn compile_and_read_rs(wj_src: &str, tmp_wj: &str, build_name: &str) -> String {
    fs::write(tmp_wj, wj_src).expect("write temp .wj");

    let output = Command::new("./target/release/wj")
        .args(["build", tmp_wj, "-o", "./build", "--no-cargo"])
        .output()
        .expect("wj build");

    if !output.status.success() {
        panic!(
            "Compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let rs_path = format!("./build/{}.rs", build_name);
    fs::read_to_string(&rs_path).unwrap_or_else(|e| panic!("read {}: {}", rs_path, e))
}

#[test]
fn test_len_minus_literal_infers_usize() {
    let test_wj = r#"
fn last_index(items: Vec<i32>) -> usize {
    items.len() - 1
}
"#;

    let rust = compile_and_read_rs(test_wj, "/tmp/test_len_minus.wj", "test_len_minus");

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' in len() subtraction. Generated:\n{}",
        rust
    );
    assert!(
        !rust.contains("1_i32"),
        "Should not default to i32 when subtracting from usize\n{}",
        rust
    );

    let _ = fs::remove_file("/tmp/test_len_minus.wj");
}

#[test]
fn test_len_plus_literal_infers_usize() {
    let test_wj = r#"
fn capacity_with_buffer(items: Vec<i32>) -> usize {
    items.len() + 10
}
"#;

    let rust = compile_and_read_rs(test_wj, "/tmp/test_len_plus.wj", "test_len_plus");

    assert!(
        rust.contains("10_usize"),
        "Expected '10_usize' in len() addition. Generated:\n{}",
        rust
    );

    let _ = fs::remove_file("/tmp/test_len_plus.wj");
}

#[test]
fn test_len_minus_in_comparison() {
    let test_wj = r#"
fn check_bounds(items: Vec<i32>, i: usize) -> bool {
    i < items.len() - 1
}
"#;

    let rust = compile_and_read_rs(
        test_wj,
        "/tmp/test_len_cmp_sub.wj",
        "test_len_cmp_sub",
    );

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' in len()-1 comparison. Generated:\n{}",
        rust
    );

    let _ = fs::remove_file("/tmp/test_len_cmp_sub.wj");
}

#[test]
fn test_len_arithmetic_in_assignment() {
    let test_wj = r#"
fn set_last_index(items: Vec<i32>) {
    let mut idx: usize = 0
    idx = items.len() - 1
}
"#;

    let rust = compile_and_read_rs(
        test_wj,
        "/tmp/test_len_assign_sub.wj",
        "test_len_assign_sub",
    );

    assert!(
        rust.contains("1_usize"),
        "Expected '1_usize' when assigning to usize var. Generated:\n{}",
        rust
    );

    let _ = fs::remove_file("/tmp/test_len_assign_sub.wj");
}
