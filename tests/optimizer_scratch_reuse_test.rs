//! scratch_reuse (auto-arena) pass: rewrites, refusals, behaviour on a compiled fixture with an independent
//! oracle (tests/fixtures/optimizer_scratch/oracle.py), and a mutant that the fixture must catch.

#[path = "fixtures/optimizer/support.rs"]
mod support;

use support::*;
use windjammer::optimizer::{run, Mutant, OptimizationConfig};

fn on() -> OptimizationConfig {
    OptimizationConfig { scratch_reuse: true, ..OptimizationConfig::disabled() }
}

fn opt(src: &str) -> String {
    run(&on(), src).code
}

#[test]
fn hoists_borrowed_string_buffer() {
    let src = "fn f(n: u32) { for i in 0..n { let mut s = String::new(); s.push_str(\"a\"); println!(\"{}\", s); } }";
    assert_eq!(
        opt(src),
        "fn f(n: u32) { { let mut s = String::new(); for i in 0..n { s.clear(); s.push_str(\"a\"); println!(\"{}\", s); } } }"
    );
}

#[test]
fn keeps_type_annotation_and_line_count() {
    let src = "fn f() {\n    while g() {\n        let mut v: Vec<i32> =\n            Vec::new();\n        v.push(1);\n        h(&v);\n    }\n}";
    let got = opt(src);
    assert!(got.contains("let mut v: Vec<i32> = Vec::new();"), "{got}");
    assert!(got.contains("v.clear();"), "{got}");
    assert_eq!(got.lines().count(), src.lines().count());
}

#[test]
fn refuses_moves_shadowing_closures_and_header_mentions() {
    for src in [
        "fn f(o: &mut Vec<String>) { for _ in 0..3 { let mut s = String::new(); o.push(s); } }",
        "fn f() { for _ in 0..3 { let mut s = String::new(); let t = s; } }",
        "fn f() { for _ in 0..3 { let mut s = String::new(); let c = || s.len(); c(); } }",
        "fn f(s: Vec<i32>) { for x in s.iter() { let mut s = Vec::new(); s.push(*x); } }",
        "fn f() { for _ in 0..3 { let mut s = String::new(); { let s = 1; } } }",
        "fn f() { for _ in 0..3 { let mut v = Vec::new(); v.push(1); for x in v { g(x); } } }",
        "fn f() { for _ in 0..3 { let mut v = Vec::new(); v.push(1); g(vec![v]); } }",
        "fn f() { for _ in 0..3 { let mut v = Vec::new(); v = vec![2]; } }",
        "fn f() { for _ in 0..3 { let s = String::new(); g(&s); } }",
    ] {
        assert_eq!(opt(src), src, "must not rewrite: {src}");
    }
}

#[test]
fn vec_refused_when_file_has_drop_impls() {
    let src = "struct D; impl Drop for D { fn drop(&mut self) {} } fn f() { for _ in 0..3 { let mut v = Vec::new(); v.push(D); } }";
    assert_eq!(opt(src), src);
}

#[test]
fn off_by_default() {
    assert!(!OptimizationConfig::production().scratch_reuse);
    assert!(OptimizationConfig::with_auto_arena().scratch_reuse);
}

#[test]
fn fixture_behaviour_is_preserved_and_the_mutant_is_caught() {
    let dir = scratch("scratch-reuse");
    let fx = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/optimizer_scratch");
    let src = std::fs::read_to_string(fx.join("s01_scratch_reuse.rs")).unwrap();
    let want: Vec<String> =
        std::fs::read_to_string(fx.join("s01_scratch_reuse.expected")).unwrap().lines().map(str::to_string).collect();
    assert_eq!(check_source(&src, &want, &dir, "plain"), Verdict::Pass, "fixture vs oracle");
    let out = run(&on(), &src);
    assert_eq!(out.applied.get("scratch_reuse"), Some(&2), "line and parts hoisted, kept not:\n{}", out.code);
    assert_eq!(check_source(&out.code, &want, &dir, "opt"), Verdict::Pass);
    let mutant = OptimizationConfig { mutant: Some(Mutant::ScratchIgnoresMoves), ..on() };
    let bad = run(&mutant, &src).code;
    assert_ne!(bad, out.code, "mutant must change the output");
    assert_ne!(check_source(&bad, &want, &dir, "mutant"), Verdict::Pass, "mutant not caught");
}
