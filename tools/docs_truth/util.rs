//! Small filesystem and string helpers (no regex dependency in this crate).

use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &["target", "node_modules", "build", "dist", "vendor"];

/// All files under `root` (repo-relative, `/`-separated, sorted), skipping VCS/build dirs.
pub fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk_into(root, root, &mut out);
    out.sort();
    out
}

fn walk_into(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if name.starts_with('.') && name != ".github" || SKIP_DIRS.contains(&name.as_str()) {
                continue;
            }
            walk_into(root, &p, out);
        } else if ft.is_file() {
            out.push(p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/"));
        }
    }
}

/// Glob with `*` (no `/`), `**` (anything) and `?`.
pub fn glob_match(pat: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some(b'*') if p.get(1) == Some(&b'*') => {
                let rest = &p[2..];
                let rest = rest.strip_prefix(b"/").unwrap_or(rest);
                (0..=t.len()).any(|i| go(rest, &t[i..]))
            }
            Some(b'*') => (0..=t.len()).take_while(|&i| i == 0 || t[i - 1] != b'/').any(|i| go(&p[1..], &t[i..])),
            Some(b'?') => !t.is_empty() && t[0] != b'/' && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    go(pat.as_bytes(), text.as_bytes())
}

pub fn exists(root: &Path, rel: &str) -> bool {
    root.join(rel).exists()
}

pub fn join(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}

/// Markdown table from a header and rows (cells are pipe-escaped).
pub fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    let esc = |s: &str| s.replace('|', "\\|").replace('\n', " ");
    let mut s = format!("| {} |\n|{}|\n", header.join(" | "), header.iter().map(|_| "---").collect::<Vec<_>>().join("|"));
    for r in rows {
        s.push_str(&format!("| {} |\n", r.iter().map(|c| esc(c)).collect::<Vec<_>>().join(" | ")));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn globs() {
        assert!(glob_match("docs/archive/**", "docs/archive/a/b.md"));
        assert!(glob_match("docs/*.md", "docs/a.md"));
        assert!(!glob_match("docs/*.md", "docs/x/a.md"));
        assert!(glob_match("**/*.md", "a/b/c.md"));
        assert!(glob_match("README.md", "README.md"));
    }
}
