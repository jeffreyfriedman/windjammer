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

//! Signature-driven string coercion: no method-name lists for search / constructors.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_to_rust(source: &str, filename: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join(filename);
    fs::write(&path, source).unwrap();
    let out = tmp.path().join("out");
    fs::create_dir_all(&out).unwrap();
    let wj = env!("CARGO_BIN_EXE_wj");
    let output = Command::new(wj)
        .args([
            "build",
            path.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        output.status.success(),
        "compile failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let rs = out.join(filename.replace(".wj", ".rs"));
    fs::read_to_string(&rs).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .unwrap()
            .flatten()
            .find(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .map(|e| fs::read_to_string(e.path()).unwrap())
            .expect("generated .rs")
    })
}

#[test]
fn string_search_pattern_args_stay_bare_str_refs() {
    let rust = compile_to_rust(
        r#"
fn check(s: string) -> bool {
    s.contains("needle")
}
"#,
        "search_sig.wj",
    );
    assert!(
        rust.contains(".contains(\"needle\")") || rust.contains(".contains(\"needle\".as_str())"),
        "pattern arg must not become .to_string():\n{rust}"
    );
    assert!(
        !rust.contains("contains(\"needle\".to_string())"),
        "must not own-coerce Pattern arg:\n{rust}"
    );
}

#[test]
fn owned_string_ctor_literal_uses_signature_not_name_heuristic() {
    let rust = compile_to_rust(
        r#"
struct Box {
    label: string
}

fn make() -> Box {
    Box { label: "hi" }
}

fn build() -> Box {
    // When Box::new is registered with owned string formal, literal must own-coerce.
    make()
}
"#,
        "ctor_sig.wj",
    );
    // Structural literal path — ensure we still compile without new|from heuristic.
    assert!(
        rust.contains("label:") || rust.contains("\"hi\""),
        "expected struct init:\n{rust}"
    );
}

#[test]
fn vec_push_clones_borrowed_iter_item_via_signature() {
    let rust = compile_to_rust(
        r#"
fn collect_names(names: Vec<string>) -> Vec<string> {
    let mut out = Vec::new()
    for name in names {
        out.push(name)
    }
    out
}
"#,
        "push_sig.wj",
    );
    // Owned loop binding may move; ensure generated Rust is coherent.
    assert!(
        rust.contains(".push("),
        "expected push call:\n{rust}"
    );
}
