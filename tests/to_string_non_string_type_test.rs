#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Verify: .to_string() on non-string types (i32, f32) is legitimate Windjammer
/// and must be preserved in generated Rust. The compiler should NOT strip it.
#[test]
fn test_to_string_on_int_preserved_for_push_str() {
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
        html.push_str(self.rows.to_string())
        html.push_str(",max=")
        html.push_str(self.max_length.to_string())
        html
    }
}
"#,
    );

    t.assert_contains("config.rs", ".to_string()");
}
