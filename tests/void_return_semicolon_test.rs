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

#[path = "common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_void_return_preserves_semicolon() {
    let code = r#"
    use std::collections::HashMap;
    
    struct Store {
        items: HashMap<string, i32>,
    }
    impl Store {
        pub fn add(self, key: string, value: i32) {
            self.items.insert(key, value);
        }
        
        pub fn remove(self, key: string) {
            self.items.remove(key);
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // The semicolons should be preserved to discard the return value.
    // The compiler may add .to_string() for String keys — that's fine,
    // we just need the statement to end with a semicolon.
    assert!(
        generated.contains("insert(key, value);")
            || generated.contains("insert(key.to_string(), value);"),
        "insert() should end with semicolon: {}",
        generated
    );
    assert!(
        generated.contains("remove(key);")
            || generated.contains("remove(&key);"),
        "remove() should end with semicolon: {}",
        generated
    );
}
