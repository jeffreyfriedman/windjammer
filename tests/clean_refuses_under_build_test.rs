//! `wj clean --all` and `wj cache prune --force` delete the shared cargo target tree, so they refuse while a build
//! holds it (the build's `flock` on `<tree>/.wj-active.lock`, see `cargo_cache::active`): a deleted tree under a
//! running cargo fails the build with "could not write output to ...: No such file or directory". The build here is a
//! child process (this test binary re-run on `holder_child`), so the lock is held by another process, as in real use.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

fn tree(root: &Path) -> PathBuf {
    root.join("cargo-target").join("shared")
}

/// The "build": holds the tree lock until the `release` file appears.
#[test]
fn holder_child() {
    let Ok(dir) = std::env::var("WJ_TEST_HOLD_DIR") else { return };
    let dir = PathBuf::from(dir);
    let _lock = windjammer::cargo_cache::active::hold_path(&tree(&dir.join("cache"))).expect("hold the tree");
    std::fs::write(dir.join("ready"), "").unwrap();
    let t = Instant::now();
    while !dir.join("release").exists() && t.elapsed() < Duration::from_secs(60) {
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn_holder(dir: &Path) -> Child {
    let name = module_path!().split_once("::").map(|x| format!("{}::holder_child", x.1)).unwrap_or_else(|| "holder_child".into());
    let child = Command::new(std::env::current_exe().unwrap())
        .args([name.as_str(), "--exact", "--nocapture", "--test-threads=1"])
        .env("WJ_TEST_HOLD_DIR", dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let t = Instant::now();
    while !dir.join("ready").exists() {
        assert!(t.elapsed() < Duration::from_secs(30), "the holder child never took the lock");
        std::thread::sleep(Duration::from_millis(20));
    }
    child
}

fn wj(dir: &Path, args: &[&str]) -> Command {
    // the binary under test, named at run time when a gate publishes a private copy (see installed_sandbox::real_wj)
    let bin = std::env::var("CARGO_BIN_EXE_wj").unwrap_or_else(|_| env!("CARGO_BIN_EXE_wj").to_string());
    let mut c = Command::new(bin);
    c.args(args).current_dir(dir).env_clear().env("HOME", dir.join("home")).env("WJ_CARGO_TARGET_ROOT", dir.join("cache")).env("NO_COLOR", "1");
    c
}

fn setup() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("home")).unwrap();
    std::fs::create_dir_all(tree(&t.path().join("cache")).join("debug")).unwrap();
    std::fs::write(tree(&t.path().join("cache")).join("debug/dummy"), "x").unwrap();
    t
}

fn refused(o: &Output, pid: u32) {
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(!o.status.success(), "must refuse while a build holds the cache: {o:?}");
    assert!(err.contains(&format!("a build is using the shared cache (pid {pid}): finish it or run with --wait")), "stderr: {err}");
}

#[test]
fn clean_all_and_prune_force_refuse_while_a_build_holds_the_cache_then_succeed() {
    let t = setup();
    let mut build = spawn_holder(t.path());
    let pid = build.id();
    for args in [&["clean", "--all"][..], &["cache", "prune", "--force"][..]] {
        refused(&wj(t.path(), args).output().unwrap(), pid);
        assert!(tree(&t.path().join("cache")).join("debug/dummy").exists(), "`wj {args:?}` deleted the tree under a build");
    }
    // the build ends: the same commands now run
    std::fs::write(t.path().join("release"), "").unwrap();
    build.wait().unwrap();
    let o = wj(t.path(), &["clean", "--all"]).output().unwrap();
    assert!(o.status.success(), "{o:?}");
    assert!(!tree(&t.path().join("cache")).exists(), "clean --all must delete the tree once no build holds it");
}

#[test]
fn wait_blocks_until_the_build_ends() {
    let t = setup();
    let mut build = spawn_holder(t.path());
    let mut waiter = wj(t.path(), &["clean", "--all", "--wait"]).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    assert!(waiter.try_wait().unwrap().is_none(), "--wait must still be waiting while the build runs");
    assert!(tree(&t.path().join("cache")).join("debug/dummy").exists());
    std::fs::write(t.path().join("release"), "").unwrap();
    build.wait().unwrap();
    assert!(waiter.wait().unwrap().success());
    assert!(!tree(&t.path().join("cache")).exists());
}

#[test]
fn a_killed_build_releases_the_cache() {
    let t = setup();
    let mut build = spawn_holder(t.path());
    build.kill().unwrap();
    build.wait().unwrap();
    assert!(wj(t.path(), &["cache", "prune", "--force"]).output().unwrap().status.success());
}
