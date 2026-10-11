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

//! Edit a dependency so its inferred ownership changes, rebuild, revert the edit, rebuild:
//! the generated output of an UNCHANGED dependent must equal a clean build of the reverted
//! sources. A stale `.wj-cache` / meta fingerprint must never leak phantom signatures.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::collections::BTreeMap;

const MOD: &str = "pub mod callee\npub mod caller\n";

const CALLER: &str = r#"
use crate::callee::measure

pub fn run(x: string) -> int {
    measure(x) + measure(x)
}
"#;

/// `s` only read: inferred borrowed.
const CALLEE_BORROWING: &str = r#"
pub fn measure(s: string) -> int { s.len() }
"#;

/// `s` stored into a struct: inferred owned.
const CALLEE_OWNING: &str = r#"
pub struct Holder { pub name: string }

pub fn measure(s: string) -> int {
    let h = Holder { name: s }
    h.name.len()
}
"#;

fn project(callee: &str) -> MultiFileTest {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", MOD);
    t.add_file("callee.wj", callee);
    t.add_file("caller.wj", CALLER);
    t
}

fn ordered(map: std::collections::HashMap<String, String>) -> BTreeMap<String, String> {
    map.into_iter().collect()
}

fn run_cycle(first: &str, second: &str) {
    let clean = ordered(project(first).compile().expect("clean build"));

    let clean_second = ordered(project(second).compile().expect("clean build of edit"));
    assert_ne!(
        clean_second.get("caller.rs"),
        clean.get("caller.rs"),
        "precondition: the edit must change the caller's emitted calls"
    );

    let mut t = project(first);
    t.compile().expect("initial build");
    t.add_file("callee.wj", second);
    let edited = ordered(t.compile().expect("edited build"));
    assert_eq!(
        edited.get("caller.rs"),
        clean_second.get("caller.rs"),
        "incremental build after editing a dependency must re-emit its unchanged dependents"
    );
    t.add_file("callee.wj", first);
    let reverted = ordered(t.compile().expect("reverted build"));

    assert_eq!(
        reverted.get("caller.rs"),
        clean.get("caller.rs"),
        "reverted incremental build must equal a clean build of the same sources"
    );
    assert_eq!(reverted.get("callee.rs"), clean.get("callee.rs"));
}

#[test]
fn revert_borrowing_to_owning_to_borrowing_matches_clean() {
    run_cycle(CALLEE_BORROWING, CALLEE_OWNING);
}

#[test]
fn revert_owning_to_borrowing_to_owning_matches_clean() {
    run_cycle(CALLEE_OWNING, CALLEE_BORROWING);
}
