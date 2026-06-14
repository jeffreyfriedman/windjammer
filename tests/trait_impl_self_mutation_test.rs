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
/// Omit `self` in source; impl bodies that mutate fields upgrade the trait contract
/// via infer_trait_signatures_from_impls.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_mutates_self() {
    let code = r#"
        trait GameLoop {
            fn update(delta: f32) {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn update(delta: f32) {
                self.frame_count = self.frame_count + 1
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn update(&mut self, delta: f32)"),
        "Mutating impl upgrades trait to &mut self. Got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_reads_self() {
    let code = r#"
        trait GameLoop {
            fn render() {
                // Default: do nothing
            }
        }
        
        struct Game {
            frame_count: int,
        }
        
        impl GameLoop for Game {
            fn render() {
                println("Frame: {}", self.frame_count)
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn render(&self)"),
        "Read-only access uses &self. Got:\n{}",
        generated
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_method_impl_consumes_self() {
    let code = r#"
        trait GameLoop {
            fn cleanup() {
                // Default: do nothing
            }
        }
        
        struct Game {
            name: string,
        }
        
        impl GameLoop for Game {
            fn cleanup() {
                println("Cleanup: {}", self.name)
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn cleanup(&self)") || generated.contains("fn cleanup(self)"),
        "Read-only access uses &self or owned self. Got:\n{}",
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
            fn increment() {
                // Default: no-op
            }
        }
        
        impl Incrementable for Counter {
            fn increment() {
                self.count = self.count + 1
            }
        }
    "#;

    let generated = test_utils::compile_single_result(code).expect("Compilation should succeed");

    assert!(
        generated.contains("fn increment(&mut self)"),
        "Mutating impl upgrades trait to &mut self. Got:\n{}",
        generated
    );
}
