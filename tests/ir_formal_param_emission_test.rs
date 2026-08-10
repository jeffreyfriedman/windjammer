//! Formal emission must prefer IR param ownership over keep-owned body walks.

use windjammer::analyzer::Analyzer;
use windjammer::codegen::rust::{CodeGenerator, IrCutoverConfig};
use windjammer::ir::annotations::OptimizationHints;
use windjammer::ir::node::IrFunction;
use windjammer::ir::safety_type::{BaseType, Region, SafetyType};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer::CompilationTarget;
use std::collections::{HashMap, HashSet};

fn parse_program(source: &str) -> windjammer::parser::Program<'static> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let parser = Box::leak(Box::new(Parser::new(tokens)));
    parser.parse().expect("parse")
}

#[test]
fn ir_borrowed_string_formal_emits_str_ref_despite_keep_owned_body() {
    // Body stores `name` into a struct field (classic keep-owned trigger). IR says
    // Borrowed — formals must emit `&str`, not owned `String`.
    let program = parse_program(
        r#"
struct Holder {
    name: string,
}

fn wrap(name: string) -> Holder {
    Holder { name: name }
}
"#,
    );
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    let mut gen = CodeGenerator::new_for_module(registry, CompilationTarget::Rust);
    gen.set_ir_cutover_config(IrCutoverConfig {
        ownership: true,
        clones: true,
        param_types: true,
        str_ref: true,
        call_sites: true,
        locals: true,
    });

    let mut param_types = HashMap::new();
    param_types.insert(
        "name".to_string(),
        SafetyType::borrowed(BaseType::String, Region::fresh(1)),
    );
    gen.set_ir_functions(vec![IrFunction {
        name: "wrap".into(),
        param_types,
        return_type: SafetyType::owned(BaseType::Custom("Holder".into())),
        mutated_locals: HashSet::new(),
        mutated_params: HashSet::new(),
        str_ref_params: HashSet::from(["name".into()]),
        optimizations: OptimizationHints::empty(),
        local_types: HashMap::new(),
        body: Vec::new(),
    }]);

    let output = gen.generate_program(&program, &analyzed);
    let wrap_sig = output
        .lines()
        .find(|l| l.contains("fn wrap("))
        .unwrap_or("(missing wrap)");
    assert!(
        wrap_sig.contains("name: &str") || wrap_sig.contains("name: &String"),
        "IR Borrowed formal must not be forced Owned by keep-owned body walk, got: {wrap_sig}\n{output}"
    );
}
