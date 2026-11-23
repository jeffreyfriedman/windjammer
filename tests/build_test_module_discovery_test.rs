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

// ---- no test file is silently unrun ----------------------------------------------------------------------------
// `Cargo.toml` sets `autotests = false`, so a file runs only as a module of `all` (see the rule above) or through a
// `[[test]]` entry. A file under `tests/` that carries a test attribute the rule does not see (`#[tokio::test]`,
// `#[test_case]`, ..., in a sub-directory) and has no `[[test]]` entry never runs. This gate lists those files.

/// Test files under `root` that neither become a module of `all` nor appear in `registered` (paths relative to `root`).
fn unrun_test_files(root: &Path, registered: &[String]) -> Vec<String> {
    fn walk(dir: &Path, rel: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let rel_path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            let path = entry.path();
            if path.is_dir() {
                if !name.starts_with('.') && !test_modules::NON_TEST_DIRS.contains(&name.as_str()) {
                    walk(&path, &rel_path, out);
                }
            } else if rel_path.ends_with(".rs") && !rel_path.ends_with(".expected.rs") {
                out.push(rel_path);
            }
        }
    }
    let discovered: Vec<String> = discover_test_modules(root).unwrap().into_iter().map(|m| m.rel_path).collect();
    let mut files = Vec::new();
    walk(root, "", &mut files);
    files.sort();
    files
        .into_iter()
        .filter(|f| f != "all.rs" && !discovered.contains(f) && !registered.contains(f))
        .filter(|f| {
            let src = fs::read_to_string(root.join(f)).unwrap_or_default();
            src.contains("#[test") || src.contains("::test]") || src.contains("::test(") || src.contains("test_case")
        })
        .collect()
}

#[test]
fn every_test_file_runs_in_all_or_has_a_test_target() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo = fs::read_to_string(manifest.join("Cargo.toml")).unwrap();
    let registered: Vec<String> = cargo
        .lines()
        .filter_map(|l| l.trim().strip_prefix("path = \"tests/"))
        .filter_map(|l| l.strip_suffix('"'))
        .map(String::from)
        .collect();
    let unrun = unrun_test_files(&manifest.join("tests"), &registered);
    assert!(
        unrun.is_empty(),
        "these files under tests/ carry tests but never run (Cargo.toml has autotests = false): {unrun:?}. \
         Name a sub-directory file so it holds a plain #[test], or add a [[test]] entry with its path."
    );
}

#[test]
fn an_unregistered_test_file_fails_the_gate() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "a_test.rs", "#[test] fn t() {}");
    write(tmp.path(), "sub/async_case.rs", "#[tokio::test] async fn t() {}");
    write(tmp.path(), "sub/helper.rs", "pub fn shared() {}");
    assert_eq!(unrun_test_files(tmp.path(), &[]), vec!["sub/async_case.rs".to_string()]);
    assert!(unrun_test_files(tmp.path(), &["sub/async_case.rs".to_string()]).is_empty());
}
