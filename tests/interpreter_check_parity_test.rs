//! Differential gate: the interpreter must not be more permissive than the compiler front end.
//!
//! Invariant: `wj run --interpret FILE` refuses (exit != 0, program never starts) every program
//! `wj check FILE` rejects, and does not refuse a program `wj check` accepts. Historical miss: the
//! interpreter ran `let x: int = "s"` and printed `s`.
//!
//! Programs: the labelled mutation operators of `common/check_diff_gen.rs` (each MUST be rejected by
//! `wj check`), unmutated controls, the interpreter corpus and the verified examples (valid programs: `wj check` must accept them, so a
//! false positive in the checker cannot silently block the interpreter).

#[path = "common/check_diff_gen.rs"]
mod check_diff_gen;

use check_diff_gen::{instantiate, OPERATORS};
use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");
const SEEDS: [u64; 2] = [7, 424242];

struct Outcome {
    check_ok: bool,
    run_ok: bool,
    run_stdout: String,
    run_stderr: String,
}

fn exercise(src: &str) -> Outcome {
    let tmp = tempfile::TempDir::new().unwrap();
    let f = tmp.path().join("prog.wj");
    std::fs::write(&f, src).unwrap();
    let check = Command::new(WJ)
        .arg("check")
        .arg(&f)
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let run = Command::new(WJ)
        .args(["run", "--interpret"])
        .arg(&f)
        .current_dir(tmp.path())
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    Outcome {
        check_ok: check.status.success(),
        run_ok: run.status.success(),
        run_stdout: String::from_utf8_lossy(&run.stdout).to_string(),
        run_stderr: String::from_utf8_lossy(&run.stderr).to_string(),
    }
}

/// Problems with one program's pair of outcomes (pure, so the self-tests can feed it broken inputs).
fn judge(name: &str, o: &Outcome) -> Option<String> {
    let refused =
        o.run_stderr.contains("not running") && o.run_stderr.contains("`wj check` reports");
    if !o.check_ok && (o.run_ok || !o.run_stdout.is_empty() || !refused) {
        return Some(format!("{name}: `wj check` rejects it but the interpreter did not refuse it (ok={}, stdout={:?})", o.run_ok, o.run_stdout));
    }
    if o.check_ok && refused {
        return Some(format!(
            "{name}: `wj check` accepts it but the interpreter refused it"
        ));
    }
    None
}

fn corpus() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in ["tests/interpreter_corpus", "examples/verified"] {
        files.extend(std::fs::read_dir(root.join(dir)).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")));
    }
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(root).unwrap().display().to_string();
            (rel, std::fs::read_to_string(&p).unwrap().replace("__SEED__", "1"))
        })
        .collect()
}

#[test]
fn interpreter_refuses_exactly_what_check_rejects() {
    let mut progs: Vec<(String, String)> = Vec::new();
    for seed in SEEDS {
        progs.push((format!("control@{seed}"), instantiate(seed, None)));
        for op in OPERATORS {
            progs.push((format!("{}@{seed}", op.name), instantiate(seed, Some(op))));
        }
    }
    progs.extend(corpus());
    let problems = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for chunk in progs.chunks(progs.len().div_ceil(4)) {
            let problems = &problems;
            s.spawn(move || {
                for (name, src) in chunk {
                    if let Some(p) = judge(name, &exercise(src)) {
                        problems.lock().unwrap().push(p);
                    }
                }
            });
        }
    });
    let problems = problems.into_inner().unwrap();
    assert!(
        problems.is_empty(),
        "{} interpreter/check disagreement(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn the_historical_miss_is_refused() {
    let o = exercise("fn main() {\n    let x: int = \"s\"\n    println(\"${x}\")\n}\n");
    assert!(
        !o.check_ok && !o.run_ok && o.run_stdout.is_empty(),
        "stdout={:?} stderr={:?}",
        o.run_stdout,
        o.run_stderr
    );
}

#[test]
fn selftest_judge_detects_a_permissive_interpreter() {
    let permissive = Outcome {
        check_ok: false,
        run_ok: true,
        run_stdout: "s\n".into(),
        run_stderr: String::new(),
    };
    assert!(judge("m", &permissive).is_some());
    let crashed_not_refused = Outcome {
        check_ok: false,
        run_ok: false,
        run_stdout: String::new(),
        run_stderr: "boom".into(),
    };
    assert!(
        judge("m", &crashed_not_refused).is_some(),
        "failing for another reason is not a refusal"
    );
    let over = Outcome {
        check_ok: true,
        run_ok: false,
        run_stdout: String::new(),
        run_stderr: "Error: not running x: `wj check` reports 1 error(s)".into(),
    };
    assert!(judge("m", &over).is_some());
    let good = Outcome {
        check_ok: false,
        run_ok: false,
        run_stdout: String::new(),
        run_stderr: "Error: not running x: `wj check` reports 1 error(s)".into(),
    };
    assert!(judge("m", &good).is_none());
}
