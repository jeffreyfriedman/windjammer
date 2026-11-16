//! `wj run --verify` (ADR-0033 option b): interpret, build compiled, and run the compiled program a second time
//! only when its effects reach nothing beyond the console and stdin (which is captured once and given to both runs).
//! Everything `--verify` says is on stderr; stdout carries the program's output exactly once.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn verify(file: &Path, stdin: &str) -> (String, String, Option<i32>) {
    let mut child = Command::new(WJ)
        .args(["run", "--verify"])
        .arg(file)
        .current_dir(file.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    let o = child.wait_with_output().unwrap();
    (String::from_utf8_lossy(&o.stdout).into(), String::from_utf8_lossy(&o.stderr).into(), o.status.code())
}

fn program(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
    let f = dir.join(name);
    std::fs::write(&f, text).unwrap();
    f
}

#[test]
fn a_console_program_with_stdin_is_compared_and_printed_once() {
    let d = tempfile::tempdir().unwrap();
    let f = program(
        d.path(),
        "echo.wj",
        "use std::fs\nuse std::process\n\nfn main() {\n    let text = fs::read_to_string(\"/dev/stdin\").unwrap_or(\"\")\n    for line in text.lines() {\n        println(\"got ${line}\")\n    }\n    process::exit(3)\n}\n",
    );
    let (out, err, code) = verify(&f, "a\nb\n");
    assert_eq!(out, "got a\ngot b\n", "stdout once, from the interpreted run: {err}");
    // the runs agree, so verification passes (exit 0) and the line reports the program's own status
    assert_eq!(code, Some(0), "agreeing runs exit 0 whatever the program's status is: {err}");
    assert!(err.contains("wj verify: OK, the compiled run matches the interpreted run (stdout 12 bytes, program exit status 3)"), "{err}");
}

#[test]
fn a_program_with_other_effects_is_built_but_not_run_again() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path(), "w.wj", "use std::fs\n\nfn main() {\n    let _ = fs::write(\"out.txt\", \"one\")\n    println(\"wrote\")\n}\n");
    let (out, err, code) = verify(&f, "");
    assert_eq!(out, "wrote\n", "{err}");
    assert_eq!(code, Some(0), "{err}");
    assert!(err.contains("compiled build OK; not run a second time (fs.write)"), "{err}");
    assert_eq!(std::fs::read_to_string(d.path().join("out.txt")).unwrap(), "one");
}

#[test]
fn a_build_the_interpreter_runs_but_rustc_rejects_fails_verification() {
    let d = tempfile::tempdir().unwrap();
    // drift family A: the backend types `five` as i32 beside the i64 parameter (ledgered, docs/drift/findings.md)
    let f = program(d.path(), "five.wj", "fn main() {\n    let five = 5\n    let add5 = |x: int| x + five\n    println(\"${add5(1)}\")\n}\n");
    let (out, err, code) = verify(&f, "");
    assert_eq!(out, "6\n", "{err}");
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains("the interpreter ran the program, but the compiled build failed"), "{err}");
}

#[test]
fn a_program_check_refuses_is_not_built() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path(), "bad.wj", "fn main() {\n    let x: int = \"s\"\n    println(\"${x}\")\n}\n");
    let (out, err, code) = verify(&f, "");
    assert_eq!(out, "");
    assert_eq!(code, Some(1));
    assert!(err.contains("not running"), "{err}");
    assert!(!err.contains("wj verify:"), "nothing to verify after a refusal: {err}");
}

#[test]
fn differences_are_located_by_line() {
    assert_eq!(windjammer::cli::verify::first_difference("a\nb\n", "a\nb\n"), None);
    let (line, i, c) = windjammer::cli::verify::first_difference("a\nb\n", "a\nc\n").unwrap();
    assert_eq!((line, i.as_str(), c.as_str()), (2, "b", "c"));
    let (line, _, c) = windjammer::cli::verify::first_difference("a\n", "a\nextra\n").unwrap();
    assert_eq!((line, c.as_str()), (2, "extra"));
}

#[test]
fn rerun_is_allowed_only_for_console_and_stdin() {
    let d = tempfile::tempdir().unwrap();
    let ok = program(d.path(), "ok.wj", "use std::fs\n\nfn main() {\n    let t = fs::read_to_string(\"/dev/stdin\").unwrap_or(\"\")\n    println(\"${t.len()}\")\n}\n");
    assert_eq!(windjammer::cli::verify::rerun_blockers(&ok), Ok(()));
    let other = program(d.path(), "other.wj", "use std::fs\n\nfn main() {\n    let t = fs::read_to_string(\"data.txt\").unwrap_or(\"\")\n    println(\"${t.len()}\")\n}\n");
    assert_eq!(windjammer::cli::verify::rerun_blockers(&other), Err(vec!["fs.read".to_string()]));
    let env = program(d.path(), "env.wj", "use std::env\n\nfn main() {\n    println(\"${env::var(\"HOME\").unwrap_or(\"\")}\")\n}\n");
    assert!(windjammer::cli::verify::rerun_blockers(&env).is_err());
}

