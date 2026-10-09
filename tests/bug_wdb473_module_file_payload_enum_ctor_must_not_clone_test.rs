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
    feature = "codegen_tests",
))]

//! WDB-473: a payload enum constructor used once must not `.clone()`.
//!
//! Product `visual_scripting/runtime.rs` `test_graph_runner`:
//!   `runner.set_variable("score".to_string(), Value::Int(0).clone())`
//! WJ is `runner.set_variable("score", Value::Int(0))`.
//! Distinct from WDB-367 (unit `Value::None`) and WDB-416 (`a.clone().as_float()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Value {
    Int(i32),
    Float(f32),
}

fn take(value: Value) -> i32 {
    match value {
        Value::Int(n) => n,
        Value::Float(_) => 0,
    }
}

pub fn go() -> i32 {
    take(Value::Int(0))
}
"#;

#[test]
fn wdb473_module_file_payload_enum_ctor_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-473 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-473 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("Value::Int(0).clone()") && !rs.contains(".clone()"),
        "WDB-473 RED: payload enum constructor cloned:\n{rs}"
    );
    let _ = test.cargo_check();
}

const METHOD_SRC: &str = r#"
pub enum Value {
    Int(i32),
    String(string),
    None,
}

pub struct Runner {
    pub n: i32,
}

impl Runner {
    pub fn set_variable(self, name: string, value: Value) {
        match value {
            Value::Int(v) => {
                self.n = v
            },
            _ => {
                self.n = 0
            },
        }
    }
}

pub fn go() -> i32 {
    let mut runner = Runner { n: 1 }
    runner.set_variable("score", Value::Int(0))
    runner.n
}
"#;

#[test]
fn wdb473_method_noncopy_payload_enum_ctor_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", METHOD_SRC);
    let map = test.compile().expect("WDB-473 method compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-473 method lib.rs:\n{rs}");
    assert!(
        !rs.contains("Value::Int(0).clone()") && !rs.contains(".clone()"),
        "WDB-473 RED: method payload enum constructor cloned:\n{rs}"
    );
}

fn wdb473_search_roots() -> Vec<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![manifest.clone()];
    let git = manifest.join(".git");
    if git.is_file() {
        if let Ok(text) = std::fs::read_to_string(&git) {
            if let Some(line) = text.lines().find(|l| l.starts_with("gitdir:")) {
                let gitdir = PathBuf::from(line.trim_start_matches("gitdir:").trim());
                if let Some(repo) = gitdir.ancestors().nth(3) {
                    roots.push(repo.to_path_buf());
                    if let Some(src_wj) = repo.parent() {
                        roots.push(src_wj.to_path_buf());
                    }
                }
            }
        }
    }
    let mut walked = manifest;
    for _ in 0..8 {
        roots.push(walked.clone());
        if let Some(parent) = walked.parent() {
            walked = parent.to_path_buf();
        } else {
            break;
        }
    }
    roots
}

#[test]
fn wdb473_tip_out_graph_runner_payload_ctor_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb473_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/visual_scripting/runtime.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/visual_scripting/runtime.rs"));
    }
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && t.contains("Value::Int(0).clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-473: visual_scripting runtime product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-473 RED: tip/product payload enum constructor cloned:\n  {}",
        bad_paths.join("\n  ")
    );
}
