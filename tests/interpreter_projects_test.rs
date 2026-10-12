//! Multi-file project parity: every fixture under `tests/interpreter_projects/` runs through the real
//! `wj` binary, interpreted and compiled, and both must match the fixture's hand-written expectation.
//!
//! Fixture layout: a project (`wj.toml` + `src/`), or `app/` + sibling packages for dependency fixtures.
//! - `expect.stdout`            exact stdout
//! - `expect.exit`              exit status (`fail` = any non-zero); default 0
//! - `expect.stderr_contains`   substring that stderr must contain (both paths)
//! - `expect.bin.<NAME>.stdout` stdout of `wj run --bin NAME`
//! - `args`                     program arguments, one per line
//!
//! The compiled half needs cargo and takes minutes; it runs when `WJ_PROJECT_COMPILED=1` (CI sets it).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interpreter_projects");
    let mut v: Vec<PathBuf> = std::fs::read_dir(root).unwrap().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    v.sort();
    v
}

fn run_dir(fixture: &Path) -> PathBuf {
    let app = fixture.join("app");
    if app.is_dir() { app } else { fixture.to_path_buf() }
}

fn read(p: &Path) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

fn args(fixture: &Path) -> Vec<String> {
    read(&fixture.join("args")).map(|s| s.lines().map(str::to_string).collect()).unwrap_or_default()
}

/// `wj run FLAGS [. ARGS]`: with no program arguments the project is found from the working directory
/// (`wj.toml` upward); with arguments the path `.` comes first, as with `go run . ARGS`.
fn wj(dir: &Path, flags: &[&str], args: &[String]) -> Output {
    let dot: &[&str] = if args.is_empty() { &[] } else { &["."] };
    Command::new(WJ)
        .arg("run")
        .args(flags)
        .args(dot)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

fn check(fixture: &Path, o: &Output, mode: &str) {
    let name = fixture.file_name().unwrap().to_string_lossy();
    let out = String::from_utf8_lossy(&o.stdout);
    let err = String::from_utf8_lossy(&o.stderr);
    let want = read(&fixture.join("expect.stdout")).unwrap();
    assert_eq!(out, want, "[{mode}] {name}: stdout\nstderr:\n{err}");
    let code = o.status.code().unwrap_or(-1);
    match read(&fixture.join("expect.exit")).map(|s| s.trim().to_string()).as_deref() {
        Some("fail") => assert_ne!(code, 0, "[{mode}] {name}: must fail\nstderr:\n{err}"),
        Some(n) => assert_eq!(code, n.parse::<i32>().unwrap(), "[{mode}] {name}: exit\nstderr:\n{err}"),
        None => assert_eq!(code, 0, "[{mode}] {name}: exit\nstderr:\n{err}"),
    }
    if let Some(needle) = read(&fixture.join("expect.stderr_contains")) {
        assert!(err.contains(needle.trim()), "[{mode}] {name}: stderr lacks {:?}:\n{err}", needle.trim());
    }
}

fn bin_expectations(fixture: &Path) -> Vec<(String, String)> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(fixture).unwrap().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if let Some(bin) = n.strip_prefix("expect.bin.").and_then(|r| r.strip_suffix(".stdout")) {
            v.push((bin.to_string(), read(&e.path()).unwrap()));
        }
    }
    v
}

#[test]
fn fixtures_exist() {
    assert!(fixtures().len() >= 6, "project fixtures missing");
}

#[test]
fn interpreted_projects_match_expectations() {
    for f in fixtures() {
        let dir = run_dir(&f);
        // no path: discovered from the working directory (wj.toml upward), like `go run .`
        check(&f, &wj(&dir, &["--interpret"], &args(&f)), "interpret");
        let auto = wj(&dir, &[], &args(&f));
        check(&f, &auto, "default");
        assert!(
            !String::from_utf8_lossy(&auto.stderr).contains("running compiled"),
            "{}: default `wj run` must not fall back for interpretable programs",
            f.display()
        );
        for (bin, want) in bin_expectations(&f) {
            let o = wj(&dir, &["--interpret", "--bin", &bin], &[]);
            assert_eq!(String::from_utf8_lossy(&o.stdout), want, "--bin {bin} in {}", f.display());
        }
    }
}

#[test]
fn compiled_projects_match_expectations() {
    if std::env::var("WJ_PROJECT_COMPILED").is_err() {
        eprintln!("skipped: set WJ_PROJECT_COMPILED=1 to build every fixture with cargo");
        return;
    }
    for f in fixtures() {
        let dir = run_dir(&f);
        check(&f, &wj(&dir, &["--compiled"], &args(&f)), "compiled");
        for (bin, want) in bin_expectations(&f) {
            let o = wj(&dir, &["--compiled", "--bin", &bin], &[]);
            assert_eq!(String::from_utf8_lossy(&o.stdout), want, "compiled --bin {bin} in {}", f.display());
        }
    }
}

