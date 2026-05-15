//! TDD: Method receiver ownership inference
//!
//! Tests for infer_method_receiver_ownership + generate_expression_with_target_ownership.
//! Verifies correct receiver generation (owned, borrowed, mut borrowed) for method calls.

#[path = "../common/test_utils.rs"]
mod test_utils;

#[test]
fn test_method_owned_receiver() {
    let src = r#"
        pub struct Timer { id: i32 }
        impl Timer {
            pub fn id(self) -> i32 { self.id }
        }
        pub fn get_id(t: Timer) -> i32 {
            t.id()
        }
    "#;

    let result = test_utils::compile_single_result(src).expect("Should compile");
    // t is Owned, id() takes Owned -> t.id() (no clone)
    assert!(result.contains("t.id()"));
}

#[test]
fn test_method_borrowed_receiver() {
    let src = r#"
        pub fn length(s: string) -> usize {
            s.len()
        }
    "#;

    let result = test_utils::compile_single_result(src).expect("Should compile");
    // s is Owned, len() takes &self -> s.len() (auto-borrow)
    assert!(result.contains("s.len()"));
}

#[test]
fn test_method_builder_pattern() {
    let src = r#"
        pub struct Builder { value: i32 }
        impl Builder {
            pub fn with_value(self, v: i32) -> Self {
                Builder { value: v }
            }
        }
        pub fn build(b: Builder) -> Builder {
            b.with_value(42)
        }
    "#;

    let result = test_utils::compile_single_result(src).expect("Should compile");
    // b is Owned, with_value() — literals may be suffixed (42_i32)
    assert!(result.contains("b.with_value(42"));
}

#[test]
fn test_method_mutating_receiver() {
    let src = r#"
        pub fn add_item(items: Vec<i32>) {
            items.push(5)
        }
    "#;

    let result = test_utils::compile_single_result(src).expect("Should compile");
    // items + push: literal may be `5_i32`
    assert!(result.contains("push(5"));
}
