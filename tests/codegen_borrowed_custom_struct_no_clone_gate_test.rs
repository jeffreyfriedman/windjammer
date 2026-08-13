#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! WDB-097: borrowed non-Copy Custom formals must not get `.clone()` at call sites.
//! Cross-file / cold-meta: callee analyzed as Borrowed with bare Custom (no
//! `emitted_rust_ref_params` yet) must still drive `&csr`, not `csr.clone()`.

use std::sync::Arc;
use windjammer::analyzer::{Analyzer, OwnershipMode, SignatureRegistry};
use windjammer::codegen::rust::CodeGenerator;
use windjammer::lexer::Lexer;
use windjammer::parser::{Parser, Type};
use windjammer::CompilationTarget;

fn analyze_registry(source: &str) -> SignatureRegistry {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("parse");
    let mut analyzer = Analyzer::new();
    let (_analyzed, registry, _) = analyzer.analyze_program(&program).expect("analyze");
    registry
}

#[test]
fn borrowed_dense_csr_cross_file_must_not_clone() {
    let file_a = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
    out_neighbors: Vec<u32>,
}

pub fn graph_bfs_run_dense(csr: DenseCsr, source: i64) -> i64 {
    let n = csr.out_offsets.len() as i64
    n + source
}
"#;
    let registry_a = analyze_registry(file_a);
    let sig = registry_a.get_signature("graph_bfs_run_dense").expect("sig");
    assert_eq!(sig.param_ownership[0], OwnershipMode::Borrowed);
    assert!(
        matches!(sig.param_types[0], Type::Custom(_))
            || matches!(sig.param_types[0], Type::Reference(_)),
        "unexpected param type: {:?}",
        sig.param_types[0]
    );

    let file_b = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
    out_neighbors: Vec<u32>,
}

pub fn make_csr() -> DenseCsr {
    DenseCsr { out_offsets: Vec::new(), out_neighbors: Vec::new() }
}

pub fn run_bfs(source: i64) -> i64 {
    let csr = make_csr()
    graph_bfs_run_dense(csr, source)
}

pub fn run_twice() -> i64 {
    let csr = make_csr()
    let a = graph_bfs_run_dense(csr, 0)
    let b = graph_bfs_run_dense(csr, 1)
    a + b
}
"#;
    let mut lexer = Lexer::new(file_b);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program_b = parser.parse().expect("parse b");
    let mut analyzer_b = Analyzer::new();
    let (analyzed_b, registry_b, _) = analyzer_b
        .analyze_program_with_global_signatures(&program_b, &registry_a)
        .expect("analyze b");
    let mut codegen = CodeGenerator::new_for_module(registry_b, CompilationTarget::Rust);
    codegen.set_global_signature_registry(Arc::new(registry_a));
    let rs = codegen.generate_program(&program_b, &analyzed_b);
    assert!(
        !rs.contains("graph_bfs_run_dense(csr.clone()"),
        "WDB-097: must not clone into &DenseCsr. Got:\n{rs}"
    );
    assert!(
        rs.contains("graph_bfs_run_dense(&csr") || rs.contains("graph_bfs_run_dense(csr,"),
        "expected borrow or already-borrowed pass. Got:\n{rs}"
    );
}

#[test]
fn borrowed_dense_csr_field_access_must_not_clone() {
    let file_a = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
}
pub fn graph_bfs_run_dense(csr: DenseCsr, source: i64) -> i64 {
    (csr.out_offsets.len() as i64) + source
}
"#;
    let registry_a = analyze_registry(file_a);

    let file_b = r#"
pub struct DenseCsr {
    out_offsets: Vec<u32>,
}
pub struct Session {
    csr: DenseCsr,
}
impl Session {
    pub fn run_bfs(self, source: i64) -> i64 {
        let a = graph_bfs_run_dense(self.csr, source)
        let b = graph_bfs_run_dense(self.csr, source)
        a + b
    }
}
"#;
    let mut lexer = Lexer::new(file_b);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program_b = parser.parse().expect("parse b");
    let mut analyzer_b = Analyzer::new();
    let (analyzed_b, registry_b, _) = analyzer_b
        .analyze_program_with_global_signatures(&program_b, &registry_a)
        .expect("analyze b");
    let mut codegen = CodeGenerator::new_for_module(registry_b, CompilationTarget::Rust);
    codegen.set_global_signature_registry(Arc::new(registry_a));
    let rs = codegen.generate_program(&program_b, &analyzed_b);
    assert!(
        !rs.contains("self.csr.clone()") && !rs.contains("graph_bfs_run_dense(self.csr.clone()"),
        "WDB-097: field into &DenseCsr must not clone. Got:\n{rs}"
    );
    assert!(
        rs.contains("graph_bfs_run_dense(&self.csr"),
        "expected &self.csr. Got:\n{rs}"
    );
}
