//! ADR-0021 E3 gates for record/replay: a recorded run replays bit-exact (stdout, stderr, exit status) in another
//! directory, with other environment variables and no standard input, performing no live effect; a request that
//! differs from the log is `WJ0049`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Header comments of a corpus program: `// allow: EFFECT=RES`, `// env: K=V`, `// args: a b`, `// stdin: text\n`.
#[derive(Default)]
struct Setup {
    allow: Vec<String>,
    env: Vec<(String, String)>,
    args: Vec<String>,
    stdin: String,
}

fn setup_of(src: &str) -> Setup {
    let mut s = Setup::default();
    for line in src.lines().take_while(|l| l.starts_with("//")) {
        let body = line.trim_start_matches("//").trim_start();
        if let Some(v) = body.strip_prefix("allow:") {
            s.allow.push(v.trim().to_string());
        } else if let Some(v) = body.strip_prefix("env:") {
            if let Some((k, v)) = v.trim().split_once('=') {
                s.env.push((k.to_string(), v.to_string()));
            }
        } else if let Some(v) = body.strip_prefix("args:") {
            s.args = v.split_whitespace().map(String::from).collect();
        } else if let Some(v) = body.strip_prefix("stdin:") {
            s.stdin = v.trim_start().replace("\\n", "\n");
        }
    }
    s
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Directory names differ between the recording and the replay; everything else must be identical.
fn normalized(o: Outcome, dirs: &[&Path]) -> Outcome {
    let scrub = |mut t: String| {
        for d in dirs {
            let canon = fs::canonicalize(d).unwrap_or(d.to_path_buf());
            for form in [canon.display().to_string(), d.display().to_string()] {
                t = t.replace(&form, "<DIR>");
            }
        }
        t
    };
    Outcome { code: o.code, stdout: scrub(o.stdout), stderr: scrub(o.stderr) }
}

fn run(dir: &Path, mode: &[String], setup: &Setup, with_env: bool, with_stdin: bool, seed: u64) -> Outcome {
    let mut cmd = Command::new(WJ);
    cmd.current_dir(dir).env_remove("WJ_CAPS").env_remove("WJ_REPLAY_VAR");
    cmd.args(["run", "--interpret", "--no-lint"]);
    cmd.args(mode);
    if mode[0] == "--record" {
        cmd.args(["--sched-seed", &seed.to_string()]);
    }
    for a in &setup.allow {
        cmd.args(["--allow", a]);
    }
    cmd.arg("main.wj").args(&setup.args);
    if with_env {
        for (k, v) in &setup.env {
            cmd.env(k, v);
        }
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    if with_stdin {
        child.stdin.take().unwrap().write_all(setup.stdin.as_bytes()).unwrap();
    } else {
        drop(child.stdin.take());
    }
    let o = child.wait_with_output().unwrap();
    Outcome { code: o.status.code().unwrap_or(-1), stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into() }
}

fn corpus() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/effects/replay");
    let mut v: Vec<PathBuf> = fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    v.sort();
    v
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into()).collect();
    v.sort();
    v
}

