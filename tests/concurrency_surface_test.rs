#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! ADR-0004 concurrency surface, stage 1 (docs/concurrency/PLAN.md): `spawn`/`async` tasks, `scope { }`, `Task.wait()`,
//! channel methods; WJ0190 (un-waited task) and WJ0191 (removed `thread { }` / `<-`) with their `wj fix` migration.
//!
//! Oracles: the conformance programs under `tests/conformance/concurrency/` state their expected output in their
//! header (worked out by hand from the semantics, independent of any backend); every program must print exactly that
//! under several schedules (`--sched-seed`), since a correct program's output depends only on `wait()`, `recv()` and
//! scope exit (SPEC-CONC-006). Diagnostics are checked against the documented codes and fix texts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

struct Out {
    stdout: String,
    stderr: String,
    exit: i32,
}

fn wj(args: &[&str], dir: &Path) -> Out {
    let o = Command::new(WJ)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    Out {
        stdout: String::from_utf8_lossy(&o.stdout).to_string(),
        stderr: String::from_utf8_lossy(&o.stderr).to_string(),
        exit: o.status.code().unwrap_or(-1),
    }
}

fn write(dir: &TempDir, src: &str) -> PathBuf {
    let f = dir.path().join("main.wj");
    fs::write(&f, src).unwrap();
    f
}

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance/concurrency")
}

fn conformance_programs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(conformance_dir())
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "wj"))
        .collect();
    v.sort();
    v
}

/// The `// EXPECTED OUTPUT:` block of a conformance program.
fn expected_output(src: &str) -> String {
    let mut out = String::new();
    let mut on = false;
    for line in src.lines() {
        if line.trim() == "// EXPECTED OUTPUT:" {
            on = true;
            continue;
        }
        if on {
            match line.strip_prefix("// ") {
                Some(l) => {
                    out.push_str(l);
                    out.push('\n');
                }
                None => break,
            }
        }
    }
    out
}

#[test]
fn conformance_programs_print_their_expected_output_under_every_schedule() {
    let programs = conformance_programs();
    assert!(
        programs.len() >= 4,
        "concurrency conformance programs missing: {programs:?}"
    );
    for p in programs {
        let src = fs::read_to_string(&p).unwrap();
        let want = expected_output(&src);
        assert!(
            want.ends_with("[all] PASSED\n"),
            "{}: no EXPECTED OUTPUT block",
            p.display()
        );
        for seed in ["0", "1", "2", "3", "7", "11"] {
            let o = wj(
                &[
                    "run",
                    "--interpret",
                    "--deterministic-time",
                    "--sched-seed",
                    seed,
                    p.to_str().unwrap(),
                ],
                &conformance_dir(),
            );
            assert_eq!(
                o.stdout,
                want,
                "{} (schedule {seed}):\n{}",
                p.display(),
                o.stderr
            );
            assert_eq!(o.exit, 0, "{} (schedule {seed}): {}", p.display(), o.stderr);
        }
    }
}

#[test]
fn conformance_programs_pass_wj_check() {
    for p in conformance_programs() {
        let o = wj(&["check", p.to_str().unwrap()], &conformance_dir());
        assert_eq!(o.exit, 0, "{}:\n{}{}", p.display(), o.stdout, o.stderr);
    }
}

/// Stage 2b (docs/concurrency/PLAN.md): every conformance program, compiled through the Rust backend
/// (`windjammer_runtime::task`, OS threads, a local queue for `async`), prints exactly the expected output, and
/// `wj run --verify` finds the compiled run identical to the interpreted one.
#[test]
fn conformance_programs_compiled_match_the_interpreter() {
    for p in conformance_programs() {
        let src = fs::read_to_string(&p).unwrap();
        let want = expected_output(&src);
        let dir = TempDir::new().unwrap();
        let f = write(&dir, &src);
        let o = wj(&["run", "--verify", f.to_str().unwrap()], dir.path());
        assert_eq!(o.stdout, want, "{}:\n{}", p.display(), o.stderr);
        assert!(o.stderr.contains("the compiled run matches"), "{}:\n{}", p.display(), o.stderr);
        assert_eq!(o.exit, 0, "{}:\n{}", p.display(), o.stderr);
    }
}

#[test]
fn a_task_panic_propagates_out_of_its_scope_after_the_other_tasks_finish() {
    let dir = TempDir::new().unwrap();
    let src = "use std::sync\n\nfn boom(n: int) -> int {\n    if n == 2 {\n        panic(\"task ${n} failed\")\n    }\n    n\n}\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    println(\"start\")\n    scope {\n        spawn { tx.send(boom(1)) }\n        spawn { tx.send(boom(2)) }\n        spawn { tx.send(boom(3)) }\n    }\n    println(\"unreachable ${rx.recv()}\")\n}\n";
    let f = write(&dir, src);
    let o = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(o.stdout, "start\n", "{}", o.stderr);
    assert_eq!(o.exit, 101, "{}", o.stderr);
    assert!(o.stderr.contains("task 2 failed"), "{}", o.stderr);
}

