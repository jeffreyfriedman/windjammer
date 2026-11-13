//! The build key: the inputs key of the tree (content_key.rs) plus the toolchain configuration. Included by
//! `build.rs` and by `tests/build_content_key_test.rs` (same source, no copy); the runtime only reads the baked value.

use super::content_key::{inputs_key, Digest};
use std::path::Path;

/// The build key of the tree at `root` for one toolchain configuration (`rustc -vV` output, target triple, profile,
/// enabled features in any order): the inputs key plus the toolchain.
pub fn content_key(root: &Path, rustc_version: &str, target: &str, profile: &str, features: &[String]) -> String {
    let mut d = Digest::new();
    d.field(inputs_key(root).as_bytes());
    let mut features: Vec<&String> = features.iter().collect();
    features.sort();
    for s in [rustc_version, target, profile] {
        d.field(s.as_bytes());
    }
    for f in features {
        d.field(f.as_bytes());
    }
    d.hex()
}
