//! `wj run --verify` (ADR-0033 option b): interpret the program, then build it compiled and, when that is safe,
//! run the compiled program a second time and compare.
//!
//! - The interpreted run is the normal one (`wj run --interpret` in a child process), its stdout streamed to ours
//!   as the program prints and kept for the comparison (once, and while the program runs). Stdin is read once and given to both runs (it is never read twice from the terminal).
//! - The build is the same as `wj run --compiled`'s (`run::build_native`) and records the compiled verdict.
//! - The compiled program runs again only when the effect analysis (ADR-0021) finds that `main` reaches nothing
//!   beyond the console and a read of `/dev/stdin`, and stdin was captured: running it again then repeats no effect,
//!   and no grant is involved. Otherwise the build's verdict is reported and the program is not run again.
//! - Everything `--verify` says goes to stderr, prefixed `wj verify:`. Exit status: the interpreted program's when it
//!   is non-zero; otherwise 1 when the build fails or the compiled run differs, else 0.

use crate::ir::effect_facts::EffectKind;
use std::io::{IsTerminal, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Why the compiled program may not run a second time, or `Ok` when it may.
pub fn rerun_blockers(entry: &Path) -> Result<(), Vec<String>> {
    let inputs = crate::check::effects::parse_app_inputs(entry).map_err(|e| vec![format!("{e}")])?;
    let a = crate::check::effects::analyze_project(entry, inputs, false);
    let main = a.facts.fns.values().find(|f| f.path == "main" || f.path.ends_with("::main"));
    let Some(main) = main else { return Err(vec!["no `main` found by the effect analysis".into()]) };
    let mut blockers: Vec<String> = Vec::new();
    for e in main.effects.keys() {
        let stdin = e.kind == EffectKind::FsRead && e.resource.literal() == Some("/dev/stdin");
        if e.kind != EffectKind::Console && !stdin {
            let name = e.kind.name().to_string();
            if !blockers.contains(&name) {
                blockers.push(name);
            }
        }
    }
    if main.open {
        blockers.push("calls through function values whose effects are not known".into());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

/// The first line where two outputs differ (1-based) and both lines.
pub fn first_difference(a: &str, b: &str) -> Option<(usize, String, String)> {
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.split('\n').collect(), b.split('\n').collect());
    (0..la.len().max(lb.len())).find(|&i| la.get(i) != lb.get(i)).map(|i| {
        (i + 1, la.get(i).unwrap_or(&"<end of output>").to_string(), lb.get(i).unwrap_or(&"<end of output>").to_string())
    })
}

fn say(msg: &str) {
    eprintln!("wj verify: {msg}");
}

pub fn execute(path: Option<&Path>, bin: Option<&str>, args: &[String], no_lint: bool, out: crate::cli::run::Output) -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let target = match crate::module_system::project::resolve_run_target(path, &cwd, bin) {
        Ok(t) => t,
        Err(m) => {
            eprintln!("error: {m}");
            return 1;
        }
    };
    let entry = target.entry.clone();
    // stdin once, for both runs; a terminal is left to the interpreted run and the compiled run is not repeated
    let stdin: Option<Vec<u8>> = if std::io::stdin().is_terminal() { None } else { Some(capture_stdin(STDIN_FIRST_BYTE_WAIT)) };
    let Ok(exe) = std::env::current_exe() else { return 1 };
    let mut interp = Command::new(&exe);
    interp.args(["run", "--interpret"]);
    if no_lint {
        interp.arg("--no-lint");
    }
    if out.format == crate::diagnostics::render::Format::Agent {
        interp.args(["--error-format", "agent"]);
    }
    interp.arg(&entry).args(args).stdout(Stdio::piped()).stderr(Stdio::inherit());
    // the interpreted stdout is written as it arrives (and kept for the comparison), not at the end
    let interp_out = match run_streaming(interp, stdin.as_deref(), &mut std::io::stdout()) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: cannot run the interpreter: {e}");
            return 1;
        }
    };
    let interp_code = interp_out.status.code().unwrap_or(1);
    // the interpreter refused the program (a `wj check` error): the build refuses it too, nothing more to say
    if interp_code != 0 && refused(&entry, no_lint) {
        return interp_code;
    }

    // what `wj run --compiled` builds: the source root for a crate root `main.wj`, else the file
    let input = match crate::cli::run_dev::build_input(&target) {
        Ok(p) => p,
        Err(m) => {
            say(&format!("the compiled build cannot run here: {m}"));
            return if interp_code != 0 { interp_code } else { 1 };
        }
    };
    let (workdir, failed) = match crate::cli::run::build_native(&input, "rust", false, out) {
        Ok(r) => r,
        Err(e) => {
            say(&format!("the compiled build could not run: {e:#}"));
            return if interp_code != 0 { interp_code } else { 1 };
        }
    };
    if failed.is_some() {
        // a recorded rejection is rustc's verdict on this content; any other failure (a toolchain, a fetch) is no verdict
        let rejected = matches!(crate::cli::verdict::lookup(&input).map(|v| v.outcome), Some(crate::cli::verdict::Outcome::Rejected { .. }));
        if rejected {
            say("the interpreter ran the program, but the compiled build failed (see the errors above): rustc is stricter than the interpreter here, so `wj build` would fail too");
        } else {
            say("the compiled build could not complete (see above); this is not a verdict on the program");
        }
        return if interp_code != 0 { interp_code } else { 1 };
    }
    let blockers = match (&stdin, rerun_blockers(&entry)) {
        (None, _) => Err(vec!["stdin is a terminal, so it cannot be given to a second run".to_string()]),
        (Some(_), r) => r,
    };
    if let Err(why) = blockers {
        say(&format!("compiled build OK; not run a second time ({})", why.join(", ")));
        return interp_code;
    }
    let _tree = crate::cargo_cache::active::hold(crate::cargo_cache::TargetKind::Shared);
    let mut compiled = crate::cli::run::compiled_command(&workdir, false, args);
    compiled.stdout(Stdio::piped()).stderr(Stdio::piped());
    let out = match run_with_stdin(compiled, stdin.as_deref()) {
        Ok(o) => o,
        Err(e) => {
            say(&format!("compiled build OK; the compiled program could not be started: {e}"));
            return if interp_code != 0 { interp_code } else { 1 };
        }
    };
    let code = out.status.code().unwrap_or(1);
    let (a, b) = (String::from_utf8_lossy(&interp_out.stdout), String::from_utf8_lossy(&out.stdout));
    if a == b && code == interp_code {
        say(&format!("compiled build OK; the compiled run matches (stdout {} bytes, exit status {code})", out.stdout.len()));
        return interp_code;
    }
    if code != interp_code {
        say(&format!("the compiled run exited with status {code}, the interpreted run with {interp_code}"));
    }
    if let Some((line, i, c)) = first_difference(&a, &b) {
        say(&format!("stdout differs at line {line}:\n  interpreted: {i}\n  compiled:    {c}"));
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if !err.trim().is_empty() {
        say(&format!("the compiled run's stderr:\n{}", err.trim_end()));
    }
    say("documented differences (hash iteration order, clocks, randomness, thread interleaving) are listed in docs/testing/INTERPRETER_CONTRACT.md; anything else is a divergence to report");
    if interp_code != 0 {
        interp_code
    } else {
        1
    }
}

