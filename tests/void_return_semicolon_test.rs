#[path = "test_utils.rs"]
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
        pub fn add(&mut self, key: string, value: i32) {
            self.items.insert(key, value);
        }
        
        pub fn remove(&mut self, key: &string) {
            self.items.remove(key);
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // The semicolons should be preserved to discard the return value
    assert!(
        generated.contains("insert(key, value);"),
        "insert() should end with semicolon: {}",
        generated
    );
    // key is already &string in the parameter, so we don't add another &
    // We just need to verify the semicolon is preserved
    assert!(
        generated.contains("remove(key);"),
        "remove() should end with semicolon: {}",
        generated
    );
}
