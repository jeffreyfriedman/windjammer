/// TDD Test: HashMap/BTreeMap with String keys - E0277 fix
///
/// Problem: HashMap<String, T>.contains_key(&name) where name is &String
/// creates &&String which doesn't satisfy Borrow trait
///
/// Solution: Strip explicit & for borrowed String params passed to HashMap key methods
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_hashmap_string_key_contains() {
    let source = r#"
use std::collections::HashMap

fn check_exists(map: HashMap<string, int>, key: string) -> bool {
    map.contains_key(&key)
}

fn main() {
    let mut map = HashMap::new()
    map.insert("foo".to_string(), 42)
    let result = check_exists(map, "foo")
    println("{}", result)
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated code:\n{}", generated);

    // Phase 2 may produce `key: &str`; conservative HashMap-key analysis yields `key: &String`.
    assert!(
        generated.contains("key: &str") || generated.contains("key: &String"),
        "Expected borrowed text key parameter.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("contains_key(key)"),
        "Should pass key directly (already &str). Generated:\n{}",
        generated
    );
}

#[test]
fn test_hashmap_string_key_get() {
    let source = r#"
use std::collections::HashMap

fn get_value(map: HashMap<string, int>, key: string) -> Option<int> {
    map.get(&key).cloned()
}

fn main() {
    let mut map = HashMap::new()
    map.insert("foo".to_string(), 42)
    let result = get_value(map, "foo")
    println("{}", result)
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated code:\n{}", generated);

    assert!(
        generated.contains("key: &str") || generated.contains("key: &String"),
        "Expected borrowed text key parameter.\nGenerated:\n{}",
        generated
    );
    assert!(
        generated.contains("map.get(key)"),
        "Should pass key directly (already &str). Generated:\n{}",
        generated
    );
}
