//! Shared helpers for the test runner (paths, directory copies).

use std::path::Path;

/// Convert a path to TOML-safe format (forward slashes, no Windows \\?\ prefix)
/// Windows canonicalize() adds \\?\ prefix; backslashes cause TOML parse errors
pub fn path_to_toml_string(path: &Path) -> String {
    let s = path.display().to_string();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    s.replace('\\', "/")
}

/// Recursively copy a directory (lives in `build_utils`, which the library needs without the CLI).
pub use crate::build_utils::copy_dir_recursive;