/// How long `--verify` waits for the first byte of a non-terminal stdin before treating it as empty.
pub const STDIN_FIRST_BYTE_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// Stdin for both runs. The rule: read to end of input once data arrives; if neither data nor the end of input
/// arrives within `wait` (an open pipe nobody writes to, as when an agent harness starts `wj` without redirecting
/// input), both runs get empty stdin and a note says so. Redirect input (`< file`, `< /dev/null`) to be explicit.
pub fn capture_stdin(wait: std::time::Duration) -> Vec<u8> {
    capture_from(std::io::stdin(), wait)
}

/// `capture_stdin` over any reader (tests drive it with a pipe).
pub fn capture_from<R: Read + Send + 'static>(mut input: R, wait: std::time::Duration) -> Vec<u8> {
    enum Msg {
        /// The first bytes arrived; the rest follows as `Done`.
        Started,
        Done(Vec<u8>),
    }
    let (tx, rx) = std::sync::mpsc::channel::<Msg>();
    std::thread::spawn(move || {
        let mut first = [0u8; 8192];
        let mut buf = Vec::new();
        if let Ok(n @ 1..) = input.read(&mut first) {
            let _ = tx.send(Msg::Started);
            buf.extend_from_slice(&first[..n]);
            let _ = input.read_to_end(&mut buf);
        }
        let _ = tx.send(Msg::Done(buf));
    });
    match rx.recv_timeout(wait) {
        Ok(Msg::Done(buf)) => buf,
        // data is arriving: read to the end of input, however long it takes
        Ok(Msg::Started) => match rx.recv() {
            Ok(Msg::Done(buf)) => buf,
            _ => Vec::new(),
        },
        Err(_) => {
            say(&format!("no input arrived on stdin within {} s; both runs get empty stdin (redirect it: `< file` or `< /dev/null`)", wait.as_secs()));
            Vec::new()
        }
    }
}

/// `wj check` reports an error that stops an interpreted run (what the child printed as "not running ...").
fn refused(entry: &Path, no_lint: bool) -> bool {
    let use_cache = crate::check::effects::manifest_path(entry).is_some();
    let Ok(c) = crate::check::check_path(entry, &crate::check::CheckOptions { use_cache }) else { return true };
    let is_lint = |code: &str| code.len() == 5 && code.starts_with('W') && code[1..].bytes().all(|b| b.is_ascii_digit());
    c.report.diagnostics.iter().any(|d| d.is_error() && (!no_lint || !is_lint(&d.code)))
}

/// Run `cmd` with `stdin` given as in `run_with_stdin`, writing its stdout to `sink` as it arrives (flushed per
/// chunk) and returning it too, with the exit status. Its stderr is whatever `cmd` was configured with.
pub fn run_streaming(mut cmd: Command, stdin: Option<&[u8]>, sink: &mut dyn Write) -> std::io::Result<std::process::Output> {
    cmd.stdout(Stdio::piped());
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::inherit() });
    let mut child = cmd.spawn()?;
    feed_stdin(&mut child, stdin);
    let mut pipe = child.stdout.take().expect("stdout was piped");
    let mut kept = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match pipe.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                kept.extend_from_slice(&chunk[..n]);
                let _ = sink.write_all(&chunk[..n]);
                let _ = sink.flush();
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    let status = child.wait()?;
    Ok(std::process::Output { status, stdout: kept, stderr: Vec::new() })
}

fn feed_stdin(child: &mut std::process::Child, stdin: Option<&[u8]>) {
    if let (Some(data), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let data = data.to_vec();
        // a writer thread, so a program that prints before reading all its input cannot deadlock on a full pipe
        std::thread::spawn(move || {
            let _ = pipe.write_all(&data);
        });
    }
}

fn run_with_stdin(mut cmd: Command, stdin: Option<&[u8]>) -> std::io::Result<std::process::Output> {
    cmd.stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::inherit() });
    let mut child = cmd.spawn()?;
    feed_stdin(&mut child, stdin);
    child.wait_with_output()
}
