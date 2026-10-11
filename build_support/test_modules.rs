//! Test-module discovery for the consolidated `tests/all.rs` binary.
//!
//! Shared by `build.rs` (which generates the module list) and
//! `tests/build_test_module_discovery_test.rs` (which checks the rule), via `#[path]`.
//!
//! Rule:
//!   * every top-level `tests/*.rs` except `all.rs` is a module named after its file stem;
//!   * every `.rs` file in a sub-directory that contains a `#[test]` is a module named after its
//!     path (`sub/dir/file.rs` -> `sub__dir__file`);
//!   * `common/` and `fixtures/` (shared helpers and data), hidden directories and
//!     `*.expected.rs` snapshot data are never modules;
//!   * two files that map to the same module name are an error, never a silent overwrite.

use std::fs;
use std::path::Path;

/// The generated entry point itself; never a module.
pub const TEST_BINARY_ENTRY: &str = "all";
/// Directories holding helpers/data, not tests.
pub const NON_TEST_DIRS: [&str; 2] = ["common", "fixtures"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestModule {
    /// Rust module name inside the generated list.
    pub name: String,
    /// Path relative to the tests directory, `/`-separated.
    pub rel_path: String,
}

pub fn discover_test_modules(tests_dir: &Path) -> Result<Vec<TestModule>, String> {
    let mut modules = Vec::new();
    walk(tests_dir, "", &mut modules)?;
    modules.sort_by(|a, b| a.name.cmp(&b.name).then(a.rel_path.cmp(&b.rel_path)));
    for pair in modules.windows(2) {
        if pair[0].name == pair[1].name {
            return Err(format!(
                "test module name `{}` is produced by both `{}` and `{}`; rename one of them",
                pair[0].name, pair[0].rel_path, pair[1].rel_path
            ));
        }
    }
    Ok(modules)
}

fn walk(dir: &Path, rel: &str, out: &mut Vec<TestModule>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let rel_path = if rel.is_empty() { file_name.clone() } else { format!("{rel}/{file_name}") };
        if path.is_dir() {
            if file_name.starts_with('.') || NON_TEST_DIRS.contains(&file_name.as_str()) {
                continue;
            }
            walk(&path, &rel_path, out)?;
        } else if let Some(module) = module_for_file(&path, &rel_path, rel.is_empty()) {
            out.push(module);
        }
    }
    Ok(())
}

fn module_for_file(path: &Path, rel_path: &str, top_level: bool) -> Option<TestModule> {
    let stem = rel_path.strip_suffix(".rs")?;
    if rel_path.ends_with(".expected.rs") {
        return None;
    }
    if top_level {
        if stem == TEST_BINARY_ENTRY {
            return None;
        }
        return Some(TestModule { name: stem.to_string(), rel_path: rel_path.to_string() });
    }
    let source = fs::read_to_string(path).ok()?;
    if !source.contains("#[test]") {
        return None; // helper module pulled in explicitly via #[path]
    }
    let name: String = stem
        .split('/')
        .collect::<Vec<_>>()
        .join("__")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    Some(TestModule { name, rel_path: rel_path.to_string() })
}