#[test]
fn a_task_panic_reaches_the_waiter() {
    let dir = TempDir::new().unwrap();
    let src = "fn boom() -> int {\n    panic(\"no value\")\n}\n\nfn main() {\n    let t = spawn boom()\n    println(\"before\")\n    println(\"${t.wait()}\")\n}\n";
    let f = write(&dir, src);
    let o = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(o.stdout, "before\n", "{}", o.stderr);
    assert_eq!(o.exit, 101, "{}", o.stderr);
    assert!(o.stderr.contains("no value"), "{}", o.stderr);
}

/// WJ0190 cases: (program, line of the task). Each is refused by `wj check` (with the code), by `wj run --interpret`
/// and by `wj build`.
const UNWAITED: &[(&str, usize)] = &[
    ("fn w() -> int {\n    1\n}\n\nfn main() {\n    spawn w()\n    println(\"x\")\n}\n", 6),
    ("fn main() {\n    let _ = async { 1 }\n    println(\"x\")\n}\n", 2),
    ("fn main() {\n    let t = spawn { 2 }\n    println(\"x\")\n}\n", 2),
    ("fn w() -> int {\n    1\n}\n\nfn start() -> Task<int> {\n    spawn w()\n}\n\nfn main() {\n    start()\n}\n", 10),
    ("fn main() {\n    for i in 0..3 {\n        spawn { println(\"${i}\") }\n    }\n}\n", 3),
];

#[test]
fn an_unwaited_task_is_refused_by_check_run_and_build() {
    for (src, line) in UNWAITED {
        let dir = TempDir::new().unwrap();
        let f = write(&dir, src);
        let c = wj(&["check", "--json", f.to_str().unwrap()], dir.path());
        assert_eq!(c.exit, 1, "{src}\n{}", c.stdout);
        assert!(
            c.stdout.contains("\"code\": \"WJ0190\""),
            "{src}\n{}",
            c.stdout
        );
        assert!(
            c.stdout.contains(&format!("\"line_start\": {line}")),
            "{src}\n{}",
            c.stdout
        );
        let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
        assert_ne!(r.exit, 0, "{src}");
        assert!(r.stderr.contains("WJ0190"), "{src}\n{}", r.stderr);
        assert_eq!(r.stdout, "", "{src}: the program must not run");
        let b = wj(&["build", "main.wj", "--no-cargo"], dir.path());
        assert_ne!(b.exit, 0, "{src}: wj build accepted it\n{}", b.stdout);
        assert!(
            format!("{}{}", b.stdout, b.stderr).contains("WJ0190"),
            "{src}\n{}{}",
            b.stdout,
            b.stderr
        );
    }
}

#[test]
fn waited_scoped_and_returned_tasks_are_accepted() {
    let src = "fn w(n: int) -> int {\n    n\n}\n\nfn start() -> Task<int> {\n    spawn w(5)\n}\n\nfn main() {\n    let t = spawn w(1)\n    scope {\n        spawn w(2)\n        async { w(3) }\n        for i in 0..2 {\n            spawn { println(\"loop ${i}\") }\n        }\n    }\n    let s = start()\n    println(\"${t.wait() + s.wait()}\")\n}\n";
    let dir = TempDir::new().unwrap();
    let f = write(&dir, src);
    let c = wj(&["check", f.to_str().unwrap()], dir.path());
    assert_eq!(c.exit, 0, "{}{}", c.stdout, c.stderr);
    let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(r.stdout, "loop 0\nloop 1\n6\n", "{}", r.stderr);
}

/// WJ0191: (program, the machine-applicable replacement the diagnostic carries).
const REMOVED: &[(&str, &str)] = &[
    ("fn main() {\n    thread {\n        println(\"x\")\n    }\n}\n", "\"replacement\": \"spawn\""),
    ("use std::sync\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    tx <- 5\n    println(\"${rx.recv()}\")\n}\n", "\"replacement\": \".send(\""),
    ("use std::sync\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    tx.send(5)\n    let v = <-rx\n    println(\"${v}\")\n}\n", "\"replacement\": \".recv()\""),
];

