//! Shared naming rules for auto-discovered test modules (`#[cfg(test)]`).

/// True when a module name represents a test module that should be gated with `#[cfg(test)]`
/// and auto-included even when mod.wj lists explicit `pub mod` declarations.
pub fn is_test_module(name: &str) -> bool {
    name == "tests"
        || name == "test_runtime"
        || name == "test_output"
        || name == "test_plugins"
        || name == "tests_build"
        || name.ends_with("_test")
        || name.ends_with("_tests")
        || name.starts_with("test_")
}

/// Directory names whose `.wj` files are test sources regardless of file name.
const TEST_DIR_NAMES: [&str; 2] = ["tests", "tests_wj"];

/// True when `path` (a `.wj` file) is a test module by location or name: it lives under a
/// `tests/` (or `tests_wj/`) directory, or its file stem satisfies [`is_test_module`].
///
/// This is the single rule shared by `wj test` discovery and module-system gating.
pub fn is_test_source_path(path: &std::path::Path) -> bool {
    if path.extension().is_none_or(|e| e != "wj") {
        return false;
    }
    let in_test_dir = path
        .parent()
        .into_iter()
        .flat_map(|p| p.components())
        .any(|c| TEST_DIR_NAMES.contains(&c.as_os_str().to_string_lossy().as_ref()));
    let stem_is_test = path
        .file_stem()
        .is_some_and(|s| is_test_module(&s.to_string_lossy()));
    in_test_dir || stem_is_test
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_prefix_and_suffix_names_are_test_modules() {
        for p in ["a/test_x.wj", "a/x_test.wj", "a/x_tests.wj", "tests.wj"] {
            assert!(is_test_source_path(Path::new(p)), "{p}");
        }
    }

    #[test]
    fn files_under_tests_dirs_are_test_sources() {
        assert!(is_test_source_path(Path::new("pkg/tests/helpers.wj")));
        assert!(is_test_source_path(Path::new("pkg/tests_wj/a/b.wj")));
    }

    #[test]
    fn ordinary_sources_and_non_wj_are_not() {
        assert!(!is_test_source_path(Path::new("pkg/src/math.wj")));
        assert!(!is_test_source_path(Path::new("pkg/tests/x.rs")));
        assert!(!is_test_source_path(Path::new("pkg/src/latest.wj")));
    }
}
