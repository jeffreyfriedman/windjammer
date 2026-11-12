//! Portability gate: recorded corpora (bench/ data, expected outputs and corpora under tests/) must not embed
//! this machine's absolute paths. Use the placeholders `{REPO}`, `{TMP}`, `{RUSTUP_HOME}`, substituted when the
//! data is loaded. Source files (`.rs`, `.wj`) are code, not recorded data, and are exempt.
//! Run: `cargo test --test recorded_corpus_location_free_gate`

use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &["/Users/", "/home/", "/private/", "/var/folders/", "~/", "C:\\Users\\"];

/// Directories under tests/ that hold hand-written prose about paths, not recorded data.
fn exempt(rel: &str) -> bool {
    rel.ends_with(".rs") || rel.ends_with(".wj") || rel.ends_with(".md")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().is_some_and(|n| n == "target" || n == ".git") {
                continue;
            }
            walk(&p, out);
        } else {
            out.push(p);
        }
    }
}

/// First offending (line number, line) of `text`, if any absolute path appears outside a placeholder.
pub fn first_absolute_path(text: &str) -> Option<(usize, String)> {
    for (i, line) in text.lines().enumerate() {
        for needle in FORBIDDEN {
            let mut from = 0;
            while let Some(at) = line[from..].find(needle) {
                let start = from + at;
                let prev = line[..start].chars().next_back();
                // `{REPO}/home/x` or `a~/b`-style continuations of a name are not absolute paths
                if !prev.is_some_and(|c| c.is_alphanumeric() || matches!(c, '_' | '{' | '}' | '.' | '-')) {
                    return Some((i + 1, line.chars().take(160).collect()));
                }
                from = start + needle.len();
            }
        }
    }
    None
}

#[test]
fn recorded_corpora_hold_no_absolute_paths() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    walk(&root.join("bench"), &mut files);
    walk(&root.join("tests"), &mut files);
    let mut bad = Vec::new();
    for f in files {
        let rel = f.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        if exempt(&rel) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if let Some((n, line)) = first_absolute_path(&text) {
            bad.push(format!("{rel}:{n}: {line}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{} recorded file(s) embed an absolute path; replace it with {{REPO}}, {{TMP}} or {{RUSTUP_HOME}}:\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn detector_catches_machine_paths_and_ignores_placeholders() {
    assert!(first_absolute_path("see /Users/me/src/x").is_some());
    assert!(first_absolute_path("at \"/var/folders/k8/abc/T/.tmpX\"").is_some());
    assert!(first_absolute_path("from ~/src/wj-wt/pilot6").is_some());
    assert!(first_absolute_path("at {REPO}/bench/x and {TMP}/y and {RUSTUP_HOME}/toolchains").is_none());
    assert!(first_absolute_path("relative bench/agent-dx/home/x").is_none());
}
