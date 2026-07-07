#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: compiler strips `.to_string()` from non-string fields passed to `push_str()`.
/// `self.rows.to_string()` (i32 → String) becomes `&self.rows` (i32), which fails
/// because `push_str` expects `&str`.
///
/// The fix must preserve `.to_string()` when it's a type conversion, not a no-op.
#[test]
fn test_to_string_on_non_string_field_in_push_str() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "textarea.wj",
        r##"
pub struct Textarea {
    rows: i32,
    max_length: i32,
    placeholder: string,
    value: string,
}

impl Textarea {
    pub fn new() -> Textarea {
        Textarea {
            rows: 4,
            max_length: 0,
            placeholder: "".to_string(),
            value: "".to_string(),
        }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        html.push_str("<textarea rows=\"")
        html.push_str(self.rows.to_string())
        html.push_str("\"")
        if self.max_length > 0 {
            html.push_str(" maxlength=\"")
            html.push_str(self.max_length.to_string())
            html.push_str("\"")
        }
        html.push_str(">")
        html.push_str(self.value)
        html.push_str("</textarea>")
        html
    }
}
"##,
    );

    // First check what code is generated
    let map = t.compile().expect("compile");
    let code = map.get("textarea.rs").expect("no textarea.rs generated");
    eprintln!("=== GENERATED textarea.rs ===\n{}\n===", code);

    // The generated Rust must preserve .to_string() on i32 fields
    assert!(
        code.contains("self.rows.to_string()"),
        "self.rows.to_string() must be preserved in generated code for i32→String conversion"
    );
    assert!(
        code.contains("self.max_length.to_string()"),
        "self.max_length.to_string() must be preserved in generated code for i32→String conversion"
    );

    // And it must compile
    t.assert_compiles_without_error();
}
