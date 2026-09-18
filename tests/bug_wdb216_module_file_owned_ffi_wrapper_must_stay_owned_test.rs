#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//! WDB-216/275: `--library --module-file` must keep pub wrappers that forward bare
//! into owned `extern fn …(Vec<…>)` as owned formals (not `&Vec` + bare into FFI).

use std::process::Command;
use tempfile::TempDir;

#[test]
fn wdb216_module_file_owned_ffi_forwarder_must_stay_owned_vec() {
    let dir = TempDir::new().expect("temp");
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("port.wj"),
        r#"extern fn graph_simd_u32_sorted_intersection_count_ffi(a: Vec<u32>, b: Vec<u32>) -> int

pub fn count_intersection_owned(a: Vec<u32>, b: Vec<u32>) -> int {
    graph_simd_u32_sorted_intersection_count_ffi(a, b)
}
"#,
    )
    .unwrap();
    std::fs::write(src.join("mod.wj"), "pub mod port\n").unwrap();
    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .status()
        .expect("spawn wj");
    assert!(status.success(), "wj build failed");
    let port = std::fs::read_to_string(out.join("port.rs")).expect("port.rs");
    assert!(
        port.contains("fn count_intersection_owned(a: Vec<u32>"),
        "WDB-216 RED: module-file demoted owned FFI forwarder to &Vec. Got:\n{port}"
    );
    assert!(
        !port.contains("fn count_intersection_owned(a: &Vec"),
        "WDB-216 RED: owned FFI forwarder emitted &Vec. Got:\n{port}"
    );
}
