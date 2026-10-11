#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "parser_tests",
))]

//! Interpreter and WASM-target coverage for the pipe corpus programs
//! (`tests/conformance_programs/15*_pipe_*.wj`). Expected output is the `// expect:` header of
//! each program (hand-derived, independent of every backend).

use std::path::PathBuf;
use windjammer::interpreter::Interpreter;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn corpus() -> Vec<(String, String, Vec<String>)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/conformance_programs");
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        if p.extension().is_some_and(|x| x == "wj") && name.contains("_pipe_") {
            let src = std::fs::read_to_string(&p).unwrap();
            let expect = src.lines().filter_map(|l| l.strip_prefix("// expect: ")).map(String::from).collect();
            out.push((name, src, expect));
        }
    }
    out.sort();
    out
}

#[test]
fn interpreter_runs_pipe_corpus_to_expected_output() {
    let programs = corpus();
    assert!(programs.len() >= 2, "pipe corpus programs missing");
    for (name, src, expect) in programs {
        let tokens = Lexer::new(&src).tokenize_with_locations();
        let mut parser = Parser::new_with_source(tokens, format!("{name}.wj"), src.clone());
        let program = parser.parse().unwrap_or_else(|e| panic!("{name}: {e}"));
        let mut it = Interpreter::new_capturing();
        it.run(&program).unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Vec<String> = it.get_output().lines().map(String::from).collect();
        assert_eq!(got, expect, "{name}");
    }
}

#[test]
fn wasm_target_emits_parenthesised_closure_callee() {
    use windjammer::{build_project, CompilationTarget};
    let tmp = tempfile::tempdir().unwrap();
    let wj = tmp.path().join("p.wj");
    std::fs::write(&wj, "fn main() {\n    let a = 3 |> |v| v + 1 |> |w| w * 2\n    println(\"{}\", a)\n}\n").unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Wasm, false).expect("wasm build");
    let rs = std::fs::read_to_string(out.join("p.rs")).unwrap();
    // `3 |> |v| v + 1 |> |w| w * 2` : the closure body swallows the second stage, so the
    // callee must be parenthesised for rustc to accept `(closure)(arg)`.
    assert!(rs.contains("(|v|") && rs.contains(")(3"), "unparenthesised closure callee:\n{rs}");
}
