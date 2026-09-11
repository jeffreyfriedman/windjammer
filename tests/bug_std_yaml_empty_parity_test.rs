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

//! P3.244 (`wj-yaml` graduation): `std::yaml.to_json` must reject empty / whitespace-only input.
//!
//! Observed while thin-wrapping `wj-yaml`:
//! - `yaml.to_json("")` succeeds via serde_yaml (returns `"null"` JSON)
//! - Ecosystem contract: empty YAML is an error (`Err("empty yaml")`)
//!
//! Wiring gate (`bug_std_yaml_module_test`) is green; this is **semantic parity**.
//! `wj-yaml` pre-checks until runtime rejects empty input.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn std_yaml_to_json_rejects_empty_input() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("main.wj");
    fs::write(
        &src,
        r#"
use std::yaml
use std::process

fn main() {
    match yaml.to_json("") {
        Ok(_) => {
            eprintln("empty yaml must fail, got Ok")
            process.exit(1)
        },
        Err(_) => {},
    }
    match yaml.to_json("   \n") {
        Ok(_) => {
            eprintln("whitespace-only yaml must fail, got Ok")
            process.exit(1)
        },
        Err(_) => {},
    }
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("build");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "wj build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new("cargo")
        .current_dir(&out)
        .args(["run", "--quiet"])
        .output()
        .expect("cargo run");
    assert!(
        run.status.success(),
        "RED P3.244: std::yaml.to_json must reject empty/whitespace input.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