/// Gate 1: bit-exact replay on every corpus program, recorded under two scheduler seeds, replayed elsewhere.
#[test]
fn recorded_runs_replay_bit_exact() {
    let programs = corpus();
    assert!(programs.len() >= 40, "the replay corpus should have at least 40 programs, found {}", programs.len());
    let mut failures = Vec::new();
    let mut runs = 0;
    for prog in &programs {
        let src = fs::read_to_string(prog).unwrap();
        let setup = setup_of(&src);
        for seed in [0u64, 7] {
            let rec = tempfile::tempdir().unwrap();
            fs::write(rec.path().join("main.wj"), &src).unwrap();
            let recorded = run(rec.path(), &["--record".into(), "run.wjlog".into()], &setup, true, true, seed);
            if recorded.code != 0 && !prog.to_string_lossy().contains("denied") {
                failures.push(format!("{} (seed {seed}): the recording itself failed: {recorded:?}", prog.display()));
                continue;
            }
            let rep = tempfile::tempdir().unwrap();
            fs::write(rep.path().join("main.wj"), &src).unwrap();
            fs::copy(rec.path().join("run.wjlog"), rep.path().join("run.wjlog")).unwrap();
            let blobs = rec.path().join("run.wjlog.blobs");
            if blobs.exists() {
                fs::create_dir_all(rep.path().join("run.wjlog.blobs")).unwrap();
                for b in fs::read_dir(&blobs).unwrap() {
                    let b = b.unwrap();
                    fs::copy(b.path(), rep.path().join("run.wjlog.blobs").join(b.file_name())).unwrap();
                }
            }
            let replayed = run(rep.path(), &["--replay".into(), "run.wjlog".into()], &setup, false, false, seed);
            runs += 1;
            // the recorded run printed its own directory; the replay must print the recorded one, so compare as recorded
            let (recorded, replayed) = (normalized(recorded, &[rec.path(), rep.path()]), normalized(replayed, &[rec.path(), rep.path()]));
            if recorded != replayed {
                failures.push(format!("{} (seed {seed}):\n  recorded: {recorded:?}\n  replayed: {replayed:?}", prog.display()));
            }
            let left = entries(rep.path());
            if left.iter().any(|n| n != "main.wj" && !n.starts_with("run.wjlog")) {
                failures.push(format!("{} (seed {seed}): replay performed a live effect, directory has {left:?}", prog.display()));
            }
        }
    }
    assert!(failures.is_empty(), "{} of {runs} replays differ:\n{}", failures.len(), failures.join("\n"));
}

/// The log is the whole story of the effects: it names the program, the seeds and every effect in order.
#[test]
fn the_log_has_a_header_and_one_event_per_effect() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), "use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n    println(fs::read_to_string(\"a.txt\").unwrap())\n}\n").unwrap();
    let o = run(d.path(), &["--record".into(), "r.wjlog".into()], &Setup::default(), true, true, 3);
    assert_eq!(o.code, 0, "{o:?}");
    let text = fs::read_to_string(d.path().join("r.wjlog")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    let head: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(head["wj_effect_log"], 1);
    assert_eq!(head["sched_seed"], 3);
    assert_eq!(head["compiler"], env!("CARGO_PKG_VERSION"));
    let ops: Vec<String> = lines[1..].iter().map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap()["op"].as_str().unwrap().to_string()).collect();
    assert_eq!(ops, ["fs::write", "fs::read_to_string"]);
    // canonical: keys sorted, so re-serializing sorted keys is the identity
    for l in lines {
        let v: serde_json::Value = serde_json::from_str(l).unwrap();
        let keys: Vec<&String> = v.as_object().unwrap().keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "{l}");
    }
}

fn record_prog(src: &str) -> (tempfile::TempDir, Outcome) {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), src).unwrap();
    let o = run(d.path(), &["--record".into(), "r.wjlog".into()], &Setup::default(), true, true, 0);
    assert_eq!(o.code, 0, "{o:?}");
    (d, o)
}

/// SPEC-EFF-031: a request that differs from the logged one at the same position is WJ0049 with both requests.
#[test]
fn a_different_request_is_a_divergence() {
    let (d, _) = record_prog("use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n    println(fs::read_to_string(\"a.txt\").unwrap())\n}\n");
    fs::write(d.path().join("main.wj"), "use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n    println(fs::read_to_string(\"b.txt\").unwrap())\n}\n").unwrap();
    let o = run(d.path(), &["--replay".into(), "r.wjlog".into()], &Setup::default(), false, false, 0);
    assert_eq!(o.code, 1, "{o:?}");
    assert!(o.stderr.contains("WJ0049"), "{}", o.stderr);
    assert!(o.stderr.contains("fs::read_to_string([{\"s\"") || o.stderr.contains("fs::read_to_string(\"a.txt\")"), "{}", o.stderr);
    assert!(o.stderr.contains("a.txt") && o.stderr.contains("b.txt"), "both requests are shown: {}", o.stderr);
    assert!(o.stderr.contains("main.wj:"), "the source location is shown: {}", o.stderr);
    assert!(!d.path().join("b.txt").exists());
}

