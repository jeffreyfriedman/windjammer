//! Which `.wj` files under a project directory are the project's own sources.
//!
//! Generated and ignored directories hold stale transpiled copies of the sources. They must never inflate the file
//! set or change a diagnostic, so the walk decides from declared facts only:
//!
//! 1. `[check] include = [...]` in `wj.toml` keeps a path whatever the other rules say;
//! 2. `[check] exclude = [...]` in `wj.toml` drops a path;
//! 3. a default list of generated output directories (`gen`, `build`, `target`, `.wj-cache`, `node_modules`, `.git`,
//!    and `out` only when it holds a `Cargo.toml` or `package.json`) is dropped;
//! 4. what git ignores (`git check-ignore`) is dropped; outside a git work tree this rule does not apply.
//!
//! `--no-ignore` turns rules 2 to 4 off. An entry of `include` / `exclude` is a path relative to the directory that
//! holds `wj.toml` (it matches that file or directory and everything below it), or a bare name, which matches a
//! directory of that name anywhere in the project.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Generated output directories that are never sources.
pub const DEFAULT_EXCLUDED: [&str; 6] = ["gen", "build", "target", ".wj-cache", "node_modules", ".git"];

/// The directories among them that hold a build's emitted code (their age is compared with the sources').
const OUTPUT_DIRS: [&str; 2] = ["gen", "build"];

/// What one directory walk saw.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// `.wj` files left out (inside skipped directories, or ignored by git).
    pub skipped: usize,
    /// Skipped generated output directories (`gen`, `build`) that hold files, with the newest file's time.
    pub generated: Vec<(PathBuf, SystemTime)>,
    /// The newest modification time among the files that were kept.
    pub newest_source: Option<SystemTime>,
    /// The directory the walk started in.
    pub root: PathBuf,
}

struct Rules {
    base: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    active: bool,
}

impl Rules {
    fn load(root: &Path, no_ignore: bool) -> Rules {
        let active = !no_ignore;
        let manifest_dir = crate::module_system::project::find_manifest_dir(root);
        let base = manifest_dir.clone().unwrap_or_else(|| abs(root));
        let table = manifest_dir
            .and_then(|d| std::fs::read_to_string(d.join("wj.toml")).ok())
            .and_then(|t| t.parse::<toml::Value>().ok())
            .and_then(|v| v.get("check").cloned());
        let list = |key: &str| -> Vec<String> {
            table
                .as_ref()
                .and_then(|t| t.get(key))
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|s| s.as_str()).map(|s| s.trim_matches('/').to_string()).collect())
                .unwrap_or_default()
        };
        Rules { base: abs(&base), include: list("include"), exclude: list("exclude"), active }
    }

    fn matches(&self, entries: &[String], path: &Path, is_dir: bool) -> bool {
        let rel = abs(path);
        let rel = rel.strip_prefix(&self.base).unwrap_or(&rel);
        entries.iter().any(|e| {
            if e.contains('/') {
                rel.starts_with(e)
            } else {
                is_dir && rel.file_name().is_some_and(|n| n == e.as_str())
            }
        })
    }

    fn included(&self, path: &Path, is_dir: bool) -> bool {
        self.matches(&self.include, path, is_dir)
    }

    /// Whether the walk leaves this directory out (rules 2 and 3).
    fn skips_dir(&self, path: &Path, name: &str) -> bool {
        if !self.active || self.included(path, true) {
            return false;
        }
        DEFAULT_EXCLUDED.contains(&name)
            || (name == "out" && (path.join("Cargo.toml").is_file() || path.join("package.json").is_file()))
            || self.matches(&self.exclude, path, true)
    }
}

fn abs(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf()))
}

/// The `.wj` files below directory `root`, sorted, with the scope rules applied (`no_ignore` turns them off), and
/// what the walk left out.
pub fn walk(root: &Path, no_ignore: bool) -> anyhow::Result<(Vec<PathBuf>, Scope)> {
    let rules = Rules::load(root, no_ignore);
    let mut scope = Scope { root: root.to_path_buf(), ..Scope::default() };
    let mut files = Vec::new();
    descend(root, &rules, &mut files, &mut scope)?;
    files.sort();
    if rules.active {
        let ignored = git_ignored(root, &files, &rules);
        if !ignored.is_empty() {
            scope.skipped += ignored.len();
            files.retain(|f| !ignored.contains(f));
        }
    }
    scope.newest_source = files.iter().filter_map(|f| mtime(f)).max();
    Ok((files, scope))
}

