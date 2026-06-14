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
