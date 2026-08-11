//! Phase 5: legacy typed_lowering bridge deleted; IR call-sites own coercion.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

fn parse_program(source: &str) -> windjammer::parser::Program<'static> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().expect("parse")
}

#[test]
fn ir_call_sites_default_on_without_typed_lowering_bridge() {
    // Representative cases that typed_lowering::correct_legacy_output used to patch.
    let program = parse_program(
        r#"
fn takes_borrowed(s: string) {
    println("{}", s)
}

fn takes_owned(s: string) -> string {
    s
}

fn main() {
    let name = "hi"
    takes_borrowed(name)
    let owned = takes_owned("x")
    let _ = owned
}
"#,
    );
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    assert!(
        gen.ir_cutover_call_sites_enabled(),
        "call_sites must be on by default"
    );
    let output = gen.generate_program(&program, &analyzed);
    assert!(
        !output.contains("compile_error!"),
        "IR path must not fail closed on same-crate callees:\n{output}"
    );
    assert!(
        output.contains("takes_borrowed") && output.contains("takes_owned"),
        "expected both callees in output:\n{output}"
    );
    // Borrowed formal: identifier should be shared-borrowed or already &str-capable.
    // Skip the `fn takes_borrowed(` definition line — match the call site only.
    let borrowed_call = output
        .lines()
        .find(|l| {
            let t = l.trim();
            t.contains("takes_borrowed(") && !t.starts_with("fn ")
        })
        .unwrap_or("(missing)");
    assert!(
        borrowed_call.contains("&name") || borrowed_call.contains("takes_borrowed(name)"),
        "IR should coerce borrowed string arg, got: {borrowed_call}"
    );
    // Owned string formal + string literal: IR should emit `.to_string()` (or owned pass).
    let owned_call = output
        .lines()
        .find(|l| {
            let t = l.trim();
            t.contains("takes_owned(") && !t.starts_with("fn ")
        })
        .unwrap_or("(missing)");
    assert!(
        owned_call.contains(".to_string()") || owned_call.contains("to_owned()"),
        "IR should coerce owned string literal, got: {owned_call}"
    );
}

#[test]
fn opt_out_call_sites_still_compiles_without_typed_lowering() {
    let program = parse_program(
        r#"
fn greet(s: string) {
    println("{}", s)
}
fn main() {
    greet("hi")
}
"#,
    );
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    let mut gen = CodeGenerator::new(registry, CompilationTarget::Rust);
    gen.set_ir_cutover_config(windjammer::codegen::rust::IrCutoverConfig {
        ownership: true,
        clones: true,
        param_types: true,
        str_ref: true,
        call_sites: false,
        locals: true,
    });
    let output = gen.generate_program(&program, &analyzed);
    assert!(
        output.contains("fn greet") && output.contains("greet("),
        "legacy opt-out path must still emit without typed_lowering bridge:\n{output}"
    );
}
