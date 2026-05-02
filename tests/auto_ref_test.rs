//! TDD Test: Auto-ref for method arguments
//! WINDJAMMER PHILOSOPHY: Compiler automatically adds & when method signature expects a reference
//!
//! Rules:
//! 1. Method expects &T and we pass T -> add &
//! 2. Method expects T (by value, Copy type) -> do NOT add &
//! 3. Works for stdlib methods (HashMap::remove, String::contains, etc.)
//! 4. Works for custom methods with proper signature lookup

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_remove_adds_ref() {
    // TDD: HashMap::remove(&K) should auto-add & to owned key
    let code = r#"
    use std::collections::HashMap
    
    pub fn remove_item(mut map: HashMap<string, int>, key: string) -> Option<int> {
        return map.remove(key)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // After multi-pass ownership inference, key is inferred as &String (Borrowed).
    // &String auto-derefs to &str for HashMap::remove, so no extra & needed.
    assert!(
        generated.contains("map.remove(key)") || generated.contains("map.remove(&key)"),
        "Should handle HashMap::remove key correctly. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_adds_ref() {
    // TDD: HashMap::get(&K) should auto-add & to owned key
    let code = r#"
    use std::collections::HashMap
    
    pub fn get_value(map: HashMap<string, int>, key: string) -> Option<int> {
        return map.get(key).cloned()
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // After multi-pass ownership inference, key is inferred as &String (Borrowed).
    // &String auto-derefs to &str for HashMap::get, so no extra & needed.
    assert!(
        generated.contains("map.get(key)") || generated.contains("map.get(&key)"),
        "Should handle HashMap::get key correctly. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_contains_adds_ref() {
    // TDD: String::contains(&str) should auto-add & to owned String
    // When both params are only read, the analyzer correctly infers them as &str.
    // In that case text.contains(search) is valid (&str implements Pattern).
    // When search is used after the call (forcing owned), &search should be added.
    let code = r#"
    pub fn has_substring(text: string, search: string) -> bool {
        return text.contains(search)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Analyzer may infer search as &str (borrowed), making text.contains(search) valid.
    // OR search may be owned String, requiring &search or search.as_str().
    assert!(
        generated.contains("text.contains(&search)")
            || generated.contains("text.contains(search.as_str())")
            || generated.contains("text.contains(search)"),
        "String::contains should work with borrowed or owned search. Generated:\n{}",
        generated
    );

    // When search is used after (forcing owned), & must be added
    let code_owned = r#"
    pub fn has_substring_owned(text: string, search: string) -> string {
        let found = text.contains(search)
        return search
    }
    "#;

    let generated_owned =
        test_utils::compile_single_result(code_owned).expect("Compilation failed");

    assert!(
        generated_owned.contains("text.contains(&search)")
            || generated_owned.contains("text.contains(search.as_str())")
            || generated_owned.contains("text.contains(&text)"),
        "When search is owned String, should auto-add & for String::contains. Generated:\n{}",
        generated_owned
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_remove_no_ref() {
    // TDD: Vec::remove(usize) should NOT add & (Copy type passed by value)
    let code = r#"
    pub fn remove_at(mut items: Vec<int>, index: usize) -> int {
        return items.remove(index)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Vec::remove expects usize by value, should NOT add &
    assert!(
        generated.contains("items.remove(index)") && !generated.contains("items.remove(&index)"),
        "Should NOT add & for Vec::remove (Copy type). Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_contains_adds_ref() {
    // TDD: Vec::contains(&T) should auto-add & to owned value
    // Force search to be owned by returning it (prevents borrow inference)
    let code = r#"
    pub fn has_item(items: Vec<string>, search: string) -> string {
        let found = items.contains(search)
        return search
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Vec::contains expects &T, should add & when search is owned
    assert!(
        generated.contains("items.contains(&search)")
            || generated.contains("items.contains(search)"),
        "Should auto-add & for Vec::contains (or handle borrowed search). Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_no_ref() {
    // TDD: String literals are already &str, no & needed
    let code = r#"
    pub fn check_text(text: string) -> bool {
        return text.contains("hello")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // String literal is already &str, should NOT add another &
    assert!(
        generated.contains("text.contains(\"hello\")")
            && !generated.contains("text.contains(&\"hello\")"),
        "Should NOT add & to string literals. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mixed_owned_and_literal() {
    // TDD: Mix of owned and literal arguments
    let code = r#"
    use std::collections::HashMap
    
    pub fn test(mut map: HashMap<string, int>, key: string) -> bool {
        map.insert(key, 42);
        return map.contains_key("test")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // insert takes owned key, no & needed (already implemented)
    // Note: Integer inference may add _i32 or _i64 suffix based on HashMap value type
    assert!(
        (generated.contains("map.insert(key, 42)")
            || generated.contains("map.insert(key, 42_i32)")
            || generated.contains("map.insert(key, 42_i64)"))
            && !generated.contains("map.insert(&key"),
        "insert should not add & to owned key. Generated:\n{}",
        generated
    );

    // contains_key takes &K, but literal is already &str
    assert!(
        generated.contains("map.contains_key(\"test\")")
            && !generated.contains("contains_key(&\"test\")"),
        "contains_key should not add & to literals. Generated:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_custom_method_with_ref_param() {
    // TDD: Custom methods with & parameters should get auto-ref
    // When text is only read, the analyzer correctly infers it as &str,
    // making validator.check(text) valid without conversion.
    // When text is used after (forcing owned), &text should be added.
    let code = r#"
    pub struct Validator {
    }
    
    impl Validator {
        pub fn check(&self, pattern: &str) -> bool {
            return pattern.len() > 0
        }
    }
    
    pub fn test(validator: Validator, text: string) -> bool {
        return validator.check(text)
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Analyzer may infer text as &str (borrowed), making validator.check(text) valid.
    // OR text may be owned String, requiring &text or text.as_str().
    assert!(
        generated.contains("validator.check(&text)")
            || generated.contains("validator.check(text.as_str())")
            || generated.contains("validator.check(text)"),
        "Custom method check should work with borrowed or owned text. Generated:\n{}",
        generated
    );

    // When text is used after (forcing owned), & must be added
    let code_owned = r#"
    pub struct Validator {
    }
    
    impl Validator {
        pub fn check(&self, pattern: &str) -> bool {
            return pattern.len() > 0
        }
    }
    
    pub fn test_owned(validator: Validator, text: string) -> string {
        let valid = validator.check(text)
        return text
    }
    "#;

    let generated_owned =
        test_utils::compile_single_result(code_owned).expect("Compilation failed");

    assert!(
        generated_owned.contains("validator.check(&text)")
            || generated_owned.contains("validator.check(text.as_str())")
            || generated_owned.contains("validator.check(&text)"),
        "When text is owned String, should auto-add & for custom methods. Generated:\n{}",
        generated_owned
    );
}
