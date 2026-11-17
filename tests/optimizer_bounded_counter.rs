//! bounded_counter (PASSES.md P10): overflow checks are removed only where they provably cannot fire.
//! Rewrites and refusals on source text, then compiled behaviour (debug profile: overflow checks on, as Windjammer
//! release builds keep them): programs whose check must stay still panic after the pass, programs it rewrites print
//! the same, and each deliberately unsound variant of the pass is caught.

#[path = "fixtures/optimizer/support.rs"]
mod support;

use support::*;
use windjammer::optimizer::{run, Mutant, OptimizationConfig};

fn only() -> OptimizationConfig {
    OptimizationConfig { bounded_counter: true, ..OptimizationConfig::disabled() }
}

fn opt(src: &str) -> String {
    run(&only(), src).code
}

fn opt_with(m: Mutant, src: &str) -> String {
    run(&OptimizationConfig { mutant: Some(m), ..only() }, src).code
}

/// loop5m-shaped: counter bounded by a literal, affine use before the increment.
const LOOP: &str = "fn main() {\n    let mut total: i64 = 0_i64;\n    let mut i: i64 = 0_i64;\n    while i < 5000_i64 {\n        let x = i * 3_i64 + 1_i64;\n        if x % 7_i64 == 0_i64 { total += x; } else { total -= 1_i64; }\n        i += 1_i64;\n    }\n    println!(\"{}\", total);\n}\n";

#[test]
fn rewrites_the_increment_and_the_bounded_affine_use() {
    let got = opt(LOOP);
    assert!(got.contains("i = i.wrapping_add(1_i64)"), "{got}");
    assert!(got.contains("let x = ((i).wrapping_mul(3_i64)).wrapping_add(1_i64);"), "{got}");
    assert!(got.contains("total += x;"), "an unbounded accumulator keeps its check:\n{got}");
    assert_eq!(got.lines().count(), LOOP.lines().count());
}

#[test]
fn refuses_le_bounds_other_writes_parameters_escapes_and_overflowing_products() {
    // `<=`: i + 1 can overflow at the type's maximum
    let le = "fn f() { let mut i = 0_u8; while i <= 255_u8 { i += 1_u8; } }";
    assert_eq!(opt(le), le);
    // a second write to the counter
    let two = "fn f() { let mut i = 0_u8; while i < 200_u8 { i = i + 1_u8; i += 1_u8; } }";
    assert_eq!(opt(two), two);
    // `&mut i` escapes
    let esc = "fn g(x: &mut u8) {} fn f() { let mut i = 0_u8; while i < 9_u8 { g(&mut i); i += 1_u8; } }";
    assert_eq!(opt(esc), esc);
    // the product can exceed u8::MAX (99 * 3 = 297): the multiplication keeps its check, the increment does not
    let prod = "fn f() { let mut i = 0_u8; while i < 100_u8 { let y = i * 3_u8; i += 1_u8; } }";
    let got = opt(prod);
    assert!(got.contains("let y = i * 3_u8;"), "{got}");
    assert!(got.contains("i = i.wrapping_add(1_u8)"), "{got}");
    // a parameter counter may start negative: no affine rewrite (the increment is still safe)
    let param = "fn f(mut i: i8) { while i < 100_i8 { let y = i * 1_i8 + 27_i8; i += 1_i8; } }";
    let got = opt(param);
    assert!(got.contains("let y = i * 1_i8 + 27_i8;"), "{got}");
    // the counter re-bound inside the loop
    let shadow = "fn f() { let mut i = 0_u8; while i < 9_u8 { let i = 3_u8; } }";
    assert_eq!(opt(shadow), shadow);
}

/// Programs whose overflow check must stay: each panics (debug build) before and after the sound pass.
const MUST_PANIC: &[(&str, &str)] = &[
    // the multiplication overflows at i = 86 (86 * 3 = 258 > 255)
    ("mul", "fn main() {\n    let mut i = 0_u8;\n    let mut s = 0_u32;\n    while i < 100_u8 {\n        let y = i * 3_u8;\n        s += y as u32;\n        i += 1_u8;\n    }\n    println!(\"{}\", s);\n}\n"),
    // `<=` lets the increment reach 256
    ("le", "fn main() {\n    let mut i = 250_u8;\n    let mut n = 0_u32;\n    while i <= 255_u8 {\n        i += 1_u8;\n        n += 1;\n        if n > 10 { break; }\n    }\n    println!(\"{}\", n);\n}\n"),
    // a second write pushes the counter past the bound before the increment
    ("two", "fn main() {\n    let mut i = 0_u8;\n    let mut n = 0_u32;\n    while i < 255_u8 {\n        i = i + 1_u8;\n        i += 1_u8;\n        n += 1;\n        if n > 300 { break; }\n    }\n    println!(\"{}\", n);\n}\n"),
];

#[test]
fn checks_that_can_fire_still_fire_after_the_pass() {
    let dir = scratch("bc-panic");
    for (name, src) in MUST_PANIC {
        for (tag, code) in [("raw", src.to_string()), ("opt", opt(src))] {
            let bin = compile(&code, &dir, &format!("{name}_{tag}")).unwrap_or_else(|e| panic!("{name} {tag}: {e}"));
            let out = run_seeds(&bin, &[0]);
            assert!(out[0].starts_with("CRASH"), "{name} ({tag}) must panic on overflow, printed {out:?}\n{code}");
        }
    }
}

#[test]
fn rewritten_programs_print_the_same() {
    let dir = scratch("bc-same");
    let raw = run_seeds(&compile(LOOP, &dir, "loop_raw").unwrap(), &[0]);
    let got = run_seeds(&compile(&opt(LOOP), &dir, "loop_opt").unwrap(), &[0]);
    assert_eq!(raw, got);
    assert!(!raw[0].starts_with("CRASH"));
}

/// Each unsound variant must change a must-panic program into one that does not panic.
#[test]
fn every_unsound_variant_is_caught() {
    let dir = scratch("bc-mut");
    for (m, prog) in [
        (Mutant::CounterIgnoresTypeMax, "mul"),
        (Mutant::CounterAcceptsLessEqual, "le"),
        (Mutant::CounterIgnoresOtherWrites, "two"),
    ] {
        let src = MUST_PANIC.iter().find(|(n, _)| *n == prog).unwrap().1;
        let code = opt_with(m, src);
        assert_ne!(code, opt(src), "{m:?} is vacuous on {prog}");
        let bin = compile(&code, &dir, &format!("{prog}_{m:?}")).unwrap_or_else(|e| panic!("{m:?}: {e}"));
        let out = run_seeds(&bin, &[0]);
        assert!(!out[0].starts_with("CRASH"), "mutant {m:?} was NOT caught: the overflow still panics on {prog}");
    }
}
