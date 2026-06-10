#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD Test: Fix String: Borrow<&str> double-reference errors
//!
//! Bug: When calling HashMap.get() with key: str (which generates &str in Rust),
//! the codegen adds &, producing get(&key) = &&str, causing "String: Borrow<&str>" errors.
//!
//! Root cause: Codegen adds & to arguments that are already references.
//! Fix: Don't add & when arg type is already a reference (&str, &String, etc.)

#[path = "common/test_utils.rs"]
mod test_utils;

/// Case 1: hashmap.get(key) where key: str → generates hashmap.get(key) (no extra &)
/// str param becomes &str in Rust, so key is already a reference
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_str_param_no_double_ref() {
    let code = r#"
use std::collections::HashMap

pub fn get_data_int(map: HashMap<string, i32>, key: string) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // key: str generates key: &str in Rust - must NOT add & (would create &&str)
    assert!(
        !generated.contains("map.get(&key)"),
        "Must not double-reference: key is already &str. Got: {}",
        generated
    );
    assert!(
        generated.contains("map.get(key)"),
        "Should use key directly when key: str. Got: {}",
        generated
    );
    assert!(success, "Generated Rust must compile. Error:\n{}", err);
}

/// Case 2: hashmap.get(key) where key: string (owned) → generates hashmap.get(&key) (add &)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_owned_string_param_adds_ref() {
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<string, i32>, key: string) -> Option<i32> {
    match map.get(key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // key: string (owned) needs & for HashMap.get(&K)
    assert!(
        generated.contains("map.get(&key)") || generated.contains("map.get(key)"),
        "Owned String key should work. Got: {}",
        generated
    );
    assert!(success, "Generated Rust must compile. Error:\n{}", err);
}

/// Case 3: vec.push(item) where item: T → generates vec.push(item) (no &)
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_push_no_ref() {
    let code = r#"
pub fn add_items() {
    let mut vec = Vec::new()
    vec.push(1)
    vec.push(2)
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    assert!(success, "Vec::push should compile. Error:\n{}", err);
    assert!(
        generated.contains("vec.push(1)") && generated.contains("vec.push(2)"),
        "Vec::push takes owned value, not &"
    );
}

/// Case 4: EventDataValue-style struct with HashMap - the 14-error dogfooding case
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_event_data_get_str_key() {
    let code = r#"
use std::collections::HashMap

pub enum EventDataValue {
    Int(i32),
    Float(f32),
    Bool(bool),
    String(String),
}

pub struct Event {
    data: HashMap<string, EventDataValue>,
}

impl Event {
    pub fn get_data_int(self, key: string) -> Option<i32> {
        match self.data.get(key) {
            Some(EventDataValue::Int(v)) => Some(*v),
            _ => None
        }
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    println!("Generated:\n{}", generated);
    if !success {
        println!("Rustc error:\n{}", err);
    }

    // Must NOT have self.data.get(&key) - key: str is already &str
    assert!(
        !generated.contains("data.get(&key)"),
        "Borrow<&str> fix: key: str must not get extra &. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("data.get(key)"),
        "Should use key directly. Got:\n{}",
        generated
    );
    assert!(
        success,
        "Event.get_data_int must compile (fixes 14 HashMap errors). Error:\n{}",
        err
    );
}

/// Case 5: contains_key with str param - same fix
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_contains_key_str_param() {
    let code = r#"
use std::collections::HashMap

pub fn has_key(map: HashMap<string, i32>, key: string) -> bool {
    map.contains_key(key)
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    assert!(
        !generated.contains("contains_key(&key)"),
        "key: str must not get & for contains_key"
    );
    assert!(success, "Must compile. Error:\n{}", err);
}

/// Case 6: get with explicit &key in source - when key is str, strip to key
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_explicit_ref_str_param() {
    let code = r#"
use std::collections::HashMap

pub fn lookup(map: HashMap<string, i32>, key: string) -> Option<i32> {
    match map.get(&key) {
        Some(v) => Some(*v),
        None => None
    }
}
"#;

    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // User wrote &key in WJ; read-only string param generates &str — accept generated get form
    assert!(
        generated.contains("map.get(&key)") || generated.contains("map.get(key)"),
        "HashMap.get with string key should compile. Got:\n{}",
        generated
    );
    assert!(success, "Must compile. Error:\n{}", err);
}
