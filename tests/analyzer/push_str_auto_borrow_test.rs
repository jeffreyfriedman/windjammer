#[path = "../common/test_utils.rs"]
mod test_utils;

/// Helper to compile Windjammer code and return the generated Rust code
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_str_with_string_variable() {
    let code = r#"
    pub fn build_html(display: string) -> string {
        let html = "<div>"
        html.push_str(display)
        html.push_str("</div>")
        return html
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Windjammer `string` params are inferred as &str, so push_str(display) is correct
    // (no extra & needed since display is already &str)
    assert!(
        generated.contains("push_str(display)") || generated.contains("push_str(&display)"),
        "push_str with string variable should work (display is &str): {}",
        generated
    );
    assert!(
        generated.contains(r#"push_str("</div>")"#),
        "push_str with string literal should not add &"
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_str_with_string_expression() {
    let code = r#"
    pub fn build_tag(tag: string) -> string {
        let html = "<"
        html.push_str(tag.clone())
        html.push_str(">")
        return html
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // tag is a `string` param (inferred &str), clone() returns String which may need &
    assert!(
        generated.contains("push_str(&tag.clone())")
            || generated.contains("push_str(&tag.to_string())")
            || generated.contains("push_str(&tag)")
            || generated.contains("push_str(tag.clone()")
            || generated.contains("push_str(tag)"),
        "push_str with string expression should work: {}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_push_str_with_conditional() {
    let code = r#"
    pub fn build_style(enabled: bool) -> string {
        let html = "style=\""
        let value = if enabled { "visible" } else { "hidden" }
        html.push_str(value)
        html.push_str("\"")
        return html
    }
    "#;
    let generated = test_utils::compile_single_result(code).expect("Compilation failed");
    // Note: if-else string literals may be converted to String for consistency,
    // in which case value needs & for push_str
    // This is correct behavior - push_str(&value) when value is String
    let has_correct_push_str =
        generated.contains("push_str(&value)") || generated.contains("push_str(value)");
    assert!(
        has_correct_push_str,
        "push_str should handle value correctly (with or without &): {}",
        generated
    );
}