fn descend(dir: &Path, rules: &Rules, files: &mut Vec<PathBuf>, scope: &mut Scope) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let p = entry?.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if p.is_dir() {
            if rules.skips_dir(&p, &name) {
                scope.skipped += count_wj(&p);
                if OUTPUT_DIRS.contains(&name.as_str()) && p.parent() == Some(scope.root.as_path()) {
                    if let Some(newest) = newest_file(&p) {
                        scope.generated.push((p, newest));
                    }
                }
                continue;
            }
            if name.starts_with('.') && !rules.included(&p, true) {
                continue;
            }
            descend(&p, rules, files, scope)?;
        } else if p.extension().is_some_and(|e| e == "wj") {
            if rules.active && rules.matches(&rules.exclude, &p, false) && !rules.included(&p, false) {
                scope.skipped += 1;
                continue;
            }
            files.push(p);
        }
    }
    Ok(())
}

fn count_wj(dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    rd.flatten()
        .map(|e| {
            let p = e.path();
            if p.is_dir() {
                count_wj(&p)
            } else {
                usize::from(p.extension().is_some_and(|x| x == "wj"))
            }
        })
        .sum()
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// The newest modification time of any file below `dir`.
fn newest_file(dir: &Path) -> Option<SystemTime> {
    let rd = std::fs::read_dir(dir).ok()?;
    rd.flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.is_dir() {
                newest_file(&p)
            } else {
                mtime(&p)
            }
        })
        .max()
}

/// The files git ignores (one `git check-ignore --stdin` call). Nothing when `root` is not in a git work tree or git
/// is not installed; files named by `include` are never submitted.
fn git_ignored(root: &Path, files: &[PathBuf], rules: &Rules) -> std::collections::HashSet<PathBuf> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut out = std::collections::HashSet::new();
    // a directory named on the command line is checked even when git ignores it
    let root_ignored = Command::new("git").arg("-C").arg(root).args(["check-ignore", "-q"]).arg(abs(root)).stderr(Stdio::null()).status().is_ok_and(|s| s.code() == Some(0));
    if root_ignored {
        return out;
    }
    let asked: Vec<&PathBuf> = files.iter().filter(|f| !rules.included(f, false)).collect();
    if asked.is_empty() {
        return out;
    }
    let Ok(mut child) = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["check-ignore", "-z", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return out;
    };
    let mut input = Vec::new();
    for f in &asked {
        input.extend_from_slice(abs(f).to_string_lossy().as_bytes());
        input.push(0);
    }
    if let Some(mut stdin) = child.stdin.take() {
        let writer = std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let Ok(result) = child.wait_with_output() else { return out };
        let _ = writer.join();
        // exit 0: some ignored, 1: none, 128: not a repository (rule 4 does not apply)
        if result.status.code() == Some(0) {
            let ignored: std::collections::HashSet<PathBuf> =
                result.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()).map(|s| PathBuf::from(String::from_utf8_lossy(s).to_string())).collect();
            for f in &asked {
                if ignored.contains(&abs(f)) {
                    out.insert((*f).clone());
                }
            }
        }
    }
    out
}

impl Scope {
    /// W0018 text for every skipped generated directory older than the newest source of the walk.
    pub fn stale_generated_messages(&self) -> Vec<String> {
        let Some(newest) = self.newest_source else { return Vec::new() };
        let mut out: Vec<String> = self
            .generated
            .iter()
            .filter(|(_, t)| *t < newest)
            .map(|(dir, _)| {
                let rel = dir.strip_prefix(&self.root).unwrap_or(dir);
                format!(
                    "generated directory {}/ is older than its sources (stale output is ignored by check; run wj build or wj clean --generated)",
                    rel.display()
                )
            })
            .collect();
        out.sort();
        out
    }

    /// The note `wj check` prints when files were left out.
    pub fn skipped_line(&self) -> Option<String> {
        (self.skipped > 0).then(|| format!("skipped {} ignored/generated files (see --no-ignore)", self.skipped))
    }
}