#[test]
fn removed_forms_are_refused_with_their_fix() {
    for (src, replacement) in REMOVED {
        let dir = TempDir::new().unwrap();
        let f = write(&dir, src);
        let c = wj(&["check", "--json", f.to_str().unwrap()], dir.path());
        assert_eq!(c.exit, 1, "{src}\n{}", c.stdout);
        assert!(
            c.stdout.contains("\"code\": \"WJ0191\""),
            "{src}\n{}",
            c.stdout
        );
        assert!(c.stdout.contains(replacement), "{src}\n{}", c.stdout);
        assert!(
            !c.stdout.contains("\"code\": \"WJ0011\""),
            "{src}: a removed form is not a generic syntax error\n{}",
            c.stdout
        );
        let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
        assert_ne!(r.exit, 0, "{src}");
        assert!(r.stderr.contains("WJ0191"), "{src}\n{}", r.stderr);
    }
}

#[test]
fn a_comparison_with_a_negative_number_still_reads_as_written_when_spaced() {
    let dir = TempDir::new().unwrap();
    let f = write(
        &dir,
        "fn main() {\n    let x = 3\n    println(\"${x < -1}\")\n}\n",
    );
    let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(r.stdout, "false\n", "{}", r.stderr);
}

/// `wj fix` migrates a whole pre-ADR-0004 program: the result runs and prints what the old program meant.
#[test]
fn wj_fix_migrates_thread_blocks_and_channel_operators() {
    let old = "use std::sync\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    thread {\n        tx <- 20\n        tx <- 22\n    }\n    let a = <-rx\n    let b = <-rx\n    println(\"${a + b}\")\n}\n";
    let dir = TempDir::new().unwrap();
    let f = write(&dir, old);
    let fix = wj(&["fix", f.to_str().unwrap()], dir.path());
    assert_eq!(fix.exit, 0, "{}{}", fix.stdout, fix.stderr);
    let new = fs::read_to_string(&f).unwrap();
    let want = "use std::sync\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    scope {\n        spawn {\n            tx.send(20)\n            tx.send(22)\n        }\n        let a = rx.recv()\n        let b = rx.recv()\n        println(\"${a + b}\")\n    }\n}\n";
    assert_eq!(new, want);
    let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(r.stdout, "42\n", "{}", r.stderr);
    // idempotent
    let again = wj(&["fix", "--check", f.to_str().unwrap()], dir.path());
    assert_eq!(again.exit, 0, "{}{}", again.stdout, again.stderr);
}

/// The tasks a task starts belong to the scope its spawner was in, also when other tasks open scopes meanwhile
/// (each task has its own stack of open scopes).
#[test]
fn scopes_are_per_task_under_interleaving() {
    let src = "use std::sync\n\nfn main() {\n    let (tx, rx) = sync::channel()\n    scope {\n        spawn {\n            scope {\n                spawn { tx.send(1) }\n            }\n            tx.send(2)\n        }\n        spawn {\n            spawn { tx.send(3) }\n        }\n    }\n    let mut v = Vec::new()\n    for _ in 0..3 {\n        v.push(rx.recv())\n    }\n    v.sort()\n    println(\"${v:?}\")\n}\n";
    let dir = TempDir::new().unwrap();
    let f = write(&dir, src);
    for seed in ["0", "1", "2", "5", "9"] {
        let r = wj(
            &[
                "run",
                "--interpret",
                "--sched-seed",
                seed,
                f.to_str().unwrap(),
            ],
            dir.path(),
        );
        assert_eq!(r.stdout, "[1, 2, 3]\n", "schedule {seed}: {}", r.stderr);
    }
}

#[test]
fn select_takes_the_first_ready_arm_and_panics_when_nothing_can_become_ready() {
    let dir = TempDir::new().unwrap();
    let src = "use std::sync\n\nfn make() -> sync::Receiver<int> {\n    let (tx, rx) = sync::channel()\n    tx.send(1)\n    rx\n}\n\nfn main() {\n    let rx = make()\n    select {\n        x = rx.recv() => println(\"got ${x}\"),\n    }\n    select {\n        x = rx.recv() => println(\"again ${x}\"),\n    }\n}\n";
    let f = write(&dir, src);
    let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_eq!(r.stdout, "got 1\n", "{}", r.stderr);
    assert_eq!(r.exit, 101, "{}", r.stderr);
    assert!(r.stderr.contains("every channel of this `select` is closed"), "{}", r.stderr);
}

#[test]
fn a_select_arm_must_wait_on_recv_wait_or_timeout() {
    let dir = TempDir::new().unwrap();
    let f = write(&dir, "fn f() -> int {\n    1\n}\n\nfn main() {\n    select {\n        x = f() => println(\"${x}\"),\n    }\n}\n");
    let r = wj(&["run", "--interpret", f.to_str().unwrap()], dir.path());
    assert_ne!(r.exit, 0);
    assert!(r.stderr.contains("a `select` arm waits on `rx.recv()`, `t.wait()` or `timeout(d)`"), "{}", r.stderr);
}
