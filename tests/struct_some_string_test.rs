#[path = "test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_some_string_literal() {
    let code = r#"
    struct Config {
        name: string,
        parent: Option<string>,
    }
    impl Config {
        pub fn new() -> Config {
            Config {
                name: "default",
                parent: Some("root"),
            }
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Some("root") should convert to Some("root".to_string()) (codegen may emit an extra .to_string())
    assert!(
        generated.contains(r#"Some("root".to_string()"#),
        "Some with string literal should auto-convert in struct field: {}",
        generated
    );
    assert!(
        generated.contains(r#"name: "default".to_string()"#),
        "Direct string literal field should also convert: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_field_ok_string_literal() {
    let code = r#"
    struct Response {
        status: Result<string, string>,
    }
    impl Response {
        pub fn success() -> Response {
            Response {
                status: Ok("success"),
            }
        }
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Ok("success") may have an extra .to_string() in expansion
    assert!(
        generated.contains(r#"Ok("success".to_string()"#),
        "Ok with string literal should auto-convert in struct field: {}",
        generated
    );
}
