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

#[allow(dead_code)]
#[path = "../../build_support/content_key.rs"]
mod content_key;
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

/// One line when this binary was built from other sources than `checkout` holds now: the content key it was built
/// from differs from the checkout's (build_support/content_key.rs, the same digest build.rs bakes in). Exact and
/// git-free. To stay off the hot path, the digest is computed only when an input file is newer than the build.
pub fn key_behind_text(id: &Identity, checkout: &Path) -> Option<String> {
    let mine = id.key.as_deref()?;
    let newest = content_key::input_files(checkout)
        .iter()
        .filter_map(|f| std::fs::metadata(checkout.join(f)).and_then(|m| m.modified()).ok())
        .filter_map(|t| t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs() as i64))
        .max()?;
    if id.build_ts.is_some_and(|built| newest <= built) {
        return None;
    }
    let theirs = content_key::inputs_key(checkout);
    if theirs.starts_with(mine) || mine.starts_with(theirs.as_str()) {
        return None;
    }
    let built = id.build_ts.map(iso_datetime).unwrap_or_else(|| "unknown".into());
    Some(format!(
        "wj: this wj (sources {}, built {built}) is not built from this checkout's sources ({}); run `wj install-dev --from <fresh build>` (for example `cargo build --release --bin wj && target/release/wj install-dev`)",
        &mine[..mine.len().min(10)],
        &theirs[..10]
    ))
}

/// The checkout enclosing the current directory (or this executable), and the line for it, if behind: by commit
/// for a build stamped with one, else by the content key of the sources.
pub fn current_behind_text() -> Option<String> {
    let checkout = std::env::current_dir()
        .ok()
        .and_then(|d| super::stale::enclosing_checkout(&d))
        .or_else(|| std::env::current_exe().ok().and_then(|e| super::stale::enclosing_checkout(&e)))?;
    let id = super::identity::current();
    if id.commit.is_some() {
        return behind_text(&id, &checkout_head(&checkout)?);
    }
    key_behind_text(&id, &checkout)
}

/// Print the line to stderr when this `wj` runs inside a compiler checkout it was not built from.
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
            key: None,
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
