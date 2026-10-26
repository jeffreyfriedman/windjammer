//! ADR-0021 E4 gates for `durable fn` on the interpreter: crash at every step boundary gives the uninterrupted result, the
//! mutation study (non-determinism injected into sample workflows is rejected by `wj check` or journaled), and the plumbing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[derive(Debug, Clone, PartialEq, Eq)]
struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn wj(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Out {
    let mut cmd = Command::new(WJ);
    cmd.current_dir(dir).env_remove("WJ_CAPS").env_remove("WJ_DRIFT").args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let o = cmd.stdin(Stdio::null()).output().unwrap();
    Out { code: o.status.code().unwrap_or(-1), stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into() }
}

/// `wj run --interpret` of `main.wj` in `dir`, journaling to `dir/journal`.
fn run_durable(dir: &Path, crash: Option<&str>, det: bool, allow: &[&str], envs: &[(&str, &str)]) -> Out {
    let mut args = vec!["run", "--interpret", "--no-lint", "--journal", "journal"];
    if det {
        args.push("--deterministic-time");
    }
    if let Some(c) = crash {
        args.extend(["--crash-after", c]);
    }
    for a in allow {
        args.extend(["--allow", a]);
    }
    args.push("main.wj");
    wj(dir, &args, envs)
}

fn scrub(t: &str, dirs: &[&Path]) -> String {
    let mut t = t.to_string();
    for d in dirs {
        let canon = fs::canonicalize(d).unwrap_or(d.to_path_buf());
        for form in [canon.display().to_string(), d.display().to_string()] {
            t = t.replace(&form, "<DIR>");
        }
    }
    t
}

fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    fn walk(base: &Path, cur: &Path, out: &mut BTreeMap<String, String>) {
        for e in fs::read_dir(cur).unwrap().flatten() {
            let p = e.path();
            let rel = p.strip_prefix(base).unwrap().display().to_string();
            if rel == "journal" || rel == "main.wj" {
                continue;
            }
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                out.insert(rel, fs::read_to_string(&p).unwrap_or_default());
            }
        }
    }
    let mut m = BTreeMap::new();
    walk(dir, dir, &mut m);
    m
}

fn steps_in(dir: &Path) -> usize {
    let j = dir.join("journal");
    let Ok(rd) = fs::read_dir(j) else { return 0 };
    rd.flatten().map(|e| fs::read_to_string(e.path()).unwrap_or_default().lines().filter(|l| l.contains("\"step\"")).count()).sum()
}

fn tmp_with(src: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), src).unwrap();
    d
}

fn list(dir: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(dir)).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    v.sort();
    v
}

/// Crash after every step (and, separately, in the window before journaling it), resume with the same journal: the final
/// output and files equal an uninterrupted run. Returns the number of crash points checked.
fn crash_everywhere(name: &str, src: &str, allow: &[&str], mid_too: bool, failures: &mut Vec<String>) -> usize {
    let clean = tmp_with(src);
    let c = run_durable(clean.path(), None, true, allow, &[]);
    if c.code != 0 {
        failures.push(format!("{name}: the uninterrupted run failed: {c:?}"));
        return 0;
    }
    let n = steps_in(clean.path());
    let want_files = snapshot(clean.path());
    let mut checked = 0;
    for k in 1..=n {
        for mid in if mid_too { vec![false, true] } else { vec![false] } {
            let spec = if mid { format!("{k}:mid") } else { k.to_string() };
            let d = tmp_with(src);
            let first = run_durable(d.path(), Some(&spec), true, allow, &[]);
            if first.code != 137 {
                failures.push(format!("{name} crash {spec}: expected exit 137, got {first:?}"));
                continue;
            }
            let second = run_durable(d.path(), None, true, allow, &[]);
            checked += 1;
            if second.code != 0 {
                failures.push(format!("{name} crash {spec}: the resumed run failed: {second:?}"));
                continue;
            }
            if mid {
                // the window between performing a step and journaling it is at-least-once: the step runs again, so the
                // result may legitimately differ for a step that is not idempotent. The run must still complete.
                if !second.stdout.contains("RESULT") && !c.stdout.is_empty() && second.stdout.is_empty() {
                    failures.push(format!("{name} crash {spec}: the resumed run printed nothing: {second:?}"));
                }
                continue;
            }
            let dirs = [clean.path(), d.path()];
            let got = scrub(&format!("{}{}", first.stdout, second.stdout), &dirs);
            let want = scrub(&c.stdout, &dirs);
            if want != got {
                failures.push(format!("{name} crash {spec}: output differs\n  want {want:?}\n  got  {got:?}"));
            }
            let files: BTreeMap<_, _> = snapshot(d.path()).into_iter().map(|(k, v)| (k, scrub(&v, &dirs))).collect();
            let wf: BTreeMap<_, _> = want_files.iter().map(|(k, v)| (k.clone(), scrub(v, &dirs))).collect();
            if files != wf {
                failures.push(format!("{name} crash {spec}: files differ\n  want {wf:?}\n  got  {files:?}"));
            }
        }
    }
    checked
}

