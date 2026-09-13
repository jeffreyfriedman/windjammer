//! P0: long consuming builder chains must not hang the compiler (scenario_presets.wj).
//!
//! Root cause: `infer_expression_type` on MethodCall re-inferred the receiver
//! three times per link (~3^depth). Depth ~32 in scenario_presets hung codegen.
//! Fixture uses a ~32-call chain; budget 15s.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

fn wj_bin() -> PathBuf {
    // Prefer freshly built tip used in this session.
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../windjammer-game/.cargo-target-wj/release/wj");
    if tip.is_file() {
        return tip;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/wj")
}

#[test]
fn consuming_builder_chain_fixture_must_transpile_under_15s() {
    let wj = wj_bin();
    assert!(
        wj.is_file(),
        "wj binary missing at {} — build windjammer first",
        wj.display()
    );

    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builder_chain_scenario_presets_hang.wj");
    let out = tempfile::tempdir().expect("tempdir");

    let start = Instant::now();
    let mut child = Command::new(&wj)
        .args([
            "build",
            fixture.to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--no-cargo",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn wj");

    let deadline = Duration::from_secs(15);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                let elapsed = start.elapsed();
                let stderr = {
                    let mut s = String::new();
                    if let Some(mut err) = child.stderr.take() {
                        use std::io::Read;
                        let _ = err.read_to_string(&mut s);
                    }
                    s
                };
                assert!(
                    status.success(),
                    "wj build failed in {:?}: {}",
                    elapsed,
                    stderr
                );
                assert!(
                    elapsed < deadline,
                    "builder-chain transpile too slow: {:?} (budget {:?}) — possible hang regression",
                    elapsed,
                    deadline
                );
                return;
            }
            None => {
                if start.elapsed() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "HANG: builder-chain fixture exceeded {:?} (scenario_presets class)",
                        deadline
                    );
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
