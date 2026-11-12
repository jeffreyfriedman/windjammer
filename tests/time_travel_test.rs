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

// ---------------------------------------------------------------------------------------------------------------------
// Second slice (ADR-0035): `print EXPR`, `next` / `finish`, `--at-effect`, `watch` and `break LINE if EXPR`, and the
// capability grant in the journal header.
// ---------------------------------------------------------------------------------------------------------------------

fn errors(events: &[Json]) -> Vec<String> {
    of_kind(events, "error").iter().map(|e| e["message"].as_str().unwrap().to_string()).collect()
}

fn stops(events: &[Json]) -> Vec<(String, u64, u64)> {
    of_kind(events, "stopped").iter().map(|e| (e["reason"].as_str().unwrap().to_string(), e["step"].as_u64().unwrap(), e["line"].as_u64().unwrap_or(0))).collect()
}

fn sim1(dir: &Path, script: &str) -> Vec<Json> {
    session(dir, &["--simulate", "--seed", "1"], script)
}

/// Hand trace of inspect.wj: line 15 starts with `xs = [3, 1, 4]`, `p = Point { x: 2, y: 5 }`, `name = "wj"` bound.
#[test]
fn print_evaluates_expressions_in_the_stopped_frame() {
    let d = program_dir("inspect.wj");
    let script = "break 15\ncontinue\nprint xs.len()\nprint xs[1] + xs[2]\nprint p.x * p.y\nprint name.to_uppercase()\nprint xs.len() > 2 && p.y == 5\nprint (p.x, name)\n";
    let ev = sim1(d.path(), script);
    assert_eq!(errors(&ev), Vec::<String>::new());
    assert_eq!(values(&ev), ["3", "5", "10", "\"WJ\"", "true", "(2, \"wj\")"]);
    let names: Vec<&str> = of_kind(&ev, "value").iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(names[1], "xs[1] + xs[2]", "the value event names the expression");
}

/// The guard refuses calls with effects and anything that changes a binding, says why, and leaves the run exactly as it was.
#[test]
fn print_refuses_effects_and_mutation_and_changes_nothing() {
    let d = program_dir("inspect.wj");
    let refused = ["describe(p)", "xs.push(9)", "xs.clear()", "println(\"hi\")", "std::fs::read_to_string(\"a\")", "xs.len() + xs.pop().unwrap()", "n = 3", "let z = 1"];
    let mut script = String::from("break 15\ncontinue\nlocals\nwhere\n");
    for r in refused {
        script.push_str(&format!("print {r}\n"));
    }
    script.push_str("locals\nwhere\nprint xs.len()\nstep\nlocals\n");
    let ev = sim1(d.path(), &script);
    let errs = errors(&ev);
    assert_eq!(errs.len(), refused.len(), "{errs:?}");
    for e in &errs {
        assert!(e.contains("read-only") && e.contains("no side effects"), "the refusal should say why: {e}");
    }
    assert!(errs[0].contains("describe"), "the refusal names the call: {}", errs[0]);
    let locals = of_kind(&ev, "locals");
    assert_eq!(locals[0], locals[1], "refused prints leave the bindings alone");
    assert_eq!(of_kind(&ev, "stack").len(), 2);
    assert_eq!(of_kind(&ev, "stack")[0], of_kind(&ev, "stack")[1]);
    assert_eq!(values(&ev), ["3"]);
    assert_eq!(stdout_of(&ev), "", "no output was written by the refused expressions");
    // stepping on from here is step 5 (the next statement after step 4), as if nothing had been printed
    assert_eq!(stops(&ev).last().map(|s| s.1), Some(5));
}

/// A refused `print` of an effect must not consume a journal event: the replay goes on to the same end it had.
#[test]
fn print_of_an_effect_does_not_consume_the_journal() {
    let rec = program_dir("sum_file.wj");
    assert_eq!(record_sum_file(rec.path()).code, 101);
    fs::remove_file(rec.path().join("numbers.txt")).unwrap();
    let ev = session(rec.path(), &["run.wjlog"], "break 6\ncontinue\nprint fs::read_to_string(\"numbers.txt\")\nprint text.len()\ncontinue\ncontinue\ncontinue\ncontinue\n");
    assert_eq!(errors(&ev).len(), 1, "only the effect is refused: {:?}", errors(&ev));
    assert_eq!(values(&ev), ["8"]);
    let exit = of_kind(&ev, "exited")[0].clone();
    assert_eq!(exit["code"], 101);
    assert!(!exit["message"].as_str().unwrap_or("").contains("WJ0049"), "the replay diverged: {exit}");
}

