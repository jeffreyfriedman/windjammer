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

//! P3.243 (`wj-mime` graduation): `std::mime.from_extension` / `from_path` must return
//! the same strings as `APPLICATION_*` / `TEXT_*` constants (including `; charset=utf-8`
//! where the constant has it).
//!
//! Observed while thin-wrapping `wj-mime`:
//! - `from_extension("json")` → `"application/json"`
//! - `APPLICATION_JSON` → `"application/json; charset=utf-8"`
//! - `from_path("index.html")` → `"text/html"` vs `TEXT_HTML` with charset
//!
//! Wiring gate (`bug_std_mime_module_wiring_test`) is green; this is **semantic parity**.
//! ✅ tip GREEN (P3.243) — `wj-mime` fully thin-wraps `std::mime`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn std_mime_from_extension_json_matches_application_json_constant() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("main.wj");
    fs::write(
        &src,
        r#"
use std::mime
use std::process

fn main() {
    let got = mime.from_extension("json")
    let want = mime.APPLICATION_JSON
    if got != want {
        eprintln("from_extension json mismatch: got=${got} want=${want}")
        process.exit(1)
    }
    let got_html = mime.from_path("index.html")
    let want_html = mime.TEXT_HTML
    if got_html != want_html {
        eprintln("from_path html mismatch: got=${got_html} want=${want_html}")
        process.exit(1)
    }
    if !mime.is_text(mime.APPLICATION_JSON) {
        eprintln("is_text(APPLICATION_JSON) must be true")
        process.exit(1)
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
        "RED P3.243: std::mime lookup must match APPLICATION_*/TEXT_* constants (charset).\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
