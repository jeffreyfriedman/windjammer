#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.590: `int` bitops whose result is `as u8` into `Vec<u8>::push` must keep
//! mask/shift literals as i64 peers of the `int` binding — not `_i32`.
//!
//! Product `wj-uuid` `v1_bytes`:
//!   `let clock_hi = ((clock_seq >> 8) & 0x3F) | 0x80`
//! Tip emits `clock_seq >> 8_i32 & 63_i32 | 128_i32` with `clock_seq: i64` →
//! E0277 / E0308. Plain bitops without `push(… as u8)` false-GREEN (`_i64`).

use std::fs;
use std::process::Command;

#[test]
fn i64_bitops_into_u8_push_must_unify_i64() {
    let src = tempfile::TempDir::new().expect("src");
    fs::write(
        src.path().join("lib.wj"),
        r#"pub fn v1_bytes(clock_seq: int) -> Vec<u8> {
    let mut out = Vec::new()
    let clock_hi = ((clock_seq >> 8) & 0x3F) | 0x80
    let clock_lo = clock_seq & 0xFF
    out.push(clock_hi as u8)
    out.push(clock_lo as u8)
    out
}
"#,
    )
    .unwrap();

    let out = tempfile::TempDir::new().expect("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.path().to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.590 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let lib_rs = fs::read_to_string(out.path().join("lib.rs")).unwrap_or_default();
    eprintln!("P3.590 lib.rs:\n{lib_rs}");

    assert!(
        !lib_rs.contains("_i32"),
        "P3.590 RED: bitop masks next to i64 int must not emit _i32 (wj-uuid v1_bytes):\n{lib_rs}"
    );

    let cargo = Command::new("cargo")
        .args(["check", "--offline", "--manifest-path"])
        .arg(out.path().join("Cargo.toml"))
        .env(
            "CARGO_TARGET_DIR",
            std::env::temp_dir().join(format!("wj_p3590_cargo_{}", std::process::id())),
        )
        .output()
        .expect("cargo check");
    assert!(
        cargo.status.success(),
        "P3.590 RED: i64 bitops into u8 push must cargo-check:\n{}\n{lib_rs}",
        String::from_utf8_lossy(&cargo.stderr)
    );
}
