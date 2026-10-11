//! Known-answer and differential tests for the post-codegen optimizer.
//!
//! Oracle: `tests/fixtures/optimizer/reference.py` (independent Python model).
//! Every fixture is run unoptimized and optimized and compared to the model.

#[path = "fixtures/optimizer/support.rs"]
mod support;

use support::*;
use windjammer::optimizer::{run, OptimizationConfig};

#[test]
fn optimizer_fixtures_there_are_at_least_fifteen() {
    assert!(fixture_names().len() >= 15);
}

#[test]
fn optimizer_fixtures_unoptimized_match_independent_reference() {
    let names = fixture_names();
    let dir = scratch("raw");
    let verdicts = par_map(&names, |n| check_source(&fixture_source(n), &expected(n), &dir, n));
    for (n, v) in names.iter().zip(verdicts) {
        assert_eq!(v, Verdict::Pass, "fixture {n} disagrees with the reference model");
    }
}

#[test]
fn optimizer_fixtures_optimized_match_independent_reference() {
    let names = fixture_names();
    let dir = scratch("opt");
    let verdicts = par_map(&names, |n| {
        let out = run(&OptimizationConfig::production(), &fixture_source(n));
        check_source(&out.code, &expected(n), &dir, &format!("{n}_opt"))
    });
    for (n, v) in names.iter().zip(verdicts) {
        assert_eq!(v, Verdict::Pass, "optimized fixture {n} diverges from the reference model");
    }
}

#[test]
fn optimizer_every_pass_fires_on_the_fixtures() {
    let mut seen = std::collections::BTreeMap::<&str, usize>::new();
    for n in fixture_names() {
        for (k, v) in run(&OptimizationConfig::production(), &fixture_source(&n)).applied {
            *seen.entry(k).or_default() += v;
        }
    }
    for pass in ["temp_clone", "macro_arg_clone", "copy_clone", "same_type_cast", "with_capacity", "string_append", "inline_policy", "dead_helpers"] {
        assert!(seen.get(pass).copied().unwrap_or(0) > 0, "pass {pass} never fired: {seen:?}");
    }
}

#[test]
fn optimizer_disabled_config_is_byte_identity_on_fixtures() {
    for n in fixture_names() {
        let src = fixture_source(&n);
        assert_eq!(run(&OptimizationConfig::disabled(), &src).code, src);
    }
}

#[test]
fn optimizer_is_idempotent_on_fixtures() {
    for n in fixture_names() {
        let once = run(&OptimizationConfig::production(), &fixture_source(&n)).code;
        let twice = run(&OptimizationConfig::production(), &once).code;
        assert_eq!(once, twice, "fixture {n}");
    }
}

#[test]
fn optimizer_metamorphic_alpha_renaming_preserves_edit_counts() {
    // Renaming bound variables must not change which rewrites apply.
    for n in fixture_names() {
        let src = fixture_source(&n);
        let renamed = rename_words(&src, &[("out", "acc_q"), ("flags", "bits_q"), ("words", "toks_q"), ("snapshot", "frozen_q")]);
        let a = run(&OptimizationConfig::production(), &src).applied;
        let b = run(&OptimizationConfig::production(), &renamed).applied;
        assert_eq!(a, b, "fixture {n}");
    }
}

#[test]
fn optimizer_metamorphic_leading_dead_statement_preserves_edit_counts() {
    for n in fixture_names() {
        let src = fixture_source(&n).replacen("fn main() {", "fn main() {\n    let _unused_marker = 0_u8;", 1);
        let base = run(&OptimizationConfig::production(), &fixture_source(&n)).applied;
        assert_eq!(base, run(&OptimizationConfig::production(), &src).applied, "fixture {n}");
    }
}

fn rename_words(src: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if let Some((_, to)) = pairs.iter().find(|(from, _)| *from == word.as_str()) {
            out.push_str(to);
        } else {
            out.push_str(word);
        }
        word.clear();
    };
    for ch in src.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}
