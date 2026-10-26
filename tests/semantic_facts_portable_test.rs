//! ADR-0017: the Go/JS/TS lowering consumes the semantic layer's `last_use` fact
//! (`ir::liveness`) and moves instead of copying at a binding's last use.
//!
//! Measurement (ratchet): defensive copies over the conformance corpus with and without the fact.
//! `tests/golden/portable_copies.txt` holds the with-facts count; it may only fall. Behavior is
//! unchanged by construction (a copy of a value nobody reads again is unobservable) and is checked
//! independently by `tests/backend_conformance_test.rs` (independent oracles, Go and Node).
//!
//! consumes: tests/conformance_programs/*.wj

use std::path::{Path, PathBuf};
use windjammer::codegen::portable::lower::{lower_program, lower_program_without_facts};

const RATCHET: &str = "tests/golden/portable_copies.txt";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn copies(m: &windjammer::codegen::portable::ir::PModule) -> usize {
    format!("{m:?}").matches("Copy(PExpr").count()
}

/// (copies with facts, copies without facts, programs lowered) over the corpus.
fn measure() -> (usize, usize, usize) {
    let dir = root().join("tests/conformance_programs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    files.sort();
    let (mut with, mut without, mut n) = (0, 0, 0);
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap();
        let mut lexer = windjammer::lexer::Lexer::new(&src);
        let tokens = lexer.tokenize_with_locations();
        let mut parser = windjammer::parser::Parser::new(tokens);
        let Ok(program) = parser.parse() else { continue };
        let Ok(ctx) = windjammer::ir::analyze_and_lower(&program) else { continue };
        let (Ok(a), Ok(b)) = (
            lower_program(&program, Some(&ctx.registry)),
            lower_program_without_facts(&program, Some(&ctx.registry)),
        ) else {
            continue;
        };
        with += copies(&a);
        without += copies(&b);
        n += 1;
    }
    (with, without, n)
}

#[test]
fn last_use_fact_removes_defensive_copies_and_count_does_not_rise() {
    let (with, without, n) = measure();
    eprintln!("portable copies over {n} conformance programs: {without} without facts, {with} with facts");
    assert!(n >= 30, "too few programs lowered: {n}");
    assert!(with <= without, "facts must never add copies");
    assert!(with < without, "the last_use fact should remove at least one copy on the corpus");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(RATCHET);
    let baseline: Option<usize> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.lines().find(|l| !l.starts_with('#')).and_then(|l| l.trim().parse().ok()));
    if std::env::var("UPDATE_GOLDEN").is_ok_and(|v| v == "1") || baseline.is_none() {
        std::fs::write(
            &path,
            format!("# Defensive copies in Go/JS lowering over tests/conformance_programs (with semantic facts). May only fall.\n# without facts: {without}\n{with}\n"),
        )
        .unwrap();
        return;
    }
    let b = baseline.unwrap();
    assert!(with <= b, "portable copies rose: {b} -> {with}");
}

/// Soundness edge cases of the fact as the portable layer uses it: loop-carried and
/// closure-captured bindings must still be copied.
#[test]
fn copies_are_kept_where_the_value_is_observed_again() {
    let src = r#"
struct Bag { items: Vec<int> }
fn keep(b: Bag) -> Bag { b }
fn main() {
    let b = Bag { items: vec![1] }
    for i in 0..2 {
        let k = keep(b)
        println("${k.items.len()}")
    }
    let c = Bag { items: vec![2] }
    let f = |x: int| c.items.len() + x
    let d = keep(c)
    println("${f(1)} ${d.items.len()}")
}
"#;
    let mut lexer = windjammer::lexer::Lexer::new(src);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new(tokens);
    let program = parser.parse().unwrap();
    let ctx = windjammer::ir::analyze_and_lower(&program).unwrap();
    let a = lower_program(&program, Some(&ctx.registry)).unwrap();
    let b = lower_program_without_facts(&program, Some(&ctx.registry)).unwrap();
    // `keep(b)` inside the loop and `keep(c)` (captured by `f`) keep their copies; only `k`/`d`
    // style last uses could be moved, and there are none at owned positions here.
    assert_eq!(copies(&a), copies(&b), "{a:?}");
}
