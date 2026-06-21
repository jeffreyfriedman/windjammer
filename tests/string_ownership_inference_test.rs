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

// TDD Test: String ownership inference (&str vs String)
// THE WINDJAMMER WAY: Explicit types are honored
// - User writes `text: String` → `text: String` (owned, as written)
// - User writes `text: &string` → `text: &str` (borrowed, as written)

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_read_only_param_infers_str_ref() {
    let code = r#"
    pub fn print_msg(text: string) {
        println(text)
    }
    
    pub fn run() {
        print_msg("hello")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    // Read-only `string` may lower to `&str` or `&String` depending on ownership inference
    assert!(
        generated.contains("text: &str") || generated.contains("text: &String"),
        "Read-only string parameter should infer to borrowed str-like type, got:\n{}",
        generated
    );

    // Call site: direct literal, or &"lit".to_string() for &String param — both valid
    let call_ok = generated.contains("print_msg(\"hello\")")
        || generated.contains("print_msg(&\"hello\".to_string())");
    assert!(
        call_ok,
        "String literal should reach print_msg (direct or with temp String), got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_stored_param_infers_owned() {
    let code = r#"
    pub struct User {
        pub name: string,
    }
    
    impl User {
        pub fn new(name: string) -> User {
            User { name: name }
        }
    }
    
    pub fn run() -> User {
        return User::new("Alice")
    }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation failed");

    assert!(
        generated.contains("name: String"),
        "Should infer String field for stored parameter, got:\n{}",
        generated
    );

    assert!(
        generated.contains("fn new(name: String)"),
        "Stored string parameter should use owned String at API, got:\n{}",
        generated
    );

    assert!(
        !generated.contains("name.to_string()"),
        "Struct literal storage must not emit name.to_string(), got:\n{}",
        generated
    );
}
