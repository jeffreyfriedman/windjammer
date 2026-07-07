#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Bug: When a method expects &str, passing self.field.clone() produces String.
// The codegen should emit &self.field (borrow) instead of self.field.clone().

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn self_string_field_to_str_param_no_clone() {
    let source = r#"
struct VNode {
    handle: i32,
}

impl VNode {
    pub fn on_click(self, handler: string) -> VNode {
        self
    }
}

struct Button {
    click_handler: string,
}

impl Button {
    pub fn render(self) -> VNode {
        let mut node = VNode { handle: 0 }
        if !self.click_handler.is_empty() {
            node = node.on_click(self.click_handler)
        }
        node
    }
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed");

    let has_clone_for_handler = generated.contains("self.click_handler.clone()");
    let has_borrow_for_handler = generated.contains("&self.click_handler");

    assert!(
        !has_clone_for_handler || has_borrow_for_handler,
        "When method expects &str, self.field should be borrowed (&self.field) not cloned.\nGenerated:\n{}",
        generated
    );
}
