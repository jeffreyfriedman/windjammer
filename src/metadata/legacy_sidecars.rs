//! Legacy `<file>.wj.meta` sidecars that older compilers wrote next to sources. They are never read
//! (see `merge_single_wj_meta_file`); this module finds them so `wj doctor` can point at
//! `wj clean --sidecars`, which removes them.

use std::path::{Path, PathBuf};

/// Directories never descended into: build output, VCS, caches, and the compiler's own hand-authored
/// `stdlib_meta/` (intentional, embedded metadata).
const SKIP_DIRS: &[&str] = &["target", "build", "out", "node_modules", "stdlib_meta", ".git", ".wj-cache"];

/// Upper bound on directory entries visited, so `wj doctor` stays instant in huge trees.
const BUDGET: usize = 50_000;

/// Every `*.wj.meta` under `root` that sits outside a cache or skipped directory.
pub fn find(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut budget = BUDGET;
    walk(root, &mut budget, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, budget: &mut usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        if *budget == 0 {
            return;
        }
        *budget -= 1;
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ty) = e.file_type() else { continue };
        if ty.is_dir() {
            if !SKIP_DIRS.contains(&name.as_str()) {
                walk(&p, budget, out);
            }
        } else if ty.is_file() && name.ends_with(".wj.meta") {
            out.push(p);
        }
    }
}

/// Delete the sidecars found under `root`; returns the removed paths.
pub fn remove(root: &Path) -> Vec<PathBuf> {
    find(root).into_iter().filter(|p| std::fs::remove_file(p).is_ok()).collect()
}
