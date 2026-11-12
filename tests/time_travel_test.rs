//! ADR-0035 gates for time-travel debugging (`wj replay`): breakpoints and values on hand-traced programs, a recorded file
//! read replayed where the file does not exist, a seeded simulated failure inspected with two commands, replay parity with
//! the recording over the replay corpus, and back-then-forward equality at every step.
//!
//! Goldens: `tests/time_travel/*.golden.jsonl`; rewrite them with `WJ_BLESS_TIME_TRAVEL=1` and review the diff by hand.

use serde_json::Value as Json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/time_travel")
}

fn program_dir(name: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::copy(fixtures().join(name), d.path().join("main.wj")).unwrap();
    d
}

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn wj(dir: &Path, args: &[&str], stdin: &str) -> Out {
    use std::io::Write;
    let mut child = Command::new(WJ)
        .current_dir(dir)
        .env_remove("WJ_CAPS")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let o = child.wait_with_output().unwrap();
    Out { code: o.status.code().unwrap_or(-1), stdout: String::from_utf8_lossy(&o.stdout).into(), stderr: String::from_utf8_lossy(&o.stderr).into() }
}

/// A `wj replay --json` session; `source` is `["LOG"]` or `["--simulate", "--seed", "N"]`.
fn session(dir: &Path, source: &[&str], script: &str) -> Vec<Json> {
    fs::write(dir.join("cmds.txt"), script).unwrap();
    let mut args = vec!["replay", "--json", "--script", "cmds.txt"];
    args.extend_from_slice(source);
    args.push("main.wj");
    let o = wj(dir, &args, "");
    assert_eq!(o.code, 0, "wj {args:?} failed: {}{}", o.stdout, o.stderr);
    o.stdout.lines().map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("not JSON ({e}): {l}"))).collect()
}

fn of_kind<'a>(events: &'a [Json], kind: &str) -> Vec<&'a Json> {
    events.iter().filter(|e| e["event"] == kind).collect()
}

fn values(events: &[Json]) -> Vec<String> {
    of_kind(events, "value").iter().map(|e| e["value"].as_str().unwrap().to_string()).collect()
}

fn stdout_of(events: &[Json]) -> String {
    events.iter().filter(|e| e["event"] == "output" && e["stream"] == "stdout").map(|e| e["text"].as_str().unwrap()).collect()
}

