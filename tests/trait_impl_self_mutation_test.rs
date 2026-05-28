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

/// Test: Trait Implementation Self Mutation
///
/// When a trait method declares `fn method(self, ...)` (no & or &mut),
/// but the implementation mutates self, the compiler should infer `&mut self`
/// for the implementation.
///
/// This is a critical feature for game engines where traits define interfaces
/// but implementations need to mutate state.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_mutates_self() {
    let code = r#"
        trait GameLoop {
            fn update(self, delta: f32) {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn update(self, delta: f32) {
                self.frame_count = self.frame_count + 1
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    // The trait should declare `fn update(&self, delta: f32)` (read-only default)
    // But the impl should use `fn update(&mut self, delta: f32)` (mutates self)
    assert!(
        generated.contains("fn update(&mut self, delta: f32)"),
        "Implementation should use &mut self when it mutates self, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_reads_self() {
    let code = r#"
        trait GameLoop {
            fn render(self) {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn render(self) {
                println!("Frame: {}", self.frame_count)
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    // Both trait and impl should use `&self` (read-only)
    assert!(
        generated.contains("fn render(&self)"),
        "Implementation should use &self when it only reads self, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_consumes_self() {
    let code = r#"
        trait GameLoop {
            fn cleanup(self) {
                // Default: do nothing
            }
        }
        
        struct Game {
            name: string,
        }
        
        impl GameLoop for Game {
            fn cleanup(self) {
                println!("Cleanup: {}", self.name)
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    // Both should use `&self` for Copy types (string becomes &str for read-only)
    // Or `self` if truly consuming
    assert!(
        generated.contains("fn cleanup(&self)") || generated.contains("fn cleanup(self)"),
        "Implementation should use &self or self for read-only access, got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_default_impl_mutates() {
    let code = r#"
        struct Counter {
            count: int,
        }
        
        trait Incrementable {
            fn increment(self) {
                // This would mutate if we had access to self
                // But trait methods can't access self fields directly
            }
        }
        
        impl Incrementable for Counter {
            fn increment(self) {
                self.count = self.count + 1
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    // The impl should use `&mut self` because it mutates
    assert!(
        generated.contains("fn increment(&mut self)"),
        "Implementation should use &mut self when it mutates self, got:\n{}",
        generated
    );
}
