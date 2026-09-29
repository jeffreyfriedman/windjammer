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

//! Product: `profile_scopes.wj` / `game_loop.wj` pass `SCOPE_*` (`pub const …: string`)
//! into `string` formals / `Vec<string>::push`.
//!
//! When the formal stays owned `String`, const must auto-own (`.to_string()`).
//! Tip may demote field-store formals to `&str` — then `&SCOPE` / bare SCOPE is OK.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SCOPES: &str = r#"
pub const SCOPE_UPDATE: string = "game_loop_update"
"#;

const LOOP: &str = r#"
use crate::scopes::SCOPE_UPDATE

pub struct Recorder {
    pub last: string,
}

impl Recorder {
    pub fn record_scope(self, name: string, duration_ms: f32) {
        self.last = name
    }
}

pub fn tick(r: Recorder, ms: f32) {
    r.record_scope(SCOPE_UPDATE, ms)
}
"#;

const PROFILE: &str = r#"
use crate::scopes::SCOPE_UPDATE

pub fn all_scope_names() -> Vec<string> {
    let mut names = Vec::new()
    names.push(SCOPE_UPDATE)
    names
}
"#;

#[test]
fn string_const_into_owned_string_formal_must_auto_own() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod scopes\npub mod loop_mod\n");
    t.add_file("scopes.wj", SCOPES);
    t.add_file("loop_mod.wj", LOOP);

    let map = t.compile().expect("compile");
    let rs = map.get("loop_mod.rs").expect("loop_mod.rs");

    let formal_owned = rs.contains("name: String");
    let formal_str = rs.contains("name: &str");
    if formal_owned {
        assert!(
            rs.contains("SCOPE_UPDATE.to_string()")
                || rs.contains("SCOPE_UPDATE.to_owned()")
                || rs.contains("&SCOPE_UPDATE.to_string()"),
            "const string into owned string formal must auto-own:\n{rs}"
        );
        assert!(
            !rs.contains("record_scope(SCOPE_UPDATE,"),
            "bare const &str into owned String is E0308:\n{rs}"
        );
    } else {
        assert!(
            formal_str,
            "record_scope name formal should be String or demoted &str:\n{rs}"
        );
    }

    t.cargo_check()
        .expect("cargo check: const string → owned formal");
}

#[test]
fn string_const_into_vec_string_push_must_auto_own() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod scopes\npub mod profile\n");
    t.add_file("scopes.wj", SCOPES);
    t.add_file("profile.wj", PROFILE);

    let map = t.compile().expect("compile");
    let rs = map.get("profile.rs").expect("profile.rs");

    // Vec::push(String) stays owned — const must auto-own (not demote).
    assert!(
        rs.contains("SCOPE_UPDATE.to_string()")
            || rs.contains("SCOPE_UPDATE.to_owned()"),
        "const string into Vec<string>::push must auto-own:\n{rs}"
    );
    assert!(
        !rs.contains("names.push(SCOPE_UPDATE)"),
        "bare const &str into Vec<String>::push is E0308:\n{rs}"
    );

    t.cargo_check()
        .expect("cargo check: const string → Vec::push");
}
