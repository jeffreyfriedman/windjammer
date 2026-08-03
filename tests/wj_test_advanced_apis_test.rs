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
))]

//! Advanced `std::testing` APIs under `wj test`: property + setup auto-imports.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn wj_test_property_test_runs_under_library_harness() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("property_smoke_test.wj"),
        r#"@property_test(20)
fn prop_abs_non_negative(x: int) {
    let y = if x < 0 { -x } else { x }
    assert(y >= 0)
}
"#,
    )
    .unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .current_dir(root)
        .args(["test", "--no-runtime-copy"])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "@property_test under wj test should pass:\n{combined}"
    );
}

#[test]
fn codegen_auto_imports_setup_teardown_for_test_decorator() {
    let source = r#"
fn setup_db() -> int {
    1
}

fn teardown_db(db: int) {
}

@test(setup = setup_db, teardown = teardown_db)
fn test_database(db: int) {
    assert_eq(db, 1)
}
"#;

    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("setup_auto_test.wj");
    fs::write(&path, source).unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let out_dir = tmp.path().join("out");
    fs::create_dir_all(&out_dir).unwrap();
    let output = Command::new(wj)
        .args([
            "build",
            path.to_str().unwrap(),
            "-o",
            out_dir.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "build should succeed:\n{combined}"
    );

    let rust = fs::read_to_string(out_dir.join("setup_auto_test.rs"))
        .or_else(|_| {
            // module-file / single-file naming may vary
            fs::read_dir(&out_dir).ok().and_then(|entries| {
                entries
                    .flatten()
                    .find(|e| e.path().extension().is_some_and(|x| x == "rs"))
                    .and_then(|e| fs::read_to_string(e.path()).ok())
            })
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no .rs"))
        })
        .expect("generated rust");

    assert!(
        rust.contains("use windjammer_runtime::setup_teardown::with_setup_teardown"),
        "must auto-import with_setup_teardown:\n{rust}"
    );
    assert!(
        rust.contains("with_setup_teardown("),
        "must emit with_setup_teardown call:\n{rust}"
    );
}
