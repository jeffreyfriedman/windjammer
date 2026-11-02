//! Sleep in the interpreter (ADR-0021): under `wj run` (live, not deterministic, not simulated, not
//! recording or replaying) a sleep the whole program waits on takes real time, so a program polling the
//! outside world (a lock directory, a child process) really waits. Everywhere a run must be reproducible
//! it stays virtual and returns at once: `--deterministic-time`, `--simulate`, `--record`/`--replay` and
//! `wj test`. `Instant::elapsed()` counts a sleep once in every mode.

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Sleeps `ms` twice (both sleep builtins) and prints what `elapsed()` saw, in milliseconds.
fn program(ms: u64) -> String {
    format!(
        "use std::async\nuse std::thread\nuse std::time\n\nfn main() {{\n    let start = Instant::now()\n    async::sleep_ms_blocking({ms})\n    thread::sleep(Duration::from_millis({ms}))\n    println(\"elapsed ${{start.elapsed().as_millis()}}\")\n}}\n"
    )
}

fn project(main: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("main.wj"), main).unwrap();
    std::fs::write(dir.path().join("wj.toml"), "[package]\nname = \"sleeper\"\nversion = \"0.1.0\"\n").unwrap();
    dir
}

/// Wall time of `wj <args>` in `dir`, its stdout and exit code.
fn timed(dir: &Path, args: &[&str]) -> (Duration, String, Option<i32>) {
    let t = Instant::now();
    let o = Command::new(WJ).args(args).current_dir(dir).stdin(std::process::Stdio::null()).output().unwrap();
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(o.status.success(), "wj {args:?}: {out}\n{}", String::from_utf8_lossy(&o.stderr));
    (t.elapsed(), out, o.status.code())
}

fn elapsed_ms(out: &str) -> u64 {
    out.lines().find_map(|l| l.strip_prefix("elapsed ")).and_then(|n| n.trim().parse().ok()).unwrap_or_else(|| panic!("no elapsed in {out}"))
}

#[test]
fn wj_run_waits_for_a_sleep_and_counts_it_once() {
    let dir = project(&program(1000));
    let (wall, out, _) = timed(dir.path(), &["run", "--interpret", "--no-lint", "main.wj"]);
    assert!(wall >= Duration::from_millis(2000), "two 1000 ms sleeps took {wall:?} of wall time");
    // the sleeps once each: 2000 ms and some interpreter work, not 4000 ms (real time plus virtual time)
    let e = elapsed_ms(&out);
    assert!((2000..3000).contains(&e), "elapsed() reported {e} ms for two 1000 ms sleeps");
}

#[test]
fn reproducible_runs_keep_virtual_time() {
    let dir = project(&program(5000));
    for (mode, args) in [
        ("--deterministic-time", vec!["run", "--interpret", "--no-lint", "--deterministic-time", "main.wj"]),
        ("--simulate", vec!["run", "--interpret", "--no-lint", "--simulate", "main.wj"]),
        ("--record", vec!["run", "--interpret", "--no-lint", "--record", "run.log", "main.wj"]),
        ("--replay", vec!["run", "--interpret", "--no-lint", "--replay", "run.log", "main.wj"]),
    ] {
        let (wall, out, _) = timed(dir.path(), &args);
        // ten seconds of sleep return at once (generous bound for a loaded machine)
        assert!(wall < Duration::from_secs(6), "{mode}: ten seconds of virtual sleep took {wall:?}");
        assert!(elapsed_ms(&out) >= 10_000, "{mode}: elapsed() must still count the virtual sleep: {out}");
    }
}

#[test]
fn wj_test_keeps_virtual_time() {
    let dir = project("fn main() {\n}\n");
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("tests")).unwrap();
    std::fs::write(dir.path().join("src/lib.wj"), "pub fn ten() -> int {\n    10\n}\n").unwrap();
    std::fs::write(
        dir.path().join("tests/sleep_test.wj"),
        "use std::async\n\nfn test_sleeps_for_ten_seconds() {\n    async::sleep_ms_blocking(10000)\n    assert(true, \"slept\")\n}\n",
    )
    .unwrap();
    let (wall, out, _) = timed(dir.path(), &["test", "--interpret"]);
    assert!(out.contains("1 passed"), "{out}");
    assert!(wall < Duration::from_secs(6), "a test sleeping ten seconds took {wall:?}");
}
