use windjammer::analyzer::SignatureRegistry;
/// TDD Test: crate:: imports (especially for FFI modules)
///
/// Tests that `use crate::ffi` and other `crate::` imports are generated correctly.
///
/// Bug: The compiler was filtering out `use crate::ffi` statements, causing
/// E0433 errors when code tried to use `ffi::function()`.
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

#[test]
fn test_crate_ffi_import() {
    // Create a Windjammer file that imports crate::ffi
    let source = r#"
use crate::ffi

pub fn call_ffi() {
    ffi::some_function()
}
"#;

    // Tokenize, parse and generate
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &[]);

    println!("Generated Rust code:\n{}", rust_code);

    // THE WINDJAMMER WAY: use crate::ffi should be emitted as-is
    assert!(
        rust_code.contains("use crate::ffi;"),
        "Generated code should contain 'use crate::ffi;'\nGenerated:\n{}",
        rust_code
    );

    // Should NOT convert to something else
    assert!(
        !rust_code.contains("use windjammer_runtime::ffi"),
        "Should not map crate::ffi to windjammer_runtime"
    );
}

#[test]
fn test_crate_module_imports() {
    // Test various crate:: import patterns
    let test_cases = vec![
        ("use crate::ffi", "use crate::ffi;"),
        ("use crate::math", "use crate::math;"),
        ("use crate::math::Vec2", "use crate::math::Vec2;"),
        (
            "use crate::rendering::Color",
            "use crate::rendering::Color;",
        ),
    ];

    for (input, expected) in test_cases {
        let source = format!("{}\n\npub fn test() {{}}", input);

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parse failed");

        let registry = SignatureRegistry::new();
        let mut generator =
            windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
        let rust_code = generator.generate_program(&program, &[]);

        assert!(
            rust_code.contains(expected),
            "Input '{}' should generate '{}'\nGenerated:\n{}",
            input,
            expected,
            rust_code
        );
    }
}

#[test]
fn test_crate_imports_not_filtered() {
    // THE WINDJAMMER WAY: crate:: imports should NEVER be filtered or remapped
    // They refer to the user's own modules, not stdlib

    let source = r#"
use crate::ffi
use crate::GameLoop
use crate::math::Vec2

pub fn test() {
    ffi::initialize()
}
"#;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    let registry = SignatureRegistry::new();
    let mut generator =
        windjammer::codegen::rust::CodeGenerator::new(registry, CompilationTarget::Rust);
    let rust_code = generator.generate_program(&program, &[]);

    println!("Generated:\n{}", rust_code);

    // All three should be present
    assert!(
        rust_code.contains("use crate::ffi;"),
        "Missing: use crate::ffi;"
    );
    assert!(
        rust_code.contains("use crate::GameLoop;"),
        "Missing: use crate::GameLoop;"
    );
    assert!(
        rust_code.contains("use crate::math::Vec2;"),
        "Missing: use crate::math::Vec2;"
    );
}
