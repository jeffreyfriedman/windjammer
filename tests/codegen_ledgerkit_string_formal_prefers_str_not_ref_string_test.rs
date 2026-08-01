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
))]

//! FAILING REPRO (LedgerKit finance-screens multipass):
//!
//! Tip `wj build --library --module-file` for `home.wj`-style KPI helpers emits
//! `cash_html: &String` instead of `&str`. Single-file compile can look green while
//! the product multipass path still breaks dogfood `&str` literals.
//!
//! Contract: multipass library emit uses `&str` (or owned `String`), never `&String`.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn multipass_read_only_string_formals_must_not_emit_ref_string() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let out = tmp.path().join("out");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(
        src.join("home.wj"),
        r#"
pub fn render_kpi_grid(cash_html: string, ar_html: string, balanced: bool) -> string {
    let mark = if balanced { "ok".to_string() } else { "bad".to_string() }
    format!("<div>{}{}{}</div>", cash_html, ar_html, mark)
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod home\n").unwrap();

    let wj = env!("CARGO_BIN_EXE_wj");
    let status = Command::new(wj)
        .args([
            "build",
            src.join("mod.wj").to_str().unwrap(),
            "--module-file",
            "--library",
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .status()
        .expect("run wj");
    assert!(status.success(), "wj library build must succeed");

    let generated = fs::read_to_string(out.join("home.rs")).expect("home.rs");
    assert!(
        !generated.contains("cash_html: &String")
            && !generated.contains("ar_html: &String")
            && !generated.contains(": &String,"),
        "multipass must not emit &String formals. Got:\n{generated}"
    );
    assert!(
        generated.contains("cash_html: &str") || generated.contains("cash_html: String"),
        "expected &str or owned String. Got:\n{generated}"
    );
}