/// Pilot 6 caveat: `--verify` started with an open pipe nobody writes to (an agent harness that does not redirect
/// input) waited until the pipe closed. Rule: no data and no end of input within the wait means empty stdin.
#[test]
fn an_open_silent_stdin_does_not_block_and_slow_input_is_read_whole() {
    use std::time::{Duration, Instant};
    let (reader, writer) = std::io::pipe().unwrap();
    let start = Instant::now();
    let got = windjammer::cli::verify::capture_from(reader, Duration::from_millis(300));
    assert!(got.is_empty());
    assert!(start.elapsed() < Duration::from_secs(2), "waited {:?}", start.elapsed());
    drop(writer);

    // the first byte inside the wait, the rest well after it: everything is read
    let (reader, mut writer) = std::io::pipe().unwrap();
    let w = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        writer.write_all(b"a").unwrap();
        std::thread::sleep(Duration::from_millis(600));
        writer.write_all(b"bc\n").unwrap();
    });
    let got = windjammer::cli::verify::capture_from(reader, Duration::from_millis(300));
    w.join().unwrap();
    assert_eq!(got, b"abc\n");
}

#[test]
fn verify_with_an_open_unwritten_stdin_finishes() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path(), "quiet.wj", "fn main() {\n    println(\"no input needed\")\n}\n");
    let mut child = Command::new(WJ)
        .args(["run", "--verify"])
        .arg(&f)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _keep_open = child.stdin.take();
    let o = child.wait_with_output().unwrap();
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(String::from_utf8_lossy(&o.stdout), "no input needed\n", "{err}");
    assert!(err.contains("no input arrived on stdin within 2 s"), "{err}");
    assert_eq!(o.status.code(), Some(0), "{err}");
}

/// The interpreted run's stdout reaches ours while the program runs, not when it ends.
#[test]
fn interpreted_stdout_streams_while_the_program_runs() {
    use std::time::{Duration, Instant};
    struct Stamped(Vec<(Instant, Vec<u8>)>);
    impl Write for Stamped {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.push((Instant::now(), b.to_vec()));
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let d = tempfile::tempdir().unwrap();
    let f = program(
        d.path(),
        "slow.wj",
        "use std::thread\nuse std::time::Duration\n\nfn main() {\n    println(\"first\")\n    thread::sleep(Duration::from_millis(1500))\n    println(\"second\")\n}\n",
    );
    let mut cmd = Command::new(WJ);
    cmd.args(["run", "--interpret"]).arg(&f).current_dir(d.path()).stderr(Stdio::piped());
    let mut sink = Stamped(Vec::new());
    let started = Instant::now();
    let o = windjammer::cli::verify::run_streaming(cmd, Some(b""), &mut sink).unwrap();
    let done = Instant::now();
    assert!(o.status.success());
    assert_eq!(String::from_utf8_lossy(&o.stdout), "first\nsecond\n", "the kept copy is the whole stdout");
    let at = |needle: &str| sink.0.iter().find(|(_, b)| String::from_utf8_lossy(b).contains(needle)).map(|(t, _)| *t).expect(needle);
    let (first, second) = (at("first"), at("second"));
    assert!(first < second, "order of arrival");
    assert!(second.duration_since(first) >= Duration::from_millis(1000), "`first` arrived {:?} before `second`", second.duration_since(first));
    assert!(done.duration_since(first) >= Duration::from_millis(1000), "`first` was in hand long before the run ended");
    assert!(first.duration_since(started) < done.duration_since(started), "sanity");
}

/// Output printed before a panic arrives, in order, and the panic keeps its exit status and message.
#[test]
fn a_program_that_prints_then_panics_streams_its_output_and_keeps_the_contract() {
    let d = tempfile::tempdir().unwrap();
    let f = program(d.path(), "boom.wj", "fn main() {\n    println(\"before\")\n    panic(\"boom now\")\n}\n");
    let mut cmd = Command::new(WJ);
    cmd.args(["run", "--interpret"]).arg(&f).current_dir(d.path()).stderr(Stdio::piped());
    let mut sink: Vec<u8> = Vec::new();
    let o = windjammer::cli::verify::run_streaming(cmd, Some(b""), &mut sink).unwrap();
    assert_eq!(String::from_utf8_lossy(&sink), "before\n");
    assert_eq!(o.status.code(), Some(101));
    // and through `wj run --verify`: stdout once, the panic on stderr, status 101, and no build (docs/economics C1)
    let (out, err, code) = verify(&f, "");
    assert_eq!(out, "before\n", "{err}");
    assert_eq!(code, Some(101), "a panic ends the verification with the panic's status: {err}");
    assert!(err.contains("boom now"), "{err}");
    assert!(err.contains("the interpreted run panicked"), "{err}");
    assert!(!err.contains("compiled build") && !err.contains("compiled run"), "no build after a panic: {err}");
}

/// The rule itself: only a panic stops the loop before the build; a program's own non-zero exit is still compared.
#[test]
fn only_a_panic_stops_before_the_build() {
    use windjammer::cli::verify::{stops_before_build, PANIC_EXIT_STATUS};
    assert!(stops_before_build(PANIC_EXIT_STATUS));
    for code in [0, 1, 2, 3, 100, 102, 126] {
        assert!(!stops_before_build(code), "exit status {code}");
    }
}