#[path = "common/project_fuzz.rs"]
mod project_fuzz;

fn fuzz_seeds() -> std::ops::Range<u64> {
    let n = std::env::var("WJ_FUZZ_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    0..n
}

/// Interpreter vs the generator's own model, for many random module trees.
#[test]
fn fuzz_interpreted_module_trees_match_model() {
    for seed in fuzz_seeds() {
        let p = project_fuzz::generate(seed);
        let d = tempfile::tempdir().unwrap();
        project_fuzz::write(&p, d.path());
        let o = wj(d.path(), &["--interpret"], &[]);
        let out = String::from_utf8_lossy(&o.stdout);
        if out != p.expected || !o.status.success() {
            let keep = std::env::temp_dir().join(format!("wj_fuzz_seed_{seed}"));
            let _ = std::fs::remove_dir_all(&keep);
            project_fuzz::write(&p, &keep);
            panic!(
                "seed {seed} (kept at {}): interpreter differs from model\n--- want\n{}--- got\n{}--- stderr\n{}",
                keep.display(),
                p.expected,
                out,
                String::from_utf8_lossy(&o.stderr)
            );
        }
    }
}

/// The generator can fail: a wrong model value or a broken import is caught (oracle self-test).
#[test]
fn fuzz_oracle_detects_a_mutated_program() {
    let mut p = project_fuzz::generate(3);
    let (k, v) = p.files.iter().find(|(k, _)| k.as_str() != "src/main.wj" && k.ends_with(".wj")).map(|(k, v)| (k.clone(), v.clone())).unwrap();
    p.files.insert(k, v.replacen("let mut acc = y\n", "let mut acc = y + 1\n", 1));
    let d = tempfile::tempdir().unwrap();
    project_fuzz::write(&p, d.path());
    let o = wj(d.path(), &["--interpret"], &[]);
    assert_ne!(String::from_utf8_lossy(&o.stdout), p.expected, "mutant must change the output");
}

/// Compiled vs interpreted for the first seeds (cargo; gated like the compiled fixtures).
#[test]
fn fuzz_compiled_module_trees_match_interpreter() {
    if std::env::var("WJ_PROJECT_COMPILED").is_err() {
        eprintln!("skipped: set WJ_PROJECT_COMPILED=1");
        return;
    }
    let n = std::env::var("WJ_FUZZ_COMPILED_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(5);
    for seed in 0..n {
        let p = project_fuzz::generate(seed);
        let d = tempfile::tempdir().unwrap();
        project_fuzz::write(&p, d.path());
        let o = wj(d.path(), &["--compiled"], &[]);
        assert_eq!(
            String::from_utf8_lossy(&o.stdout),
            p.expected,
            "seed {seed}: compiled differs\n{}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
}

/// `wj run --hot` (ADR-0021 spike): editing a function body changes the running program's behaviour within
/// a few loop iterations while its state (the counter) survives; the process is never restarted.
#[test]
fn hot_reload_swaps_a_body_and_keeps_state() {
    use std::io::{BufRead, BufReader};
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("wj.toml"), "[package]\nname = \"hot\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::create_dir_all(d.path().join("src")).unwrap();
    std::fs::write(d.path().join("src/step.wj"), "pub fn step(n: int) -> int {\n    n + 1\n}\n").unwrap();
    std::fs::write(
        d.path().join("src/main.wj"),
        "use std::time\nuse step\n\nfn main() {\n    let mut state = 0\n    let mut i = 0\n    while i < 3000 {\n        state = step::step(state)\n        println(\"${state}\")\n        time::sleep_millis(2)\n        i += 1\n    }\n}\n",
    )
    .unwrap();
    let mut child = Command::new(WJ).args(["run", "--hot"]).current_dir(d.path()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let mut before = 0i64;
    for _ in 0..20 {
        before = lines.next().unwrap().unwrap().trim().parse().unwrap();
    }
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(d.path().join("src/step.wj"), "pub fn step(n: int) -> int {\n    n + 1000\n}\n").unwrap();
    let mut prev = before;
    let mut jumped = false;
    for _ in 0..1500 {
        let Some(Ok(l)) = lines.next() else { break };
        let v: i64 = l.trim().parse().unwrap();
        assert!(v > prev, "state must keep growing (no restart): {prev} then {v}");
        if v - prev == 1000 {
            jumped = true;
            break;
        }
        prev = v;
    }
    let _ = child.kill();
    assert!(jumped, "the edited body never ran (last value {prev}, before edit {before})");
    assert!(prev >= before, "state was reset");
}
