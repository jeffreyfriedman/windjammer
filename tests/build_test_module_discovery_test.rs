//! The rule build.rs uses to turn files under `tests/` into modules of the `all` binary.

#[path = "../build_support/test_modules.rs"]
mod test_modules;

use std::fs;
use std::path::Path;
use test_modules::{discover_test_modules, TestModule};

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, body).unwrap();
}

fn names(root: &Path) -> Vec<String> {
    discover_test_modules(root).unwrap().into_iter().map(|m| m.name).collect()
}

#[test]
fn top_level_files_are_modules_except_the_entry_point() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "all.rs", "");
    write(tmp.path(), "a_test.rs", "#[test] fn t() {}");
    write(tmp.path(), "notes.txt", "x");
    assert_eq!(names(tmp.path()), vec!["a_test"]);
}

#[test]
fn subdirectory_files_with_tests_become_path_named_modules() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "codegen/backend/emit_test.rs", "#[test] fn t() {}");
    let found = discover_test_modules(tmp.path()).unwrap();
    assert_eq!(
        found,
        vec![TestModule {
            name: "codegen__backend__emit_test".into(),
            rel_path: "codegen/backend/emit_test.rs".into()
        }]
    );
}

#[test]
fn helpers_data_and_snapshots_are_not_modules() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "common/helpers.rs", "#[test] fn t() {}");
    write(tmp.path(), "fixtures/case.rs", "#[test] fn t() {}");
    write(tmp.path(), "snapshots/x.expected.rs", "#[test] fn t() {}");
    write(tmp.path(), "sub/helper_only.rs", "pub fn help() {}");
    write(tmp.path(), ".hidden/h.rs", "#[test] fn t() {}");
    assert!(names(tmp.path()).is_empty());
}

#[test]
fn colliding_module_names_are_a_hard_error() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a__b.rs", "#[test] fn t() {}");
    write(tmp.path(), "a/b.rs", "#[test] fn t() {}");
    let err = discover_test_modules(tmp.path()).unwrap_err();
    assert!(err.contains("a__b") && err.contains("a/b.rs"), "{err}");
}
