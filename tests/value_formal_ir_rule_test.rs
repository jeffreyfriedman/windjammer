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

//! Value formals spelled from the IR mode (convergence O4), public ones included: shapes a larger
//! program hit once public formals joined the rule. Each program must be accepted by `rustc`.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn type_checks(src: &str) {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("m.wj"), src).unwrap();
    let o = Command::new(WJ).args(["build", "m.wj", "--no-cargo", "--output", "out"]).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "wj build failed:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    let rs = dir.path().join("out/m.rs");
    let r = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(&rs)
        .output()
        .unwrap();
    assert!(
        r.status.success(),
        "generated Rust does not type-check:\n{}\n--- generated\n{}",
        String::from_utf8_lossy(&r.stderr),
        fs::read_to_string(&rs).unwrap_or_default()
    );
}

/// A parameter bound to a local that is then moved into a struct is owned (OWN-STORED through a `let`
/// alias, like OWN-RETURNED already flows through one).
#[test]
fn param_stored_through_a_let_alias_is_owned() {
    type_checks(
        r#"pub struct Request {
    pub headers: Vec<int>,
    pub tag: string,
}

pub fn make_request(headers: Vec<int>, tag: string) -> Request {
    let kept = headers
    Request { headers: kept, tag: tag }
}
"#,
    );
}

/// A map formal iterated by value and compared element-wise keeps the owned spelling: borrowing it
/// would make `pair.0` a `&String` compared with a `String`.
#[test]
fn map_formal_iterated_and_compared_type_checks() {
    type_checks(
        r#"pub fn query_value(query: std::collections::HashMap<string, string>, key: string) -> Option<string> {
    let wanted = key
    for pair in query {
        if pair.0 == wanted {
            return Some(pair.1)
        }
    }
    None
}
"#,
    );
}
