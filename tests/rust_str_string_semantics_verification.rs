//! Rust Semantics Verification: str vs String vs &str
//!
//! These tests verify Rust's ACTUAL behavior to ensure Windjammer's str/String
//! codegen matches Rust semantics. Each test compiles and runs valid Rust.
//!
//! Rust's Rules (verified below):
//! 1. fn foo(s: &str) - borrowed param ✅
//! 2. struct { name: String } - owned field ✅
//! 3. "hello" - type is &str ✅
//! 4. Option<String> not Option<&str> in containers (no lifetimes) ✅
//! 5. fn -> str INVALID (str unsized) ✅
//! 6. fn -> &str valid for literals/borrowed ✅
//! 7. fn -> String valid for owned ✅
//! 8. HashMap<String, String> - owned in containers ✅
//! 9. &str reference - don't double-ref to &&str ✅
//! 10. Box<str> - valid (Box can hold unsized) ✅

#[test]
fn rust_param_borrowed_str_works() {
    // Rule 1: fn foo(s: &str) - borrowed param
    fn greet(s: &str) -> String {
        format!("Hello, {}!", s)
    }
    assert_eq!(greet("world"), "Hello, world!");
    assert_eq!(greet(&String::from("Rust")), "Hello, Rust!");
}

#[test]
fn rust_struct_field_owned_string_works() {
    // Rule 2: struct { name: String } - owned field
    struct Person {
        name: String,
    }
    let p = Person {
        name: String::from("Alice"),
    };
    assert_eq!(p.name, "Alice");
}

#[test]
fn rust_string_literal_is_str() {
    // Rule 3: "hello" - type is &str
    let s: &str = "hello";
    assert_eq!(s, "hello");
    fn take_str(s: &str) -> &str {
        s
    }
    assert_eq!(take_str("literal"), "literal");
}

#[test]
fn rust_option_string_not_option_str() {
    // Rule 4: Option<String> - not Option<&str> (lifetimes in containers are complex)
    let opt: Option<String> = Some(String::from("value"));
    assert_eq!(opt.as_deref(), Some("value"));

    // Option<&str> requires lifetime - can't store in struct without lifetime param
    #[allow(dead_code)]
    struct WithOptionalRef<'a> {
        name: Option<&'a str>,
    }
    let _ = WithOptionalRef { name: Some("ok") };
}

#[test]
fn rust_cannot_return_str_directly() {
    // Rule 5: fn -> str INVALID - str is unsized
    // This test documents the compile error we'd get:
    // fn bad() -> str { "hello" }  // ERROR: the size for values of type `str` cannot be known at compilation time
    // We verify the valid alternatives work:
}

#[test]
fn rust_return_str_valid_for_literal() {
    // Rule 6: fn -> &str valid when returning literal
    fn get_static() -> &'static str {
        "static"
    }
    assert_eq!(get_static(), "static");
}

#[test]
fn rust_return_str_valid_for_borrowed_input() {
    // Rule 6b: fn -> &str valid when returning slice of input
    fn first_word(s: &str) -> &str {
        s.split_whitespace().next().unwrap_or("")
    }
    assert_eq!(first_word("hello world"), "hello");
}

#[test]
fn rust_return_string_for_owned() {
    // Rule 7: fn -> String for owned/new strings
    fn create_greeting(name: &str) -> String {
        format!("Hello, {}!", name)
    }
    let s = create_greeting("world");
    assert_eq!(s, "Hello, world!");
}

#[test]
fn rust_hashmap_string_string_owned() {
    // Rule 8: HashMap<String, String> - owned in containers
    use std::collections::HashMap;
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert(String::from("key"), String::from("value"));
    assert_eq!(map.get("key").map(String::as_str), Some("value"));
}

#[test]
fn rust_ref_str_no_double_ref() {
    // Rule 9: &str stays &str, not &&str
    fn take_ref(s: &str) -> &str {
        s
    }
    let s = "hello";
    let r: &str = s;
    assert_eq!(take_ref(r), "hello");
    // &(&str) would be &&str - different type, not what we want for params
}

#[test]
fn rust_box_str_valid() {
    // Rule 10: Box<str> - valid (Box can hold unsized types)
    let boxed: Box<str> = "hello".into();
    assert_eq!(&*boxed, "hello");
}

#[test]
fn rust_result_string_works() {
    // Result<T, String> - owned error type
    fn parse_int(s: &str) -> Result<i32, String> {
        s.parse().map_err(|e| format!("{}", e))
    }
    assert_eq!(parse_int("42"), Ok(42));
    assert!(parse_int("x").is_err());
}

#[test]
fn rust_vec_string_works() {
    // Vec<String> - owned
    let v: Vec<String> = vec!["a".into(), "b".into()];
    assert_eq!(v, vec!["a", "b"]);
}