/// Asking for fewer or more effects than the log has is a divergence too.
#[test]
fn leftover_and_missing_events_are_divergences() {
    let (d, _) = record_prog("use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n    println(\"{}\", fs::exists(\"a.txt\"))\n}\n");
    fs::write(d.path().join("main.wj"), "use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n}\n").unwrap();
    let fewer = run(d.path(), &["--replay".into(), "r.wjlog".into()], &Setup::default(), false, false, 0);
    assert_eq!(fewer.code, 1, "{fewer:?}");
    assert!(fewer.stderr.contains("WJ0049"), "{}", fewer.stderr);
    fs::write(d.path().join("main.wj"), "use std::fs\nfn main() {\n    fs::write(\"a.txt\", \"hi\").unwrap()\n    println(\"{}\", fs::exists(\"a.txt\"))\n    println(\"{}\", fs::exists(\"b.txt\"))\n}\n").unwrap();
    let more = run(d.path(), &["--replay".into(), "r.wjlog".into()], &Setup::default(), false, false, 0);
    assert_eq!(more.code, 1, "{more:?}");
    assert!(more.stderr.contains("WJ0049") && more.stderr.contains("no more events"), "{}", more.stderr);
}

/// A sink that returns a live handle cannot be recorded; the error says which and why.
#[test]
fn recording_a_handle_returning_effect_is_refused_by_name() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), "use std::subprocess\nfn main() {\n    let h = subprocess::spawn(\"echo\", [\"hi\"])\n    println(\"spawned\")\n}\n").unwrap();
    let setup = Setup { allow: vec!["process=echo".into()], ..Setup::default() };
    let o = run(d.path(), &["--record".into(), "r.wjlog".into()], &setup, true, true, 0);
    assert_ne!(o.code, 0, "{o:?}");
    assert!(o.stderr.contains("subprocess::spawn") && o.stderr.contains("--record"), "{}", o.stderr);
}

/// The capability gate still runs in every mode: a replay under a narrower grant is denied like a live run.
#[test]
fn replay_keeps_the_capability_gate() {
    let (d, _) = record_prog("use std::fs\nfn main() {\n    println(\"{}\", fs::exists(\"a.txt\"))\n}\n");
    fs::write(d.path().join("wj.toml"), "[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[capabilities]\n").unwrap();
    let o = run(d.path(), &["--replay".into(), "r.wjlog".into()], &Setup::default(), false, false, 0);
    assert_eq!(o.code, 126, "{o:?}");
}

/// Host observations come from the log, not from the machine: the replay prints the recorded directory, not its own.
#[test]
fn replay_answers_from_the_log_not_the_machine() {
    let src = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/effects/replay/env_current_dir.wj")).unwrap();
    let rec = tempfile::tempdir().unwrap();
    let rep = tempfile::tempdir().unwrap();
    for d in [&rec, &rep] {
        fs::write(d.path().join("main.wj"), &src).unwrap();
    }
    let o = run(rec.path(), &["--record".into(), "r.wjlog".into()], &Setup::default(), true, true, 0);
    assert_eq!(o.code, 0, "{o:?}");
    fs::copy(rec.path().join("r.wjlog"), rep.path().join("r.wjlog")).unwrap();
    let r = run(rep.path(), &["--replay".into(), "r.wjlog".into()], &Setup::default(), false, false, 0);
    let recorded_dir = fs::canonicalize(rec.path()).unwrap();
    assert!(r.stdout.contains(recorded_dir.to_str().unwrap()), "{r:?}");
    assert!(!r.stdout.contains(fs::canonicalize(rep.path()).unwrap().to_str().unwrap()), "{r:?}");
}
