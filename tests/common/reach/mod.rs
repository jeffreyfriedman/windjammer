//! Dead-capability detection: which code in `src/` can a user actually reach from `wj`, the LSP
//! server or the MCP server? See docs/testing/INVARIANTS.md ("Dead-capability gate").

pub mod graph;
pub mod scan;

use scan::{Item, Kind};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Entry points: everything a user can invoke. The legacy `windjammer` binary (src/main.rs) is
/// deliberately NOT an entry point (WP13: single driver).
pub const ROOTS: [&str; 6] = [
    "src/bin/wj.rs",
    "src/bin/wj-lint.rs",
    "src/bin/wj_cli_args.rs",
    "src/bin/wj_cli_cmds.rs",
    "crates/windjammer-lsp/src/",
    "crates/windjammer-mcp/src/",
];

/// Roots of the legacy binary, used only to classify "legacy-only" capabilities.
pub const LEGACY_ROOTS: [&str; 1] = ["src/main.rs"];

/// Developer tools that are neither product entry points nor product code.
const EXCLUDED: [&str; 1] = ["src/bin/wj-docs.rs"];

pub fn is_under(file: &str, roots: &[&str]) -> bool {
    roots.iter().any(|r| if r.ends_with('/') { file.starts_with(r) } else { file == *r })
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `include!("x.rs")` splices a file into its includer's module: `included -> includer` (relative paths).
fn included_files(root: &Path, files: &[PathBuf]) -> BTreeMap<String, String> {
    let rel = |p: &Path| p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
    let mut map = BTreeMap::new();
    for f in files {
        let src = std::fs::read_to_string(f).unwrap_or_default();
        for chunk in src.split("include!(\"").skip(1) {
            if let Some(name) = chunk.split('"').next().filter(|n| n.ends_with(".rs")) {
                map.insert(rel(&f.parent().unwrap().join(name)), rel(f));
            }
        }
    }
    map
}

/// All items of `src/` plus the LSP and MCP crates (paths relative to `root`).
pub fn scan_repo(root: &Path) -> (Vec<Item>, Vec<String>) {
    let mut files = Vec::new();
    for d in ["src", "crates/windjammer-lsp/src", "crates/windjammer-mcp/src"] {
        walk(&root.join(d), &mut files);
    }
    files.sort();
    let mut items = Vec::new();
    let mut errors = Vec::new();
    let includer = included_files(root, &files);
    for f in files {
        let rel = f.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if EXCLUDED.contains(&rel.as_str()) || rel.ends_with("_tests.rs") || rel.ends_with("/tests.rs") || rel.contains("/tests/") {
            continue;
        }
        let src = std::fs::read_to_string(&f).unwrap();
        match scan::scan_source(&rel, &src) {
            Ok(mut v) => {
                if let Some(owner) = includer.get(&rel) {
                    v.iter_mut().for_each(|i| i.scope = owner.clone());
                }
                items.extend(v)
            }
            Err(e) => errors.push(e),
        }
    }
    (items, errors)
}

/// Unreached capabilities grouped by file.
#[derive(Debug, Default)]
pub struct Report {
    /// Files of `src/` in which no item is reachable from an entry point.
    pub dead_files: Vec<String>,
    /// `file -> [pub fn / method names]` that are unreachable (in files that are otherwise live).
    pub dead_pub_fns: BTreeMap<String, Vec<String>>,
    /// Unreachable from `wj` but reachable from the legacy `windjammer` binary.
    pub legacy_only: BTreeMap<String, Vec<String>>,
}

pub fn report(items: &[Item]) -> Report {
    let live = graph::reach(items, |f| is_under(f, &ROOTS));
    let legacy = graph::reach(items, |f| is_under(f, &LEGACY_ROOTS));
    let mut per_file: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let mut r = Report::default();
    for (i, it) in items.iter().enumerate() {
        if !it.file.starts_with("src/") || is_under(&it.file, &ROOTS) {
            continue;
        }
        let e = per_file.entry(&it.file).or_default();
        e.0 += 1;
        if live.reached[i] {
            e.1 += 1;
            continue;
        }
        let is_fn = matches!(it.kind, Kind::Fn | Kind::Method { .. });
        if legacy.reached[i] && is_fn {
            r.legacy_only.entry(it.file.clone()).or_default().push(it.name.clone());
        }
        if it.is_pub && is_fn {
            r.dead_pub_fns.entry(it.file.clone()).or_default().push(format!("{}:{}", it.name, it.line));
        }
    }
    for (f, (total, live_n)) in per_file {
        if total > 0 && live_n == 0 {
            r.dead_files.push(f.to_string());
            r.dead_pub_fns.remove(f);
        }
    }
    r
}
