//! Mutation self-test: every deliberately unsound pass variant must be caught
//! by the fixture suite AND by the generative suite. A mutant that changes no
//! output is itself a test failure (vacuous mutant).

#[path = "fixtures/optimizer/support.rs"]
mod support;
#[path = "fixtures/optimizer/gen.rs"]
mod gen;

use support::*;
use windjammer::optimizer::{run, Mutant, OptimizationConfig};

fn cfg(m: Mutant) -> OptimizationConfig {
    OptimizationConfig { mutant: Some(m), ..OptimizationConfig::production() }
}

/// (mutant, fixture that must expose it)
const EXPECTED: &[(Mutant, &str)] = &[
    (Mutant::CopyCloneIgnoresShadowing, "p07_copy_clone"),
    (Mutant::TempCloneOnAnyIdent, "p06_clone_temp"),
    (Mutant::MacroCloneInVec, "p20_macro_clone"),
    (Mutant::CastDropsAnyIdent, "p08_casts"),
    (Mutant::CapacityIgnoresBreak, "p09_early_break"),
    (Mutant::CapacityAnyStart, "p10_nonzero_start"),
    (Mutant::AppendIgnoresSelfReference, "p05_string_selfref"),
    (Mutant::HelpersIgnoreReferences, "p11_repr_to_bytes"),
    (Mutant::ReprIgnoresUnsafe, "p19_repr_layout"),
];

#[test]
fn optimizer_mutation_every_mutant_is_caught_by_fixtures() {
    let dir = scratch("mut-fx");
    let res = par_map(EXPECTED, |(m, fx)| {
        let src = fixture_source(fx);
        let mutated = run(&cfg(*m), &src).code;
        let sound = run(&OptimizationConfig::production(), &src).code;
        assert_ne!(mutated, sound, "{m:?} is vacuous on {fx}");
        check_source(&mutated, &expected(fx), &dir, &format!("{fx}_{m:?}"))
    });
    for ((m, fx), v) in EXPECTED.iter().zip(res) {
        assert_ne!(v, Verdict::Pass, "mutant {m:?} was NOT caught by fixture {fx}");
    }
}

#[test]
fn optimizer_mutation_every_mutant_is_caught_by_generative_suite() {
    let dir = scratch("mut-gen");
    let cases: Vec<gen::Case> = (0..150u64).map(|s| gen::gen_case(s + 1, s as usize)).collect();
    let mutants: Vec<Mutant> = EXPECTED.iter().map(|(m, _)| *m).collect();
    let res = par_map(&mutants, |m| gen::fails(&cases, &cfg(*m), &dir, &format!("gen_{m:?}")));
    for (m, r) in mutants.iter().zip(res) {
        assert!(r.is_some(), "mutant {m:?} was NOT caught by the generative suite");
    }
}

#[test]
fn optimizer_mutation_production_config_has_no_mutant() {
    assert_eq!(OptimizationConfig::production().mutant, None);
    assert_eq!(OptimizationConfig::default().mutant, None);
}
