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

//! WDB-402: Copy `HostType::F32` / `ShaderType::F32` must not emit `.clone()`.
//!
//! WDB-384/392/397/401 cover FaceDirection / Direction / TileType / StreamState;
//! product gen/rendering/type_safety_validator.rs still has
//! `HostType::F32.clone()` and `ShaderType::F32.clone()`.
//! WJ source is `HostType::F32` / `ShaderType::F32` in UniformBinding::new.
//! Numbered 402 because 395–400 are reserved or used by the compiler agent.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum HostType {
    F32,
    U32,
}

pub enum ShaderType {
    F32,
    U32,
}

pub struct Binding {
    pub host: HostType,
    pub shader: ShaderType,
}

pub fn screen_width() -> Binding {
    Binding {
        host: HostType::F32,
        shader: ShaderType::F32,
    }
}

pub fn sample_count() -> Binding {
    Binding {
        host: HostType::U32,
        shader: ShaderType::U32,
    }
}
"#;

#[test]
fn wdb402_module_file_hosttype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-402 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-402 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("HostType::F32.clone()")
        || rs.contains("HostType::U32.clone()")
        || rs.contains("ShaderType::F32.clone()")
        || rs.contains("ShaderType::U32.clone()");
    assert!(
        !bad,
        "WDB-402 RED: Copy HostType/ShaderType unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-402 cargo-check");
}

#[test]
fn wdb402_tip_out_game_core_type_safety_must_not_clone_hosttype() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/type_safety_validator.rs"),
        tip.join("type_safety_validator.rs"),
        game.join("gen/rendering/type_safety_validator.rs"),
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
            let t = line.trim_start();
            !t.starts_with("//")
                && (t.contains("HostType::") || t.contains("ShaderType::"))
                && t.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-402: type_safety_validator product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-402 RED: tip/product HostType::/ShaderType::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
