//! Check inputs that are not `.wj` sources, for the cache keys: the `.wj.meta` sidecars the numeric inference
//! reads relative to the working directory (`<cwd>/<module>/<x>.wj.meta` and `<cwd>/.wj-cache/...`, see
//! `type_inference/float_inference/load_imported_metadata.rs`). Candidate paths come from top-level `use` paths,
//! so they lie within `use_depth` directory levels of the working directory.

use crate::parser::Item;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Directory entries a fingerprint may visit; beyond it the run is not cached at all.
const WALK_BUDGET: usize = 20_000;

/// Deepest `.wj.meta` candidate (directory levels below the working directory, file included) any top-level
/// `use` of `programs` can name, with one level of slack.
pub fn use_depth<'a>(programs: impl IntoIterator<Item = &'a crate::parser::Program<'static>>) -> usize {
    let mut depth = 0;
    for p in programs {
        for item in &p.items {
            if let Item::Use { path, .. } = item {
                let levels: usize = path.iter().map(|s| 1 + s.matches(['/', '\\']).count()).sum();
                depth = depth.max(levels + 2);
            }
        }
    }
    depth
}

/// Digest of the working directory and every `*.wj.meta` within `depth` levels of it and of its `.wj-cache/`.
/// `None` when the working directory is unknown or the walk exceeds its budget: the caller must not cache.
pub fn fingerprint(depth: usize) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let mut files = Vec::new();
    let mut budget = WALK_BUDGET;
    walk(&cwd, depth, &mut budget, &mut files)?;
    walk(&cwd.join(".wj-cache"), depth + 1, &mut budget, &mut files)?;
    files.sort();
    files.dedup();
    let mut h = Sha256::new();
    part(&mut h, cwd.display().to_string().as_bytes());
    for f in &files {
        part(&mut h, f.display().to_string().as_bytes());
        part(&mut h, &std::fs::read(f).unwrap_or_default());
    }
    Some(format!("{:x}", h.finalize()))
}

/// Length-prefixed, so adjacent parts cannot run together.
pub fn part(h: &mut Sha256, bytes: &[u8]) {
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

fn walk(dir: &Path, depth: usize, budget: &mut usize, out: &mut Vec<std::path::PathBuf>) -> Option<()> {
    if depth == 0 {
        return Some(());
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return Some(()) };
    for e in entries.flatten() {
        *budget = budget.checked_sub(1)?;
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        if p.is_dir() {
            // Module directories never start with a dot; `.wj-cache` is walked separately.
            if !name.starts_with('.') {
                walk(&p, depth - 1, budget, out)?;
            }
        } else if name.ends_with(".wj.meta") {
            out.push(p);
        }
    }
    Some(())
}