#[test]
fn sample_workflows_survive_a_crash_at_every_step_boundary() {
    let flows = list("tests/effects/durable");
    assert_eq!(flows.len(), 10);
    let mut failures = Vec::new();
    let mut points = 0;
    for f in &flows {
        let src = fs::read_to_string(f).unwrap();
        points += crash_everywhere(&f.file_stem().unwrap().to_string_lossy(), &src, &[], true, &mut failures);
    }
    eprintln!("crash points checked on the sample workflows: {points}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(points >= 90, "{points}");
}

/// The replay corpus, each program turned into a durable function: the same property on 40-odd more programs.
#[test]
fn the_replay_corpus_survives_crashes_when_wrapped_in_a_durable_fn() {
    // nondeterministic by nature of the host (process id, password salts) or fed from standard input
    let skip = ["process_pid", "crypto_password_hash", "stdin_read_line", "stdin_struct_read_line"];
    let mut failures = Vec::new();
    let (mut run, mut rejected, mut points) = (0, 0, 0);
    for prog in list("tests/effects/replay") {
        let name = prog.file_stem().unwrap().to_string_lossy().to_string();
        if skip.contains(&name.as_str()) {
            continue;
        }
        let src = fs::read_to_string(&prog).unwrap();
        let allow: Vec<String> = src.lines().take_while(|l| l.starts_with("//")).filter_map(|l| l.trim_start_matches("//").trim().strip_prefix("allow:").map(|v| v.trim().to_string())).collect();
        let allow: Vec<&str> = allow.iter().map(|a| a.as_str()).collect();
        let wrapped = format!("{}\nfn main() {{ wj_body() }}\n", src.replace("fn main()", "durable fn wj_body()"));
        // the check decides first: a program the durable rules reject never runs
        let probe = tmp_with(&wrapped);
        let check = wj(probe.path(), &["check", "main.wj"], &[]);
        let out = format!("{}{}", check.stdout, check.stderr);
        if ["WJ0045", "WJ0046", "WJ0047", "WJ0048"].iter().any(|c| out.contains(c)) {
            rejected += 1;
            continue;
        }
        run += 1;
        points += crash_everywhere(&name, &wrapped, &allow, false, &mut failures);
    }
    eprintln!("replay corpus as durable fns: {run} run, {rejected} rejected by the durable rules, {points} crash points");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(run >= 30, "only {run} programs ran");
}

const MARK: &str = "    let wj_taint = \"\"";

/// Mutations that inject non-determinism into the durable body. `Rejected` ones must be stopped by `wj check`; the rest
/// must be journaled: replaying the finished journal gives the same result although the outside world has changed.
struct Mutation {
    name: &'static str,
    code: &'static str,
    rejected_by: Option<&'static str>,
}

const MUTATIONS: &[Mutation] = &[
    Mutation { name: "clock", code: "    let wj_taint = \"${time::now_millis()}\"", rejected_by: None },
    Mutation { name: "random", code: "    let wj_taint = \"${random::range(0, 1000000)}\"", rejected_by: None },
    Mutation { name: "file", code: "    let wj_taint = fs::read_to_string(\"drift.txt\") ?? \"none\"", rejected_by: None },
    Mutation { name: "env", code: "    let wj_taint = env::var(\"WJ_DRIFT\") ?? \"unset\"", rejected_by: None },
    Mutation {
        name: "hash order",
        code: "    let mut wj_h = HashMap::new()\n    wj_h.insert(\"x\", 1)\n    wj_h.insert(\"y\", 2)\n    let mut wj_taint = \"\"\n    for (k, v) in wj_h {\n        wj_taint = \"${wj_taint}${k}\"\n    }",
        rejected_by: Some("WJ0048"),
    },
    Mutation { name: "channel", code: "    let (wj_tx, wj_rx) = sync::channel()\n    wj_tx.send(1)\n    let wj_taint = \"${wj_rx.recv()}\"", rejected_by: Some("WJ0047") },
];

fn last_result(o: &Out) -> String {
    o.stdout.lines().filter(|l| l.starts_with("RESULT ")).last().unwrap_or("").to_string()
}

#[test]
fn injected_nondeterminism_is_rejected_or_journaled() {
    let flows = list("tests/effects/durable");
    let (mut mutants, mut rejected, mut journaled, mut escaped) = (0, 0, 0, Vec::new());
    for f in &flows {
        let base = fs::read_to_string(f).unwrap();
        assert!(base.contains(MARK), "{} lacks the mutation point", f.display());
        for m in MUTATIONS {
            mutants += 1;
            let src = base.replace(MARK, m.code);
            let d = tmp_with(&src);
            let allow = ["env.read=WJ_DRIFT", "fs.read=./**"];
            fs::write(d.path().join("drift.txt"), "first").unwrap();
            let check = wj(d.path(), &["check", "main.wj"], &[]);
            let out = format!("{}{}", check.stdout, check.stderr);
            if ["WJ0045", "WJ0046", "WJ0047", "WJ0048"].iter().any(|c| out.contains(c)) {
                let want = m.rejected_by.unwrap_or("(none)");
                if out.contains(want) {
                    rejected += 1;
                } else {
                    escaped.push(format!("{} / {}: rejected for the wrong reason:\n{out}", f.display(), m.name));
                }
                continue;
            }
            if m.rejected_by.is_some() {
                escaped.push(format!("{} / {}: should have been rejected by {}", f.display(), m.name, m.rejected_by.unwrap()));
                continue;
            }
            // run 1 on the real clock, then the world changes and the finished journal is replayed
            let first = run_durable(d.path(), None, false, &allow, &[("WJ_DRIFT", "first")]);
            if first.code != 0 {
                escaped.push(format!("{} / {}: first run failed: {first:?}", f.display(), m.name));
                continue;
            }
            fs::write(d.path().join("drift.txt"), "second").unwrap();
            // drop the `done` record so the function body re-runs against the journal
            for e in fs::read_dir(d.path().join("journal")).unwrap().flatten() {
                let text = fs::read_to_string(e.path()).unwrap();
                let kept: Vec<&str> = text.lines().filter(|l| !l.contains("\"done\"")).collect();
                fs::write(e.path(), kept.join("\n") + "\n").unwrap();
            }
            let second = run_durable(d.path(), None, false, &allow, &[("WJ_DRIFT", "second")]);
            let (a, b) = (scrub(&last_result(&first), &[d.path()]), scrub(&last_result(&second), &[d.path()]));
            if a != b || a.is_empty() {
                escaped.push(format!("{} / {}: replay differs: {a:?} vs {b:?}", f.display(), m.name));
                continue;
            }
            // the oracle can fail: without a journal the changed world shows through
            let fresh = tmp_with(&src);
            fs::write(fresh.path().join("drift.txt"), "second").unwrap();
            let unjournaled = run_durable(fresh.path(), None, false, &allow, &[("WJ_DRIFT", "second")]);
            if matches!(m.name, "file" | "env") && last_result(&unjournaled) == last_result(&first) {
                escaped.push(format!("{} / {}: the oracle could not tell a journaled value from a live one", f.display(), m.name));
                continue;
            }
            journaled += 1;
        }
    }
    eprintln!("mutation study: {mutants} mutants, {rejected} rejected by wj check, {journaled} journaled, {} escaped", escaped.len());
    assert!(escaped.is_empty(), "{}", escaped.join("\n"));
    assert_eq!(rejected + journaled, mutants);
}

#[test]
fn a_finished_run_is_not_run_again_and_a_step_runs_once() {
    let src = "use std::fs\nfn bump() -> int {\n    fs::append(\"hits.txt\", \"x\") ?? ()\n    1\n}\ndurable fn run() -> int {\n    bump() + bump()\n}\nfn main() {\n    println(\"${run()}\")\n}\n";
    let d = tmp_with(src);
    assert_eq!(run_durable(d.path(), None, true, &[], &[]).stdout, "2\n");
    assert_eq!(fs::read_to_string(d.path().join("hits.txt")).unwrap(), "xx");
    // the same call again returns the journaled result and performs nothing
    assert_eq!(run_durable(d.path(), None, true, &[], &[]).stdout, "2\n");
    assert_eq!(fs::read_to_string(d.path().join("hits.txt")).unwrap(), "xx");
}

#[test]
fn a_changed_program_cannot_resume_a_journal_that_disagrees() {
    let a = "use std::fs\nfn one() -> int { fs::append(\"o.txt\", \"1\") ?? ()\n 1 }\nfn two() -> int { fs::append(\"o.txt\", \"2\") ?? ()\n 2 }\ndurable fn run() -> int {\n    one() + two()\n}\nfn main() { println(\"${run()}\") }\n";
    let d = tmp_with(a);
    let first = run_durable(d.path(), Some("1"), true, &[], &[]);
    assert_eq!(first.code, 137);
    fs::write(d.path().join("main.wj"), a.replace("one() + two()", "two() + one()")).unwrap();
    let o = run_durable(d.path(), None, true, &[], &[]);
    assert_ne!(o.code, 0, "{o:?}");
    assert!(o.stderr.contains("WJ0049") && o.stderr.contains("diverged"), "{}", o.stderr);
}

#[test]
fn durable_fn_without_failure_behaves_like_a_plain_fn() {
    let body = "fn total(xs: Vec<int>) -> int {\n    let mut s = 0\n    for x in xs { s += x }\n    s\n}\nfn main() {\n    println(\"${total([1, 2, 3])}\")\n}\n";
    let plain = tmp_with(body);
    let durable = tmp_with(&body.replace("fn total", "durable fn total"));
    assert_eq!(wj(plain.path(), &["run", "--interpret", "main.wj"], &[]).stdout, wj(durable.path(), &["run", "--interpret", "main.wj"], &[]).stdout);
}

#[test]
fn compiled_builds_refuse_a_durable_fn_by_name() {
    let d = tmp_with("durable fn job() -> int { 1 }\nfn main() { println(\"${job()}\") }\n");
    let o = wj(d.path(), &["build", "main.wj", "-o", "out"], &[]);
    assert_ne!(o.code, 0, "{o:?}");
    assert!(format!("{}{}", o.stdout, o.stderr).contains("durable fn") && format!("{}{}", o.stdout, o.stderr).contains("interpreter"), "{o:?}");
}

/// Documents the at-least-once window: a crash after performing a step but before journaling it runs the step again.
#[test]
fn a_crash_between_performing_and_journaling_runs_the_step_again() {
    let src = "use std::fs\nfn mark() -> int {\n    fs::append(\"marks.txt\", \"x\") ?? ()\n    1\n}\ndurable fn run() -> int {\n    mark()\n}\nfn main() { println(\"${run()}\") }\n";
    let after = tmp_with(src);
    assert_eq!(run_durable(after.path(), Some("1"), true, &[], &[]).code, 137);
    assert_eq!(run_durable(after.path(), None, true, &[], &[]).stdout, "1\n");
    assert_eq!(fs::read_to_string(after.path().join("marks.txt")).unwrap(), "x", "journaled before the crash: performed once");
    let mid = tmp_with(src);
    assert_eq!(run_durable(mid.path(), Some("1:mid"), true, &[], &[]).code, 137);
    assert_eq!(run_durable(mid.path(), None, true, &[], &[]).stdout, "1\n");
    assert_eq!(fs::read_to_string(mid.path().join("marks.txt")).unwrap(), "xx", "crashed before journaling: performed again");
}
