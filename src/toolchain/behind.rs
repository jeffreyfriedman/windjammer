//! Commit-based staleness: is this `wj` built from an older commit than the compiler checkout it runs in?
//!
//! The age-based check (`stale.rs`) waits 30 days; a `wj` a few days behind its checkout already lacks
//! subcommands and accepted inputs (`wj eval`, `wj run -`), and the failure it gives ("plugin not found", "no
//! such file") hides the cause. This check compares the binary's build commit with the checkout's HEAD and says so
//! in one line.
//!
//! Cheap and git-free: HEAD, the ref it names (loose or packed) and, for "older", the HEAD commit's committer
//! date read from its loose object (zlib). No checkout, no git, or a packed HEAD commit: nothing is claimed that
//! cannot be read. `WJ_CHECKOUT_HEAD=<sha>[@<unix seconds>]` stands in for the checkout's HEAD (tests).

use super::identity::Identity;
use super::timefmt::iso_datetime;
use std::path::{Path, PathBuf};

/// The checkout's HEAD commit and, when readable, its committer date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Head {
    pub sha: String,
    pub commit_ts: Option<i64>,
}

/// HEAD of the checkout at `checkout` (or the `WJ_CHECKOUT_HEAD` override).
pub fn checkout_head(checkout: &Path) -> Option<Head> {
    if let Some(v) = super::env::get("WJ_CHECKOUT_HEAD").filter(|v| !v.is_empty()) {
        let (sha, ts) = match v.split_once('@') {
            Some((s, t)) => (s.to_string(), t.parse().ok()),
            None => (v, None),
        };
        return Some(Head { sha, commit_ts: ts });
    }
    let git = git_dir(checkout)?;
    let common = common_dir(&git);
    let head = std::fs::read_to_string(git.join("HEAD")).ok()?;
    let head = head.trim();
    let sha = match head.strip_prefix("ref: ") {
        Some(r) => read_ref(&git, &common, r)?,
        None => head.to_string(),
    };
    let commit_ts = commit_ts(&common, &sha);
    Some(Head { sha, commit_ts })
}

fn git_dir(checkout: &Path) -> Option<PathBuf> {
    let dot = checkout.join(".git");
    if dot.is_dir() {
        return Some(dot);
    }
    let text = std::fs::read_to_string(&dot).ok()?;
    let p = PathBuf::from(text.trim().strip_prefix("gitdir:")?.trim());
    Some(if p.is_absolute() { p } else { checkout.join(p) })
}

/// A worktree's git dir keeps HEAD; refs and objects live in the common dir (`commondir`).
fn common_dir(git: &Path) -> PathBuf {
    match std::fs::read_to_string(git.join("commondir")) {
        Ok(rel) => {
            let p = PathBuf::from(rel.trim());
            if p.is_absolute() { p } else { git.join(p) }
        }
        Err(_) => git.to_path_buf(),
    }
}

fn read_ref(git: &Path, common: &Path, name: &str) -> Option<String> {
    for dir in [git, common] {
        if let Ok(s) = std::fs::read_to_string(dir.join(name)) {
            return Some(s.trim().to_string());
        }
    }
    let packed = std::fs::read_to_string(common.join("packed-refs")).ok()?;
    packed.lines().find_map(|l| {
        let (sha, r) = l.split_once(' ')?;
        (r == name).then(|| sha.to_string())
    })
}

/// Committer date of a loose commit object (`None` when packed or unreadable).
fn commit_ts(common: &Path, sha: &str) -> Option<i64> {
    use std::io::Read;
    if sha.len() < 3 {
        return None;
    }
    let raw = std::fs::read(common.join("objects").join(&sha[..2]).join(&sha[2..])).ok()?;
    let mut text = String::new();
    flate2::read::ZlibDecoder::new(&raw[..]).read_to_string(&mut text).ok()?;
    let line = text.lines().find(|l| l.contains("committer "))?;
    let mut words = line.rsplit(' ');
    let _tz = words.next()?;
    words.next()?.parse().ok()
}

/// One line when this binary is behind `head`: built from another commit that is older than HEAD (or another
/// commit whose age cannot be read). `None` when the commits are the same, or this binary is the newer one.
pub fn behind_text(id: &Identity, head: &Head) -> Option<String> {
    let commit = id.commit.as_deref()?;
    if head.sha.starts_with(commit) || commit.starts_with(head.sha.as_str()) {
        return None;
    }
    let older = match (id.commit_ts, head.commit_ts) {
        (Some(mine), Some(theirs)) => mine < theirs,
        _ => true,
    };
    if !older {
        return None;
    }
    let built = id.build_ts.map(iso_datetime).unwrap_or_else(|| "unknown".into());
    let short = |s: &str| s[..s.len().min(12)].to_string();
    let relation = if head.commit_ts.is_some() { "is older than" } else { "is not built from" };
    Some(format!(
        "wj: this wj (commit {}, built {built}) {relation} this checkout (commit {}); run `wj install-dev --from <fresh build>` (for example `cargo build --release --bin wj && target/release/wj install-dev`)",
        short(commit),
        short(&head.sha)
    ))
}

/// What goes into a `wj` binary: the compiler's sources, the embedded std and runtime crates, the build script and the
/// manifests. A commit that changes nothing here (documentation, tests, benchmark data, scripts) leaves a binary
/// built before it current.
pub const COMPILER_INPUTS: &[&str] = &["src", "std", "crates", "build.rs", "build_support", "Cargo.toml", "Cargo.lock"];

