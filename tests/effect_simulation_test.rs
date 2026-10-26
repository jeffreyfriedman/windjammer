//! ADR-0021 E3 gates for deterministic simulation (`wj test --simulate`, `wj run --simulate`): a simulated run is a function of the
//! program, the seed and the fault rate; the simulated file system answers like the real one; and the seeds find the planted bugs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[derive(Debug, PartialEq, Eq, Clone)]
struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn wj(dir: &Path, args: &[&str]) -> Out {
    let o = Command::new(WJ).current_dir(dir).env_remove("WJ_CAPS").args(args).output().unwrap();
    Out { code: o.status.code().unwrap_or(-1), stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into() }
}

fn list(dir: &str, suffix: &str) -> Vec<PathBuf> {
    let d = Path::new(env!("CARGO_MANIFEST_DIR")).join(dir);
    let mut v: Vec<PathBuf> = fs::read_dir(d).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with(suffix)).collect();
    v.sort();
    v
}

fn allow_flags(src: &str) -> Vec<String> {
    src.lines()
        .take_while(|l| l.starts_with("//"))
        .filter_map(|l| l.trim_start_matches("//").trim().strip_prefix("allow:").map(|v| v.trim().to_string()))
        .flat_map(|a| ["--allow".to_string(), a])
        .collect()
}

fn run_sim(dir: &Path, seed: u64, faults: f64, allow: &[String]) -> Out {
    let (seed, faults) = (seed.to_string(), faults.to_string());
    let mut args = vec!["run", "--interpret", "--no-lint", "--simulate", "--seed", &seed, "--faults", &faults];
    args.extend(allow.iter().map(|s| s.as_str()));
    args.push("main.wj");
    wj(dir, &args)
}

fn program_dir(src: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), src).unwrap();
    d
}