fn check_golden(name: &str, events: &[Json], header: &str) {
    let text = format!("{header}{}\n", events.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n"));
    let path = fixtures().join(name);
    if std::env::var("WJ_BLESS_TIME_TRAVEL").is_ok() {
        fs::write(&path, &text).unwrap();
        return;
    }
    let want = fs::read_to_string(&path).unwrap_or_else(|_| panic!("missing golden {} (WJ_BLESS_TIME_TRAVEL=1 writes it)", path.display()));
    assert_eq!(text, want, "golden {name} differs (WJ_BLESS_TIME_TRAVEL=1 rewrites it; review the diff)");
}

/// Hand trace of squares.wj: `square` runs for x = 1, 2, 3, so `y` is 1, 4, 9 at its tail (line 3), called from line 10.
#[test]
fn breakpoints_stop_where_the_hand_trace_says() {
    let d = program_dir("squares.wj");
    let ev = session(d.path(), &["--simulate", "--seed", "1"], "break 3\ncontinue\nprint y\nwhere\ncontinue\nprint y\ncontinue\nprint y\nprint x\ncontinue\n");
    assert_eq!(values(&ev), ["1", "4", "9", "3"]);
    let stops = of_kind(&ev, "stopped");
    assert_eq!(stops[0]["reason"], "start");
    for s in &stops[1..] {
        assert_eq!((s["reason"].as_str(), s["line"].as_u64(), s["function"].as_str()), (Some("breakpoint"), Some(3), Some("square")));
    }
    let frames = of_kind(&ev, "stack")[0]["frames"].as_array().unwrap().clone();
    let at: Vec<(String, u64)> = frames.iter().map(|f| (f["function"].as_str().unwrap().to_string(), f["line"].as_u64().unwrap())).collect();
    assert_eq!(at, [("square".to_string(), 3), ("main".to_string(), 10)]);
    assert_eq!(stdout_of(&ev), "acc 14\n");
    let exit = of_kind(&ev, "exited");
    assert_eq!(exit.len(), 1);
    assert_eq!(exit[0]["code"], 0);
    check_golden("squares.golden.jsonl", &ev, "");
}

fn record_sum_file(dir: &Path) -> Out {
    fs::write(dir.join("numbers.txt"), "10\n20\nx\n").unwrap();
    wj(dir, &["run", "--interpret", "--no-lint", "--record", "run.wjlog", "main.wj"], "")
}

/// A recorded file read, replayed in another directory where the file does not exist: the debugger stops at the failing
/// parse and reads the bindings there (hand-derived: the third line is "x", the sum so far is 10 + 20).
#[test]
fn recorded_file_read_replays_without_the_file() {
    let rec = program_dir("sum_file.wj");
    let r = record_sum_file(rec.path());
    assert_eq!(r.code, 101, "the recording should fail on the bad line: {}{}", r.stdout, r.stderr);
    let rep = program_dir("sum_file.wj");
    fs::copy(rec.path().join("run.wjlog"), rep.path().join("run.wjlog")).unwrap();
    assert!(!rep.path().join("numbers.txt").exists());
    let ev = session(rep.path(), &["run.wjlog"], "continue\nprint line\nprint sum\nwhere\ncontinue\n");
    let stops = of_kind(&ev, "stopped");
    let fail = stops.last().unwrap();
    assert_eq!((fail["reason"].as_str(), fail["function"].as_str(), fail["line"].as_u64()), (Some("failure"), Some("total"), Some(6)));
    assert_eq!(fail["effect"], 1, "one journal event (the file read) answered before the failure");
    assert_eq!(values(&ev), ["\"x\"", "30"]);
    assert_eq!(stdout_of(&ev), "read 8 bytes\n"); // "10\n20\nx\n"
    assert_eq!(of_kind(&ev, "exited")[0]["code"], 101);
    assert!(!rep.path().join("numbers.txt").exists(), "replay must not create the file");
    check_golden("sum_file.golden.jsonl", &ev, "");
}

/// The first seed in 1..=60 whose simulated run of dice.wj panics, and that run's stdout.
fn failing_dice_seed(dir: &Path) -> (u64, String) {
    for seed in 1..=60u64 {
        let o = wj(dir, &["run", "--interpret", "--no-lint", "--simulate", "--seed", &seed.to_string(), "main.wj"], "");
        if o.code == 101 {
            return (seed, o.stdout);
        }
    }
    panic!("no seed in 1..=60 rolls a six in five rolls");
}

/// ADR-0035 acceptance 1: a seeded failing run, and the value of a binding at the failure with exactly two commands. The
/// oracle is the failing run's own output: `total` is the sum of the rolls it printed.
#[test]
fn seeded_failure_is_read_with_two_commands() {
    let d = program_dir("dice.wj");
    let (seed, out) = failing_dice_seed(d.path());
    let rolls: Vec<i64> = out.lines().map(|l| l.rsplit(' ').next().unwrap().parse().unwrap()).collect();
    assert_eq!(*rolls.last().unwrap(), 6);
    let seed_s = seed.to_string();
    let ev = session(d.path(), &["--simulate", "--seed", &seed_s], "continue\nprint total\n");
    let fail = of_kind(&ev, "stopped").last().cloned().unwrap();
    assert_eq!(fail["reason"], "failure");
    assert_eq!(fail["message"], "rolled a six");
    assert_eq!(values(&ev), [rolls.iter().sum::<i64>().to_string()]);
    assert_eq!(stdout_of(&ev), out, "the session reports the output the failing run printed");
    check_golden("dice.golden.jsonl", &ev, &format!("# seed {seed}\n"));
}

/// Corpus header comments (as in effect_replay_test): `// allow:`, `// env:`, `// args:`, `// stdin:`.
fn header(src: &str, key: &str) -> Vec<String> {
    src.lines().take_while(|l| l.starts_with("//")).filter_map(|l| l.trim_start_matches("//").trim_start().strip_prefix(key).map(|v| v.trim().to_string())).collect()
}

/// ADR-0035 acceptance 2: a session that only says `continue` reports exactly the recording's stdout, stderr and exit
/// status, for every replay-corpus program and for the failing file read.
#[test]
fn replay_session_output_equals_the_recording() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/effects/replay");
    let mut programs: Vec<PathBuf> = fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "wj")).collect();
    programs.sort();
    programs.push(fixtures().join("sum_file.wj"));
    let mut failures = Vec::new();
    for prog in &programs {
        let src = fs::read_to_string(prog).unwrap();
        if !header(&src, "args:").is_empty() || !header(&src, "env:").is_empty() {
            continue; // `wj replay` takes the arguments from the log; the environment is answered from it too, but the
                      // recording needs them set, which this loop does not do (effect_replay_test covers those)
        }
        let rec = tempfile::tempdir().unwrap();
        fs::write(rec.path().join("main.wj"), &src).unwrap();
        if prog.ends_with("sum_file.wj") {
            fs::write(rec.path().join("numbers.txt"), "10\n20\nx\n").unwrap();
        }
        let allow: Vec<String> = header(&src, "allow:").into_iter().flat_map(|a| ["--allow".to_string(), a]).collect();
        let stdin = header(&src, "stdin:").first().map(|s| s.replace("\\n", "\n")).unwrap_or_default();
        let mut args: Vec<&str> = vec!["run", "--interpret", "--no-lint", "--record", "run.wjlog"];
        args.extend(allow.iter().map(String::as_str));
        args.push("main.wj");
        let recorded = wj(rec.path(), &args, &stdin);
        let mut source: Vec<&str> = vec!["run.wjlog"];
        source.extend(allow.iter().map(String::as_str));
        fs::write(rec.path().join("cmds.txt"), "continue\ncontinue\n").unwrap();
        let mut rargs = vec!["replay", "--json", "--script", "cmds.txt"];
        rargs.extend(source.iter().copied());
        rargs.push("main.wj");
        let o = wj(rec.path(), &rargs, "");
        let ev: Vec<Json> = o.stdout.lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
        let Some(exit) = of_kind(&ev, "exited").first().cloned().cloned() else {
            failures.push(format!("{}: no exited event: {}{}", prog.display(), o.stdout, o.stderr));
            continue;
        };
        let mut stderr: String = ev.iter().filter(|e| e["event"] == "output" && e["stream"] == "stderr").map(|e| e["text"].as_str().unwrap()).collect();
        if let Some(m) = exit["message"].as_str() {
            stderr.push_str(m);
            stderr.push('\n');
        }
        let got = (exit["code"].as_i64().unwrap() as i32, stdout_of(&ev), stderr);
        // the recording's stderr starts with the lint report of `wj check`, which runs before the program; the session
        // reports what the program itself wrote
        let program_stderr: String = recorded.stderr.lines().filter(|l| !l.starts_with("error[W") && !l.starts_with("warning[W")).map(|l| format!("{l}\n")).collect();
        let want = (recorded.code, recorded.stdout.clone(), program_stderr);
        if got != want {
            failures.push(format!("{}:\n  recorded {want:?}\n  session  {got:?}", prog.display()));
        }
    }
    assert!(failures.is_empty(), "sessions that differ from their recording:\n{}", failures.join("\n"));
}

