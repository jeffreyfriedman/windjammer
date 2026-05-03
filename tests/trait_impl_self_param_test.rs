// Test: Trait implementation should match trait method signatures
//
// Bug: Analyzer infers `&self` for methods that access fields,
// but trait requires `self` (owned). This causes E0053 errors.
//
// Expected: When implementing a trait method, use the trait's
// self parameter type, not the inferred type.

#[path = "test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_self_param_owned() {
    // FIXED: The analyzer now infers owned self when a non-Copy field is returned

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

    // Should compile successfully
    assert!(
        result.is_ok(),
        "Trait impl should compile: {:?}",
        result.err()
    );

    let generated = result.unwrap();

    // Verify generated Rust uses owned self (matches trait)
    assert!(
        generated.contains("fn render(self) -> String"),
        "Expected 'fn render(self)' but got:\n{}",
        generated
    );
    assert!(
        !generated.contains("fn render(&self)"),
        "Should NOT use &self when trait requires self"
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
