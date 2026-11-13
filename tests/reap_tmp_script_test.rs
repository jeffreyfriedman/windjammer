//! `scripts/reap-tmp.sh --selftest`: the temp-dir reaper removes dead-owner idle scratch dirs and old `.tmp*` dirs,
//! keeps live-owner, recent and foreign ones, and its `--disk` mode returns 0 / 10 / 20. The self-test runs in a private
//! base under a scratch `TMPDIR` and must leave that base empty.

#[path = "common/scratch.rs"]
mod scratch;

#[cfg(unix)]
#[test]
fn reap_tmp_script_selftest_passes_and_leaves_nothing_behind() {
    let tmp = scratch::Scratch::new("reapself");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/reap-tmp.sh");
    let out = std::process::Command::new("bash").arg(&script).arg("--selftest").env("TMPDIR", tmp.path()).output().expect("run bash");
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success() && stdout.contains("checks passed"), "self-test failed:\n{stdout}\n{stderr}");
    let left: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().flatten().map(|e| e.file_name()).collect();
    assert!(left.is_empty(), "self-test left files in its TMPDIR: {left:?}");
}
