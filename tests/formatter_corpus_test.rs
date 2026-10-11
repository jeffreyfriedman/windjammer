#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
))]

//! Corpus properties of the Windjammer formatter: idempotence, semantic preservation,
//! comment preservation over every parseable `.wj` file in the repository.

use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use windjammer::formatter::{comments, format_source, FormatOptions};
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                continue;
            }
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "wj") {
            out.push(p);
        }
    }
}

fn corpus() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for d in ["tests", "examples", "std"] {
        collect(&root.join(d), &mut files);
    }
    files.sort();
    files
}

/// Debug rendering of the AST with source positions removed; None if it does not parse.
fn ast_signature(src: &str) -> Option<String> {
    let src = src.to_string();
    catch_unwind(AssertUnwindSafe(|| {
        let tokens = Lexer::new(&src).tokenize_with_locations();
        let parser = Box::leak(Box::new(Parser::new(tokens)));
        parser.parse().ok().map(|p| strip_positions(&format!("{:?}", p)))
    }))
    .ok()
    .flatten()
}

fn strip_positions(dbg: &str) -> String {
    let mut out = String::with_capacity(dbg.len());
    let mut rest = dbg;
    while let Some(pos) = ["line: ", "column: "].iter().filter_map(|k| rest.find(k).map(|p| (p, k.len()))).min() {
        out.push_str(&rest[..pos.0 + pos.1]);
        rest = rest[pos.0 + pos.1..].trim_start_matches(|c: char| c.is_ascii_digit());
    }
    out.push_str(rest);
    out
}

struct Stats {
    total: usize,
    parsed: usize,
}

fn for_each_parsed(mut f: impl FnMut(&Path, &str, &str, &str)) -> Stats {
    let opts = FormatOptions::default();
    let mut st = Stats { total: 0, parsed: 0 };
    for path in corpus() {
        st.total += 1;
        let Ok(src) = fs::read_to_string(&path) else { continue };
        let Some(sig) = ast_signature(&src) else { continue };
        st.parsed += 1;
        let once = format_source(&src, &opts)
            .unwrap_or_else(|e| panic!("{}: format failed: {e}", path.display()));
        f(&path, &src, &once, &sig);
    }
    eprintln!("formatter corpus: {} files, {} parse", st.total, st.parsed);
    st
}

#[test]
fn formatter_corpus_is_nonempty_and_mostly_parses() {
    let st = for_each_parsed(|_, _, _, _| {});
    assert!(st.parsed > 50, "corpus too small: {}", st.parsed);
}

#[test]
fn formatter_is_idempotent_over_corpus() {
    let opts = FormatOptions::default();
    let mut bad = Vec::new();
    for_each_parsed(|p, _, once, _| {
        let twice = format_source(once, &opts).unwrap();
        if &twice != once {
            bad.push(p.display().to_string());
        }
    });
    assert!(bad.is_empty(), "not idempotent: {:#?}", &bad[..bad.len().min(10)]);
}

#[test]
fn formatter_preserves_ast_over_corpus() {
    let mut bad = Vec::new();
    for_each_parsed(|p, _, once, sig| {
        if ast_signature(once).as_deref() != Some(sig) {
            bad.push(p.display().to_string());
        }
    });
    assert!(bad.is_empty(), "AST changed: {:#?}", &bad[..bad.len().min(10)]);
}

#[test]
fn formatter_preserves_comments_over_corpus() {
    let mut bad = Vec::new();
    for_each_parsed(|p, src, once, _| {
        if comments(src).unwrap() != comments(once).unwrap() {
            bad.push(p.display().to_string());
        }
    });
    assert!(bad.is_empty(), "comments changed: {:#?}", &bad[..bad.len().min(10)]);
}
