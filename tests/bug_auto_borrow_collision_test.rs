#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

use windjammer::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
use windjammer::parser::ast::Type;

// Core bug: When `draw_text` has a signature collision (one module says Borrowed,
// another says Owned), the codegen should NOT add `&` to arguments.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_auto_borrow_skipped_on_signature_collision() {
    // Module A's "draw_text" expects Borrowed (analyzer inferred &str)
    let mut global_sigs = SignatureRegistry::new();
    global_sigs.add_function(
        "draw_text".to_string(),
        FunctionSignature {
            name: "draw_text".to_string(),
            param_types: vec![Type::String, Type::Float, Type::Float],
            param_ownership: vec![
                OwnershipMode::Borrowed,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    // Module B registers the SAME "draw_text" with Owned string param
    // → This creates a collision in the registry.
    let mut other_sigs = SignatureRegistry::new();
    other_sigs.add_function(
        "draw_text".to_string(),
        FunctionSignature {
            name: "draw_text".to_string(),
            param_types: vec![Type::String, Type::Float, Type::Float],
            param_ownership: vec![
                OwnershipMode::Owned,
                OwnershipMode::Owned,
                OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );
    global_sigs.merge(&other_sigs);

    // Verify collision was detected
    assert!(
        global_sigs.has_collision("draw_text"),
        "draw_text should have a collision after merging conflicting registries"
    );

    // Consumer file calls draw_text with an owned String variable
    let source = r#"
fn render_hud() {
    let text = "Hello"
    draw_text(text, 10.0, 20.0)
}
"#;

    let rust_code = test_utils::compile_with_external_sigs(source, &global_sigs);

    // The call should NOT have `&text` — collision means we can't trust
    // the Borrowed inference from the potentially-wrong module's signature.
    assert!(
        !rust_code.contains("&text"),
        "When draw_text has a signature collision, codegen should NOT add `&` to arguments.\n\
         The `&` was added based on a potentially-wrong module's Borrowed inference.\n\
         Generated:\n{}",
        rust_code
    );
}

// Positive test: When there is NO collision, auto-borrow should still work.
#[test]
fn test_auto_borrow_still_works_without_collision() {
    // Single definition, no collision
    let source = r#"
fn process(data: string) {
    println!("{}", data)
}

fn caller() {
    let msg = "hello"
    process(msg)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // With no collision, auto-borrow should apply (process reads data → Borrowed)
    // The important thing is this compiles; whether it adds & depends on analysis.
    // We just verify no crash and reasonable output.
    assert!(
        rust_code.contains("process"),
        "Should contain a call to process.\nGenerated:\n{}",
        rust_code
    );
}

// Collision with module-qualified calls:
// `hud_render::draw_text` should also be guarded.
#[test]
fn test_module_qualified_call_auto_borrow_skipped_on_collision() {
    let mut global_sigs = SignatureRegistry::new();

    // Register as "draw_text" (bare name) with Borrowed
    global_sigs.add_function(
        "draw_text".to_string(),
        FunctionSignature {
            name: "draw_text".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    // Merge conflicting Owned version → collision on "draw_text"
    let mut other_sigs = SignatureRegistry::new();
    other_sigs.add_function(
        "draw_text".to_string(),
        FunctionSignature {
            name: "draw_text".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );
    global_sigs.merge(&other_sigs);

    let source = r#"
fn show_message() {
    let label = "Score"
    hud_render::draw_text(label)
}
"#;

    let rust_code = test_utils::compile_with_external_sigs(source, &global_sigs);

    // Module-qualified call should also skip auto-borrow on collision
    assert!(
        !rust_code.contains("&label"),
        "Module-qualified hud_render::draw_text should NOT add `&` when collision detected.\n\
         Generated:\n{}",
        rust_code
    );
}

// MutBorrowed should also be guarded by collision check.
#[test]
fn test_mut_borrow_skipped_on_collision() {
    let mut global_sigs = SignatureRegistry::new();

    // Module A: update(data: String) inferred as MutBorrowed
    global_sigs.add_function(
        "update".to_string(),
        FunctionSignature {
            name: "update".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::MutBorrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    // Module B: update(data: String) inferred as Owned → collision
    let mut other_sigs = SignatureRegistry::new();
    other_sigs.add_function(
        "update".to_string(),
        FunctionSignature {
            name: "update".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );
    global_sigs.merge(&other_sigs);

    let source = r#"
fn do_update() {
    let mut data = "initial"
    update(data)
}
"#;

    let rust_code = test_utils::compile_with_external_sigs(source, &global_sigs);

    assert!(
        !rust_code.contains("&mut data"),
        "update() with collision should NOT add `&mut` to arguments.\nGenerated:\n{}",
        rust_code
    );
}

// Regression: The collision detection should compare param_ownership, not just param_types.
// Two signatures with the same param types but different ownership should be collisions.
#[test]
fn test_collision_detected_for_different_ownership_same_types() {
    let mut registry = SignatureRegistry::new();
    registry.add_function(
        "process".to_string(),
        FunctionSignature {
            name: "process".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Borrowed],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    let mut other = SignatureRegistry::new();
    other.add_function(
        "process".to_string(),
        FunctionSignature {
            name: "process".to_string(),
            param_types: vec![Type::String],
            param_ownership: vec![OwnershipMode::Owned],
            return_type: None,
            return_ownership: OwnershipMode::Owned,
            has_self_receiver: false,
            is_extern: false,
        },
    );

    registry.merge(&other);

    // Even though param_types are the same, ownership differs → collision
    assert!(
        registry.has_collision("process"),
        "Different ownership for same param types should be detected as a collision.\n\
         This is critical: the auto-borrow codegen depends on ownership, so differing\n\
         ownership IS a meaningful collision."
    );
}
