//! The shared cargo-target cache is pruned automatically (`cargo_cache::reclaim`, low disk or over budget). A
//! prune must never delete what an active build holds: a build holds its target tree for the length of its cargo
//! run (`cargo_cache::active::hold`), and a prune skips a held tree, and anything modified within the last hour.
//! `wj cache prune --force` (`PruneOptions::force`) still clears everything.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};
use windjammer::cargo_cache::{self, active, PruneOptions, TargetKind};

fn file(p: &Path, len: usize, age: Duration) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, vec![0u8; len]).unwrap();
    let f = fs::File::options().write(true).open(p).unwrap();
    f.set_modified(SystemTime::now() - age).unwrap();
}

/// A verify tree and a shared tree with incremental state, all old enough to prune.
fn populate(root: &Path) {
    let old = Duration::from_secs(3 * 3600);
    file(&root.join("cargo-target/verify/debug/deps/libx.rlib"), 1024, old);
    file(&root.join("cargo-target/shared/debug/incremental/x-1/s.bin"), 1024, old);
    file(&root.join("cargo-target/shared/release/deps/liby.rlib"), 1024, old);
}

fn aggressive(force: bool) -> PruneOptions {
    PruneOptions { aggressive: true, auto_low_disk: true, force, ..PruneOptions::default() }
}

#[test]
fn a_prune_skips_trees_that_builds_hold_and_recent_files() {
    let _scr = Scratch::new("cache-active");
    let root = _scr.path().to_path_buf();
    let _ = fs::remove_dir_all(&root);
    std::env::set_var("WJ_CARGO_TARGET_ROOT", &root);
    let verify = cargo_cache::target_dir(TargetKind::Verify);
    let shared = cargo_cache::target_dir(TargetKind::Shared);

    // two builds running: one on each tree
    populate(&root);
    let a = active::hold(TargetKind::Verify).expect("verify lock");
    let b = active::hold(TargetKind::Shared).expect("shared lock");
    let report = cargo_cache::prune(aggressive(false));
    assert!(verify.join("debug/deps/libx.rlib").exists(), "the held verify tree was deleted: {:?}", report.removed_paths);
    assert!(shared.join("debug/incremental/x-1/s.bin").exists(), "the held shared incremental state was deleted");
    assert!(report.messages.iter().any(|m| m.contains("in use")), "{:?}", report.messages);

    // one build finished: its tree goes, the other stays
    drop(a);
    cargo_cache::prune(aggressive(false));
    assert!(!verify.exists(), "the released verify tree survived");
    assert!(shared.join("debug/incremental/x-1/s.bin").exists(), "the still-held shared tree was pruned");
    drop(b);

    // nothing held, but a file written within the hour: its tree stays
    populate(&root);
    file(&verify.join("debug/deps/libnew.rlib"), 10, Duration::from_secs(60));
    cargo_cache::prune(aggressive(false));
    assert!(verify.join("debug/deps/libnew.rlib").exists(), "a tree written a minute ago was deleted");

    // mutation check: --force ignores both guards and clears what is held
    let held = active::hold(TargetKind::Verify).expect("verify lock");
    cargo_cache::prune(aggressive(true));
    assert!(!verify.join("debug/deps/libx.rlib").exists(), "--force kept a held tree: the guard is not what kept it");
    drop(held);

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_size_budget_never_removes_files_of_a_held_tree_or_recent_files() {
    let _scr = Scratch::new("cache-budget");
    let root = _scr.path().to_path_buf();
    let _ = fs::remove_dir_all(&root);
    let tree = root.join("cargo-target");
    file(&tree.join("release/deps/old.rlib"), 4096, Duration::from_secs(3 * 3600));
    file(&tree.join("release/deps/fresh.rlib"), 4096, Duration::from_secs(60));
    // over a 1-byte budget: only the old file may go
    let mut report = cargo_cache::PruneReport::default();
    cargo_cache::enforce_size_budget_for_test(&tree, 1, &mut report);
    assert!(!tree.join("release/deps/old.rlib").exists());
    assert!(tree.join("release/deps/fresh.rlib").exists(), "a file written a minute ago was removed");
    // held: nothing goes
    file(&tree.join("release/deps/old2.rlib"), 4096, Duration::from_secs(3 * 3600));
    let held = active::hold_path(&tree).expect("lock");
    cargo_cache::enforce_size_budget_for_test(&tree, 1, &mut report);
    assert!(tree.join("release/deps/old2.rlib").exists(), "the budget removed a file of a held tree");
    drop(held);
    let _ = fs::remove_dir_all(&root);
}