/// Did any of `COMPILER_INPUTS` change between commits `from` and `to` of `checkout`? `None`: git cannot say (no git,
/// a commit it does not have), and the caller keeps the commit comparison.
pub fn inputs_changed(checkout: &Path, from: &str, to: &str) -> Option<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["diff", "--quiet", from, to, "--"])
        .args(COMPILER_INPUTS)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    match status.code() {
        Some(0) => Some(false),
        Some(1) => Some(true),
        _ => None,
    }
}

/// `behind_text`, unless the commits differ only outside the compiler's inputs (asked of git only when the commits
/// differ, so a current binary pays nothing).
pub fn behind_text_in(checkout: &Path, id: &Identity, head: &Head) -> Option<String> {
    let line = behind_text(id, head)?;
    let commit = id.commit.as_deref()?;
    if super::env::get("WJ_CHECKOUT_HEAD").is_none() && inputs_changed(checkout, commit, &head.sha) == Some(false) {
        return None;
    }
    Some(line)
}

/// The checkout enclosing the current directory (or this executable), and the line for it, if behind.
pub fn current_behind_text() -> Option<String> {
    let checkout = std::env::current_dir()
        .ok()
        .and_then(|d| super::stale::enclosing_checkout(&d))
        .or_else(|| std::env::current_exe().ok().and_then(|e| super::stale::enclosing_checkout(&e)))?;
    behind_text_in(&checkout, &super::identity::current(), &checkout_head(&checkout)?)
}

/// Print the line to stderr when this `wj` runs inside a compiler checkout whose HEAD is newer.
pub fn warn_if_behind() {
    if let Some(line) = current_behind_text() {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(commit: &str, ts: i64) -> Identity {
        Identity {
            version: "0.50.0".into(),
            commit: Some(commit.into()),
            commit_ts: Some(ts),
            dirty: false,
            profile: Some("release".into()),
            build_ts: Some(ts),
        }
    }

    #[test]
    fn an_older_commit_is_reported_and_the_same_or_a_newer_one_is_not() {
        let head = Head { sha: "bbbbbbbbbbbbbbbb".into(), commit_ts: Some(2_000) };
        let line = behind_text(&id("aaaaaaaaaaaaaaaa", 1_000), &head).expect("older binary");
        assert!(line.contains("commit aaaaaaaaaaaa") && line.contains("is older than this checkout (commit bbbbbbbbbbbb)"), "{line}");
        assert!(line.contains("wj install-dev --from"), "{line}");
        // mutation check: matching commits never warn, even when the dates say otherwise
        assert_eq!(behind_text(&id("bbbbbbbbbbbbbbbb", 1_000), &head), None);
        assert_eq!(behind_text(&id("bbbbbbbbbbbb", 1_000), &head), None, "a short commit prefix matches");
        // a binary built from a newer commit than the checkout (a worktree build run in an older checkout)
        assert_eq!(behind_text(&id("cccccccccccccccc", 3_000), &head), None);
        // HEAD's date unreadable (packed): another commit is reported without claiming "older"
        let packed = Head { sha: "bbbb".into(), commit_ts: None };
        assert!(behind_text(&id("aaaa", 1_000), &packed).unwrap().contains("is not built from this checkout"));
    }

    /// A binary is behind only when the compiler's inputs changed since its commit: a commit that touches only data
    /// or documentation leaves it current. Mutation-checked both ways: the docs-only case fails if every difference
    /// counted, the source case fails if none did.
    #[test]
    fn only_a_change_to_the_compiler_inputs_makes_a_binary_behind() {
        let dir = std::env::temp_dir().join(format!("wj-behind-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        let git = |args: &[&str]| -> String {
            let o = std::process::Command::new("git").arg("-C").arg(&dir).args(args).output().expect("git");
            assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        };
        git(&["init", "-q"]);
        let commit = |msg: &str| {
            git(&["add", "-A"]);
            git(&["-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-q", "-m", msg]);
            git(&["rev-parse", "HEAD"])
        };
        std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.join("docs/a.md"), "a\n").unwrap();
        let built = commit("one");
        std::fs::write(dir.join("docs/a.md"), "b\n").unwrap();
        let docs_only = commit("docs");
        std::fs::write(dir.join("src/lib.rs"), "fn b() {}\n").unwrap();
        let source = commit("source");
        assert_eq!(inputs_changed(&dir, &built, &docs_only), Some(false));
        assert_eq!(inputs_changed(&dir, &built, &source), Some(true));
        assert_eq!(inputs_changed(&dir, &built, "0000000000000000000000000000000000000000"), None, "an unknown commit is no answer");
        let me = id(&built, 1_000);
        assert_eq!(behind_text_in(&dir, &me, &Head { sha: docs_only, commit_ts: Some(2_000) }), None);
        assert!(behind_text_in(&dir, &me, &Head { sha: source, commit_ts: Some(3_000) }).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn head_is_read_from_a_worktree_checkout_without_git() {
        let dir = std::env::temp_dir().join(format!("wj-behind-{}", std::process::id()));
        let common = dir.join("main/.git");
        let wt = common.join("worktrees/w");
        std::fs::create_dir_all(wt.clone()).unwrap();
        std::fs::create_dir_all(common.join("refs/heads")).unwrap();
        std::fs::create_dir_all(dir.join("w")).unwrap();
        std::fs::write(dir.join("w/.git"), format!("gitdir: {}\n", wt.display())).unwrap();
        std::fs::write(wt.join("HEAD"), "ref: refs/heads/topic\n").unwrap();
        std::fs::write(wt.join("commondir"), "../..\n").unwrap();
        std::fs::write(common.join("packed-refs"), "# pack-refs\n1234567890abcdef refs/heads/topic\n").unwrap();
        let head = checkout_head(&dir.join("w"));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(head, Some(Head { sha: "1234567890abcdef".into(), commit_ts: None }));
    }
}