/// Group the events into states: one stop with the locals and stack read there. The reason is how the session got
/// there, not part of the state.
fn states(events: &[Json]) -> Vec<Vec<Json>> {
    let mut out: Vec<Vec<Json>> = Vec::new();
    for e in events.iter().filter(|e| matches!(e["event"].as_str(), Some("stopped" | "locals" | "stack"))) {
        let mut e = e.clone();
        if e["event"] == "stopped" {
            e.as_object_mut().unwrap().remove("reason");
            out.push(Vec::new());
        }
        if let Some(g) = out.last_mut() {
            g.push(e);
        }
    }
    out
}

/// A state's identity: its step, and whether it is the stop at a failure (which shares the failing statement's step).
fn key(g: &[Json]) -> (u64, bool) {
    (g[0]["step"].as_u64().unwrap(), g[0].get("message").is_some())
}

/// ADR-0035 acceptance 3: from every step, `back` and then `step` forward again land in states identical to the ones an
/// uninterrupted pass through the run saw at those steps (stops, locals, stacks).
fn back_then_forward_holds(dir: &Path, source: &[&str]) {
    let probe = session(dir, source, "continue\ncontinue\n");
    let total = of_kind(&probe, "exited")[0]["steps"].as_u64().unwrap() as usize;
    assert!(total >= 5, "too short a run to test: {total} steps");
    // the uninterrupted pass: every state, once
    let pass = session(dir, source, &"locals\nwhere\nstep\n".repeat(total + 1));
    let original: std::collections::BTreeMap<(u64, bool), Vec<Json>> = states(&pass).into_iter().map(|g| (key(&g), g)).collect();
    assert!(original.len() >= total, "one state per step: {} for {total} steps", original.len());
    // forward one, back one, forward one again, at every step of the run
    let ev = session(dir, source, &"step\nback\nlocals\nwhere\nstep\nlocals\nwhere\n".repeat(total));
    let got: Vec<Vec<Json>> = states(&ev).into_iter().filter(|g| g.len() == 3).collect();
    assert!(got.len() >= 2 * (total - 1), "{} checked states for {total} steps", got.len());
    for g in &got {
        let want = original.get(&key(g)).unwrap_or_else(|| panic!("state {:?} never seen in the uninterrupted pass", key(g)));
        assert_eq!(g, want, "the state at step {:?} differs after going back", key(g));
    }
    // `back` lands exactly one step before where the session was (the end counts as one step after the last statement)
    let mut was: Option<u64> = None;
    for e in &ev {
        match e["event"].as_str() {
            Some("stopped") => {
                let step = e["step"].as_u64().unwrap();
                if e["reason"] == "back" {
                    assert_eq!(Some(step + 1), was, "`back` from {was:?} went to step {step}");
                }
                was = Some(step);
            }
            Some("exited") => was = Some(e["steps"].as_u64().unwrap() + 1),
            _ => {}
        }
    }
    let backs = of_kind(&ev, "stopped").iter().filter(|e| e["reason"] == "back").count();
    assert!(backs >= total - 1, "{backs} back stops for {total} steps");
    assert_eq!(stdout_of(&ev), stdout_of(&pass), "each byte of output is reported once, however often `back` re-runs it");
}

