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

//! P3.526: `strings.starts_with(t, "?")` then `substring(t, …)` / return `t`
//! must borrow `t`. Runtime `starts_with<S: AsRef<str>>(s, …)` takes `t` by
//! value and moves it (E0382).
//!
//! Product `wj-querystring` `strip_question` (tip p3520 20:49) — `$WJ test`
//! RED. Isolates match product. Do not reshape the package.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const SOURCE: &str = r#"
use std::strings

fn strip_question(text: string) -> string {
    let t = strings.trim(text)
    if strings.starts_with(t, "?") {
        return strings.substring(t, 1, strings.len(t))
    }
    t
}

pub fn body(text: string) -> string {
    strip_question(text)
}
"#;

fn moves_t_into_starts_with(rs: &str) -> bool {
    rs.contains("starts_with(t,") && !rs.contains("starts_with(&t,") && !rs.contains("starts_with(t.as_str()")
}

#[test]
fn starts_with_must_borrow_then_reuse() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("lib.wj"), SOURCE).unwrap();
    let out = tmp.path().join("gen");

    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.526 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let rs = fs::read_to_string(out.join("lib.rs")).unwrap_or_default();
    eprintln!("P3.526 strip_question:\n{rs}");
    assert!(
        !moves_t_into_starts_with(&rs),
        "P3.526 RED: starts_with must borrow t so substring/return can reuse it:\n{rs}"
    );
}
