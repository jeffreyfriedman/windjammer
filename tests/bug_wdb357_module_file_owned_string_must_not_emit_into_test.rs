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

//! WDB-357: owned `string` formals must not emit `impl Into<String>` + `.into()`.
//!
//! Product tip game-core `console/console_command.rs` (+ dialogue/serialization/scripting):
//!   `pub fn new(name: impl Into<String>, …) { … name.into() … }`
//! Windjammer source is `name: string`. Prefer `name: String` + move (no `.into()`).
//! Rust leakage / non-idiomatic codegen.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct ConsoleCommand {
    pub name: string,
    pub description: string,
}

pub fn new(name: string, description: string) -> ConsoleCommand {
    ConsoleCommand {
        name: name,
        description: description,
    }
}
"#;

#[test]
fn wdb357_module_file_owned_string_must_not_emit_into() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-357 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-357 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("impl Into<")
        || rs.contains(".into()")
        || rs.contains("Into<String>");
    assert!(
        !bad,
        "WDB-357 RED: owned string emitted Into/into:\n{rs}"
    );
    test.cargo_check().expect("WDB-357 cargo-check");
}

#[test]
fn wdb357_tip_out_game_core_must_not_emit_into_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("console_command.rs"),
        tip.join("console/console_command.rs"),
        tip.join("dialogue_system.rs"),
        tip.join("scene_serializer.rs"),
        tip.join("serialization/scene_serializer.rs"),
        game.join("gen/console/console_command.rs"),
        game.join("gen/dialogue_system.rs"),
        game.join("gen/serialization/scene_serializer.rs"),
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
            (line.contains("impl Into<String>") || line.contains(".into()"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-357: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-357 RED: tip/product Into/into in:\n  {}",
        bad_paths.join("\n  ")
    );
}
