//! One privacy rule for `wj check`, `wj run --interpret` and `wj build` (WJ0019): a private item (not declared `pub`)
//! may be called, imported or re-exported (`pub use`) only inside its module and that module's descendants.
//!
//! Each `*_private_fn` fixture under `tests/interpreter_projects/` must fail all three commands with the same
//! diagnostic and the fix hint (`declare it `pub fn ...` in <file>`); its `*_public_fn` twin (the same program with the
//! function declared `pub`) must pass all three. The pairs are the gate's own mutation check: removing the `pub`
//! from a passing program must fail, adding it to a failing one must pass.

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/interpreter_projects").join(name)
}

fn run(dir: &Path, args: &[&str]) -> (bool, String) {
    let o = Command::new(WJ).args(args).current_dir(dir).output().unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    (o.status.success(), text)
}

fn three_ways(name: &str) -> Vec<(&'static str, bool, String)> {
    let dir = fixture(name);
    let out = std::env::temp_dir().join(format!("wj-vis-{name}-{}", std::process::id()));
    let out_s = out.to_string_lossy().to_string();
    let mut v = Vec::new();
    for (label, args) in [
        ("check", vec!["check"]),
        ("interpret", vec!["run", "--interpret"]),
        ("build", vec!["build", "src", "--no-cargo", "-o", out_s.as_str()]),
    ] {
        let (ok, text) = run(&dir, &args);
        v.push((label, ok, text));
    }
    let _ = std::fs::remove_dir_all(&out);
    v
}

fn assert_private(name: &str, item: &str, file: &str) {
    for (label, ok, text) in three_ways(name) {
        assert!(!ok, "[{label}] {name} must fail:\n{text}");
        assert!(text.contains("is private"), "[{label}] {name}: no privacy diagnostic:\n{text}");
        assert!(text.contains(item), "[{label}] {name}: diagnostic lacks {item}:\n{text}");
        if label != "interpret" {
            assert!(text.contains("WJ0019"), "[{label}] {name}: code WJ0019 missing:\n{text}");
            assert!(text.contains("declare it `pub fn"), "[{label}] {name}: fix hint missing:\n{text}");
            assert!(text.contains(file), "[{label}] {name}: hint lacks the declaring file {file}:\n{text}");
        }
    }
}

fn assert_clean(name: &str) {
    for (label, ok, text) in three_ways(name) {
        assert!(ok, "[{label}] {name} must pass:\n{text}");
        assert!(!text.contains("WJ0019"), "[{label}] {name}: unexpected WJ0019:\n{text}");
    }
}

#[test]
fn private_fn_reexport_fails_everywhere() {
    assert_private("reexport_private_fn", "mesh_generator::build_mesh", "mesh_generator.wj");
}

#[test]
fn public_fn_reexport_passes_everywhere() {
    assert_clean("reexport_public_fn");
}

#[test]
fn private_fn_import_and_call_fails_everywhere() {
    assert_private("import_private_fn", "mathx::helper", "mathx.wj");
}

#[test]
fn public_fn_import_and_call_passes_everywhere() {
    assert_clean("import_public_fn");
}
