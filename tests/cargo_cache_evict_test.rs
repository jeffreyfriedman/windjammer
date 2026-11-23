//! The cache size budget must never corrupt a cargo target. Deleting single files out of one (a build script's
//! `bindgen.rs`, one `.rlib`) leaves cargo's fingerprints saying "fresh" over missing outputs (E0463, missing
//! `bindgen.rs`, linker errors). Eviction is by whole unit, only inside trees wj marked as its own.
//! Design: private-docs/design/shared-target-hazard.md, cause A.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use windjammer::cargo_cache::{self, PruneReport, TargetKind};

const MARKER: &str = ".wj-cache-root";
const SQLITE: &str = "libsqlite3-sys-89a4fa35d4939027";

fn old_file(p: &Path, len: usize) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, vec![7u8; len]).unwrap();
    let f = fs::File::options().write(true).open(p).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(48 * 3600)).unwrap();
}

/// A cargo-shaped profile with the sqlite3 build-script unit and one more unit, all two days old.
fn seed_cargo_target(tree: &Path) -> Vec<PathBuf> {
    let p = tree.join("debug");
    let files = vec![
        p.join(format!("build/{SQLITE}/out/bindgen.rs")),
        p.join(format!("build/{SQLITE}/out/sqlite3.o")),
        p.join(format!("build/{SQLITE}/out/libsqlite3.a")),
        p.join(format!(".fingerprint/{SQLITE}/run-build-script-build-script-build")),
        p.join("deps/libsqlite3_sys-89a4fa35d4939027.rlib"),
        p.join("deps/libserde-0123456789abcdef.rlib"),
        p.join(".fingerprint/serde-0123456789abcdef/lib-serde"),
    ];
    for f in &files {
        old_file(f, 4096);
    }
    files
}

/// For every unit (the hash its files carry), either all of its files are present or none: nothing is half deleted.
fn assert_units_whole(files: &[PathBuf]) {
    for hash in ["89a4fa35d4939027", "0123456789abcdef"] {
        let of: Vec<&PathBuf> = files.iter().filter(|f| f.to_string_lossy().contains(hash)).collect();
        let present = of.iter().filter(|f| f.exists()).count();
        assert!(present == 0 || present == of.len(), "unit {hash} is partially deleted: {present} of {} files left", of.len());
    }
}

#[test]
fn a_sweep_never_leaves_a_unit_partly_deleted() {
    let scr = Scratch::new("evict-whole");
    let tree = scr.join("cargo-target/shared");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join(MARKER), b"").unwrap();
    let files = seed_cargo_target(&tree);
    // a budget in the middle: some units go, some stay, none is cut
    for budget in [0u64, 9000, 16000, 20000, 1 << 30] {
        let mut report = PruneReport::default();
        cargo_cache::enforce_size_budget_for_test(&tree, budget, &mut report);
        assert_units_whole(&files);
    }
    // budget 0: everything old went, as whole units
    assert!(!tree.join(format!("debug/build/{SQLITE}")).exists());
}

#[test]
fn bindgen_rs_is_not_deleted_alone() {
    let scr = Scratch::new("evict-bindgen");
    let tree = scr.join("cargo-target/shared");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join(MARKER), b"").unwrap();
    let files = seed_cargo_target(&tree);
    let bindgen = files[0].clone();
    let sqlite_o = files[1].clone();
    // a budget that only needs one file's worth of space (the oldest file used to be bindgen.rs)
    let size: u64 = cargo_cache::dir_size(&tree);
    let mut report = PruneReport::default();
    cargo_cache::enforce_size_budget_for_test(&tree, size - 1, &mut report);
    assert_eq!(bindgen.exists(), sqlite_o.exists(), "bindgen.rs and sqlite3.o must go together or stay together");
    assert_units_whole(&files);
}

#[test]
fn a_tree_without_the_marker_is_untouched() {
    let scr = Scratch::new("evict-unmarked");
    // what an agent's compiler build looks like: a cargo target wj did not create
    let tree = scr.join("cargo-target/debug-from-cargo");
    fs::create_dir_all(&tree).unwrap();
    let files = seed_cargo_target(&tree);
    let mut report = PruneReport::default();
    let freed = cargo_cache::enforce_size_budget_for_test(&tree, 0, &mut report);
    assert_eq!(freed, 0);
    assert!(files.iter().all(|f| f.exists()), "an unmarked tree lost files");
    assert_eq!(report.messages.len(), 1, "one warning line expected: {:?}", report.messages);
    assert!(report.messages[0].contains("not swept"), "{:?}", report.messages);
}

#[test]
fn eviction_by_whole_unit_frees_space_in_a_wj_cache() {
    let scr = Scratch::new("evict-frees");
    let tree = scr.join("cargo-target/shared");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join(MARKER), b"").unwrap();
    seed_cargo_target(&tree);
    let before = cargo_cache::dir_size(&tree);
    let mut report = PruneReport::default();
    let freed = cargo_cache::enforce_size_budget_for_test(&tree, 0, &mut report);
    let after = cargo_cache::dir_size(&tree);
    assert!(freed >= 7 * 4096, "freed only {freed}");
    assert!(after < before && before - after >= freed);
}

#[test]
fn an_unevictable_overage_warns_once_and_deletes_nothing() {
    let scr = Scratch::new("evict-warn");
    let tree = scr.join("cargo-target/shared");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join(MARKER), b"").unwrap();
    // data that is no cargo unit (no hash) is not ours to cut
    let loose = tree.join("debug/deps/notes.txt");
    old_file(&loose, 4096);
    fs::create_dir_all(tree.join("debug/.fingerprint")).unwrap();
    let mut report = PruneReport::default();
    cargo_cache::enforce_size_budget_for_test(&tree, 0, &mut report);
    assert!(loose.exists());
    assert_eq!(report.messages.len(), 1, "{:?}", report.messages);
    assert!(report.messages[0].contains("nothing more can be evicted safely"));
}

/// wj writes the marker itself into the trees it creates. Child process: it alone sees the private cache root.
#[test]
fn wj_marks_the_trees_it_creates() {
    let scr = Scratch::new("evict-marker");
    let root = scr.join("root");
    scratch::run_ignored_test_in_child(module_path!(), "marker_body", &[("WJ_CARGO_TARGET_ROOT", root.as_os_str())]);
}

#[test]
#[ignore = "child body of wj_marks_the_trees_it_creates"]
fn marker_body() {
    if !scratch::in_test_child() {
        return;
    }
    let shared = cargo_cache::target_dir(TargetKind::Shared);
    let verify = cargo_cache::target_dir(TargetKind::Verify);
    assert!(shared.join(MARKER).is_file(), "shared tree has no marker");
    assert!(verify.join(MARKER).is_file(), "verify tree has no marker");
    // the compiler's own build dirs next to them carry none
    assert!(!shared.parent().unwrap().join(MARKER).exists());
}
