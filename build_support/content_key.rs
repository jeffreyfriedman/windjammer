//! The compiler's build key: a digest of every input the build reads, content only.
//!
//! Included by `build.rs` (which bakes the keys into the binary as `WJ_INPUTS_KEY` / `WJ_BUILD_KEY`), by
//! `src/toolchain/behind.rs` (the staleness check recomputes a checkout's inputs key) and by the gates (same source,
//! no copy). The toolchain part of the build key is `build_key.rs`. The key names what was compiled, never where or when:
//! file paths enter only relative to the manifest directory, mtimes and git state never enter, so two checkouts of
//! the same inputs (or copies of the same binary) share caches, and any change to an input's bytes, the toolchain,
//! the target, the profile or the features starts fresh ones.

use std::path::Path;

/// Inputs under the manifest directory (files or directories, walked recursively). `build.rs` also watches exactly
/// these (`cargo:rerun-if-changed`).
pub const INPUTS: &[&str] = &[
    "src",
    "std",
    "stdlib_meta",
    "crates/windjammer-runtime/Cargo.toml",
    "crates/windjammer-runtime/src",
    "Cargo.toml",
    "Cargo.lock",
    "build.rs",
    "build_support",
];

/// FNV-1a over 128 bits (two independent 64-bit lanes): stable across Rust versions and machines, no dependencies.
pub struct Digest(u64, u64);

impl Digest {
    pub fn new() -> Digest {
        Digest(0xcbf29ce484222325, 0x6c62272e07bb0142)
    }
    pub fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0 ^ b as u64).wrapping_mul(0x100000001b3);
            self.1 = (self.1 ^ b as u64).wrapping_mul(0x00000100000001b3).rotate_left(5);
        }
    }
    pub fn field(&mut self, bytes: &[u8]) {
        self.update(&(bytes.len() as u64).to_le_bytes());
        self.update(bytes);
    }
    pub fn hex(&self) -> String {
        format!("{:016x}{:016x}", self.0, self.1)
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(root, &p, out);
        } else if let Ok(rel) = p.strip_prefix(root) {
            out.push(rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/"));
        }
    }
}

/// The relative paths of every input file under `root`, sorted.
pub fn input_files(root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for rel in INPUTS {
        let p = root.join(rel);
        if p.is_dir() {
            walk(root, &p, &mut files);
        } else if p.is_file() {
            files.push(rel.to_string());
        }
    }
    files.sort();
    files.dedup();
    files
}

/// The digest of the input files under `root` alone (relative paths and bytes): what `wj --version` shows and what
/// the staleness check compares with a checkout.
pub fn inputs_key(root: &Path) -> String {
    let mut d = Digest::new();
    for f in input_files(root) {
        d.field(f.as_bytes());
        d.field(&std::fs::read(root.join(&f)).unwrap_or_default());
    }
    d.hex()
}

/// Whether any input file under `root` was modified after `secs` (Unix seconds): one pass, one `stat` per file
/// (directory entries are told apart by their type, without a `stat`), stopping at the first newer file. The
/// staleness check asks this on every `wj` run inside a checkout, so it must not build the path list first.
#[allow(dead_code)]
pub fn any_input_newer_than(root: &Path, secs: i64) -> bool {
    fn newer(path: &Path, secs: i64) -> bool {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .is_some_and(|d| d.as_secs() as i64 > secs)
    }
    fn walk_newer(dir: &Path, secs: i64) -> bool {
        let Ok(rd) = std::fs::read_dir(dir) else { return false };
        for e in rd.flatten() {
            let is_dir = match e.file_type() {
                Ok(t) if t.is_symlink() => e.path().is_dir(),
                Ok(t) => t.is_dir(),
                Err(_) => false,
            };
            if if is_dir { walk_newer(&e.path(), secs) } else { newer(&e.path(), secs) } {
                return true;
            }
        }
        false
    }
    INPUTS.iter().any(|rel| {
        let p = root.join(rel);
        if p.is_dir() {
            walk_newer(&p, secs)
        } else {
            p.is_file() && newer(&p, secs)
        }
    })
}
