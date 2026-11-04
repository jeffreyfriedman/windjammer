//! The key under which `build.rs` remembers when a toolchain build happened (`WJ_BUILD_TS`).
//!
//! Included by `build.rs` and by `tests/toolchain_dirty_key_test.rs` (same source, no copy). Two builds of one commit
//! with different uncommitted changes must have different keys: otherwise they share a build timestamp and
//! `wj install` treats the second binary as already installed, so a stale binary keeps running.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Digest of the uncommitted state: the tracked diff against HEAD plus the untracked source files (name and bytes).
/// `0` for a clean tree.
pub fn dirty_digest(diff_against_head: &str, untracked_files: &[(String, Vec<u8>)]) -> u64 {
    if diff_against_head.is_empty() && untracked_files.is_empty() {
        return 0;
    }
    let mut h = DefaultHasher::new();
    diff_against_head.hash(&mut h);
    let mut files: Vec<&(String, Vec<u8>)> = untracked_files.iter().collect();
    files.sort();
    for (name, bytes) in files {
        name.hash(&mut h);
        bytes.hash(&mut h);
    }
    h.finish()
}

pub fn identity_key(commit: &str, dirty: bool, digest: u64, profile: &str) -> String {
    format!("{commit} {dirty} {digest:016x} {profile}")
}