#[test]
fn print_errors_in_the_expression_are_reported_not_fatal() {
    let d = program_dir("inspect.wj");
    let ev = sim1(d.path(), "break 15\ncontinue\nprint nothing + 1\nprint xs[10]\nprint 1 +\nprint p.x\n");
    let errs = errors(&ev);
    assert_eq!(errs.len(), 3, "{errs:?}");
    assert_eq!(values(&ev), ["2"], "the session goes on after a bad expression");
}

/// squares.wj: `next` at the call on line 10 (step 4) skips the steps of `square` (5 and 6) and stops at line 11 (step 7).
#[test]
fn next_steps_over_a_call() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 10\ncontinue\nnext\nprint acc\nnext\nnext\n");
    let want: Vec<(String, u64, u64)> = [("start", 1, 7), ("breakpoint", 4, 10), ("next", 7, 11), ("next", 8, 10), ("next", 11, 11)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev), want);
    assert_eq!(values(&ev), ["1"], "acc after the first call: square(1)");
}

/// A breakpoint inside the stepped-over call still stops the run there; `next` from its last statement lands in the caller.
#[test]
fn next_stops_at_a_breakpoint_inside_the_call() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 10\ncontinue\nbreak 3\nnext\nnext\n");
    let want: Vec<(String, u64, u64)> = [("start", 1, 7), ("breakpoint", 4, 10), ("breakpoint", 6, 3), ("next", 7, 11)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev), want);
    assert_eq!(of_kind(&ev, "stopped")[2]["function"], "square");
    assert_eq!(of_kind(&ev, "stopped")[3]["function"], "main");
}

/// `finish` from the last statement of `square` (step 6, y = 1) says what it returned, then stops in the caller (step 7);
/// the second call returns 4.
#[test]
fn finish_reports_the_return_value() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 3\ncontinue\nfinish\ncontinue\nfinish\n");
    let ret: Vec<(&str, &str)> = of_kind(&ev, "returned").iter().map(|e| (e["function"].as_str().unwrap(), e["value"].as_str().unwrap())).collect();
    assert_eq!(ret, [("square", "1"), ("square", "4")]);
    let want: Vec<(String, u64, u64)> = [("start", 1, 7), ("breakpoint", 6, 3), ("finish", 7, 11), ("breakpoint", 10, 3), ("finish", 11, 11)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev), want);
    // `returned` comes just before the stop it belongs to
    let order: Vec<&str> = ev.iter().filter(|e| matches!(e["event"].as_str(), Some("returned" | "stopped"))).map(|e| e["event"].as_str().unwrap()).collect();
    assert_eq!(order, ["stopped", "stopped", "returned", "stopped", "stopped", "returned", "stopped"]);
}

/// inspect.wj: `describe` returns `p.x + p.y` = 7 as its tail expression (step 6); `finish` from step 5 reports it.
#[test]
fn finish_reports_a_tail_expression_and_the_end_of_main() {
    let d = program_dir("inspect.wj");
    let ev = sim1(d.path(), "break 7\ncontinue\nfinish\nfinish\n");
    assert_eq!(of_kind(&ev, "returned")[0]["function"], "describe");
    assert_eq!(of_kind(&ev, "returned")[0]["value"], "7");
    assert_eq!(stops(&ev).last().cloned(), Some(("finish".to_string(), 7, 16)));
    // the second `finish` is in main: it runs to the end, reports main's value, then the exit
    let kinds: Vec<&str> = ev.iter().map(|e| e["event"].as_str().unwrap()).collect();
    let tail: Vec<&str> = kinds.iter().rev().take(3).rev().copied().collect();
    assert_eq!(tail, ["output", "returned", "exited"]);
    assert_eq!(of_kind(&ev, "returned")[1]["function"], "main");
    assert_eq!(stdout_of(&ev), "3 wj 7\n");
}

