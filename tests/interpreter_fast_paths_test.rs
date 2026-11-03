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
    ("bench/interp/hashmap.wj", &["mapget", "mapins", "unboxed"]),
    ("bench/interp/sieve.wj", &["packed", "packedidx"]),
    ("bench/interp/nbody.wj", &["vecresolve"]),
    ("bench/interp/strings.wj", &["pushfmt"]),
    ("bench/interp/closures.wj", &["closureenv", "closurescope"]),
    ("tests/engine_parity/str_fast.wj", &["fmt", "pushstr", "pushfmt"]),
    ("tests/engine_parity/jit_loop_inline.wj", &["inline"]),
    ("tests/engine_parity/map_get_match.wj", &["mapget", "mapins", "unboxed"]),
    ("tests/engine_parity/option_arm_values.wj", &["mapget", "mapins"]),
    ("tests/engine_parity/packed_vecs.wj", &["packed"]),
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
