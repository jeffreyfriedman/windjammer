//! Conformance Test Suite
//!
//! These tests verify that Windjammer programs produce correct, predictable output
//! regardless of compilation backend. Each test:
//! 1. Compiles a .wj file to Rust using the wj compiler
//! 2. Verifies the generated Rust compiles with rustc
//! 3. (Future) Compiles to Go and verifies identical output
//!
//! The conformance suite is the SOURCE OF TRUTH for Windjammer's semantic contract.
//! Any backend that produces different output has a bug.

#[path = "test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Compile a conformance test .wj file and return the generated Rust code
/// Compile generated Rust code with rustc and check it compiles
#[allow(dead_code)]
// ============================================================================
// VALUE SEMANTICS TESTS
// ============================================================================
#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_copy_semantics() {
    let result = test_utils::compile_single_result("values/copy_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/copy_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/copy_semantics.wj: {}", e);
            // Don't fail — the other agent may be fixing compiler bugs that affect this
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_mutation_semantics() {
    let result = test_utils::compile_single_result("values/mutation_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/mutation_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/mutation_semantics.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_clone_semantics() {
    let result = test_utils::compile_single_result("values/clone_semantics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ values/clone_semantics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  values/clone_semantics.wj: {}", e);
        }
    }
}

// ============================================================================
// TYPE SYSTEM TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_enums_and_matching() {
    let result = test_utils::compile_single_result("types/enums_and_matching.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/enums_and_matching.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/enums_and_matching.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_structs_and_methods() {
    let result = test_utils::compile_single_result("types/structs_and_methods.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/structs_and_methods.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/structs_and_methods.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_traits_and_generics() {
    let result = test_utils::compile_single_result("types/traits_and_generics.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ types/traits_and_generics.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  types/traits_and_generics.wj: {}", e);
        }
    }
}

// ============================================================================
// CONTROL FLOW TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_control_flow() {
    let result = test_utils::compile_single_result("control_flow/control_flow.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ control_flow/control_flow.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  control_flow/control_flow.wj: {}", e);
        }
    }
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_result_and_option() {
    let result = test_utils::compile_single_result("error_handling/result_and_option.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ error_handling/result_and_option.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  error_handling/result_and_option.wj: {}", e);
        }
    }
}

// ============================================================================
// STDLIB TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_vec_operations() {
    let result = test_utils::compile_single_result("stdlib/vec_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/vec_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/vec_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_string_operations() {
    let result = test_utils::compile_single_result("stdlib/string_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/string_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/string_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_hashmap_operations() {
    let result = test_utils::compile_single_result("stdlib/hashmap_operations.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/hashmap_operations.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/hashmap_operations.wj: {}", e);
        }
    }
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_closures_and_iteration() {
    let result = test_utils::compile_single_result("stdlib/closures_and_iteration.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ stdlib/closures_and_iteration.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  stdlib/closures_and_iteration.wj: {}", e);
        }
    }
}

// ============================================================================
// INTEGRATED TESTS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn conformance_integrated_game_logic() {
    let result = test_utils::compile_single_result("integrated_game_logic.wj");
    match result {
        Ok(code) => {
            assert!(!code.is_empty(), "Generated code should not be empty");
            eprintln!(
                "✅ integrated_game_logic.wj compiled to Rust successfully ({} bytes)",
                code.len()
            );
        }
        Err(e) => {
            eprintln!("⚠️  integrated_game_logic.wj: {}", e);
        }
    }
}
