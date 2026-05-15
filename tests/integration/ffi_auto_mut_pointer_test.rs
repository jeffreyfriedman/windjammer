//! TDD tests for automatic `&mut` → `*mut` conversion in FFI calls.
//!
//! Bug: Windjammer .wj files must write `&mut x` when calling extern fn
//!      with `*mut` parameters, which violates "no Rust leakage" philosophy.
//! Root Cause: Codegen doesn't auto-convert mutable locals to raw pointers
//!             when the extern fn parameter type is `*mut T`.
//! Fix: When calling an extern fn where param type is `*mut T` and the
//!      argument is a `mut` local variable, auto-generate `&mut x as *mut T`.
//!
//! This allows idiomatic Windjammer:
//!   let mut x = 0.0
//!   ffi_fn(x)        // compiler generates: ffi_fn(&mut x as *mut f32)
//! Instead of Rust-leaking:
//!   let mut x = 0.0
//!   ffi_fn(&mut x)   // explicit &mut is Rust leakage!

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_and_generate(code: &str) -> String {
    let mut lexer = Lexer::new(code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_ffi_mut_local_passed_to_raw_pointer_param() {
    let source = r#"
    extern fn get_value(out: *mut f32)

    fn use_it() -> f32 {
        let mut x: f32 = 0.0
        get_value(x)
        x
    }
    "#;
    let output = parse_and_generate(source);
    // The generated Rust should pass &mut x (as raw pointer) to the FFI function
    assert!(
        output.contains("&mut x") || output.contains("&mut x as *mut"),
        "Should auto-convert mut local to &mut for *mut FFI param. Got:\n{}",
        output
    );
    // Should NOT require explicit &mut in source
    assert!(
        !source.contains("&mut"),
        "Source should NOT have explicit &mut (that's Rust leakage)"
    );
}

#[test]
fn test_ffi_multiple_mut_out_params() {
    let source = r#"
    extern fn get_position(x: *mut f32, y: *mut f32, z: *mut f32)

    fn use_it() {
        let mut x: f32 = 0.0
        let mut y: f32 = 0.0
        let mut z: f32 = 0.0
        get_position(x, y, z)
    }
    "#;
    let output = parse_and_generate(source);
    assert!(
        output.contains("&mut x") && output.contains("&mut y") && output.contains("&mut z"),
        "Should auto-convert all mut locals to &mut for *mut FFI params. Got:\n{}",
        output
    );
}

#[test]
fn test_ffi_non_mut_param_not_converted() {
    let source = r#"
    extern fn set_value(val: f32)

    fn use_it() {
        let x: f32 = 5.0
        set_value(x)
    }
    "#;
    let output = parse_and_generate(source);
    // Regular (non-pointer) params should NOT get &mut
    assert!(
        !output.contains("&mut x"),
        "Non-pointer params should NOT be converted to &mut. Got:\n{}",
        output
    );
}

#[test]
fn test_ffi_mixed_params() {
    let source = r#"
    extern fn ffi_call(handle: u64, out_x: *mut f32, name_len: i32)

    fn use_it() {
        let h: u64 = 42
        let mut x: f32 = 0.0
        let n: i32 = 5
        ffi_call(h, x, n)
    }
    "#;
    let output = parse_and_generate(source);
    // Only the *mut param should get &mut conversion
    assert!(
        output.contains("&mut x"),
        "Only *mut param should get &mut. Got:\n{}",
        output
    );
    assert!(
        !output.contains("&mut h"),
        "Non-pointer u64 should NOT get &mut. Got:\n{}",
        output
    );
}
