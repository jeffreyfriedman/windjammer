/// TDD Test: crate:: imports should NOT include the output directory name
///
/// Bug: When compiling to ./build/ and build/lib.rs doesn't exist yet,
/// the compiler incorrectly rewrites `use crate::math::Vec3` to 
/// `use crate::build::math::Vec3` because it finds the parent crate's
/// lib.rs and treats the build/ directory as a submodule.
///
/// Fix: Known output directories (build, generated, out) should always
/// be treated as the crate root, not as submodules.

use windjammer::analyzer::SignatureRegistry;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_crate_import_no_build_prefix_with_parent_lib() {
    let source = r#"
use crate::math::Vec3
use crate::ai::PerceptionState

pub fn test_func() -> i32 {
    42
}
"#;

    // Reproduce the exact scenario: parent directory has lib.rs from a different crate
    let temp_dir = TempDir::new().unwrap();
    let parent_dir = temp_dir.path();
    let build_dir = parent_dir.join("build");
    let ai_dir = build_dir.join("ai");
    fs::create_dir_all(&ai_dir).unwrap();

    // Parent has lib.rs (existing Rust crate, NOT part of the output)
    fs::write(parent_dir.join("lib.rs"), "// existing rust crate").unwrap();

    // build/lib.rs does NOT exist yet (CLI generates it AFTER individual files)
    // This is the chicken-and-egg scenario

    let output_file = ai_dir.join("npc_behavior.rs");

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);

    generator.set_is_module(true);
    generator.set_output_file(&output_file);

    let rust_code = generator.generate_program(&program, &[]);

    println!("Generated Rust:\n{}", rust_code);

    // CRITICAL: crate:: imports should NOT include "build::" prefix
    assert!(
        !rust_code.contains("crate::build::"),
        "BUG: crate:: imports should NOT include build:: prefix.\n\
         The parent lib.rs belongs to a different crate, not this output.\n\
         Expected: use crate::math::Vec3;\n\
         Got crate::build:: in generated code:\n{}",
        rust_code
    );

    assert!(
        rust_code.contains("use crate::math::Vec3;"),
        "Should contain 'use crate::math::Vec3;'\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_crate_import_correct_when_build_has_lib() {
    let source = r#"
use crate::math::Vec3

pub fn test_func() -> i32 {
    42
}
"#;

    // When build/lib.rs EXISTS, it should be treated as crate root (no prefix)
    let temp_dir = TempDir::new().unwrap();
    let parent_dir = temp_dir.path();
    let build_dir = parent_dir.join("build");
    let ai_dir = build_dir.join("ai");
    fs::create_dir_all(&ai_dir).unwrap();

    // Both parent and build have lib.rs
    fs::write(parent_dir.join("lib.rs"), "// parent crate").unwrap();
    fs::write(build_dir.join("lib.rs"), "// generated crate root").unwrap();

    let output_file = ai_dir.join("handler.rs");

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);

    generator.set_is_module(true);
    generator.set_output_file(&output_file);

    let rust_code = generator.generate_program(&program, &[]);

    assert!(
        !rust_code.contains("crate::build::"),
        "When build/lib.rs exists, should NOT add build:: prefix.\nGenerated:\n{}",
        rust_code
    );
}
