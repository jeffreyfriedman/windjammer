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

//! `timestamp_millis()` assigned to `now` must compare with `deadline` at one width.
//!
//! Ecosystem `wj-retry` `pause_ms`: `while now < deadline` emits
//! `while (now as i32) < deadline` with `deadline: i64` (E0308).
//! Blocks `wj-fetch` `$WJ test`. Distinct from WDB-395 (compare-to-zero usize)
//! and `std_time_utc_now_timestamp_millis_codegen` (link-only).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn timestamp_millis_loop_must_unify_int() {
    let tmp = TempDir::new().expect("tempdir");
    let wj = env!("CARGO_BIN_EXE_wj");

    let src = tmp.path().join("t.wj");
    let out = tmp.path().join("out");
    fs::write(
        &src,
        r#"
use std::time

pub fn pause_ms(ms: int) {
    if ms <= 0 {
        return
    }
    let start = time.utc_now().timestamp_millis()
    let deadline = start + ms
    let mut now = start
    while now < deadline {
        now = time.utc_now().timestamp_millis()
    }
}
"#,
    )
    .unwrap();

    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("build");
    assert!(
        build.status.success(),
        "isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("t.rs"))
        .or_else(|_| fs::read_to_string(out.join("lib.rs")))
        .unwrap_or_else(|_| {
            let mut acc = String::new();
            if let Ok(entries) = fs::read_dir(&out) {
                for e in entries.flatten() {
                    if e.path().extension().and_then(|s| s.to_str()) == Some("rs") {
                        acc.push_str(&fs::read_to_string(e.path()).unwrap_or_default());
                    }
                }
            }
            acc
        });
    eprintln!("generated:\n{rs}");
    assert!(
        !rs.contains("(now as i32) < deadline") && !rs.contains("(now as i32)< deadline"),
        "timestamp_millis loop must not compare i32 now to i64 deadline:\n{rs}"
    );

    let check = Command::new("cargo")
        .current_dir(&out)
        .args(["check", "--quiet"])
        .output()
        .expect("cargo check");
    assert!(
        check.status.success(),
        "timestamp_millis loop must cargo-check.\n{rs}\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
}