#[test]
fn next_and_finish_survive_going_back() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 10\ncontinue\nnext\nlocals\nback\nlocals\nstep\nlocals\nfinish\n");
    let want: Vec<(String, u64, u64)> = [("start", 1, 7), ("breakpoint", 4, 10), ("next", 7, 11), ("back", 6, 3), ("step", 7, 11)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev)[..5], want[..]);
    let locals = of_kind(&ev, "locals");
    assert_eq!(locals[0], locals[2], "step 7 reached by `next` and by `back` then `step` is the same state");
}

#[test]
fn next_and_finish_after_the_end_are_reported() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "continue\nnext\nfinish\nwatch acc\n");
    let errs = errors(&ev);
    assert_eq!(errs.len(), 3, "{errs:?}");
    assert!(errs.iter().all(|e| e.contains("the program has exited")));
}

fn record_two_reads(dir: &Path) {
    fs::write(dir.join("a.txt"), "alpha\n").unwrap();
    fs::write(dir.join("b.txt"), "beta\n").unwrap();
    let o = wj(dir, &["run", "--interpret", "--no-lint", "--record", "run.wjlog", "main.wj"], "");
    assert_eq!(o.code, 0, "{}{}", o.stdout, o.stderr);
    assert_eq!(o.stdout, "alpha beta\n");
    fs::remove_file(dir.join("a.txt")).unwrap();
    fs::remove_file(dir.join("b.txt")).unwrap();
}

/// Hand trace of two_reads.wj: step 1 `let a` (line 9), 2 `let text` (line 4; the journal's event 1), 3 `text.trim()`,
/// 4 `let b` (line 10), 5 `let text` (line 4; event 2), 6 `text.trim()`, 7 `println`. The K-th event is answered by the
/// statement that starts the step before the first one that sees K events answered.
#[test]
fn at_effect_starts_at_the_statement_that_answers_the_event() {
    let d = program_dir("two_reads.wj");
    record_two_reads(d.path());
    let ev = session(d.path(), &["--at-effect", "2", "run.wjlog"], "print path\nwhere\nstep\nprint text\ncontinue\n");
    let first = &of_kind(&ev, "stopped")[0];
    assert_eq!((first["reason"].as_str(), first["step"].as_u64(), first["line"].as_u64(), first["function"].as_str(), first["effect"].as_u64()), (Some("effect"), Some(5), Some(4), Some("first_line"), Some(1)));
    assert_eq!(values(&ev), ["\"b.txt\"", format!("{:?}", "beta\n").as_str()]);
    let frames = of_kind(&ev, "stack")[0]["frames"].as_array().unwrap().clone();
    assert_eq!((frames[1]["function"].as_str(), frames[1]["line"].as_u64()), (Some("main"), Some(10)));
    assert_eq!(of_kind(&ev, "stopped")[1]["effect"], 2, "the step answered event 2");
    assert_eq!(stdout_of(&ev), "alpha beta\n");

    let ev1 = session(d.path(), &["--at-effect", "1", "run.wjlog"], "print path\n");
    let s1 = &of_kind(&ev1, "stopped")[0];
    assert_eq!((s1["reason"].as_str(), s1["step"].as_u64(), s1["line"].as_u64(), s1["effect"].as_u64()), (Some("effect"), Some(2), Some(4), Some(0)));
    assert_eq!(values(&ev1), ["\"a.txt\""]);
}

#[test]
fn at_effect_session_can_go_back_to_the_start() {
    let d = program_dir("two_reads.wj");
    record_two_reads(d.path());
    let ev = session(d.path(), &["--at-effect", "2", "run.wjlog"], "back 4\nwhere\nstep 4\n");
    let want: Vec<(String, u64, u64)> = [("effect", 5, 4), ("back", 1, 9), ("step", 5, 4)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev), want);
}

#[test]
fn at_effect_beyond_the_journal_and_bad_uses_are_refused() {
    let d = program_dir("two_reads.wj");
    record_two_reads(d.path());
    fs::write(d.path().join("cmds.txt"), "continue\n").unwrap();
    let run = |extra: &[&str]| {
        let mut args = vec!["replay", "--json", "--script", "cmds.txt"];
        args.extend_from_slice(extra);
        args.push("main.wj");
        wj(d.path(), &args, "")
    };
    let o = run(&["--at-effect", "3", "run.wjlog"]);
    assert_eq!(o.code, 1, "{}{}", o.stdout, o.stderr);
    assert!(o.stderr.contains("--at-effect 3") && o.stderr.contains("holds only 2 events"), "{}", o.stderr);
    let o = run(&["--at-effect", "0", "run.wjlog"]);
    assert_eq!(o.code, 2, "{}{}", o.stdout, o.stderr);
    let o = run(&["--simulate", "--at-effect", "1"]);
    assert_eq!(o.code, 2, "{}{}", o.stdout, o.stderr);
    assert!(o.stderr.contains("--simulate"), "{}", o.stderr);
}

