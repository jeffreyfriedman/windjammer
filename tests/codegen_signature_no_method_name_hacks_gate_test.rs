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

//! Signature-driven ownership: no method-name lists for storage / consume / Option adapters.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile(source: &str, name: &str) -> String {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join(name);
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
    let rs = out.join(name.replace(".wj", ".rs"));
    fs::read_to_string(&rs).unwrap_or_else(|_| {
        fs::read_dir(&out)
            .unwrap()
            .flatten()
            .find(|e| e.path().extension().is_some_and(|x| x == "rs"))
            .map(|e| e.path())
            .map(|p| fs::read_to_string(p).unwrap())
            .expect("generated .rs")
    })
}

/// Domain type named `push` must coerce via signature (owned string formal), not
/// because the method spelling matches Vec::push.
#[test]
fn domain_push_owned_string_literal_via_signature_not_name() {
    let rust = compile(
        r#"
struct EventLog {
    last: string
}

impl EventLog {
    fn new() -> EventLog {
        EventLog { last: "" }
    }

    fn push(self, msg: string) {
        self.last = msg
    }
}

fn emit() {
    let mut log = EventLog::new()
    log.push("evt")
}
"#,
        "domain_push.wj",
    );
    assert!(
        rust.contains("push(\"evt\".to_string())") || rust.contains("push(String::from(\"evt\"))"),
        "owned string formal on domain push must coerce literal:\n{rust}"
    );
}

/// Borrowed iterator item calling a consuming method must clone via signature
/// (owned self), not because the method is named something special.
#[test]
fn borrowed_iter_consuming_method_clones_via_signature() {
    let rust = compile(
        r#"
struct Token {
    text: string
}

impl Token {
    fn into_text(self) -> string {
        self.text
    }
}

fn collect_texts(tokens: Vec<Token>) -> Vec<string> {
    let mut out = Vec::new()
    for t in tokens {
        out.push(t.into_text())
    }
    out
}
"#,
        "consume_sig.wj",
    );
    // for-in on Vec yields owned elements in WJ → Rust often borrows; consuming
    // into_text must clone when the loop var is borrowed.
    let has_consume = rust.contains("into_text(");
    let clones_before_consume = rust.contains(".clone().into_text(")
        || rust.contains("clone()\n") && rust.contains("into_text");
    assert!(
        has_consume,
        "expected into_text call:\n{rust}"
    );
    // If the loop is `for t in &tokens` / borrowed, clone is required.
    if rust.contains("for t in &") || rust.contains("for t in tokens.iter") {
        assert!(
            clones_before_consume || rust.contains(".clone().into_text"),
            "borrowed loop var + owned-self method must clone:\n{rust}"
        );
    }
}

/// Borrowed Option field + adapter that Rust implements by-value (`map`) must
/// lower via `.as_ref()` — driven by Option::{method} ownership, not a name list.
#[test]
fn borrowed_option_map_uses_as_ref_via_signature() {
    let rust = compile(
        r#"
struct Node {
    child: Option<string>
}

fn child_lens(node: Node) -> Option<int> {
    node.child.map(|s| s.len())
}
"#,
        "opt_map.wj",
    );
    assert!(
        rust.contains(".map(") || rust.contains(".map (|") || rust.contains("map("),
        "expected map in generated Rust:\n{rust}"
    );
    // Borrowed `node` + Option::map (by-value in Rust) → `.as_ref().map(...)`.
    if rust.contains("node: &") || rust.contains("node:&") {
        assert!(
            rust.contains(".as_ref()") || rust.contains("as_ref()"),
            "map on borrowed Option path needs as_ref:\n{rust}"
        );
    }
}
