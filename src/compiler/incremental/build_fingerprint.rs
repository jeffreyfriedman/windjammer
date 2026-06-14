//! Content + compiler fingerprints for incremental cache validation.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

/// Fingerprint of a source file for analysis cache validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFingerprint {
    pub content_hash: u64,
    pub compiler_version: String,
    pub dep_metadata_epoch: u64,
}

pub fn hash_source(source: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

/// Unique identity for this compiler build (version + binary hash).
/// Used in stamps and `.wj.meta` fingerprints so rebuilds invalidate stale caches.
pub fn compiler_build_identity() -> String {
    let mut hasher = DefaultHasher::new();
    env!("CARGO_PKG_VERSION").hash(&mut hasher);
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(meta) = std::fs::metadata(&exe) {
            if let Ok(mtime) = meta.modified() {
                mtime.hash(&mut hasher);
            }
            meta.len().hash(&mut hasher);
        }
        if let Ok(mut file) = std::fs::File::open(&exe) {
            use std::io::Read;
            let mut buf = [0u8; 65536];
            if let Ok(n) = file.read(&mut buf) {
                buf[..n].hash(&mut hasher);
            }
        }
    }
    format!("{}:{:016x}", env!("CARGO_PKG_VERSION"), hasher.finish())
}

/// Check whether `.wj-compiler-stamp` in `output` matches this compiler binary.
pub fn is_compiler_stamp_fresh(output: &Path) -> bool {
    const COMPILER_STAMP_FILE: &str = ".wj-compiler-stamp";
    let stamp_path = output.join(COMPILER_STAMP_FILE);
    let stamp_mtime = match std::fs::metadata(&stamp_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(meta) = std::fs::metadata(&exe) {
            if let Ok(compiler_mtime) = meta.modified() {
                if compiler_mtime > stamp_mtime {
                    return false;
                }
            }
        }
    }
    match std::fs::read_to_string(&stamp_path) {
        Ok(content) => content.trim() == compiler_build_identity(),
        Err(_) => false,
    }
}

pub fn write_compiler_stamp(output: &Path) -> std::io::Result<()> {
    let stamp_path = output.join(".wj-compiler-stamp");
    std::fs::write(&stamp_path, format!("{}\n", compiler_build_identity()))
}

pub fn dep_metadata_epoch(dep_roots: &[std::path::PathBuf]) -> u64 {
    let mut max_mtime = SystemTime::UNIX_EPOCH;
    for dep_path in dep_roots {
        if let Ok(meta) = std::fs::metadata(dep_path) {
            if let Ok(mtime) = meta.modified() {
                if mtime > max_mtime {
                    max_mtime = mtime;
                }
            }
        }
    }
    let mut hasher = DefaultHasher::new();
    max_mtime.hash(&mut hasher);
    hasher.finish()
}

pub fn compute_fingerprint(source: &str, dep_roots: &[std::path::PathBuf]) -> SourceFingerprint {
    SourceFingerprint {
        content_hash: hash_source(source),
        compiler_version: compiler_build_identity(),
        dep_metadata_epoch: dep_metadata_epoch(dep_roots),
    }
}

/// Like [`compute_fingerprint`] but pins `dep_metadata_epoch` for the duration of one build.
/// Prevents flaky stale-output checks when parallel tests mutate dependency trees mid-build.
pub fn compute_fingerprint_with_dep_epoch(source: &str, dep_epoch: u64) -> SourceFingerprint {
    SourceFingerprint {
        content_hash: hash_source(source),
        compiler_version: compiler_build_identity(),
        dep_metadata_epoch: dep_epoch,
    }
}

pub fn fingerprint_matches_cached(
    source: &str,
    source_path: &Path,
    dep_roots: &[std::path::PathBuf],
) -> bool {
    fingerprint_matches(source_path, &compute_fingerprint(source, dep_roots))
}

pub fn fingerprint_matches_cached_with_dep_epoch(
    source: &str,
    source_path: &Path,
    dep_epoch: u64,
) -> bool {
    fingerprint_matches(
        source_path,
        &compute_fingerprint_with_dep_epoch(source, dep_epoch),
    )
}

fn fingerprint_matches(source_path: &Path, current: &SourceFingerprint) -> bool {
    let meta_path = crate::metadata::meta_cache_path(source_path);
    let Ok(content) = std::fs::read_to_string(&meta_path) else {
        return false;
    };
    let Ok(meta) = serde_json::from_str::<crate::metadata::ModuleMetadata>(&content) else {
        return false;
    };
    meta.analysis_fingerprint.as_ref().is_some_and(|cached| {
        cached.content_hash == current.content_hash
            && cached.compiler_version == current.compiler_version
            && cached.dep_metadata_epoch == current.dep_metadata_epoch
    })
}

fn is_output_mtime_fresh(source: &Path, output: &Path) -> bool {
    let source_mtime = match std::fs::metadata(source).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let output_mtime = match std::fs::metadata(output).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    output_mtime >= source_mtime
}

/// Authoritative codegen skip check (content fingerprint + mtime sanity).
pub fn is_codegen_cache_valid(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    dep_roots: &[std::path::PathBuf],
) -> bool {
    if !output_path.exists() {
        return false;
    }
    if !is_output_mtime_fresh(source_path, output_path) {
        return false;
    }
    fingerprint_matches_cached(source, source_path, dep_roots)
}

/// Like [`is_codegen_cache_valid`] but uses a dep-metadata epoch snapshot from build start.
pub fn is_codegen_cache_valid_with_dep_epoch(
    source: &str,
    source_path: &Path,
    output_path: &Path,
    dep_epoch: u64,
) -> bool {
    if !output_path.exists() {
        return false;
    }
    if !is_output_mtime_fresh(source_path, output_path) {
        return false;
    }
    fingerprint_matches_cached_with_dep_epoch(source, source_path, dep_epoch)
}

impl From<SourceFingerprint> for crate::metadata::AnalysisFingerprint {
    fn from(fp: SourceFingerprint) -> Self {
        crate::metadata::AnalysisFingerprint {
            content_hash: fp.content_hash,
            compiler_version: fp.compiler_version,
            dep_metadata_epoch: fp.dep_metadata_epoch,
        }
    }
}
