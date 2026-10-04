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

//! P3.638: product `Event::get_data_string` — `use std::map::Map` + cross-module
//! `EventType` registers `Map::get -> Option<V>` (no `&`). Match must still treat
//! the payload as `&String` and `.clone()` into `Option<String>`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;
use std::process::Command;

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
    eprintln!("P3.638 multipass emit:\n{rs}");
    let arm_ok = rs.lines().any(|l| {
        l.contains("EventDataValue::String(value)")
            && (l.contains("clone()") || l.contains("to_owned()") || l.contains("String::from"))
    });
    assert!(
        arm_ok,
        "P3.638 RED: Map::get String match payload must clone:\n{rs}"
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
    assert!(arm_ok, "P3.638 RED single-file:\n{rs}");
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
    let clones = rs.lines().any(|l| {
        l.contains("EventDataValue::String(value)")
            && (l.contains("clone()") || l.contains("to_owned()"))
    });
    assert!(
        clones,
        "P3.638 RED tip-out: get_data_string must clone in {}",
        game.display()
    );
}

#[test]
fn tip_wj_product_event_module_must_clone() {
    let wj = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/.cargo-target-wj/release/wj");
    if !wj.is_file() {
        eprintln!("skip: no tip wj at {}", wj.display());
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core/src/event");
    let out = tempfile::tempdir().expect("tmpdir");
    let status = Command::new(&wj)
        .args([
            "build",
            root.join("mod.wj").to_str().unwrap(),
            "--output",
            out.path().to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .status()
        .expect("wj");
    assert!(status.success(), "tip wj build event module failed");
    let rs = std::fs::read_to_string(out.path().join("event.rs")).expect("event.rs");
    let clones = rs.lines().any(|l| {
        l.contains("EventDataValue::String(value)")
            && (l.contains("clone()") || l.contains("to_owned()"))
    });
    assert!(
        clones,
        "P3.638 RED tip product event module:\n{}",
        rs.lines()
            .filter(|l| l.contains("get_data") || l.contains("String(value)"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
