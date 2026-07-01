#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Bug: the codegen must auto-convert non-string types (i32, f32) to string
/// when passed to functions expecting string params (like push_str).
/// The user should NOT need to write .to_string() — that's Rust leakage.
#[test]
fn test_int_field_auto_converts_to_string_for_push_str() {
    let mut t = MultiFileTest::new();
    t.add_file(
        "config.wj",
        r#"
pub struct Config {
    rows: i32,
    max_length: i32,
}

impl Config {
    pub fn new() -> Config {
        Config { rows: 5, max_length: 100 }
    }

    pub fn render(self) -> string {
        let mut html = String::new()
        html.push_str("rows=")
        html.push_str(self.rows)
        html.push_str(",max=")
        html.push_str(self.max_length)
        html
    }
}
"#,
    );

    t.assert_contains("config.rs", ".to_string()");
}
