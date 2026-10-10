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

//! `EventType` registers `Map::get -> Option<V>` (no `&`). Match must still treat
//! the payload as `&String` and `.clone()` into `Option<String>`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const EVENT_TYPE: &str = r#"
pub struct EventType {
    name: string,
}
impl EventType {
    pub fn new(name: string) -> EventType {
        EventType { name: name }
    }
}
"#;

const EVENT: &str = r#"
use std::map::Map

pub enum EventDataValue {
    String(string),
    Int(i32),
}

pub struct Event {
    event_type: EventType,
    data: Map<string, EventDataValue>,
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
fn module_file_map_get_string_payload_must_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "mod event_type\nmod event\n");
    test.add_file("event_type.wj", EVENT_TYPE);
    test.add_file("event.wj", EVENT);
    let map = test.compile().expect("compile multipass");
    let rs = map
        .get("event.rs")
        .cloned()
        .or_else(|| {
            map.values()
                .find(|s| s.contains("get_data_string"))
                .cloned()
        })
        .expect("event.rs with get_data_string");
    eprintln!("multipass emit:\n{rs}");
    let arm_ok = rs.lines().any(|l| {
        l.contains("EventDataValue::String(value)")
            && (l.contains("clone()") || l.contains("to_owned()") || l.contains("String::from"))
    });
    assert!(
        arm_ok,
        "RED: Map::get String match payload must clone:\n{rs}"
    );
}

#[test]
fn single_file_hashmap_get_string_payload_must_clone() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "lib.wj",
        r#"
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
"#,
    );
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    let arm_ok = rs.contains("value.clone()")
        || rs.contains("value.to_owned()")
        || rs.contains("(*value).clone()");
    assert!(arm_ok, "RED single-file:\n{rs}");
    test.cargo_check().expect("cargo-check");
}
