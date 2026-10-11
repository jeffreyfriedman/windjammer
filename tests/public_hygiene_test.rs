//! Public-hygiene guard: no tracked file name or file content may match the forbidden-term
//! patterns in `docs/public-hygiene-patterns.txt` (this repository is public).
//! Known exceptions live in `docs/public-hygiene-allowlist.txt` and may only shrink.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const PATTERNS: &str = "docs/public-hygiene-patterns.txt";
const ALLOWLIST: &str = "docs/public-hygiene-allowlist.txt";

struct Term {
    seps: Vec<char>,
    pieces: Vec<String>,
}

fn parse_patterns(text: &str) -> Vec<Term> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (seps, pieces) = l.split_once(':').expect("pattern line needs ':'");
            Term {
                seps: seps.chars().collect(),
                pieces: pieces.split('|').map(|p| p.to_lowercase()).collect(),
            }
        })
        .collect()
}

fn match_from(term: &Term, hay: &str, idx: usize, pos: usize) -> bool {
    if idx == term.pieces.len() {
        return true;
    }
    let piece = &term.pieces[idx];
    if !hay[pos..].starts_with(piece.as_str()) {
        return false;
    }
    let after = pos + piece.len();
    if idx + 1 == term.pieces.len() {
        return true;
    }
    if match_from(term, hay, idx + 1, after) {
        return true;
    }
    if let Some(c) = hay[after..].chars().next() {
        if term.seps.contains(&c) {
            return match_from(term, hay, idx + 1, after + c.len_utf8());
        }
    }
    false
}

fn text_matches(terms: &[Term], text: &str) -> bool {
    let hay = text.to_lowercase();
    terms.iter().any(|t| {
        let first = &t.pieces[0];
        hay.match_indices(first.as_str()).any(|(i, _)| match_from(t, &hay, 0, i))
    })
}

/// Relative paths of offending files (name or content matches).
fn scan(root: &Path, files: &[PathBuf], terms: &[Term]) -> Vec<String> {
    let mut bad = Vec::new();
    for rel in files {
        let rel_s = rel.to_string_lossy().to_string();
        let mut hit = text_matches(terms, &rel_s);
        if !hit {
            if let Ok(bytes) = fs::read(root.join(rel)) {
                if let Ok(s) = String::from_utf8(bytes) {
                    hit = text_matches(terms, &s);
                }
            }
        }
        if hit {
            bad.push(rel_s);
        }
    }
    bad
}

fn tracked_files(root: &Path) -> Vec<PathBuf> {
    let out = Command::new("git").arg("-C").arg(root).args(["ls-files", "-z"]).output();
    if let Ok(o) = out {
        if o.status.success() {
            return String::from_utf8_lossy(&o.stdout)
                .split('\0')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
                .filter(|p| root.join(p).is_file())
                .collect();
        }
    }
    let mut v = Vec::new();
    walk(root, root, &mut v);
    v
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name == ".git" || name == "target" {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out);
        } else if let Ok(r) = p.strip_prefix(root) {
            out.push(r.to_path_buf());
        }
    }
}

fn load_terms(root: &Path) -> Vec<Term> {
    parse_patterns(&fs::read_to_string(root.join(PATTERNS)).expect("patterns file"))
}

fn load_allowlist(root: &Path) -> BTreeSet<String> {
    fs::read_to_string(root.join(ALLOWLIST))
        .expect("allowlist file")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect()
}

#[test]
fn tracked_files_contain_no_private_project_names() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let terms = load_terms(&root);
    let allow = load_allowlist(&root);
    let files = tracked_files(&root);
    let bad: BTreeSet<String> = scan(&root, &files, &terms).into_iter().collect();
    let unexpected: Vec<_> = bad.difference(&allow).collect();
    assert!(
        unexpected.is_empty(),
        "forbidden private-project terms found in (see docs/public-hygiene-patterns.txt): {unexpected:#?}"
    );
    let stale: Vec<_> = allow.difference(&bad).collect();
    assert!(
        stale.is_empty(),
        "stale entries in {ALLOWLIST} (file no longer matches; remove them): {stale:#?}"
    );
}

#[test]
fn guard_detects_planted_terms_in_content_and_names() {
    let terms = parse_patterns(&fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join(PATTERNS),
    )
    .unwrap());
    let dir = std::env::temp_dir().join(format!("wj_hygiene_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // Build the planted strings at runtime so this file never spells them out.
    let ticket = ["W", "DB", "-123"].concat();
    let path_term = ["/Users/", "jeffrey", "friedman", "/x"].concat();
    let game_name = ["windjammer", "-", "ga", "me"].concat();
    fs::write(dir.join("clean.txt"), "nothing to see here").unwrap();
    fs::write(dir.join("ticket.txt"), format!("// {ticket}: foo")).unwrap();
    fs::write(dir.join("path.sh"), format!("cd {path_term}")).unwrap();
    fs::write(dir.join(format!("{game_name}.md")), "innocent body").unwrap();
    let files: Vec<PathBuf> = ["clean.txt", "ticket.txt", "path.sh"]
        .iter()
        .map(PathBuf::from)
        .chain(std::iter::once(PathBuf::from(format!("{game_name}.md"))))
        .collect();
    let mut bad = scan(&dir, &files, &terms);
    bad.sort();
    let mut want = vec!["path.sh".to_string(), "ticket.txt".to_string(), format!("{game_name}.md")];
    want.sort();
    assert_eq!(bad, want);
    let _ = fs::remove_dir_all(&dir);
}
