//! Generative differential testing: seeded UB-free programs, unoptimized vs
//! optimized vs a native reference value, with automatic shrinking.

#[path = "fixtures/optimizer/support.rs"]
mod support;
#[path = "fixtures/optimizer/gen.rs"]
mod gen;

use windjammer::optimizer::OptimizationConfig;

const BATCH: usize = 25;

#[cfg(not(feature = "optimizer-soak"))]
const SEEDS: u64 = 200;
#[cfg(feature = "optimizer-soak")]
const SEEDS: u64 = 5000;

fn batches() -> Vec<Vec<gen::Case>> {
    (0..SEEDS)
        .map(|s| gen::gen_case(s + 1, s as usize))
        .collect::<Vec<_>>()
        .chunks(BATCH)
        .map(|c| c.to_vec())
        .collect()
}

#[test]
fn optimizer_generative_unoptimized_programs_match_native_reference() {
    let dir = support::scratch("gen-raw");
    let all = batches();
    let results = support::par_map(&all, |b| {
        gen::run_program(&gen::program(b), &dir, &format!("raw{}", b[0].body.len())).map(|got| got == gen::expected_lines(b))
    });
    for (i, r) in results.iter().enumerate() {
        assert_eq!(r, &Ok(true), "generator/reference disagree in batch {i}");
    }
}

#[test]
fn optimizer_generative_optimized_programs_match_native_reference() {
    let dir = support::scratch("gen-opt");
    let cfg = OptimizationConfig::production();
    let all = batches();
    let failures = support::par_map(&all, |b| gen::fails(b, &cfg, &dir, &format!("opt{}", b[0].body.len())));
    for (b, f) in all.iter().zip(failures) {
        if let Some(why) = f {
            let small = gen::shrink(b, &cfg, &dir);
            panic!("optimizer broke a program ({why}); minimal repro:\n{}", gen::program(&small));
        }
    }
}

#[test]
fn optimizer_generative_programs_are_actually_rewritten() {
    let cfg = OptimizationConfig::production();
    let mut applied = std::collections::BTreeMap::<&str, usize>::new();
    for b in batches() {
        for (k, v) in windjammer::optimizer::run(&cfg, &gen::program(&b)).applied {
            *applied.entry(k).or_default() += v;
        }
    }
    for pass in ["temp_clone", "macro_arg_clone", "with_capacity", "string_append", "same_type_cast", "copy_clone"] {
        assert!(applied.get(pass).copied().unwrap_or(0) > 0, "{pass} never fired: {applied:?}");
    }
}
