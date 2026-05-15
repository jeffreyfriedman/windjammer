/// TDD Test: Nested Generics Parser Fix
///
/// Tests that the parser can handle nested generic types like HashMap<K, Vec<V>>
/// where the >> at the end should be treated as two > tokens.
#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_generics_hashmap_vec() {
    let code = r#"
use std::collections::HashMap

pub struct Test {
    // This should parse correctly: HashMap<i64, Vec<i64>>
    // The >> should be treated as two > tokens
    map: HashMap<i64, Vec<i64>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            map: HashMap::new(),
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = test_utils::compile_single_result(code).expect("Should compile");

    // Verify the generated Rust code contains the nested generic type
    assert!(
        rust_code.contains("HashMap<i64, Vec<i64>>"),
        "Generated Rust should contain HashMap<i64, Vec<i64>>"
    );
    assert!(
        rust_code.contains("pub struct Test"),
        "Generated Rust should contain struct Test"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_nested_generics_option_vec() {
    let code = r#"
pub struct Test {
    opt: Option<Vec<i64>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            opt: None,
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = test_utils::compile_single_result(code).expect("Should compile");

    // Verify the generated Rust code contains the nested generic type
    assert!(
        rust_code.contains("Option<Vec<i64>>"),
        "Generated Rust should contain Option<Vec<i64>>"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_triple_nested_generics() {
    let code = r#"
use std::collections::HashMap

pub struct Test {
    // Triple nesting: HashMap<i64, Option<Vec<i64>>>
    // This has >>> which should be treated as three > tokens
    map: HashMap<i64, Option<Vec<i64>>>,
}

impl Test {
    pub fn new() -> Test {
        Test {
            map: HashMap::new(),
        }
    }
}

fn main() {
    let test = Test::new()
    println!("Created test")
}
"#;

    let rust_code = test_utils::compile_single_result(code).expect("Should compile");

    // Verify the generated Rust code contains the triple nested generic type
    assert!(
        rust_code.contains("HashMap<i64, Option<Vec<i64>>>"),
        "Generated Rust should contain HashMap<i64, Option<Vec<i64>>>"
    );
}
