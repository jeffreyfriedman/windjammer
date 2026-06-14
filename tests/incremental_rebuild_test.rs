//! Incremental dependency graph and reanalysis set tests.

use std::collections::HashSet;
use std::fs;
use tempfile::TempDir;
use windjammer::compiler::cache_management;
use windjammer::compiler::incremental::{compute_reanalysis_set, DependencyGraph};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn parse_file(path: &std::path::Path, source: &str) -> (Parser, windjammer::parser::Program<'static>) {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new_with_source(
        tokens,
        path.to_string_lossy().to_string(),
        source.to_string(),
    );
    let program = parser.parse().expect("parse test fixture");
    (parser, program)
}

#[test]
fn test_dependency_graph_transitive_dependents() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();

    let a = src.join("a.wj");
    let b = src.join("b.wj");
    fs::write(&a, "fn a_fn() {}\n").unwrap();
    fs::write(&b, "use crate::a;\nfn b_fn() {}\n").unwrap();

    let sources = vec![
        (a.clone(), fs::read_to_string(&a).unwrap()),
        (b.clone(), fs::read_to_string(&b).unwrap()),
    ];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _ = parsers;

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let mut dirty = HashSet::new();
    dirty.insert(0);
    let dependents = graph.transitive_dependents(&dirty);
    assert!(dependents.contains(&0));
}

#[test]
fn test_compute_reanalysis_set_all_dirty_without_meta() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    let wj = src.join("x.wj");
    fs::write(&wj, "fn x() {}\n").unwrap();
    cache_management::write_compiler_stamp(dir.path()).unwrap();

    let sources = vec![(wj, "fn x() {}\n".to_string())];
    let mut parsers = Vec::new();
    let mut programs = Vec::new();
    for (file, source) in &sources {
        let (parser, program) = parse_file(file, source);
        parsers.push(parser);
        programs.push(program);
    }
    let _ = parsers;

    let graph = DependencyGraph::build(&sources, &programs, &src);
    let set = compute_reanalysis_set(&sources, &src, dir.path(), &[], &graph);
    assert_eq!(set.len(), 1);
}
