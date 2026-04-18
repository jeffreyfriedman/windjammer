use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;

/// Simulate single-file compilation where cross-file functions don't have signatures.
/// The caller file is compiled alone, so the callee's signature is NOT in the registry.
fn compile_single_file(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed_functions, analyzed_structs, _analyzed_trait_methods) =
        analyzer.analyze_program(&program).unwrap();
    let mut generator = CodeGenerator::new_for_module(analyzed_structs, CompilationTarget::Rust);
    generator.generate_program(&program, &analyzed_functions)
}

#[test]
fn test_cross_file_fn_call_field_access_no_clone() {
    // Bug: When calling a cross-file function (no signature in registry),
    // field access arguments get auto-clone from the analysis pass.
    // This generates `fn_call(obj.field.clone())` which fails when
    // the callee expects `&FieldType`.
    //
    // Real-world case: pass_id_to_label(pass.pass_id.clone()) should NOT
    // have .clone() when the function might expect &PassId.
    //
    // The key: pass is used AFTER the field access (via pass.shader_id),
    // so the auto-clone analysis marks pass.label as needing clone.
    // But without signature info, we shouldn't clone - the function
    // might expect a reference.
    let code = r#"
use crate::debug::debug_labels::format_label

pub struct CompiledPass {
    pub label: string,
    pub shader_id: u32,
}

pub struct ShaderGraph {
    pub passes: Vec<CompiledPass>,
}

impl ShaderGraph {
    fn execute_pass(self, pass: CompiledPass) {
        let result = format_label(pass.label)
        println(result)
        println(pass.shader_id)
    }
}
"#;

    let output = compile_single_file(code);
    eprintln!("=== CROSS-FILE TEST OUTPUT ===\n{}", output);

    // The generated code should NOT have .clone() on the argument
    // when there's no signature (cross-file call).
    assert!(
        !output.contains("format_label(pass.label.clone())"),
        "Cross-file function call should NOT add .clone() to arguments without signature info.\nGenerated:\n{}",
        output
    );
}

#[test]
fn test_same_file_fn_call_with_signature_borrowed() {
    // When the function IS in the same file and has Borrowed param,
    // arguments should get & prefix, not .clone()
    let code = r#"
pub enum PassId {
    Raymarch,
    Lighting,
}

pub fn pass_id_to_label(pass_id: PassId) -> string {
    match pass_id {
        PassId::Raymarch => "Raymarch",
        PassId::Lighting => "Lighting",
    }
}

pub struct CompiledPass {
    pub pass_id: PassId,
    pub shader_id: u32,
}

pub fn execute(pass: CompiledPass) {
    let label = pass_id_to_label(pass.pass_id)
    println(label)
}
"#;

    let output = compile_single_file(code);
    println!("Generated:\n{}", output);

    // When signature IS available and param is inferred as borrowed,
    // the call site should use & (not .clone())
    assert!(
        !output.contains("pass_id_to_label(pass.pass_id.clone())"),
        "Same-file function call with borrowed param should use &, not .clone().\nGenerated:\n{}",
        output
    );
}
