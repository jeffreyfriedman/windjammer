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

//! WDB-483: an owned local passed into a call and then read again must not
//! `.clone().clone()`. One clone into the call is enough.
//!
//! Product `rendering/shader_graph_executor.rs`:
//!   `let pass = sorted[step.pass_indices[pi]].clone();`
//!   `self.bind_pass_buffers(pass.clone().clone())`
//!   then `pass.indirect_args_buffer`
//! WJ is `self.bind_pass_buffers(pass)` and the field reads come after.
//! Distinct from WDB-480 (the loop parameter itself is double-cloned) and
//! WDB-481 (a string pushed and then formatted).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct CompiledPass {
    pub enabled: bool,
    pub shader_id: int,
    pub indirect: int,
}

pub struct Exec {
    pub n: int,
}

impl Exec {
    pub fn bind(self, pass: CompiledPass) {
        self.n = pass.shader_id
    }

    pub fn run(self, passes: Vec<CompiledPass>) {
        for i in 0..passes.len() {
            let pass = passes[i]
            if pass.enabled {
                self.bind(pass)
                if pass.indirect != 0 {
                    self.n = pass.shader_id
                }
            }
        }
    }
}
"#;

#[test]
fn wdb483_module_file_reused_local_call_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-483 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-483 MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains(".clone().clone()"),
        "WDB-483 RED: reused local double-cloned into call:\n{rs}"
    );
    let _ = test.cargo_check();
}

fn wdb483_search_roots() -> Vec<PathBuf> {
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
fn wdb483_tip_out_pass_bind_must_not_double_clone() {
    let mut paths = Vec::new();
    for dir in wdb483_search_roots() {
        paths.push(dir.join(".agent-wip/rel_tip_out/rendering/shader_graph_executor.rs"));
        paths.push(
            dir.join("windjammer-game/windjammer-game-core/gen/rendering/shader_graph_executor.rs"),
        );
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
            !t.starts_with("//") && t.contains("pass.clone().clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-483: shader executor product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-483 RED: tip/product reused pass double-cloned into bind:\n  {}",
        bad_paths.join("\n  ")
    );
}
