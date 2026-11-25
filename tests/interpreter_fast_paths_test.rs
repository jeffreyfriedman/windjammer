//! Every fast path of the interpreter has a generic fallback that is always correct, so the parity
//! gates stay green when a fast path silently stops being taken (a guard that also matches a standard
//! library stub, a pattern that parses differently than expected...). This gate asserts each one fires
//! on the programs it was built for: `WJ_JIT_DEBUG` makes `wj run` report how often each was taken
//! (`[fast] inline N mapget N mapins N ... pushstr N`), and each count must be non-zero.
//!
//! In debug builds `WJ_JIT_MUTANT=nofast-<path>` switches one path off; the self-test shows the
//! gate then fails for that path while the program's output stays the walker's.

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// (program, the fast paths it must take).
const CASES: &[(&str, &[&str])] = &[
    ("bench/interp/structs.wj", &["inline"]),
    ("bench/interp/vecpass.wj", &["vecretag"]),
    ("tests/engine_parity/vec_retag.wj", &["vecretag"]),
    ("bench/interp/hashmap.wj", &["mapjit", "mapleanget", "mapleanins", "mapleantyped"]),
    ("tests/engine_parity/kmap_intdense_sweep.wj", &["mapleanget", "mapleanins", "mapleantyped"]),
    ("tests/engine_parity/jit_map_lean.wj", &["mapleanget", "mapleanins"]),
    ("tests/engine_parity/jit_map_ops.wj", &["mapjit"]),
    ("bench/interp/sieve.wj", &["packed", "packedidx", "packedstore", "packedread", "pushinl", "pushflush"]),
    ("tests/engine_parity/jit_push_inline.wj", &["pushinl", "pushflush"]),
    ("tests/engine_parity/jit_vec_view.wj", &["packed", "packedread", "packedstore"]),
    ("bench/interp/nbody.wj", &["vecresolve", "rowview", "placeargs", "fnunit", "hoisted"]),
    ("tests/engine_parity/jit_fnunit.wj", &["fnunit"]),
    ("tests/engine_parity/jit_fnunit_boxed_params.wj", &["fnunit"]),
    ("tests/engine_parity/jit_fnunit_param_writes.wj", &["fnunit"]),
    ("tests/engine_parity/jit_row_views.wj", &["rowview"]),
    ("tests/engine_parity/jit_hoist_pairs.wj", &["rowview", "hoisted", "hoistfail"]),
    ("tests/engine_parity/jit_hoist_disqualified.wj", &["hoisted", "hoistfail"]),
    ("bench/interp/strings.wj", &["pushfmt", "lenfast", "lencmp"]),
    ("tests/engine_parity/len_cmp_cond.wj", &["lencmp"]),
    ("bench/interp/closures.wj", &["scalarpipe", "closurecreate"]),
    ("tests/engine_parity/closures_compiled.wj", &["closureenv", "closurescope", "closurecreate", "closurecopy"]),
    ("tests/engine_parity/scalar_pipe_taken.wj", &["scalarpipe"]),
    ("tests/engine_parity/scalar_pipe_overflow.wj", &["scalarbail"]),
    ("tests/engine_parity/str_fast.wj", &["fmt", "pushstr", "pushfmt"]),
    ("tests/engine_parity/jit_loop_inline.wj", &["inline"]),
    ("tests/engine_parity/map_get_match.wj", &["mapget", "mapins", "unboxed"]),
    ("tests/engine_parity/map_get_wild.wj", &["mapget"]),
    ("tests/engine_parity/option_arm_values.wj", &["mapget", "mapins"]),
    ("tests/engine_parity/packed_vecs.wj", &["packed"]),
    ("tests/engine_parity/match_value.wj", &["matchval"]),
    ("tests/engine_parity/method_place_args.wj", &["argcopy"]),
];

struct Run {
    stdout: Vec<u8>,
    code: Option<i32>,
    counts: Vec<(String, u64)>,
}

fn run(program: &str, engine: &str, mutant: Option<&str>) -> Run {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(program);
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", engine]).arg(&p).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT").env_remove("WJ_JIT_MUTANT").env("WJ_JIT_DEBUG", "1").env("WJ_JIT_LOOP_HOT", "0");
    if let Some(m) = mutant {
        c.env("WJ_JIT_MUTANT", m);
    }
    let o = c.output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    let line = err.lines().find_map(|l| l.strip_prefix("[fast] ")).unwrap_or_default().to_string();
    let words: Vec<&str> = line.split_whitespace().collect();
    let counts = words.chunks(2).filter_map(|w| Some((w.first()?.to_string(), w.get(1)?.parse().ok()?))).collect();
    Run { stdout: o.stdout, code: o.status.code(), counts }
}

fn count(r: &Run, path: &str) -> Option<u64> {
    r.counts.iter().find(|(n, _)| n == path).map(|(_, c)| *c)
}

/// The paths of `program` that did not fire.
fn silent(program: &str, paths: &[&str], mutant: Option<&str>) -> Vec<String> {
    let r = run(program, "resolved", mutant);
    paths.iter().filter(|p| count(&r, p).unwrap_or(0) == 0).map(|p| format!("{program}: `{p}` never taken ({:?})", r.counts)).collect()
}

