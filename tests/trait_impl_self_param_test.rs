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

// Test: Trait implementation self parameter inference
//
// DESIGN DECISION: When a method body is `self.field` (returning a non-Copy
// field), the compiler infers `&self` + auto-clone. This applies even in
// trait impls — both trait and impl get `&self` consistently. This prevents
// cascading E0382 errors at callsites.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_field_return_infers_borrowed_self() {
    // Windjammer Way: field-return methods get &self + auto-clone
    let code = r#"
        trait Renderable {
            fn render(self) -> string
        }
        
        struct Text {
            content: string
        }
        
        impl Renderable for Text {
            fn render(self) -> string {
                self.content
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);

    assert!(
        result.is_ok(),
        "Trait impl should compile: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Both trait and impl use &self for field-return methods
    assert!(
        generated.contains("fn render(&self) -> String"),
        "Field-return methods should use &self (auto-clone pattern). Got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_self_param_borrowed() {
    // TDD: Test that &self in trait is respected

    let code = r#"
        trait Displayable {
            fn display(&self) -> string
        }
        
        struct Label {
            text: string
        }
        
        impl Displayable for Label {
            fn display(&self) -> string {
                self.text.clone()
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);

    // Should compile successfully
    assert!(
        result.is_ok(),
        "Trait impl should compile: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify generated Rust uses &self (matches trait)
    assert!(
        generated.contains("fn display(&self) -> String"),
        "Expected 'fn display(&self)' but got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_self_param_mutable() {
    // TDD: Test that &mut self in trait is respected

    let code = r#"
        trait Updatable {
            fn update(&mut self, value: int)
        }
        
        struct Counter {
            count: int
        }
        
        impl Updatable for Counter {
            fn update(&mut self, value: int) {
                self.count = value
            }
        }
    "#;

    let result = test_utils::compile_single_result(code);

    // Should compile successfully
    assert!(
        result.is_ok(),
        "Trait impl should compile: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify generated Rust uses &mut self (matches trait)
    assert!(
        generated.contains("fn update(&mut self, value: i64)"),
        "Expected 'fn update(&mut self, value: i64)' but got:\n{}",
        generated
    );
}
