#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

//! Read-only parameters are borrowed (a database's key comparison found every call cloning both keys), an
//! explicit `&Vec<u8>` is one reference, a comparison is a bool, and a value is compared with a borrowed peer.
//! Each generated library must compile under rustc. The project case `readonly_byte_formals_borrow` runs them.

#[path = "common/test_utils.rs"]
mod test_utils;

fn rustc_accepts(rs: &str) -> Result<(), String> {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("lib.rs");
    std::fs::write(&file, rs).unwrap();
    let out = std::process::Command::new("rustc")
        .args(["--edition", "2021", "--crate-type", "lib", "-A", "warnings", "--emit", "metadata", "--out-dir"])
        .arg(dir.path())
        .arg(&file)
        .output()
        .unwrap();
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

#[test]
fn branch_values_that_only_read_a_parameter_keep_it_borrowed() {
    let rs = test_utils::compile_single(
        "pub fn cmp_bytes(a: Vec<u8>, b: Vec<u8>) -> i32 {\n    let n = if a.len() < b.len() { a.len() } else { b.len() }\n    let mut i: usize = 0\n    while i < n {\n        if a[i] < b[i] {\n            return -1\n        }\n        i = i + 1\n    }\n    0\n}\n\npub fn find(keys: Vec<Vec<u8>>, key: Vec<u8>) -> i32 {\n    cmp_bytes(keys[0], key)\n}\n",
    );
    assert!(rs.contains("fn cmp_bytes(a: &Vec<u8>, b: &Vec<u8>)"), "read-only byte vectors must be borrowed:\n{rs}");
    assert!(rs.contains("key: &Vec<u8>") && !rs.contains(".clone()"), "the caller must pass on its borrow, with no copy:\n{rs}");
    rustc_accepts(&rs).unwrap_or_else(|e| panic!("{e}\n{rs}"));
}

#[test]
fn an_explicit_vec_reference_is_one_slice_reference() {
    let rs = test_utils::compile_single(
        "pub fn first_diff(a: &Vec<u8>, b: &Vec<u8>) -> usize {\n    let mut i: usize = 0\n    while i < a.len() && i < b.len() && a[i] == b[i] {\n        i = i + 1\n    }\n    i\n}\n",
    );
    assert!(rs.contains("fn first_diff(a: &[u8], b: &[u8])"), "`&Vec<u8>` is `&[u8]`, never `&&[u8]`:\n{rs}");
    rustc_accepts(&rs).unwrap_or_else(|e| panic!("{e}\n{rs}"));
}

#[test]
fn a_comparison_of_vectors_is_a_bool_and_compares_values() {
    let rs = test_utils::compile_single(
        "pub fn ordered(x: Vec<u8>, y: Vec<u8>) -> bool {\n    let mut before = x < y\n    if x.len() == 0 {\n        before = true\n    }\n    before\n}\n\npub fn count_root(rows: Vec<Vec<u8>>, root: Vec<u8>) -> int {\n    let mut n = 0\n    for r in rows {\n        if r == root {\n            n = n + 1\n        }\n    }\n    n\n}\n",
    );
    assert!(!rs.contains("before: Vec"), "`x < y` is a bool:\n{rs}");
    rustc_accepts(&rs).unwrap_or_else(|e| panic!("{e}\n{rs}"));
}
