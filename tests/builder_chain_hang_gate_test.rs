//! P0: long consuming builder chains must not hang the compiler.
//!
//! Root cause of the first hang: `infer_expression_type` on MethodCall re-inferred the receiver
//! three times per link (~3^depth). Depth ~32 in a long builder chain hung codegen. The second time
//! (stream8's `ref_default_call_type`) the same class returned through `wj check`, which every
//! `wj run --interpret` also runs: it inferred the receiver of every one-argument call with a text
//! argument before asking whether the method could qualify. Both commands are gated here, each in a
//! child process with a hard deadline, so a regression fails fast instead of hanging a suite.
//! Fixture uses a ~32-call chain; budget 15s.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn wj_bin() -> PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_wj"))
}

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/builder_chain_scenario_presets_hang.wj")
}

/// Run `wj args...` and wait at most `deadline`; panics (after killing the child) on a hang, and when the command fails
/// or is slower than the budget. Returns the elapsed time.
fn run_within(what: &str, args: &[&str], deadline: Duration) -> Duration {
    let wj = wj_bin();
    assert!(wj.is_file(), "wj binary missing at {} — build windjammer first", wj.display());
    let cache = tempfile::tempdir().expect("tempdir");
    let start = Instant::now();
    let mut child = Command::new(&wj)
        .args(args)
        .env("WJ_CARGO_TARGET_ROOT", cache.path())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn wj");
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                let elapsed = start.elapsed();
                let mut stderr = String::new();
                if let Some(mut err) = child.stderr.take() {
                    use std::io::Read;
                    let _ = err.read_to_string(&mut stderr);
                }
                assert!(status.success(), "{what} failed in {elapsed:?}: {stderr}");
                assert!(elapsed < deadline, "{what}: builder-chain fixture too slow: {elapsed:?} (budget {deadline:?}), possible hang regression");
                return elapsed;
            }
            None if start.elapsed() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("HANG: `{what}` on the builder-chain fixture exceeded {deadline:?} (long builder chain class)");
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

#[test]
fn consuming_builder_chain_fixture_must_transpile_under_15s() {
    let out = tempfile::tempdir().expect("tempdir");
    let fixture = fixture();
    run_within("wj build", &["build", fixture.to_str().unwrap(), "-o", out.path().to_str().unwrap(), "--no-cargo"], Duration::from_secs(15));
}

/// `wj check` is also the gate of every interpreted run, so a hang here is a hang of `wj run`.
#[test]
fn consuming_builder_chain_fixture_must_check_under_15s() {
    let fixture = fixture();
    run_within("wj check", &["check", fixture.to_str().unwrap()], Duration::from_secs(15));
}
