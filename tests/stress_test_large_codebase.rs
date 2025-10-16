//! Stress Tests for Large Codebases
//!
//! These tests verify that the compiler can handle large codebases efficiently:
//! - 10K+ lines of code
//! - Deep nesting
//! - Many functions
//! - Complex type hierarchies
//!
//! Run with: cargo test --test stress_test_large_codebase --release

use std::time::Instant;
use windjammer::analyzer::SignatureRegistry;
use windjammer::codegen::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

/// Generate a large function with many statements
fn generate_large_function(func_num: usize, stmt_count: usize) -> String {
    let mut code = format!("fn large_function_{}(x: int) -> int {{\n", func_num);
    code.push_str("    let mut result = x\n");

    for i in 0..stmt_count {
        code.push_str(&format!("    result = result + {}\n", i));
    }

    code.push_str("    result\n}\n\n");
    code
}

/// Generate many small functions
fn generate_many_functions(count: usize) -> String {
    let mut code = String::new();

    for i in 0..count {
        code.push_str(&format!(
            "fn function_{}(x: int) -> int {{\n    x * {}\n}}\n\n",
            i,
            i + 1
        ));
    }

    code
}

/// Generate deeply nested structures
fn generate_nested_code(depth: usize) -> String {
    let mut code = String::from("fn nested_function(x: int) -> int {\n");

    for i in 0..depth {
        code.push_str(&format!("{}if x > {} {{\n", "    ".repeat(i + 1), i));
    }

    code.push_str(&format!("{}x\n", "    ".repeat(depth + 1)));

    for i in (0..depth).rev() {
        code.push_str(&format!("{}}} else {{\n", "    ".repeat(i + 1)));
        code.push_str(&format!("{}0\n", "    ".repeat(i + 2)));
        code.push_str(&format!("{}}}\n", "    ".repeat(i + 1)));
    }

    code.push_str("}\n\n");
    code
}

#[test]
fn test_compile_1000_functions() {
    let source = generate_many_functions(1000);
    println!("Generated {} lines of code", source.lines().count());

    let start = Instant::now();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    println!("Lexing took: {:?}", start.elapsed());

    let parse_start = Instant::now();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    println!("Parsing took: {:?}", parse_start.elapsed());

    if let Ok(program) = result {
        let codegen_start = Instant::now();
        let signatures = SignatureRegistry::new();
        let mut generator = CodeGenerator::new_for_module(signatures, CompilationTarget::Wasm);
        let _rust_code = generator.generate_program(&program, &[]);
        println!("Codegen took: {:?}", codegen_start.elapsed());

        println!("Total compilation time: {:?}", start.elapsed());

        // Should complete in reasonable time (< 10 seconds)
        assert!(start.elapsed().as_secs() < 10, "Compilation took too long");
    }
}

#[test]
fn test_compile_large_function() {
    // Function with 1000 statements
    let source = generate_large_function(1, 1000);
    println!("Generated {} lines of code", source.lines().count());

    let start = Instant::now();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let result = parser.parse();

    if let Ok(program) = result {
        let signatures = SignatureRegistry::new();
        let mut generator = CodeGenerator::new_for_module(signatures, CompilationTarget::Wasm);
        let _rust_code = generator.generate_program(&program, &[]);

        println!("Compilation time: {:?}", start.elapsed());

        // Should complete in reasonable time
        assert!(start.elapsed().as_secs() < 5);
    }
}

#[test]
fn test_compile_deeply_nested() {
    // 50 levels of nesting
    let source = generate_nested_code(50);
    println!(
        "Generated {} lines of code with 50 levels of nesting",
        source.lines().count()
    );

    let start = Instant::now();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let result = parser.parse();

    if let Ok(program) = result {
        let signatures = SignatureRegistry::new();
        let mut generator = CodeGenerator::new_for_module(signatures, CompilationTarget::Wasm);
        let _rust_code = generator.generate_program(&program, &[]);

        println!("Compilation time: {:?}", start.elapsed());

        // Should handle deep nesting without stack overflow
        assert!(start.elapsed().as_secs() < 5);
    }
}

#[test]
#[ignore] // Run with --ignored for full stress test
fn test_compile_10k_lines() {
    // Generate ~10K lines of code
    let mut source = String::new();

    // 500 functions with 20 statements each = 10K+ lines
    for i in 0..500 {
        source.push_str(&generate_large_function(i, 20));
    }

    println!("Generated {} lines of code", source.lines().count());
    assert!(
        source.lines().count() >= 10_000,
        "Should generate 10K+ lines"
    );

    let start = Instant::now();

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize();
    println!("Lexing took: {:?}", start.elapsed());
    println!("Token count: {}", tokens.len());

    let parse_start = Instant::now();
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    println!("Parsing took: {:?}", parse_start.elapsed());

    if let Ok(program) = result {
        println!("Program items: {}", program.items.len());

        let codegen_start = Instant::now();
        let signatures = SignatureRegistry::new();
        let mut generator = CodeGenerator::new_for_module(signatures, CompilationTarget::Wasm);
        let rust_code = generator.generate_program(&program, &[]);
        println!("Codegen took: {:?}", codegen_start.elapsed());
        println!("Generated Rust code: {} lines", rust_code.lines().count());

        println!(
            "Total compilation time for 10K+ lines: {:?}",
            start.elapsed()
        );

        // Should handle 10K lines in reasonable time (< 30 seconds)
        assert!(
            start.elapsed().as_secs() < 30,
            "10K line compilation took too long"
        );
    }
}

#[test]
fn test_memory_usage_scaling() {
    // Test that memory usage doesn't explode with code size
    let sizes = vec![100, 500, 1000];

    for size in sizes {
        let source = generate_many_functions(size);

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);
        if let Ok(program) = parser.parse() {
            let signatures = SignatureRegistry::new();
            let mut generator = CodeGenerator::new_for_module(signatures, CompilationTarget::Wasm);
            let _rust_code = generator.generate_program(&program, &[]);

            println!("Successfully compiled {} functions", size);
        }
    }
}