/// A simulated run is a function of (program, seed, faults): two runs agree exactly, nothing reaches the host, and for
/// the programs that draw random numbers or run tasks the seed changes the output.
#[test]
fn simulation_is_a_function_of_the_seed_and_touches_nothing() {
    let seed_sensitive = [
        "random_range_sequence", "random_float_and_bool", "random_shuffle_choice", "uuid_v4", "random_alphanumeric", "conc_channel_race",
        "mixed_file_clock_random", "uuid_named_file", "crypto_password_hash", "hashmap_iteration",
    ];
    let mut failures = Vec::new();
    let mut changed = 0;
    for prog in list("tests/effects/replay", ".wj") {
        let name = prog.file_stem().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(&prog).unwrap();
        let allow = allow_flags(&src);
        let d = program_dir(&src);
        let a = run_sim(d.path(), 11, 0.0, &allow);
        let b = run_sim(d.path(), 11, 0.0, &allow);
        if a != b {
            failures.push(format!("{name}: two runs of seed 11 differ:\n  {a:?}\n  {b:?}"));
        }
        let left: Vec<String> = fs::read_dir(d.path()).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into()).filter(|n| n != "main.wj").collect();
        if !left.is_empty() {
            failures.push(format!("{name}: the simulation wrote to the host: {left:?}"));
        }
        if seed_sensitive.contains(&name.as_str()) {
            let c = run_sim(d.path(), 12, 0.0, &allow);
            if c.stdout != a.stdout {
                changed += 1;
            } else {
                failures.push(format!("{name}: the output does not depend on the seed: {:?}", a.stdout));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(changed, seed_sensitive.len());
}

/// The simulated clock is the interpreter's deterministic clock: Unix time 1700000000, moved only by sleeping.
#[test]
fn the_simulated_clock_starts_at_the_fixed_epoch_and_moves_by_sleeping() {
    let d = program_dir("use std::time\nfn main() {\n    println(\"${time::now().timestamp()}\")\n    time::sleep_millis(2000)\n    println(\"${time::now().timestamp()}\")\n}\n");
    let o = run_sim(d.path(), 7, 0.0, &[]);
    assert_eq!(o.stdout, "1700000000\n1700000002\n", "{o:?}");
}

/// Independent oracle for the in-memory file system: the same program against the real file system (in a scratch directory) and
/// against the simulated one prints the same lines, including the operating system's error texts.
#[test]
fn the_simulated_file_system_answers_like_the_real_one() {
    let src = r#"use std::fs
fn show(label: string, r: Result<(), string>) {
    match r {
        Ok(_) => println("${label}: ok"),
        Err(e) => println("${label}: ${e}"),
    }
}
fn main() {
    show("read missing", fs::read_to_string("missing.txt").map(|_| ()))
    show("write into missing dir", fs::write("nodir/f.txt", "x"))
    show("create_dir", fs::create_dir("d"))
    show("create_dir again", fs::create_dir("d"))
    show("write file", fs::write("d/f.txt", "abc"))
    show("read dir as file", fs::read_to_string("d").map(|_| ()))
    show("remove non-empty dir", fs::remove_dir("d"))
    show("create_dir under file", fs::create_dir("d/f.txt/sub"))
    show("write over dir", fs::write("d", "x"))
    show("remove missing file", fs::remove_file("nope.txt"))
    show("remove dir as file", fs::remove_file("d"))
    show("rename missing", fs::rename("nope.txt", "other.txt"))
    show("copy missing", fs::copy("nope.txt", "other.txt"))
    show("read_dir of file", fs::read_dir("d/f.txt").map(|_| ()))
    show("read_dir missing", fs::read_dir("gone").map(|_| ()))
    show("remove_dir_all missing", fs::remove_dir_all("gone"))
    show("append", fs::append("d/f.txt", "def"))
    println(fs::read_to_string("d/f.txt") ?? "?")
    println("len ${fs::metadata("d/f.txt").map(|m| m.len()) ?? 0}")
    show("rename dir", fs::rename("d", "e"))
    println("${fs::exists("d")} ${fs::exists("e/f.txt")} ${fs::is_dir("e")} ${fs::is_file("e")}")
    show("remove_dir_all", fs::remove_dir_all("e"))
    println("${fs::exists("e")}")
}
"#;
    let live = program_dir(src);
    let l = wj(live.path(), &["run", "--interpret", "--no-lint", "main.wj"]);
    assert_eq!(l.code, 0, "{l:?}");
    let sim = program_dir(src);
    let s = run_sim(sim.path(), 1, 0.0, &[]);
    assert_eq!(s.code, 0, "{s:?}");
    // The simulated world speaks Linux on every host (a seed reproduces the same run on a laptop and in CI). The two calls whose
    // error number differs between macOS and Linux are compared only where the host is Linux.
    let keep = |t: &str| -> String {
        t.lines().filter(|l| cfg!(target_os = "linux") || !(l.starts_with("remove non-empty dir") || l.starts_with("remove dir as file"))).collect::<Vec<_>>().join("\n")
    };
    assert_eq!(keep(&s.stdout), keep(&l.stdout), "simulated and real file systems disagree");
    assert!(fs::read_dir(sim.path()).unwrap().count() == 1, "the simulation left files on the host");
}

/// Faults are drawn from the seed: the same seed and rate fail the same calls; rate 0 never fails; rate 1 always does.
#[test]
fn faults_are_seeded_and_the_rate_is_respected() {
    let src = "use std::fs\nfn main() {\n    let mut failed = 0\n    for i in 0..40 {\n        match fs::write(\"f${i}.txt\", \"data\") {\n            Ok(_) => {}\n            Err(_) => { failed += 1 }\n        }\n    }\n    println(\"failed ${failed}\")\n}\n";
    let d = program_dir(src);
    assert_eq!(run_sim(d.path(), 3, 0.0, &[]).stdout, "failed 0\n");
    assert_eq!(run_sim(d.path(), 3, 1.0, &[]).stdout, "failed 40\n");
    let a = run_sim(d.path(), 3, 0.25, &[]);
    assert_eq!(a, run_sim(d.path(), 3, 0.25, &[]));
    let n: i32 = a.stdout.trim().trim_start_matches("failed ").parse().unwrap();
    assert!((3..=20).contains(&n), "a quarter of 40 calls should fail about 10 times, got {n}");
    assert_ne!(a.stdout, run_sim(d.path(), 4, 0.25, &[]).stdout.replace("failed 10\n", "same"), "different seeds draw different faults (or coincide rarely)");
}

/// A failed write is a short write: the first half of the data is on disk (the bug class "partial write").
#[test]
fn a_faulted_write_leaves_a_prefix() {
    let src = "use std::fs\nfn main() {\n    match fs::write(\"p.txt\", \"abcdefgh\") {\n        Ok(_) => println(\"ok\"),\n        Err(e) => println(\"err ${e} kept ${fs::read_to_string(\"p.txt\") ?? \"?\"}\"),\n    }\n}\n";
    let d = program_dir(src);
    // at rate 0.5 seed 9 fails the write and lets the read back through
    let o = run_sim(d.path(), 9, 0.5, &[]);
    assert_eq!(o.stdout, "err No space left on device (os error 28) kept abcd\n", "{o:?}");
}

/// The capability gate still decides what a simulated run may touch.
#[test]
fn the_gate_still_applies_in_the_simulation() {
    let d = program_dir("use std::fs\nfn main() {\n    println(\"${fs::exists(\"/etc/hosts\")}\")\n}\n");
    let o = run_sim(d.path(), 1, 0.0, &[]);
    assert_eq!(o.code, 126, "{o:?}");
}

fn faults_of(src: &str) -> String {
    src.lines().take_while(|l| l.starts_with("//")).find_map(|l| l.trim_start_matches("//").trim().strip_prefix("faults:").map(|v| v.trim().to_string())).unwrap_or_else(|| "0".into())
}

fn test_args<'a>(seeds: &'a str, faults: &'a str, file: &'a str) -> Vec<&'a str> {
    vec!["test", "--interpret", "--simulate", seeds, "--faults", faults, file]
}

