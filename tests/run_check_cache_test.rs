//! The check before an interpreted run is cached by content (`src/check/run_cache.rs`): a repeat run of an unchanged
//! program replays the stored outcome, and any change to an input (a sibling module, a flag, the `WJ_*` environment,
//! the working directory) checks again, so a cached run never accepts a program the check would refuse.
//! Run: cargo test --release --test all -- run_check_cache

use std::path::Path;
use std::process::{Command, Output};

const PROG: &str = "mod dep\n\nfn main() {\n    let x = dep::f(1)\n    println(\"${x}\")\n}\n";
const DEP: &str = "pub fn f(x: int) -> int {\n    x + 1\n}\n";
/// Stands in for a stored outcome's text: printed only when the stored entry was used.
const MARK: &str = "@@stored-outcome@@";

fn run(dir: &Path, cwd: &Path, extra: &[&str], env: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    c.current_dir(cwd).arg("run").arg("--interpret").args(extra).arg(dir.join("prog.wj"));
    c.env_remove("WJ_PHASES");
    c.env("WJ_CARGO_TARGET_ROOT", dir.join("cache"));
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().expect("run wj")
}

fn entries(dir: &Path) -> Vec<std::path::PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir.join("cache/runs")) else { return Vec::new() };
    rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect()
}

/// Replace the stored outcome's text with [`MARK`], so the next run shows whether it used the entry.
fn mark_entries(dir: &Path) {
    for p in entries(dir) {
        let text = std::fs::read_to_string(&p).unwrap();
        let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
        v["stderr"] = serde_json::Value::String(format!("{MARK}\n"));
        std::fs::write(&p, v.to_string()).unwrap();
    }
}

fn setup() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("prog.wj"), PROG).unwrap();
    std::fs::write(d.path().join("dep.wj"), DEP).unwrap();
    d
}

#[test]
fn an_unchanged_program_replays_the_stored_check_and_a_changed_input_checks_again() {
    let d = setup();
    let dir = d.path();
    let first = run(dir, dir, &[], &[]);
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert_eq!(String::from_utf8_lossy(&first.stdout), "2\n");
    assert!(!entries(dir).is_empty(), "the clean check is stored");
    let again = run(dir, dir, &[], &[]);
    assert_eq!(again.stdout, first.stdout);
    assert_eq!(again.stderr, first.stderr, "a hit prints exactly what the check printed");

    mark_entries(dir);
    let hit = run(dir, dir, &[], &[]);
    assert!(String::from_utf8_lossy(&hit.stderr).contains(MARK), "an unchanged program uses the stored outcome");
    assert_eq!(String::from_utf8_lossy(&hit.stdout), "2\n");

    // every one of these is an input of the check: none may use the stored outcome
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    let misses: Vec<(&str, Output)> = vec![
        ("--no-lint", run(dir, dir, &["--no-lint"], &[])),
        ("a WJ_ variable", run(dir, dir, &[], &[("WJ_RUN_CACHE_TEST", "1")])),
        ("another working directory", run(dir, &sub, &[], &[])),
    ];
    for (what, out) in &misses {
        assert!(out.status.success(), "{what}: {}", String::from_utf8_lossy(&out.stderr));
        assert!(!String::from_utf8_lossy(&out.stderr).contains(MARK), "{what} changes the key");
    }

    // a sibling module (never the entry file) gains a parameter: the call in prog.wj is now an error
    mark_entries(dir);
    std::fs::write(dir.join("dep.wj"), DEP.replace("(x: int)", "(x: int, y: int)")).unwrap();
    let broken = run(dir, dir, &[], &[]);
    assert!(!broken.status.success(), "the edited program is refused, not replayed");
    assert!(!String::from_utf8_lossy(&broken.stderr).contains(MARK));
    assert!(String::from_utf8_lossy(&broken.stderr).contains("not running"));
    let n = entries(dir).len();
    let refused = run(dir, dir, &[], &[]);
    assert!(!refused.status.success(), "a refused program is never stored");
    assert_eq!(entries(dir).len(), n);
}

/// The solver's literal widths the interpreter reads (ADR-0038 C6b) are stored with the checks: a repeat run uses the
/// stored widths (an entry rewritten to "no widths" changes the behaviour, so it was read), and an edited file is
/// solved again.
#[test]
fn literal_widths_are_stored_by_content_and_used_on_a_repeat_run() {
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/drift_corpus/repros/index_countdown_usize_underflow.wj")).unwrap();
    std::fs::write(dir.join("prog.wj"), &src).unwrap();
    let first = run(dir, dir, &[], &[]);
    assert!(!first.status.success(), "the usize countdown panics as compiled: {}", String::from_utf8_lossy(&first.stderr));
    let facts: Vec<_> = entries(dir).into_iter().filter(|p| std::fs::read_to_string(p).unwrap().contains(":usize")).collect();
    assert_eq!(facts.len(), 1, "the literal widths are stored");
    let again = run(dir, dir, &[], &[]);
    assert_eq!((again.status.code(), &again.stdout, &again.stderr), (first.status.code(), &first.stdout, &first.stderr));

    // the stored widths are what the run uses: without them the countdown ends at -1 and does not panic
    let text = std::fs::read_to_string(&facts[0]).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["stderr"] = serde_json::Value::String("-".into());
    std::fs::write(&facts[0], v.to_string()).unwrap();
    let forged = run(dir, dir, &[], &[]);
    assert!(forged.status.success(), "the forged entry was read");

    // an edit is a new key: solved again, the panic is back
    std::fs::write(dir.join("prog.wj"), format!("{src}\n// edited\n")).unwrap();
    let edited = run(dir, dir, &[], &[]);
    assert!(!edited.status.success(), "an edited file is solved again");
}
