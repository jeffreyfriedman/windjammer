//! E0053 / E0186 edge cases: std traits not defined in `.wj`, qualified trait keys, AST type fallback.

#[path = "integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

/// Rust `Drop::drop` takes `&mut self`. Windjammer sources use `fn drop(self)`; codegen must match std.
#[test]
fn test_stdlib_drop_impl_generates_mut_self() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "droppable.wj",
        r#"
// Non-Copy shell: Rust forbids `Copy` + `Drop` on the same type; all-`i32` structs get `Copy` derived.
pub struct Droppable {
    pub tag: i32,
    pub label: string,
}

impl Droppable {
    pub fn new(tag: i32) -> Droppable {
        Droppable { tag, label: String::new() }
    }
}

impl Drop for Droppable {
    fn drop(self) {
        let _ = self.tag
    }
}
"#,
    );
    test.assert_contains("droppable.rs", "fn drop(&mut self)");
    test.assert_compiles_without_error();
}
