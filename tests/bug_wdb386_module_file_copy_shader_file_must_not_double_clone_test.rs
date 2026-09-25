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

//! WDB-386: indexed Copy ShaderFile must not emit `].clone().shader_file.clone()`.
//!
//! Product tip game-core `rendering/shader_graph_compiler.rs` + tests:
//!   `sorted[i].clone().shader_file.clone()`
//!   `g.passes[0].clone().shader_file`
//! Prefer `sorted[i].shader_file` (Copy enum). Twin of WDB-377.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum ShaderFile {
    A,
    B,
}

pub struct Pass {
    pub shader_file: ShaderFile,
}

pub fn first(passes: Vec<Pass>) -> ShaderFile {
    passes[0].shader_file
}
"#;

#[test]
fn wdb386_module_file_copy_shader_file_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-386 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-386 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().shader_file") || rs.contains(".shader_file.clone()");
    assert!(
        !bad,
        "WDB-386 RED: indexed Copy ShaderFile cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-386 cargo-check");
}

#[test]
fn wdb386_tip_out_game_core_must_not_double_clone_shader_file() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/shader_graph_compiler.rs"),
        tip.join("shader_graph_compiler.rs"),
        tip.join("tests/shader_effect_test.rs"),
        game.join("gen/rendering/shader_graph_compiler.rs"),
        game.join("gen/tests/shader_effect_test.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            !line.trim_start().starts_with("//") && line.contains("].clone().shader_file")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-386: shader_file product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-386 RED: tip/product indexed shader_file clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