/// Hand trace of watch_me.wj (14 steps): `n` is bound by step 1 (seen at step 2) and set to 1, 2, 3, 4 by steps 4, 6, 9, 11,
/// which the next statement start reports (steps 5, 7, 10, 12); `hits` is bound by step 2 (seen at step 3) and set by
/// steps 8 and 13, reported at steps 9 and 14.
#[test]
fn watch_stops_when_a_local_changes() {
    let d = program_dir("watch_me.wj");
    let ev = sim1(d.path(), "watch n\ncontinue\ncontinue\ncontinue\ncontinue\ncontinue\ncontinue\n");
    let w = of_kind(&ev, "watching");
    assert_eq!((w[0]["name"].as_str(), w[0]["function"].as_str(), w[0]["value"].is_null()), (Some("n"), Some("main"), true));
    let seen: Vec<(u64, Option<&str>, Option<&str>)> = of_kind(&ev, "stopped").iter().filter(|e| e["reason"] == "watch").map(|e| (e["step"].as_u64().unwrap(), e["old"].as_str(), e["new"].as_str())).collect();
    assert_eq!(seen, [(2, None, Some("0")), (5, Some("0"), Some("1")), (7, Some("1"), Some("2")), (10, Some("2"), Some("3")), (12, Some("3"), Some("4"))]);
    assert_eq!(of_kind(&ev, "stopped").iter().filter(|e| e["reason"] == "watch").map(|e| e["watch"].as_str().unwrap()).collect::<Vec<_>>(), ["n"; 5]);
    assert_eq!(of_kind(&ev, "exited")[0]["code"], 0);
    assert_eq!(stdout_of(&ev), "hits 2\n");
}

#[test]
fn watch_finds_the_change_again_after_going_back() {
    let d = program_dir("watch_me.wj");
    let ev = sim1(d.path(), "watch hits\ncontinue\ncontinue\nback 2\nlocals\ncontinue\nlocals\n");
    let w: Vec<&Json> = of_kind(&ev, "stopped").into_iter().filter(|e| e["reason"] == "watch").collect();
    assert_eq!(w.len(), 3, "step 3 (bound), 9 (hits = 1), then 9 again after going back and on");
    assert!(of_kind(&ev, "locals")[1].to_string().contains("{\"name\":\"hits\",\"value\":\"1\"}"));
    assert_eq!((w[1]["step"].as_u64(), w[1]["old"].as_str(), w[1]["new"].as_str()), (Some(9), Some("0"), Some("1")));
    assert_eq!(w[1], w[2], "the same stop, found again by re-execution");
}

#[test]
fn watch_in_a_callee_is_per_activation() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 3\ncontinue\nwatch y\ncontinue\ncontinue\n");
    let w = of_kind(&ev, "watching");
    assert_eq!((w[0]["function"].as_str(), w[0]["value"].as_str()), (Some("square"), Some("1")));
    // each call of `square` is a new activation: `y` is unbound at its first statement and 4, then 9, when the breakpoint
    // on line 3 is reached (steps 10 and 14); the watch change rides on the breakpoint stop
    let s: Vec<(u64, &str, Option<&str>, Option<&str>)> =
        of_kind(&ev, "stopped").iter().filter(|e| e["watch"] == "y").map(|e| (e["step"].as_u64().unwrap(), e["reason"].as_str().unwrap(), e["old"].as_str(), e["new"].as_str())).collect();
    assert_eq!(s, [(10, "breakpoint", None, Some("4")), (14, "breakpoint", None, Some("9"))]);
}

