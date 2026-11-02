//! `wj doctor`: every `wj` on PATH (resolved, version, commit, mtime), duplicates with different commits,
//! stale binaries, the shared runtime cache, and the exact commands that fix each problem.

use super::identity::{self, Identity};
use super::pin::DEFAULT_REPO;
use super::stale::{self, Staleness, DAY};
use super::timefmt::{iso_date, iso_datetime, now_ts};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Binary {
    /// The PATH entry as found (may be a symlink).
    pub found: PathBuf,
    pub resolved: PathBuf,
    pub identity: Option<Identity>,
    pub error: Option<String>,
    pub mtime: Option<i64>,
    pub stale: Option<Staleness>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub kind: &'static str,
    pub message: String,
    pub fix: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CacheState {
    pub dir: PathBuf,
    pub exists: bool,
    pub age_days: Option<i64>,
}

pub const CACHE_OLD_DAYS: i64 = 60;

pub struct Report {
    pub current: Identity,
    pub current_exe: PathBuf,
    pub checkout: Option<PathBuf>,
    pub binaries: Vec<Binary>,
    pub cache: CacheState,
    pub problems: Vec<Problem>,
}

// ---- discovery ----

/// Every distinct executable named `wj` along `path_env`, in PATH order (symlinks resolved, duplicates by
/// resolved path collapsed onto the first occurrence).
pub fn find_on_path(path_env: &str) -> Vec<(PathBuf, PathBuf)> {
    let mut out: Vec<(PathBuf, PathBuf)> = Vec::new();
    for dir in std::env::split_paths(path_env) {
        let cand = dir.join(if cfg!(windows) { "wj.exe" } else { "wj" });
        if !cand.is_file() {
            continue;
        }
        let resolved = std::fs::canonicalize(&cand).unwrap_or_else(|_| cand.clone());
        if !out.iter().any(|(_, r)| *r == resolved) {
            out.push((cand, resolved));
        }
    }
    out
}

/// Run `<bin> --version` with a 5 second limit.
pub fn probe(bin: &Path) -> Result<Identity, String> {
    use std::io::Read;
    let mut child = std::process::Command::new(bin)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run: {e}"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
            Ok(None) => {
                let _ = child.kill();
                return Err("`--version` timed out".into());
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    let mut text = String::new();
    if let Some(mut so) = child.stdout.take() {
        let _ = so.read_to_string(&mut text);
    }
    identity::parse_version_line(&text).ok_or_else(|| {
        format!(
            "unrecognised `--version` output: {:?}",
            text.lines().next().unwrap_or("")
        )
    })
}

fn mtime_of(p: &Path) -> Option<i64> {
    std::fs::metadata(p)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

pub fn cache_state(now: i64) -> CacheState {
    let dir = crate::cargo_cache::cache_root()
        .join("cargo-target")
        .join("shared");
    if !dir.is_dir() {
        return CacheState {
            dir,
            exists: false,
            age_days: None,
        };
    }
    let newest = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| mtime_of(&e.path()))
        .chain(mtime_of(&dir))
        .max();
    CacheState {
        dir,
        exists: true,
        age_days: newest.map(|t| (now - t) / DAY),
    }
}

// ---- diagnosis (pure) ----

fn same_commit(a: &Identity, b: &Identity) -> bool {
    match (&a.commit, &b.commit) {
        (Some(x), Some(y)) => x.starts_with(y.as_str()) || y.starts_with(x.as_str()),
        (None, None) => a.version == b.version,
        _ => false,
    }
}

pub struct Inputs<'a> {
    pub binaries: &'a [Binary],
    pub current: &'a Identity,
    pub current_exe: &'a Path,
    pub checkout: Option<&'a Path>,
    pub cache: &'a CacheState,
}

/// The command that installs a fresh `wj`.
pub fn refresh_command(i: &Inputs) -> String {
    if let Some(c) = i.checkout {
        format!(
            "cd {} && cargo build --release --bin wj && target/release/wj install-dev",
            c.display()
        )
    } else if i
        .current
        .commit_ts
        .is_some_and(|t| stale::assess(t, now_ts(), None).is_none())
    {
        format!("{} install-dev", i.current_exe.display())
    } else {
        format!("cargo install --git {DEFAULT_REPO} --bin wj --locked --force")
    }
}

pub fn diagnose(i: &Inputs) -> Vec<Problem> {
    let mut problems = Vec::new();
    let refresh = refresh_command(i);
    if i.binaries.is_empty() {
        problems.push(Problem {
            kind: "missing",
            message: "no `wj` found on PATH".into(),
            fix: vec![refresh.clone()],
        });
    }
    for b in i.binaries {
        match (&b.identity, &b.error) {
            (_, Some(e)) => problems.push(Problem {
                kind: "broken",
                message: format!("{} does not run: {e}", b.resolved.display()),
                fix: vec![refresh.clone()],
            }),
            (Some(id), _) if id.commit.is_none() => problems.push(Problem {
                kind: "stale",
                message: format!(
                    "{} (wj {}) predates build identity stamping, so its age cannot be known",
                    b.resolved.display(),
                    id.version
                ),
                fix: vec![refresh.clone()],
            }),
            (Some(id), _) => {
                if let (Some(s), Some(ts)) = (&b.stale, id.commit_ts) {
                    let against = if s.vs_checkout {
                        "the newest commit in the enclosing checkout"
                    } else {
                        "today"
                    };
                    problems.push(Problem {
                        kind: "stale",
                        message: format!(
                            "{} is from commit {} ({}), {} days behind {against}",
                            b.resolved.display(),
                            id.short(),
                            iso_date(ts),
                            s.days
                        ),
                        fix: vec![refresh.clone()],
                    });
                }
            }
            _ => {}
        }
    }
    let known: Vec<&Binary> = i.binaries.iter().filter(|b| b.identity.is_some()).collect();
    if let Some(first) = known.first() {
        let differing: Vec<&&Binary> = known
            .iter()
            .skip(1)
            .filter(|b| {
                !same_commit(
                    b.identity.as_ref().unwrap(),
                    first.identity.as_ref().unwrap(),
                )
            })
            .collect();
        if !differing.is_empty() {
            let mut fix = vec![format!(
                "{} install-dev --to {}",
                i.current_exe.display(),
                first.found.display()
            )];
            fix.extend(
                differing
                    .iter()
                    .map(|b| format!("rm {}", b.found.display())),
            );
            problems.push(Problem {
                kind: "duplicate",
                message: format!(
                    "{} different `wj` builds on PATH; {} runs first ({}), shadowing {}",
                    differing.len() + 1,
                    first.found.display(),
                    first.identity.as_ref().unwrap().short(),
                    differing
                        .iter()
                        .map(|b| format!(
                            "{} ({})",
                            b.found.display(),
                            b.identity.as_ref().unwrap().short()
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                fix,
            });
        }
        let cur_is_newer = i
            .current
            .commit_ts
            .zip(first.identity.as_ref().unwrap().commit_ts)
            .is_some_and(|(c, f)| c > f);
        if cur_is_newer && !same_commit(i.current, first.identity.as_ref().unwrap()) {
            problems.push(Problem {
                kind: "shadowed",
                message: format!(
                    "the `wj` that would run from PATH ({}) is older than the one running now (commit {})",
                    first.found.display(),
                    i.current.short()
                ),
                fix: vec![format!("{} install-dev", i.current_exe.display())],
            });
        }
    }
    if !i.cache.exists {
        problems.push(Problem {
            kind: "cache",
            message: format!(
                "shared runtime cache is missing ({})",
                i.cache.dir.display()
            ),
            fix: vec!["wj cache setup".into(), "wj cache warm".into()],
        });
    } else if i.cache.age_days.is_some_and(|d| d > CACHE_OLD_DAYS) {
        problems.push(Problem {
            kind: "cache",
            message: format!(
                "shared runtime cache is {} days old ({})",
                i.cache.age_days.unwrap(),
                i.cache.dir.display()
            ),
            fix: vec!["wj cache prune".into(), "wj cache warm".into()],
        });
    }
    problems
}

// ---- collection ----

pub fn collect(path_env: &str) -> Report {
    let now = now_ts();
    let current = identity::current();
    let current_exe = std::env::current_exe().unwrap_or_default();
    let checkout = std::env::current_dir()
        .ok()
        .and_then(|d| stale::enclosing_checkout(&d))
        .or_else(|| stale::enclosing_checkout(&current_exe));
    let newest = checkout.as_deref().and_then(newest_in_checkout);
    let binaries: Vec<Binary> = find_on_path(path_env)
        .into_iter()
        .map(|(found, resolved)| {
            let (identity, error) = match probe(&resolved) {
                Ok(id) => (Some(id), None),
                Err(e) => (None, Some(e)),
            };
            let stale = identity
                .as_ref()
                .and_then(|id| id.commit_ts)
                .and_then(|ts| stale::assess(ts, now, newest));
            let mtime = mtime_of(&resolved);
            Binary {
                found,
                resolved,
                identity,
                error,
                mtime,
                stale,
            }
        })
        .collect();
    let cache = cache_state(now);
    let problems = diagnose(&Inputs {
        binaries: &binaries,
        current: &current,
        current_exe: &current_exe,
        checkout: checkout.as_deref(),
        cache: &cache,
    });
    Report {
        current,
        current_exe,
        checkout,
        binaries,
        cache,
        problems,
    }
}

fn newest_in_checkout(c: &Path) -> Option<i64> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(c)
        .args(["log", "-1", "--format=%ct", "--all"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

/// `wj doctor --expect-head`: the `wj` that PATH resolves first must be built from the HEAD of the checkout
/// around the current directory. The merge gate puts its own build first on PATH, which is how a build is
/// "explicitly marked" as the one under test.
pub fn expect_head(path_env: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let checkout = stale::enclosing_checkout(&cwd).ok_or("not inside a windjammer checkout")?;
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(&checkout)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    let head = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let (found, resolved) = find_on_path(path_env)
        .into_iter()
        .next()
        .ok_or("no `wj` on PATH")?;
    let id = probe(&resolved)?;
    let commit = id
        .commit
        .clone()
        .ok_or_else(|| format!("{} predates build identity stamping", found.display()))?;
    if head.starts_with(&commit) {
        Ok(format!(
            "{} is built from HEAD {}",
            found.display(),
            &head[..12]
        ))
    } else {
        Err(format!(
            "{} is built from {} but HEAD is {}.\n  fix: cd {} && cargo build --release --bin wj && target/release/wj install-dev",
            found.display(),
            id.short(),
            &head[..12],
            checkout.display()
        ))
    }
}

// ---- rendering ----

pub fn render_text(r: &Report) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "running: {}\n  {}\n",
        r.current_exe.display(),
        r.current.version_line()
    ));
    if let Some(c) = &r.checkout {
        s.push_str(&format!("compiler checkout: {}\n", c.display()));
    }
    s.push_str("\nwj on PATH:\n");
    if r.binaries.is_empty() {
        s.push_str("  (none)\n");
    }
    for (n, b) in r.binaries.iter().enumerate() {
        let link = if b.found != b.resolved {
            format!(" -> {}", b.resolved.display())
        } else {
            String::new()
        };
        let (ver, commit) = match &b.identity {
            Some(id) => (
                id.version.clone(),
                format!("{}{}", id.short(), if id.dirty { "+dirty" } else { "" }),
            ),
            None => ("?".into(), "?".into()),
        };
        let date = b
            .identity
            .as_ref()
            .and_then(|i| i.commit_ts)
            .map(iso_date)
            .unwrap_or_else(|| "unknown".into());
        let mtime = b.mtime.map(iso_datetime).unwrap_or_else(|| "?".into());
        s.push_str(&format!(
            "  {}. {}{}\n     version {ver}  commit {commit} ({date})  mtime {mtime}{}\n",
            n + 1,
            b.found.display(),
            link,
            if b.stale.is_some() { "  STALE" } else { "" }
        ));
    }
    s.push_str(&format!(
        "\nshared runtime cache: {} ({})\n",
        r.cache.dir.display(),
        match (r.cache.exists, r.cache.age_days) {
            (false, _) => "missing".to_string(),
            (true, Some(d)) => format!("updated {d} days ago"),
            (true, None) => "present".to_string(),
        }
    ));
    if r.problems.is_empty() {
        s.push_str("\nno problems found\n");
    } else {
        s.push_str(&format!("\n{} problem(s):\n", r.problems.len()));
        for p in &r.problems {
            s.push_str(&format!("  [{}] {}\n", p.kind, p.message));
            for f in &p.fix {
                s.push_str(&format!("      fix: {f}\n"));
            }
        }
    }
    s
}

pub fn render_json(r: &Report) -> Value {
    json!({
        "running": { "path": r.current_exe, "identity": r.current.to_json() },
        "checkout": r.checkout,
        "binaries": r.binaries.iter().map(|b| json!({
            "path": b.found,
            "resolved": b.resolved,
            "identity": b.identity.as_ref().map(|i| i.to_json()),
            "error": b.error,
            "mtime": b.mtime.map(iso_datetime),
            "stale_days": b.stale.as_ref().map(|s| s.days),
        })).collect::<Vec<_>>(),
        "cache": { "dir": r.cache.dir, "exists": r.cache.exists, "age_days": r.cache.age_days },
        "problems": r.problems.iter().map(|p| json!({ "kind": p.kind, "message": p.message, "fix": p.fix })).collect::<Vec<_>>(),
        "ok": r.problems.is_empty(),
    })
}
