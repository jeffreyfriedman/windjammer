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
))]

//! P3.731: string literal passed to an owned `String` formal must emit `.to_string()`.
//!
//! Engine tip-out (`gen/plugin/core.rs` `record_resource`, `gen/scripting/live_reload.rs`
//! `add_dependency`) is the dominant E0308 class: expected `String`, found `&str`
//! (46 of 272 mismatches on windjammer-game-core cargo, 2026-10-07).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn owned_string_formal_literal_must_to_string() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod plugin
pub mod boot
"#,
    );
    test.add_file(
        "plugin.wj",
        r#"
pub struct App {
    pub resources: Vec<string>,
}
impl App {
    pub fn record_resource(self, name: string) {
        self.resources.push(name)
    }
}
"#,
    );
    test.add_file(
        "boot.wj",
        r#"
use crate::plugin::App

pub fn boot(ctx: App) {
    ctx.record_resource("audio_initialized")
}
"#,
    );
    let map = test
        .compile()
        .expect("P3.731: owned string formal fixture must transpile");
    let body = map
        .get("boot.rs")
        .or_else(|| map.get("boot/mod.rs"))
        .unwrap_or_else(|| panic!("boot.rs missing; keys={:?}", map.keys().collect::<Vec<_>>()));
    assert!(
        body.contains("audio_initialized") && body.contains("to_string()"),
        "literal into owned String formal must .to_string(); got:\n{body}"
    );
}