/// squares.wj: `square` runs at line 3 with y = 1, 4, 9 (steps 6, 10, 14).
#[test]
fn conditional_breakpoints_stop_only_when_the_condition_holds() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 3 if y > 3\ncontinue\ncontinue\ncontinue\n");
    let want: Vec<(String, u64, u64)> = [("start", 1, 7), ("breakpoint", 10, 3), ("breakpoint", 14, 3)].iter().map(|(r, s, l)| (r.to_string(), *s, *l)).collect();
    assert_eq!(stops(&ev), want);
    assert_eq!(of_kind(&ev, "stopped")[1]["condition"], "y > 3");
    let ev = sim1(d.path(), "break 3 if y == 9 && x == 3\nbreak 11 if acc == 99\ncontinue\nprint y\ncontinue\n");
    assert_eq!(stops(&ev).iter().map(|s| s.1).collect::<Vec<_>>(), [1, 14]);
    assert_eq!(values(&ev), ["9"]);
    assert_eq!(of_kind(&ev, "exited")[0]["steps"], 16);
}

/// A loop body of one statement is the same line every iteration: a breakpoint (and its condition) is checked on each one.
#[test]
fn a_breakpoint_in_a_one_statement_loop_body_is_checked_every_iteration() {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("main.wj"), "fn main() {\n    let mut n = 0\n    while n < 3 {\n        n = n + 1\n    }\n    println(\"${n}\")\n}\n").unwrap();
    let ev = sim1(d.path(), "break 4\ncontinue\ncontinue\ncontinue\n");
    assert_eq!(stops(&ev).iter().map(|s| s.1).collect::<Vec<_>>(), [1, 3, 4, 5]);
    let ev = sim1(d.path(), "break 4 if n == 2\ncontinue\nprint n\n");
    assert_eq!(stops(&ev).iter().map(|s| s.1).collect::<Vec<_>>(), [1, 5]);
    assert_eq!(values(&ev), ["2"]);
}

#[test]
fn a_broken_condition_stops_and_says_so() {
    let d = program_dir("squares.wj");
    let ev = sim1(d.path(), "break 3 if nothing_here > 1\nbreak 11 if acc\nbreak 10 if square(2) == 4\ncontinue\ncontinue\ncontinue\n");
    // the stops come in program order: line 10 (step 4), line 3 (step 6), line 11 (step 7)
    let msgs: Vec<String> = of_kind(&ev, "stopped").iter().skip(1).filter_map(|e| e["message"].as_str().map(String::from)).collect();
    assert_eq!(msgs.len(), 3, "{msgs:?}");
    assert!(msgs[0].contains("read-only") && msgs[0].contains("square"), "a condition with a call is refused like a print: {}", msgs[0]);
    assert!(msgs[1].contains("could not be evaluated") && msgs[1].contains("nothing_here"), "{}", msgs[1]);
    assert!(msgs[2].contains("not a boolean"), "{}", msgs[2]);
}

#[test]
fn the_text_of_a_watch_and_a_condition_stop_is_readable() {
    let d = program_dir("watch_me.wj");
    fs::write(d.path().join("cmds.txt"), "watch hits\ncontinue\ncontinue\n").unwrap();
    let o = wj(d.path(), &["replay", "--simulate", "--seed", "1", "--script", "cmds.txt", "main.wj"], "");
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.contains("watching hits in main (now <unset>)"), "{}", o.stdout);
    assert!(o.stdout.contains("[step 9] watch in main at main.wj:5:") && o.stdout.contains(": hits: 0 -> 1"), "{}", o.stdout);
}

/// Occurrences of the field `prefix` (`.debug`), not of longer names such as `.debug_string` or of a method `.debug()`.
fn field_uses(text: &str, prefix: &str) -> usize {
    text.match_indices(prefix).filter(|(i, _)| !text[i + prefix.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '(')).count()
}

/// ADR-0035 section 6: without a debugger the cost is one `Option` test per statement. This is the shape of the hot path,
/// checked in the source so that a second check, or work done before the test, cannot slip in unnoticed.
#[test]
fn the_hot_path_still_has_one_option_test_per_statement() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/interpreter");
    let stmt = fs::read_to_string(root.join("eval_stmt.rs")).unwrap();
    let at = stmt.find("pub(crate) fn exec_stmt(").expect("exec_stmt");
    let body: String = stmt[at..].lines().take_while(|l| !l.starts_with("    }")).collect::<Vec<_>>().join("\n");
    assert_eq!(field_uses(&body, "self.debug"), 1, "exec_stmt must test the debugger exactly once:\n{body}");
    assert!(body.contains("if self.debug.is_some()") && body.contains("self.exec_stmt_plain(s)"), "{body}");
    // nothing else in the interpreter outside the debugger touches the debugger field, so no other statement or call path pays for it
    let mut users = Vec::new();
    for e in fs::read_dir(&root).unwrap().filter_map(Result::ok) {
        let p = e.path();
        if p.extension().is_some_and(|x| x == "rs") {
            let n = field_uses(&fs::read_to_string(&p).unwrap(), "self.debug");
            if n > 0 {
                users.push((p.file_name().unwrap().to_string_lossy().to_string(), n));
            }
        }
    }
    users.sort();
    assert_eq!(users, [("eval_stmt.rs".to_string(), 1)], "the debugger field is read only by the statement hook (the Interpreter declares it in engine.rs)");
}