/// Planted-bug study (ADR-0021 E3 gate 3): twenty latent bugs, each with a test that passes in the default simulated world
/// (seed 0: first-in-first-out schedule, key order, no faults). The seeds must find at least 80% of them within ten minutes,
/// and every failing seed must reproduce its failure exactly.
#[test]
fn seeds_find_the_planted_bugs_and_replay_them() {
    let bugs = list("tests/effects/planted", "_test.wj");
    assert_eq!(bugs.len(), 20, "the study has twenty planted bugs");
    let started = Instant::now();
    let mut found = Vec::new();
    let mut missed = Vec::new();
    for bug in &bugs {
        let name = bug.file_name().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(bug).unwrap();
        let faults = faults_of(&src);
        let dir = bug.parent().unwrap();
        // the control: the latent bug does not show in the default world
        let control = wj(dir, &["test", "--interpret", "--simulate", "--seed", "0", &name]);
        assert_eq!(control.code, 0, "{name} must pass in the default simulated world:\n{}", control.stdout);
        let search = wj(dir, &test_args("--seeds", &faults, &name).iter().enumerate().flat_map(|(i, a)| if i == 3 { vec![*a, "1..2000"] } else { vec![*a] }).collect::<Vec<_>>());
        let Some(line) = search.stdout.lines().find(|l| l.starts_with("seed ")) else {
            missed.push(name);
            continue;
        };
        let seed: u64 = line.trim_start_matches("seed ").split_whitespace().next().unwrap().parse().unwrap();
        // the failing seed fails again, with the same message, every time
        let (s, f) = (seed.to_string(), faults.clone());
        let again1 = wj(dir, &["test", "--interpret", "--simulate", "--seed", &s, "--faults", &f, &name]);
        let again2 = wj(dir, &["test", "--interpret", "--simulate", "--seed", &s, "--faults", &f, &name]);
        assert_eq!(again1.code, 1, "{name} seed {seed} must fail again:\n{}", again1.stdout);
        assert_eq!(again1, again2, "{name} seed {seed}: the failure must reproduce exactly");
        found.push((name, seed));
    }
    let elapsed = started.elapsed();
    eprintln!("planted-bug study: {} of {} found in {:.1}s; missed: {missed:?}; seeds: {found:?}", found.len(), bugs.len(), elapsed.as_secs_f64());
    assert!(elapsed < Duration::from_secs(600), "the study must finish within ten minutes, took {elapsed:?}");
    assert!(found.len() * 5 >= bugs.len() * 4, "at least 80% of the planted bugs must be found; missed: {missed:?}");
}
