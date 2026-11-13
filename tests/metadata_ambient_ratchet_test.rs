//! Ratchet: `src/metadata` decides where analysis metadata lives from its explicit inputs only. It reads no
//! environment variable and no OS temp dir (the shared cache root arrives as a parameter, resolved once by the driver
//! through `CompilerConfig::cache_root`). The target is zero `std::env` / `temp_dir` uses; the remaining ones are
//! listed in `ALLOWED` with their exact count, and the list may only shrink: a new use fails, and so does a count
//! that is higher than the code needs (lower it when a use goes away).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// `(file, pattern, count)`: the remaining ambient reads. Shrink only.
const ALLOWED: &[(&str, &str, usize)] = &[
    // making a relative source path absolute against the working directory (no cache location depends on it)
    ("src/metadata/crate_metadata.rs", "env::current_dir", 2),
];

/// Ambient inputs: anything under `std::env` (variables, the working directory, the temp dir) and `temp_dir`
/// however it is imported.
const PATTERNS: &[&str] = &["env::var", "env::current_dir", "env::temp_dir", "env::home_dir", "env::args", "temp_dir(", "use std::env"];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

/// `(file, pattern) -> count` over the non-comment lines of `src/metadata`.
fn ambient_reads(root: &Path) -> BTreeMap<(String, String), usize> {
    let mut files = Vec::new();
    rs_files(&root.join("src/metadata"), &mut files);
    let mut found = BTreeMap::new();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        let text = std::fs::read_to_string(&f).unwrap();
        for line in text.lines().filter(|l| !l.trim_start().starts_with("//")) {
            // `temp_dir(` also matches inside `env::temp_dir(`: count each use once, under its most specific pattern
            let mut line = line.to_string();
            for pat in PATTERNS {
                let n = line.matches(pat).count();
                if n > 0 {
                    *found.entry((rel.clone(), pat.to_string())).or_insert(0) += n;
                    line = line.replace(pat, "");
                }
            }
        }
    }
    found
}

#[test]
fn metadata_reads_no_environment_and_no_temp_dir() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let found = ambient_reads(root);
    let allowed: BTreeMap<(String, String), usize> =
        ALLOWED.iter().map(|(f, p, n)| ((f.to_string(), p.to_string()), *n)).collect();
    let grown: Vec<String> = found
        .iter()
        .filter(|(k, n)| allowed.get(*k).is_none_or(|a| *n > a))
        .map(|((f, p), n)| format!("{f}: {n} x `{p}` (allowed {})", allowed.get(&(f.clone(), p.clone())).copied().unwrap_or(0)))
        .collect();
    assert!(
        grown.is_empty(),
        "src/metadata must not read the environment or the temp dir; take the value as a parameter instead:\n{}",
        grown.join("\n")
    );
    let loose: Vec<String> = allowed
        .iter()
        .filter(|(k, a)| found.get(*k).copied().unwrap_or(0) < **a)
        .map(|((f, p), a)| format!("{f}: `{p}` allowed {a}, found {}", found.get(&(f.clone(), p.clone())).copied().unwrap_or(0)))
        .collect();
    assert!(loose.is_empty(), "lower ALLOWED in this file to the counts found (it only shrinks):\n{}", loose.join("\n"));
}

/// Mutation self-test: the scan sees each kind of read it forbids.
#[test]
fn the_scan_catches_every_forbidden_read() {
    let d = tempfile::tempdir().unwrap();
    let meta = d.path().join("src/metadata");
    std::fs::create_dir_all(&meta).unwrap();
    std::fs::write(
        meta.join("bad.rs"),
        "fn a() { let _ = std::env::var(\"X\"); }\nfn b() { let _ = std::env::temp_dir(); }\n// std::env::var in a comment\nfn c() { let _ = std::env::var_os(\"Y\"); }\n",
    )
    .unwrap();
    let found = ambient_reads(d.path());
    let get = |p: &str| found.get(&("src/metadata/bad.rs".to_string(), p.to_string())).copied().unwrap_or(0);
    assert_eq!(get("env::var"), 2, "{found:?}");
    assert_eq!(get("env::temp_dir"), 1, "{found:?}");
    assert_eq!(get("temp_dir("), 0, "a use is counted once: {found:?}");
}
