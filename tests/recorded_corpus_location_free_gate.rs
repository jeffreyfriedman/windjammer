//! Portability gate: recorded corpora (bench/ data, expected outputs and corpora under tests/) must not embed
//! this machine's absolute paths. Use the placeholders `{REPO}`, `{TMP}`, `{CACHE}`, `{RUSTUP_HOME}`, substituted when the
//! data is loaded. Source files (`.rs`, `.wj`) are code, not recorded data, and are exempt.
//! Run: `cargo test --test recorded_corpus_location_free_gate`

use std::path::{Path, PathBuf};

/// `~/.wj` and friends name the product's own home layout and are fine; these name a developer's machine.
const FORBIDDEN: &[&str] =
    &["/Users/", "/home/", "/private/", "/var/folders/", "~/src/", "~/Library/", "~/.rustup", "~/.cargo", "~/Documents/", "C:\\Users\\"];

/// Directories under tests/ that hold hand-written prose about paths, not recorded data.
fn exempt(rel: &str) -> bool {
    [".rs", ".wj", ".md", ".py", ".sh"].iter().any(|e| rel.ends_with(e))
}

/// Tracked files plus untracked-but-not-ignored ones under `dirs`. Gitignored files (tool caches such as
/// `.wj-cache`, build output) are tool output, not recorded corpora.
fn recorded_files(repo: &Path, dirs: &[&str]) -> Vec<PathBuf> {
    let mut args = vec!["ls-files", "-z", "--cached", "--others", "--exclude-standard", "--"];
    args.extend_from_slice(dirs);
    let out = std::process::Command::new("git").current_dir(repo).args(&args).output().expect("run git");
    assert!(out.status.success(), "git ls-files failed: {}", String::from_utf8_lossy(&out.stderr));
    let mut v: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|s| repo.join(s))
        .filter(|p| p.is_file())
        .collect();
    v.sort();
    v.dedup();
    v
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
    let files = recorded_files(&root, &["bench", "tests"]);
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
        "{} recorded file(s) embed an absolute path; replace it with {{REPO}}, {{TMP}}, {{CACHE}} or {{RUSTUP_HOME}}:\n{}",
        bad.len(),
        bad.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn detector_catches_machine_paths_and_ignores_placeholders() {
    assert!(first_absolute_path("see /Users/me/src/x").is_some());
    assert!(first_absolute_path("at \"/var/folders/k8/abc/T/.tmpX\"").is_some());
    assert!(first_absolute_path("from ~/src/wj-wt/pilot6").is_some());
    assert!(first_absolute_path("at {REPO}/bench/x and {TMP}/y and {CACHE}/run/ab and {RUSTUP_HOME}/toolchains").is_none());
    assert!(first_absolute_path("relative bench/agent-dx/home/x").is_none());
}

#[test]
fn gitignored_tool_caches_are_not_scanned() {
    let tmp = tempfile::tempdir().unwrap();
    let r = tmp.path();
    let run = |a: &[&str]| assert!(std::process::Command::new("git").current_dir(r).args(a).output().unwrap().status.success());
    run(&["init", "-q"]);
    std::fs::create_dir_all(r.join("tests/.wj-cache")).unwrap();
    std::fs::write(r.join(".gitignore"), ".wj-cache/\n").unwrap();
    std::fs::write(r.join("tests/.wj-cache/c.json"), "{\"/home/x\":1}").unwrap();
    std::fs::write(r.join("tests/tracked.json"), "{}").unwrap();
    std::fs::write(r.join("tests/new.json"), "{}").unwrap();
    run(&["add", ".gitignore", "tests/tracked.json"]);
    let got: Vec<String> = recorded_files(r, &["tests"]).iter().map(|p| p.strip_prefix(r).unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(got, vec!["tests/new.json", "tests/tracked.json"]);
}
