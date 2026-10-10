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

//! WDB-482: a local `string` passed into a call on one `if let` arm and
//! returned on the other must not `.clone()`. The arms are exclusive.
//!
//! Product `dialogue_system.rs` `current_text`:
//!   `self.substitute_variables(text.clone().clone(), world)`
//!   then `return text`
//! WJ is `return self.substitute_variables(text, world)` or `return text`.
//! Distinct from WDB-478 (a `for` binding into `push`) and WDB-481 (a string
//! pushed and then used again, where one clone is required).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Node {
    pub body: string,
}

pub fn node_text(node: Node) -> string {
    node.body
}

pub struct World {
    pub flag: int,
}

pub fn substitute(text: string, world: World) -> string {
    text
}

pub fn current_text(node: Option<Node>, world: Option<World>) -> string {
    if let Some(n) = node {
        let text = node_text(n)
        if let Some(w) = world {
            return substitute(text, w)
        }
        return text
    }
    ""
}
"#;

#[test]
fn wdb482_module_file_exclusive_arm_string_call_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-482 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-482 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("text.clone()"),
        "WDB-482 RED: exclusive-arm string cloned into call:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb482_search_roots() -> Vec<PathBuf> {
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
fn wdb482_tip_out_dialogue_text_must_not_clone() {
    let mut paths = Vec::new();
    for dir in wdb482_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/dialogue_system.rs"));
        paths.push(dir.join("windjammer-game/windjammer-game-core/gen/dialogue_system.rs"));
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
            !t.starts_with("//") && t.contains("substitute_variables(text.clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-482: dialogue product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-482 RED: tip/product exclusive-arm string cloned into call:\n  {}",
        bad_paths.join("\n  ")
    );
}
