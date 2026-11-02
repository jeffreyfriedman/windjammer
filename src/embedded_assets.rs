//! Runtime assets (`std/`, `stdlib_meta/`, the `windjammer-runtime` crate) baked into
//! the `wj` binary by `build.rs`, and materialized on demand into a versioned cache dir.
//!
//! An installed `wj` must behave identically in every working directory and must not
//! depend on the compiler source tree it was built from. Nothing at runtime may locate
//! these assets through `CARGO_MANIFEST_DIR` or the current directory.

use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/embedded_assets_generated.rs"));

/// Which embedded tree to materialize.
#[derive(Clone, Copy)]
pub enum Asset {
    StdlibMeta,
    StdWj,
    RuntimeCrate,
}

impl Asset {
    fn table(self) -> &'static [(&'static str, &'static [u8])] {
        match self {
            Asset::StdlibMeta => STDLIB_META,
            Asset::StdWj => STD_WJ,
            Asset::RuntimeCrate => RUNTIME_CRATE,
        }
    }

    fn hash(self) -> u64 {
        match self {
            Asset::StdlibMeta => STDLIB_META_HASH,
            Asset::StdWj => STD_WJ_HASH,
            Asset::RuntimeCrate => RUNTIME_CRATE_HASH,
        }
    }

    fn slug(self) -> &'static str {
        match self {
            Asset::StdlibMeta => "stdlib_meta",
            Asset::StdWj => "std",
            Asset::RuntimeCrate => "windjammer-runtime",
        }
    }
}

/// Cache root: the OS temp dir (never the cwd or the build checkout). Core code reads no
/// env vars; if the dir is unwritable, [`materialize`] falls back to a private per-process dir.
fn cache_root() -> PathBuf {
    std::env::temp_dir().join("wj-embedded")
}

