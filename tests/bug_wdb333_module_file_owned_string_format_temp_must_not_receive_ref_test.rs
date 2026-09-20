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

//! WDB-333: format temp into owned `String` formal must not receive `&_temp`.
//!
//! Product tip game-core `assets/loader.rs`:
//!   `loader.load(_temp0, &_temp1, …)` with `path: String` → E0308.
//! Twin of WDB-306/325 (owned String call args). Prefer move `_temp1`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// `.len()` demotes to `&str`. Force owned String formals via struct store.
const SRC: &str = r#"
pub struct Asset {
    pub name: string,
    pub path: string,
    pub size: i64,
}

pub fn load(name: string, path: string, size: i64) -> Asset {
    Asset {
        name: name,
        path: path,
        size: size,
    }
}

pub fn load_level(level_name: string) -> Asset {
    let name = level_name + "_tilemap"
    let path = "levels/" + level_name + "/tilemap.json"
    load(name, path, 8192)
}
"#;

#[test]
fn wdb333_module_file_owned_string_format_temp_must_not_receive_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-333 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-333 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn load(name: String") || rs.contains("fn load(mut name: String");
    assert!(owned, "WDB-333: expected owned String formals on load:\n{rs}");
    let bad = rs.contains("load(&name")
        || rs.contains("load(name, &path")
        || rs.contains("&_temp")
        || rs.contains("load(&");
    assert!(
        !bad,
        "WDB-333 RED: owned String load received &name/&path:\n{rs}"
    );
    test.cargo_check().expect("WDB-333 cargo-check");
}

#[test]
fn wdb333_tip_out_game_core_loader_must_not_pass_ref_format_temp() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("loader.rs"),
        tip.join("assets/loader.rs"),
        game.join("gen/assets/loader.rs"),
        game.join("assets/loader.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("loader");
        // Only fail when load takes owned String path and call passes &_temp1.
        let formal_owned = text.contains("path: String") || text.contains("path: String,");
        let bad_call = text.lines().any(|line| {
            line.contains(".load(") && (line.contains("&_temp") || line.contains(", &path)"))
        });
        if formal_owned && bad_call {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-333: game-core/tip loader missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-333 RED: tip/product passes &_temp into owned String path in:\n  {}",
        bad_paths.join("\n  ")
    );
}
