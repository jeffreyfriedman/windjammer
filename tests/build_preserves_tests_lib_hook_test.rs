//! `wj build` regenerates the crate-root `lib.rs`; it must keep (re-add) the `tests/lib.rs`
//! hook for crates that have a generated `tests/lib.rs`, otherwise `cargo test --lib` silently
//! stops running the crate's Rust integration tests.

use std::fs;
use std::path::Path;

const HOOK_BEGIN: &str = "WINDJAMMER_AUTO_RUST_TESTS_BEGIN";

fn crate_with_tests_lib(root: &Path) {
    fs::write(root.join("api.rs"), "pub fn api() -> i32 { 1 }\n").unwrap();
    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(root.join("tests/lib.rs"), "#[cfg(test)]\npub mod it;\n").unwrap();
    fs::write(root.join("tests/it.rs"), "#[test]\nfn works() {}\n").unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"lib.rs\"\n",
    )
    .unwrap();
}

#[test]
fn regenerating_lib_rs_re_adds_the_tests_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    crate_with_tests_lib(root);

    windjammer::build_utils::generate_mod_file(root).unwrap();
    let first = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(first.contains(HOOK_BEGIN), "hook missing after first build:\n{first}");

    // A rebuild overwrites lib.rs from mod.rs again; the hook must survive.
    fs::remove_file(root.join("lib.rs")).unwrap();
    windjammer::build_utils::generate_mod_file(root).unwrap();
    let second = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(second.contains(HOOK_BEGIN), "hook lost on rebuild:\n{second}");
    assert_eq!(second.matches(HOOK_BEGIN).count(), 1, "hook duplicated:\n{second}");
}

#[test]
fn crates_without_tests_lib_get_no_hook() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::write(root.join("api.rs"), "pub fn api() -> i32 { 1 }\n").unwrap();
    windjammer::build_utils::generate_mod_file(root).unwrap();
    let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(!lib.contains(HOOK_BEGIN), "{lib}");
}
