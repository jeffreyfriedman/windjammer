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

//! P3.418: `self.search_query = query` when tip demotes `query: string` to `&mut String`
//! must emit `.to_string()` (or `.clone()`) into owned `String` field — console/hierarchy.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Console {
    pub search_query: string,
}

impl Console {
    pub fn set_search_query(query: string) {
        self.search_query = query
    }
}
"#;

#[test]
fn mut_string_formal_assign_to_owned_string_field_must_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.418 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.418 MultiFile lib.rs:\n{rs}");
    assert!(
        rs.contains("query: &mut String")
            || rs.contains("query: &String")
            || rs.contains("query: &str"),
        "P3.418: expected demoted query formal:\n{rs}"
    );
    assert!(
        !rs.contains("search_query = query;"),
        "P3.418 RED: bare assign of demoted query into owned String:\n{rs}"
    );
    assert!(
        rs.contains("search_query = query.to_string()")
            || rs.contains("search_query = query.clone()"),
        "P3.418: expected to_string/clone into owned field:\n{rs}"
    );
    test.cargo_check().expect("P3.418 cargo-check");
}

#[test]
fn tip_out_editor_console_set_search_query_must_to_string() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        game.join("gen/editor/console.rs"),
        game.join("gen/editor/hierarchy_panel.rs"),
    ];
    let mut saw = false;
    let mut bad = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        for (i, line) in text.lines().enumerate() {
            let t = line.trim();
            if t.contains("search_query = query")
                && !t.contains("to_string")
                && !t.contains("clone")
                && !t.starts_with("//")
            {
                bad.push(format!("{}:{}: {}", path.display(), i + 1, t));
            }
        }
    }
    assert!(saw, "P3.418 tip product files missing");
    assert!(
        bad.is_empty(),
        "P3.418 RED: tip bare demoted assign:\n  {}",
        bad.join("\n  ")
    );
}
