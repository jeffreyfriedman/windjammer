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

//! P3.638: product `Event::get_data_string` matches `HashMap::get` → `&EventDataValue::String(value)`
//! and returns `Some(value)` into `Option<String>` without `.clone()` → E0308 (`&String`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum EventDataValue {
    String(string),
    Int(int),
}

pub struct Event {
    pub data: Map<string, EventDataValue>,
}

impl Event {
    pub fn get_data_string(self, key: string) -> Option<string> {
        match self.data.get(key) {
            Some(EventDataValue::String(value)) => Some(value),
            _ => None,
        }
    }
}
"#;

#[test]
fn hashmap_get_match_string_payload_must_clone_into_owned() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("P3.638 emit:\n{rs}");
    // Must clone (or to_owned) the borrowed match payload into owned String
    let arm_ok = rs.contains("value.clone()")
        || rs.contains("value.to_owned()")
        || rs.contains("(*value).clone()")
        || rs.contains("String::from(value)")
        || rs.contains("value.to_string()");
    assert!(
        arm_ok,
        "P3.638 RED: HashMap get match String payload must clone into owned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn tip_out_event_get_data_string_must_clone() {
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/gen/event/event.rs");
    let rs = std::fs::read_to_string(&game).unwrap_or_default();
    assert!(!rs.is_empty(), "missing {}", game.display());
    let bad = rs.lines().any(|l| {
        l.contains("EventDataValue::String(value)") && l.contains("Some(value)") && !l.contains("clone")
    });
    assert!(
        !bad,
        "P3.638 RED tip-out: get_data_string returns &String without clone in {}",
        game.display()
    );
}