/// The runtime crate's manifest inherits `version`/`authors`/`license` from the compiler
/// workspace; a standalone copy needs them spelled out.
fn standalone_runtime_manifest(src: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    src.lines()
        .map(|l| match l.trim() {
            "version.workspace = true" => format!("version = \"{version}\""),
            "authors.workspace = true" => "authors = [\"Windjammer Contributors\"]".to_string(),
            "license.workspace = true" => "license = \"MIT OR Apache-2.0\"".to_string(),
            _ => l.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn write_tree(asset: Asset, dest: &Path) -> std::io::Result<()> {
    for (rel, bytes) in asset.table() {
        let out = dest.join(rel);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if matches!(asset, Asset::RuntimeCrate) && *rel == "Cargo.toml" {
            let text = String::from_utf8_lossy(bytes);
            std::fs::write(&out, standalone_runtime_manifest(&text))?;
        } else {
            std::fs::write(&out, bytes)?;
        }
    }
    Ok(())
}

/// Number of files in the embedded tree.
pub fn file_count(asset: Asset) -> usize {
    asset.table().len()
}

/// Write the embedded tree for `asset` into `dest` (created if needed).
pub fn write_to(asset: Asset, dest: &Path) -> std::io::Result<()> {
    write_tree(asset, dest)
}

/// Cache directory name for a materialized asset. The content hash is part of the name, so any
/// edit to the embedded tree (for example a runtime source change) lands in a fresh directory
/// and therefore a fresh cargo fingerprint: a stale pre-built runtime cannot be reused.
fn cache_dir_name(slug: &str, version: &str, hash: u64) -> String {
    format!("{slug}-{version}-{hash:016x}")
}

/// FNV-1a over `path NUL bytes` for every file, the same function `build.rs` bakes in as `*_HASH`.
fn content_hash(table: &[(&str, &[u8])]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (rel, bytes) in table {
        for b in rel.bytes().chain([0u8]).chain(bytes.iter().copied()) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

/// Materialize `asset` and return its directory. Idempotent and safe under concurrent
/// `wj` processes (write to a private temp sibling, then rename into place).
pub fn materialize(asset: Asset) -> std::io::Result<PathBuf> {
    let dir = cache_root().join(cache_dir_name(asset.slug(), env!("CARGO_PKG_VERSION"), asset.hash()));
    if dir.join(".complete").is_file() {
        return Ok(dir);
    }
    if try_populate(asset, &dir).is_ok() && dir.join(".complete").is_file() {
        return Ok(dir);
    }
    // Shared temp dir unusable (e.g. owned by another user): private per-process copy.
    let private = std::env::temp_dir()
        .join(format!("wj-embedded-{}", std::process::id()))
        .join(format!("{}-{:016x}", asset.slug(), asset.hash()));
    try_populate(asset, &private)?;
    Ok(private)
}

fn try_populate(asset: Asset, dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.parent().unwrap_or(Path::new(".")))?;
    let tmp = dir.with_extension(format!("tmp{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    write_tree(asset, &tmp)?;
    std::fs::write(tmp.join(".complete"), b"")?;
    if dir.join(".complete").is_file() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Ok(());
    }
    let _ = std::fs::remove_dir_all(dir);
    if std::fs::rename(&tmp, dir).is_err() {
        // Lost a race with another process: theirs is complete, drop ours.
        let _ = std::fs::remove_dir_all(&tmp);
        if !dir.join(".complete").is_file() {
            return Err(std::io::Error::other("embedded asset cache is unwritable"));
        }
    }
    Ok(())
}

/// Like [`materialize`], but `None` (with a warning on stderr) when the cache is unwritable.
pub fn materialize_or_warn(asset: Asset) -> Option<PathBuf> {
    match materialize(asset) {
        Ok(p) => Some(p),
        Err(e) => {
            eprintln!("warning: cannot materialize embedded {}: {e}", asset.slug());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_mirror_source_trees() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(STDLIB_META.iter().any(|(p, _)| *p == "string.wj.meta"));
        assert!(STD_WJ.iter().any(|(p, _)| p.ends_with(".wj")));
        assert!(RUNTIME_CRATE.iter().any(|(p, _)| *p == "Cargo.toml"));
        assert!(RUNTIME_CRATE.iter().any(|(p, _)| *p == "src/lib.rs"));
        let on_disk = std::fs::read_dir(root.join("stdlib_meta")).unwrap().count();
        assert_eq!(STDLIB_META.len(), on_disk, "stdlib_meta table out of sync");
    }

    #[test]
    fn runtime_manifest_is_standalone() {
        let dir = materialize(Asset::RuntimeCrate).unwrap();
        let m = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(
            !m.contains(".workspace"),
            "workspace inheritance leaked:\n{m}"
        );
        assert!(m.contains("name = \"windjammer-runtime\""));
        assert!(dir.join("src/lib.rs").is_file());
    }
}

#[cfg(test)]
mod cache_key_tests {
    use super::*;

    /// The property the shared pre-built runtime cache depends on: a different content hash must
    /// give a different cache directory.
    fn key_invalidates(key: fn(&str, &str, u64) -> String) -> bool {
        key("windjammer-runtime", "0.50.0", 1) != key("windjammer-runtime", "0.50.0", 2)
    }

    #[test]
    fn runtime_edit_changes_the_cache_directory() {
        assert!(key_invalidates(cache_dir_name));
    }

    /// Mutation check: a key that drops the content hash (the bug this gate guards against) must
    /// be caught by the same property.
    #[test]
    fn mutant_key_without_hash_is_detected() {
        fn mutant(slug: &str, version: &str, _hash: u64) -> String {
            format!("{slug}-{version}")
        }
        assert!(!key_invalidates(mutant));
    }

    #[test]
    fn editing_one_byte_changes_the_content_hash() {
        let a: &[(&str, &[u8])] = &[("src/lib.rs", b"pub fn f() -> u8 { 1 }")];
        let b: &[(&str, &[u8])] = &[("src/lib.rs", b"pub fn f() -> u8 { 2 }")];
        assert_ne!(content_hash(a), content_hash(b));
    }

    #[test]
    fn baked_hashes_match_the_runtime_hash_function() {
        assert_eq!(content_hash(RUNTIME_CRATE), RUNTIME_CRATE_HASH);
        assert_eq!(content_hash(STD_WJ), STD_WJ_HASH);
    }
}
