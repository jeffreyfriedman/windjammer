//! Counted fallbacks (ADR-0038, correctness and performance ladders): every slower path the resolved engine can take
//! is correct, so neither the output nor the parity gates show it. A walker island inside a hot loop, a call
//! that deopts, a loop machine code refuses or gives up on: each costs speed in silence. `WJ_JIT_DEBUG` makes
//! `wj run` print `[fallback] island N walkercall N deopt N jitrefused N jitdead N jitexit N`; this gate holds
//! the counts of the benchmark kernels to `bench/interp/fallbacks.tsv`, exactly (the programs are deterministic).
//! A count that grew is a regression; one that shrank is progress that must be recorded, so the ledger
//! only ever moves on purpose. `WJ_FALLBACK_RECORD=1` rewrites the ledger from this run.

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const LEDGER: &str = "bench/interp/fallbacks.tsv";
const COUNTERS: [&str; 6] = ["island", "walkercall", "deopt", "jitrefused", "jitdead", "jitexit"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The programs the ledger covers: every kernel of `bench/interp` and the startup program.
fn programs() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(root().join("bench/interp"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".wj"))
        .map(|n| format!("bench/interp/{n}"))
        .collect();
    v.push("bench/interp/startup/hello.wj".into());
    v.sort();
    v
}

/// The `[fallback]` counts of one resolved-engine run of `program` (default machine-code settings).
fn counts(program: &Path, extra: &[&str]) -> Vec<u64> {
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", "resolved"]).args(extra).arg(program).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT").env_remove("WJ_JIT_MUTANT").env_remove("WJ_JIT_LOOP_HOT").env("WJ_JIT_DEBUG", "1");
    let o = c.output().unwrap();
    assert!(o.status.success(), "{}: exit {:?}\n{}", program.display(), o.status.code(), String::from_utf8_lossy(&o.stderr));
    let err = String::from_utf8_lossy(&o.stderr);
    let line = err.lines().find_map(|l| l.strip_prefix("[fallback] ")).unwrap_or_else(|| panic!("{}: no [fallback] line", program.display()));
    parse_counts(line)
}

/// `island 3 walkercall 0 ...` -> the counts in `COUNTERS` order (a missing name is a failure).
fn parse_counts(line: &str) -> Vec<u64> {
    let words: Vec<&str> = line.split_whitespace().collect();
    COUNTERS
        .iter()
        .map(|name| {
            let at = words.iter().position(|w| w == name).unwrap_or_else(|| panic!("`{name}` missing from `{line}`"));
            words[at + 1].parse().unwrap()
        })
        .collect()
}

fn ledger_line(program: &str, c: &[u64]) -> String {
    let nums: Vec<String> = c.iter().map(|n| n.to_string()).collect();
    format!("{program}\t{}", nums.join("\t"))
}

/// The recorded counts per program.
fn ledger() -> Vec<(String, Vec<u64>)> {
    let text = std::fs::read_to_string(root().join(LEDGER)).unwrap_or_default();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let mut f = l.split('\t');
            let p = f.next().unwrap().to_string();
            (p, f.map(|n| n.parse().unwrap()).collect())
        })
        .collect()
}

/// What differs between the recorded and the measured counts of one program ("" when they agree).
fn judge(program: &str, recorded: Option<&Vec<u64>>, now: &[u64]) -> String {
    let Some(was) = recorded else { return format!("{program}: not in {LEDGER}; add `{}`", ledger_line(program, now)) };
    let mut out = Vec::new();
    for ((name, w), n) in COUNTERS.iter().zip(was).zip(now) {
        if n > w {
            out.push(format!("{program}: `{name}` grew {w} -> {n} (a slower path is taken more often)"));
        } else if n < w {
            out.push(format!("{program}: `{name}` shrank {w} -> {n}: record it (`{}`)", ledger_line(program, now)));
        }
    }
    out.join("\n")
}

#[test]
fn kernel_fallbacks_match_the_ledger() {
    let book = ledger();
    let mut problems = Vec::new();
    let mut lines = vec![
        "# Fallback counts of the resolved engine per program (tests/interpreter_fallbacks_test.rs, ADR-0038).".to_string(),
        format!("# program\t{}", COUNTERS.join("\t")),
    ];
    for p in programs() {
        let now = counts(&root().join(&p), &[]);
        let was = book.iter().find(|(q, _)| *q == p).map(|(_, c)| c);
        let j = judge(&p, was, &now);
        if !j.is_empty() {
            problems.push(j);
        }
        lines.push(ledger_line(&p, &now));
    }
    if std::env::var_os("WJ_FALLBACK_RECORD").is_some() {
        std::fs::write(root().join(LEDGER), lines.join("\n") + "\n").unwrap();
        return;
    }
    assert!(problems.is_empty(), "fallback counts differ from {LEDGER}:\n{}", problems.join("\n"));
}

/// The gate can fail: a count above the ledger is reported as growth, one below as unrecorded progress, and a
/// program missing from the ledger is reported too.
#[test]
fn the_ledger_judge_notices_growth_shrinkage_and_new_programs() {
    let was = vec![0, 2, 0, 1, 0, 5];
    assert_eq!(judge("p", Some(&was), &[0, 2, 0, 1, 0, 5]), "");
    assert!(judge("p", Some(&was), &[1, 2, 0, 1, 0, 5]).contains("`island` grew 0 -> 1"));
    assert!(judge("p", Some(&was), &[0, 2, 0, 1, 0, 4]).contains("`jitexit` shrank 5 -> 4"));
    assert!(judge("p", None, &[0; 6]).contains("not in"));
}

/// The counters are wired: programs built to take each slower path report it (a counter that stays 0
/// would make the ledger above vacuous).
#[test]
fn each_counter_counts_on_a_program_that_takes_its_path() {
    let dir = std::env::temp_dir().join(format!("wj-fallbacks-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // a generic function is never compiled (walkercall); a loop over a closure-held value runs outside machine code
    let walker = dir.join("walker.wj");
    std::fs::write(&walker, "fn id<T>(x: T) -> T {\n    x\n}\n\nfn main() {\n    let mut s = 0\n    let mut i = 0\n    while i < 1000 {\n        s += id(i)\n        i += 1\n    }\n    println(\"${s}\")\n}\n").unwrap();
    let c = counts(&walker, &[]);
    assert!(c[1] >= 1000, "walkercall did not count the generic calls: {c:?}");
    let _ = std::fs::remove_dir_all(&dir);
    let nbody = counts(&root().join("bench/interp/nbody.wj"), &[]);
    assert!(nbody[0] > 0, "the nbody main loop's call runs as an island today; island did not count: {nbody:?}");
    let closures = counts(&root().join("bench/interp/closures.wj"), &[]);
    assert!(closures[3] > 0, "closures' iterator-chain loop is refused by machine code today; jitrefused did not count: {closures:?}");
}
