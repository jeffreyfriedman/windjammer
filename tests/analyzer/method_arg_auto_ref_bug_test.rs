// TDD Test: Compiler incorrectly adds & to Copy type method arguments
// Vec::remove expects usize (by value), not &usize

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_should_not_add_reference() {
    // BUG: Compiler incorrectly adds & to usize argument for Vec::remove
    let code = r#"
    pub fn remove_item(mut items: Vec<i32>, index: usize) -> i32 {
        return items.remove(index)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT add & to the argument
    assert!(
        !generated.contains("items.remove(&index)"),
        "Should NOT add & to usize argument for Vec::remove, got:\n{}",
        generated
    );

    // Should pass by value
    assert!(
        generated.contains("items.remove(index)"),
        "Should pass usize by value to Vec::remove, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_with_cast_should_not_add_reference() {
    // Real case from components.rs
    let code = r#"
    pub fn remove_at(mut dense: Vec<i32>, index: i64) -> i32 {
        let idx: usize = index as usize
        return dense.remove(idx)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT add & or .clone()
    assert!(
        !generated.contains("dense.remove(&idx") && !generated.contains("&idx.clone()"),
        "Should NOT add & or .clone() to usize variable, got:\n{}",
        generated
    );

    // Should be simple: dense.remove(idx)
    assert!(
        generated.contains("dense.remove(idx)"),
        "Should pass usize variable by value, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_remove_expects_reference() {
    // HashMap::remove DOES expect &K, should add &
    let code = r#"
    use std::collections::HashMap
    
    pub fn remove_key(mut map: HashMap<string, i32>, key: string) -> Option<i32> {
        return map.remove(key)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Phase-2 &str optimization can be suppressed when the param flows into HashMap.remove
    // (conservative &String path in analyzer). Rust still accepts remove(key) via coercion.
    assert!(
        generated.contains("key: &str") || generated.contains("key: &String"),
        "key parameter should be borrowed text (prefer &str, else &String); got:\n{}",
        generated
    );
    assert!(
        generated.contains("map.remove(key)"),
        "HashMap::remove should receive key directly (accepts &str), got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_method_signature_determines_ref_not_type() {
    // The method signature should determine if & is needed, not the type
    let code = r#"
    pub struct Container {
        pub items: Vec<i32>,
    }
    
    impl Container {
        pub fn remove_at(&mut self, index: usize) -> i32 {
            return self.items.remove(index)
        }
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Should NOT add & to usize for Vec::remove
    assert!(
        !generated.contains("self.items.remove(&index)"),
        "Should NOT add & to usize for Vec::remove in method, got:\n{}",
        generated
    );
}
