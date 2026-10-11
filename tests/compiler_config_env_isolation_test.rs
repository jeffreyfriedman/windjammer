//! Ambient configuration is read in exactly one place. Production code outside the
//! allow-list must not call `env::var*` (tests and production run identical code).

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: &[&str] = &[
    "src/compiler_config.rs",
    "src/cli/",
    "src/cargo_cache/",
    "src/debug_env.rs",
    "src/test_runner/",
    "src/error_statistics.rs",
];

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn env_var_reads_only_in_allow_listed_modules() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rs_files(&root.join("src"), &mut files);
    let mut offenders = Vec::new();
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if ALLOWED.iter().any(|a| rel.starts_with(a)) {
            continue;
        }
        let text = fs::read_to_string(&f).unwrap();
        for (i, line) in text.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue;
            }
            if line.contains("env::var(") || line.contains("env::var_os(") {
                offenders.push(format!("{rel}:{}: {}", i + 1, t));
            }
        }
    }
    assert!(offenders.is_empty(), "env reads outside allow-list:\n{}", offenders.join("\n"));
}
