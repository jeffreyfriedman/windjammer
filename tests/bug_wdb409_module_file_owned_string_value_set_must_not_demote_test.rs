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

//! WDB-409: owned `string` + value formals that store into fields must stay owned.
//!
//! Product:
//!   `VariableScope::set(&mut self, name: &str, value: &Value)` then
//!   `self.variables[i].value = value` (E0308: expected Value, found &Value)
//!   and `push(Variable { name: name.to_string(), value: value.clone(), ... })`
//!   `GraphRunner::set_variable(..., name: &String, value: &Value)`
//!   `UnifiedRenderer::load_gltf(..., name: &String, path: &str)`
//! WJ is `set(name: string, value: Value)` / `set_variable(name: string, value: Value)`
//! / `load_gltf(name: string, path: string)` — formals are stored or forwarded owned.
//! Distinct from WDB-407 (Vec `new`) and WDB-186 (call-site `&String` into still-owned).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Val {
    N(i32),
}

pub struct Binding {
    pub name: string,
    pub value: Val,
}

pub struct Scope {
    pub bindings: Vec<Binding>,
}

impl Scope {
    pub fn new() -> Scope {
        Scope { bindings: Vec::new() }
    }

    pub fn set(self, name: string, value: Val) {
        let mut i: usize = 0
        while i < self.bindings.len() {
            if self.bindings[i].name == name {
                self.bindings[i].value = value
                return
            }
            i = i + 1
        }
        self.bindings.push(Binding { name: name, value: value })
    }
}

pub struct Runner {
    pub scope: Scope,
}

impl Runner {
    pub fn set_variable(self, name: string, value: Val) {
        self.scope.set(name, value)
    }
}
"#;

#[test]
fn wdb409_module_file_owned_string_value_set_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-409 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-409 MultiFile lib.rs:\n{rs}");
    let demoted_set = rs.contains("fn set(") && (rs.contains("name: &str")
        || rs.contains("name: &String")
        || rs.contains("value: &Val"));
    let demoted_wrapper = rs.contains("fn set_variable(")
        && (rs.contains("name: &String")
            || rs.contains("name: &str")
            || rs.contains("value: &Val"));
    assert!(
        !demoted_set && !demoted_wrapper,
        "WDB-409 RED: owned string/value set() demoted to refs:\n{rs}"
    );
    test.cargo_check().expect("WDB-409 cargo-check");
}

fn wdb409_search_roots() -> Vec<PathBuf> {
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
fn wdb409_tip_out_game_core_owned_string_value_set_must_not_demote() {
    let mut paths = Vec::new();
    for dir in wdb409_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/visual_scripting/runtime.rs"));
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/unified_renderer.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/visual_scripting/runtime.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/rendering/unified_renderer.rs"));
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
            !t.starts_with("//")
                && (t.contains("fn set(&mut self, name: &str, value: &Value)")
                    || t.contains("fn set_variable(&mut self, name: &String, value: &Value)")
                    || t.contains("fn load_gltf(&mut self, name: &String"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-409: set/load_gltf product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-409 RED: tip/product owned string/value set() demoted in:\n  {}",
        bad_paths.join("\n  ")
    );
}
