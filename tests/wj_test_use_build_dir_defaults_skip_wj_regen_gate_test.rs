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

//! FAILING REPRO — `wj test --use-build-dir` must default `SKIP_WJ_REGEN=1` for
//! Cargo children so path-dep crates with hand-maintained generated trees are
//! not re-transpiled by tip `wj` during the test link.
//!
//! Dogfood libraries path-depend on UI crates whose `build.rs` runs `wj` unless
//! `SKIP_WJ_REGEN` is set. Tip lint/codegen drift then fails the *dependency*
//! build before any `@test` runs — even when `--use-build-dir` already has a
//! good outbound `build/` for the package under test.
//!
//! Language-only; no product/repo names.
//! Gap doc: `docs/USER_LIBRARY_TEST_GAPS.md` §2 / §4 / acceptance #4.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn write_path_dep_with_regen_guard(dep_root: &std::path::Path) {
    fs::create_dir_all(dep_root.join("src")).unwrap();
    fs::write(
        dep_root.join("Cargo.toml"),
        r#"[package]
name = "regen-guard-ui"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[lib]
name = "regen_guard_ui"
path = "src/lib.rs"
"#,
    )
    .unwrap();
    fs::write(
        dep_root.join("src/lib.rs"),
        "pub fn marker() -> &'static str { \"ok\" }\n",
    )
    .unwrap();
    // Fail the build unless SKIP_WJ_REGEN=1 — mirrors UI crates that refuse tip regen.
    fs::write(
        dep_root.join("build.rs"),
        r#"fn main() {
    match std::env::var("SKIP_WJ_REGEN") {
        Ok(v) if v == "1" || v.eq_ignore_ascii_case("true") => {}
        _ => panic!(
            "regen-guard-ui: SKIP_WJ_REGEN must be set for dogfood wj test \
             (path-dep must not tip-transpile)"
        ),
    }
}
"#,
    )
    .unwrap();
}

#[test]
fn wj_test_use_build_dir_defaults_skip_wj_regen_for_path_deps() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();
    let dep = root.join("regen-guard-ui");
    write_path_dep_with_regen_guard(&dep);

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("mod.wj"),
        r#"pub fn hello() -> string {
    "hi"
}
"#,
    )
    .unwrap();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "dogfood-lib"
version = "0.1.0"
edition = "2021"

[lib]
name = "dogfood_lib"
path = "build/lib.rs"

[dependencies]
regen-guard-ui = { path = "regen-guard-ui" }
"#,
    )
    .unwrap();

    let build = root.join("build");
    let wj = env!("CARGO_BIN_EXE_wj");

    let build_out = Command::new(wj)
        .current_dir(root)
        .args([
            "build",
            "src/mod.wj",
            "--library",
            "--module-file",
            "-o",
            build.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build_out.status.success(),
        "prebuild failed:\n{}{}",
        String::from_utf8_lossy(&build_out.stdout),
        String::from_utf8_lossy(&build_out.stderr)
    );

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("hello_test.wj"),
        r#"use dogfood_lib::hello

@test
fn test_hello() {
    assert_eq(hello(), "hi")
}
"#,
    )
    .unwrap();

    // Intentionally do NOT set SKIP_WJ_REGEN in the parent env — harness must inject it.
    let test_out = Command::new(wj)
        .current_dir(root)
        .env_remove("SKIP_WJ_REGEN")
        .args([
            "test",
            "--use-build-dir",
            "build",
            "--use-project-cargo",
            "--no-runtime-copy",
        ])
        .output()
        .expect("wj test");

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&test_out.stdout),
        String::from_utf8_lossy(&test_out.stderr)
    );

    assert!(
        test_out.status.success(),
        "wj test --use-build-dir must default SKIP_WJ_REGEN=1 for path-dep Cargo \
         builds so dogfood UI crates are not tip-transpiled:\n{combined}"
    );
    assert!(
        combined.contains("test_hello") || combined.contains("1 passed"),
        "expected test to run:\n{combined}"
    );
}
