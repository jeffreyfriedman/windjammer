//! Discovering test sources under a package root.
//!
//! Rule (single source of truth, shared with `crate::test_module_gate`): every `.wj` file under
//! the root (skipping hidden, `target`, `build`, `node_modules`) is scanned. A file contributes
//! tests when it parses and contains at least one test function (see `test_extraction`).
//! Files that are test modules by name/location are always parsed; other files are parsed only
//! when their text mentions `@test` or `test_`, so unrelated sources cannot break discovery.
//! Per-file failures never abort the scan; they are returned in [`Discovery::errors`].

use super::test_extraction::{extract_tests, TestFunction};
use crate::test_module_gate::is_test_source_path;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const SKIPPED_DIRS: [&str; 4] = ["target", "build", "node_modules", ".git"];

/// A source file that contains tests.
#[derive(Debug, Clone)]
pub(crate) struct DiscoveredFile {
    pub(crate) path: PathBuf,
    /// Unique Rust module name for the compiled file (see [`assign_module_names`]).
    pub(crate) module_name: String,
    pub(crate) tests: Vec<TestFunction>,
}

/// A file that could not be read or parsed.
#[derive(Debug, Clone)]
pub(crate) struct FileError {
    pub(crate) path: PathBuf,
    pub(crate) message: String,
}

/// Result of scanning a package for tests.
#[derive(Debug, Default)]
pub(crate) struct Discovery {
    pub(crate) files: Vec<DiscoveredFile>,
    pub(crate) errors: Vec<FileError>,
}

impl Discovery {
    pub(crate) fn test_count(&self) -> usize {
        self.files.iter().map(|f| f.tests.len()).sum()
    }

    pub(crate) fn all_tests(&self) -> Vec<TestFunction> {
        self.files.iter().flat_map(|f| f.tests.clone()).collect()
    }

    /// One readable block listing every failing file.
    pub(crate) fn error_summary(&self) -> String {
        let mut out = format!("{} file(s) could not be scanned for tests:\n", self.errors.len());
        for e in &self.errors {
            out.push_str(&format!("  {}: {}\n", e.path.display(), e.message));
        }
        out
    }
}

/// Scan `root` (a directory or a single `.wj` file) for tests.
pub(crate) fn discover(root: &Path) -> Result<Discovery> {
    let mut sources = Vec::new();
    let scan_root = if root.is_file() {
        sources.push(root.to_path_buf());
        root.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else {
        collect_wj_files(root, &mut sources)?;
        root.to_path_buf()
    };
    sources.sort();

    let mut discovery = Discovery::default();
    for path in sources {
        let relative = path.strip_prefix(&scan_root).unwrap_or(&path);
        let must_parse = is_test_source_path(relative);
        match scan_file(&path, must_parse) {
            Ok(tests) if tests.is_empty() => {}
            Ok(tests) => discovery.files.push(DiscoveredFile {
                path,
                module_name: String::new(),
                tests,
            }),
            Err(message) => discovery.errors.push(FileError { path, message }),
        }
    }
    let paths: Vec<PathBuf> = discovery.files.iter().map(|f| f.path.clone()).collect();
    let names = assign_module_names(&paths, &scan_root);
    for (file, name) in discovery.files.iter_mut().zip(names) {
        file.module_name = name;
    }
    Ok(discovery)
}

fn collect_wj_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string());
        let name = name.unwrap_or_default();
        if path.is_dir() {
            if name.starts_with('.') || SKIPPED_DIRS.contains(&name.as_str()) {
                continue;
            }
            collect_wj_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "wj") {
            out.push(path);
        }
    }
    Ok(())
}

fn scan_file(path: &Path, must_parse: bool) -> std::result::Result<Vec<TestFunction>, String> {
    let source = fs::read_to_string(path).map_err(|e| format!("cannot read: {e}"))?;
    if !must_parse && !source.contains("@test") && !source.contains("test_") {
        return Ok(Vec::new());
    }
    parse_tests(&source, path)
}

/// Parse `source` and extract its tests.
pub(crate) fn parse_tests(source: &str, path: &Path) -> std::result::Result<Vec<TestFunction>, String> {
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("parse error: {e}"))?;
    Ok(extract_tests(&program.items, path))
}

/// Collision-safe Rust module names, one per path (same order as `paths`).
///
/// A file keeps its stem when no other file shares it; colliding files get their root-relative
/// path with separators turned into `__` (`a/util_test.wj` -> `a__util_test`).
pub(crate) fn assign_module_names(paths: &[PathBuf], root: &Path) -> Vec<String> {
    let stem = |p: &Path| p.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for p in paths {
        *counts.entry(stem(p)).or_default() += 1;
    }
    paths
        .iter()
        .map(|p| {
            let s = stem(p);
            if counts[&s] == 1 {
                return reserve_module_name(s);
            }
            let rel = p.strip_prefix(root).unwrap_or(p).with_extension("");
            let joined: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();
            reserve_module_name(sanitize_ident(&joined.join("__")))
        })
        .collect()
}

/// The harness crate root is `lib.rs`; a test file named `lib.wj` (the `lib` template's
/// own source) would emit `lib.rs` over it and `pub mod lib;` ("circular modules").
/// Rust path keywords cannot be module names either.
fn reserve_module_name(name: String) -> String {
    match name.as_str() {
        "lib" | "self" | "super" | "crate" | "mod" => format!("wj_{name}"),
        _ => name,
    }
}

fn sanitize_ident(raw: &str) -> String {
    let mut s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s.insert(0, '_');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    #[test]
    fn colliding_stems_get_path_derived_names_and_unique_stems_keep_theirs() {
        let root = Path::new("/p");
        let paths = vec![
            root.join("a/util_test.wj"),
            root.join("b/util_test.wj"),
            root.join("c/solo_test.wj"),
        ];
        let names = assign_module_names(&paths, root);
        assert_eq!(names, vec!["a__util_test", "b__util_test", "solo_test"]);
    }

    #[test]
    fn lib_wj_gets_a_module_name_that_cannot_clobber_the_harness_root() {
        let root = Path::new("/p");
        let names = assign_module_names(&[root.join("src/lib.wj"), root.join("src/mod.wj")], root);
        assert_eq!(names, vec!["wj_lib", "wj_mod"]);
    }

    #[test]
    fn discovers_at_test_in_plain_source_files_and_skips_build_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "src/math.wj", "@test\nfn adds() {}\n");
        write(tmp.path(), "src/plain.wj", "fn helper() {}\n");
        write(tmp.path(), "build/gen_test.wj", "fn test_hidden() {}\n");
        let d = discover(tmp.path()).unwrap();
        let names: Vec<String> = d.all_tests().iter().map(|t| t.display_name()).collect();
        assert_eq!(names, vec!["adds"]);
        assert!(d.errors.is_empty());
    }

    #[test]
    fn parse_error_in_one_file_does_not_hide_the_others() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "tests/ok_test.wj", "fn test_ok() {}\n");
        write(tmp.path(), "tests/bad_test.wj", "fn test_bad( {{{\n");
        let d = discover(tmp.path()).unwrap();
        assert_eq!(d.test_count(), 1);
        assert_eq!(d.errors.len(), 1);
        assert!(d.error_summary().contains("bad_test.wj"));
    }
}
