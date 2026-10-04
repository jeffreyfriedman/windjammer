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

//! P3.590: owned `String` path formal must not receive `&path` / `&_temp` at
//! `load(` call sites. `detect_format(&path)` into `&str` is fine.

use std::path::PathBuf;

#[test]
fn tip_out_loader_owned_string_path_must_not_borrow() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/assets/loader.rs");
    let rs = std::fs::read_to_string(&game).unwrap_or_default();
    assert!(!rs.is_empty(), "missing {}", game.display());
    assert!(
        rs.contains("path: String") && rs.contains("pub fn load("),
        "expected owned path: String formal"
    );
    let mut bad = Vec::new();
    for (i, line) in rs.lines().enumerate() {
        let l = line.trim();
        if !l.contains("load(") || l.contains("pub fn load") || l.contains("fn load") {
            continue;
        }
        if l.contains("&_temp") || l.contains(", &path,") || l.contains("(name.clone(), &path") {
            bad.push(format!("{}:{}", i + 1, l));
        }
    }
    assert!(
        bad.is_empty(),
        "P3.590 RED: owned String path borrowed at load() sites:\n  {}",
        bad.join("\n  ")
    );
}