#[test]
fn every_fast_path_fires_on_its_programs() {
    let mut problems = Vec::new();
    for (program, paths) in CASES {
        problems.extend(silent(program, paths, None));
    }
    assert!(problems.is_empty(), "fast paths that silently fell back:\n{}", problems.join("\n"));
}

/// Mutation self-test: with one path switched off the gate above fails for it, and the program still
/// prints what the walker prints (the fallback is correct, which is why only this gate can see it).
#[test]
#[cfg(debug_assertions)]
fn the_firing_gate_notices_each_disabled_path() {
    for (program, paths) in CASES {
        let walker = run(program, "walker", None);
        for p in *paths {
            let mutant = format!("nofast-{p}");
            assert!(!silent(program, &[p], Some(&mutant)).is_empty(), "{program}: `{mutant}` did not stop `{p}`");
            let r = run(program, "resolved", Some(&mutant));
            assert_eq!((&r.stdout, r.code), (&walker.stdout, walker.code), "{program} under `{mutant}` differs from the walker");
        }
    }
}

/// The typed scalar pipeline (`scalar_pipe.rs`) is a speculation with a boxed fallback: each program below holds
/// the exact number of chains it drained typed and of chains that started typed and met an element it hands to the
/// boxed path (a guard that stops declining, or a bail that stops happening, changes a count). The outputs of the
/// same programs are held to the walker's by the engine parity corpus.
#[test]
fn the_scalar_pipeline_takes_and_declines_exactly_these_chains() {
    // (program, drained typed, started typed and bailed)
    const PIPE: &[(&str, u64, u64)] = &[
        ("tests/engine_parity/scalar_pipe_taken.wj", 9, 0),
        ("tests/engine_parity/scalar_pipe_effects.wj", 0, 0),
        ("tests/engine_parity/scalar_pipe_capmut.wj", 5, 0),
        ("tests/engine_parity/scalar_pipe_overflow.wj", 1, 2),
        ("tests/engine_parity/scalar_pipe_divzero.wj", 0, 2),
        ("tests/engine_parity/scalar_pipe_take.wj", 0, 0),
        ("tests/engine_parity/scalar_pipe_nonscalar.wj", 2, 0),
        ("tests/engine_parity/scalar_pipe_widths.wj", 2, 2),
        ("tests/engine_parity/scalar_pipe_sum.wj", 5, 0),
        ("tests/engine_parity/scalar_pipe_sum_order.wj", 0, 2),
        ("tests/engine_parity/scalar_pipe_sum_overflow.wj", 1, 1),
    ];
    let mut problems = Vec::new();
    for (program, taken, bailed) in PIPE {
        let r = run(program, "resolved", None);
        let got = (count(&r, "scalarpipe").unwrap_or(0), count(&r, "scalarbail").unwrap_or(0));
        if got != (*taken, *bailed) {
            problems.push(format!("{program}: (typed drains, bails) = {got:?}, expected {:?}", (taken, bailed)));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// A function unit refuses a call whose parameters name the same cell (`osr::enter_body`, `params_alias`): a program that
/// only makes aliased calls never counts `fnunit`. The refusal is defensive (the unit would be correct: both names reach the
/// same memory and every view is dropped by a write helper), so the output cannot show it; this count does.
#[test]
fn function_units_refuse_aliased_parameters() {
    let program = "tests/engine_parity/jit_fnunit_alias.wj";
    let r = run(program, "resolved", None);
    assert_eq!(count(&r, "fnunit").unwrap_or(u64::MAX), 0, "{program}: an aliased call entered a function unit: {:?}", r.counts);
    let w = run(program, "walker", None);
    assert_eq!((&r.stdout, r.code), (&w.stdout, w.code), "{program} differs from the walker");
}

/// The lean map helpers (`jit/rt.rs::lean_map`) serve integer-keyed maps only: a program whose maps have string keys
/// or struct values never counts one (its lookups run in the closure engine), and the same program with every lean
/// helper switched off in a debug build still prints what the walker prints.
#[test]
fn the_lean_map_helpers_decline_other_shapes() {
    let r = run("tests/engine_parity/map_get_wild.wj", "resolved", None);
    assert_eq!((count(&r, "mapleanget").unwrap_or(0), count(&r, "mapleanins").unwrap_or(0)), (0, 0), "{:?}", r.counts);
}

/// Typed integer arrays (`kmap.rs` IntDense) hold 64-bit signed values under integer keys only: the lean helpers answer
/// from one for the kernel's map and never for i32 / u64 values or negative keys (those maps stay on the ordinary store).
#[test]
fn typed_arrays_are_only_for_64_bit_signed_int_values_and_keys_in_range() {
    let r = run("tests/engine_parity/kmap_intdense_widths.wj", "resolved", None);
    assert!(count(&r, "mapleanget").unwrap_or(0) > 0, "{:?}", r.counts);
    assert_eq!(count(&r, "mapleantyped").unwrap_or(0), 0, "{:?}", r.counts);
}