#[test]
fn back_then_forward_reproduces_every_step() {
    let d = program_dir("squares.wj");
    back_then_forward_holds(d.path(), &["--simulate", "--seed", "1"]);
    let rec = program_dir("sum_file.wj");
    assert_eq!(record_sum_file(rec.path()).code, 101);
    fs::remove_file(rec.path().join("numbers.txt")).unwrap();
    back_then_forward_holds(rec.path(), &["run.wjlog"]);
    let dice = program_dir("dice.wj");
    let (seed, _) = failing_dice_seed(dice.path());
    back_then_forward_holds(dice.path(), &["--simulate", "--seed", &seed.to_string()]);
}

#[test]
fn bad_commands_and_impossible_requests_are_reported() {
    let d = program_dir("squares.wj");
    fs::write(d.path().join("cmds.txt"), "step\nfly away\n").unwrap();
    let o = wj(d.path(), &["replay", "--simulate", "--script", "cmds.txt", "main.wj"], "");
    assert_eq!(o.code, 1);
    assert!(o.stderr.contains("script line 2: unknown command `fly`"), "{}", o.stderr);
    let ev = session(d.path(), &["--simulate", "--seed", "1"], "back\nprint nothing_here\ncontinue\nprint acc\nback 1000\n");
    let errors: Vec<String> = of_kind(&ev, "error").iter().map(|e| e["message"].as_str().unwrap().to_string()).collect();
    assert_eq!(errors.len(), 4, "{errors:?}");
    assert!(errors[0].contains("cannot go back 1 from step 1"));
    assert!(errors[1].contains("no binding `nothing_here`"));
    assert!(errors[2].contains("the program has exited"));
    assert!(errors[3].contains("cannot go back 1000"));
}

#[test]
fn an_edited_program_diverges_from_its_journal() {
    let rec = program_dir("sum_file.wj");
    record_sum_file(rec.path());
    let src = fs::read_to_string(rec.path().join("main.wj")).unwrap().replace("numbers.txt", "other.txt");
    fs::write(rec.path().join("main.wj"), src).unwrap();
    let ev = session(rec.path(), &["run.wjlog"], "continue\ncontinue\n");
    let exit = of_kind(&ev, "exited")[0].clone();
    assert!(exit["message"].as_str().unwrap_or("").contains("WJ0049"), "{exit}");
}
