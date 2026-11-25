//! Hoisted row views (versioned loops): a trap (index past the end, integer overflow) in the middle of a versioned
//! pair loop leaves machine code through the statement's ordinary exit and the closure engine resumes there. The
//! programs in `tests/hoist_traps` end in a runtime error; the walker, the resolved engine and the resolved engine
//! with every loop in machine code at its first iteration must agree on stdout, exit code and stderr (the failing
//! frame's scalar locals show how many iterations committed; struct-vector locals print differently in the walker
//! and the resolved engine in every program, so those lines are dropped), and the eager run must have taken the
//! fast copy (`hoisted`).

use std::path::Path;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn run(program: &Path, engine: &str, hot0: bool) -> (Vec<u8>, Option<i32>, String, u64) {
    let mut c = Command::new(WJ);
    c.args(["run", "--interpret", "--no-lint", "--engine", engine]).arg(program).stdin(std::process::Stdio::null());
    c.env_remove("WJ_JIT").env_remove("WJ_JIT_MUTANT").env_remove("WJ_JIT_LOOP_HOT").env("WJ_JIT_DEBUG", "1").env("WJ_NO_STALE_WARN", "1");
    if hot0 {
        c.env("WJ_JIT_LOOP_HOT", "0");
    }
    let o = c.output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let hoisted = err
        .lines()
        .find_map(|l| l.strip_prefix("[fast] "))
        .and_then(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            let i = w.iter().position(|x| *x == "hoisted")?;
            w.get(i + 1)?.parse().ok()
        })
        .unwrap_or(0);
    let keep: String = err.lines().filter(|l| !l.starts_with('[') && !l.contains("{")).map(|l| format!("{l}\n")).collect();
    (o.stdout, o.status.code(), keep, hoisted)
}

#[test]
fn a_trap_inside_a_hoisted_loop_matches_the_walker() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/hoist_traps");
    let mut n = 0;
    for e in std::fs::read_dir(&dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "wj") {
            continue;
        }
        n += 1;
        let w = run(&p, "walker", false);
        for (hot0, name) in [(false, "resolved"), (true, "resolved eager")] {
            let r = run(&p, "resolved", hot0);
            assert_eq!((&w.0, w.1, &w.2), (&r.0, r.1, &r.2), "{}: {name} differs from the walker", p.display());
            assert!(w.1 == Some(101), "{}: the program must end in a runtime error ({:?})", p.display(), w.1);
            if hot0 {
                assert!(r.3 > 0, "{}: no versioned loop ran its fast copy", p.display());
            }
        }
    }
    assert!(n >= 2, "no trap programs found");
}