/// The recording writes its grants into the header (format 2); a replay re-applies them with no flags.
#[test]
fn a_replay_reapplies_the_recorded_grants() {
    let rec = tempfile::tempdir().unwrap();
    fs::write(rec.path().join("main.wj"), "use std::env\n\nfn main() {\n    let v = env::var(\"WJ_TT_GRANT\").unwrap_or(\"none\")\n    println(v)\n}\n").unwrap();
    let run = |args: &[&str]| {
        let mut c = Command::new(WJ);
        c.current_dir(rec.path()).env_remove("WJ_CAPS").env("WJ_TT_GRANT", "granted").args(args).stdin(Stdio::null());
        let o = c.output().unwrap();
        (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).to_string(), String::from_utf8_lossy(&o.stderr).to_string())
    };
    let (code, out, err) = run(&["run", "--interpret", "--no-lint", "--allow", "env.read=WJ_TT_GRANT", "--record", "run.wjlog", "main.wj"]);
    assert_eq!((code, out.as_str()), (0, "granted\n"), "{err}");
    let log = fs::read_to_string(rec.path().join("run.wjlog")).unwrap();
    let head: Json = serde_json::from_str(log.lines().next().unwrap()).unwrap();
    assert_eq!(head["wj_effect_log"], 2);
    assert_eq!(head["grants"], serde_json::json!(["env.read=WJ_TT_GRANT"]));
    // no --allow, and the variable is not even set: the journal answers, and the recorded grant lets it through
    fs::write(rec.path().join("cmds.txt"), "break 5\ncontinue\nprint v\n").unwrap();
    let mut c = Command::new(WJ);
    c.current_dir(rec.path()).env_remove("WJ_CAPS").env_remove("WJ_TT_GRANT").args(["replay", "--script", "cmds.txt", "run.wjlog", "main.wj"]);
    let o = c.output().unwrap();
    let text = String::from_utf8_lossy(&o.stdout).to_string();
    assert!(text.contains("v = \"granted\""), "{text}{}", String::from_utf8_lossy(&o.stderr));
    // a recording made without the grant has no grants to re-apply
    let (code, _, _) = run(&["run", "--interpret", "--no-lint", "--caps", "off", "--record", "plain.wjlog", "main.wj"]);
    assert_eq!(code, 0);
    let plain: Json = serde_json::from_str(fs::read_to_string(rec.path().join("plain.wjlog")).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(plain["grants"], serde_json::json!([]));
}

/// A log from before the header carried grants (format 1) is refused with the way out, by `wj replay` and `wj run --replay`.
#[test]
fn a_log_of_the_old_format_is_refused_clearly() {
    let d = program_dir("squares.wj");
    fs::write(d.path().join("old.wjlog"), "{\"args\":[],\"compiler\":\"0.50.0\",\"det_time\":false,\"hash_seed\":0,\"program\":\"x\",\"rng_seed\":1,\"sched_seed\":0,\"sim_seed\":null,\"wj_effect_log\":1}\n").unwrap();
    fs::write(d.path().join("cmds.txt"), "continue\n").unwrap();
    let o = wj(d.path(), &["replay", "--script", "cmds.txt", "old.wjlog", "main.wj"], "");
    assert_eq!(o.code, 1, "{}{}", o.stdout, o.stderr);
    assert!(o.stderr.contains("format 1") && o.stderr.contains("format 2") && o.stderr.contains("wj run --record"), "{}", o.stderr);
    let o = wj(d.path(), &["run", "--interpret", "--replay", "old.wjlog", "main.wj"], "");
    assert_ne!(o.code, 0);
    assert!(o.stderr.contains("format 1") && o.stderr.contains("wj run --record"), "{}", o.stderr);
}
