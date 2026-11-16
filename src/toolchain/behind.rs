//! Staleness: is this `wj` built from other sources than the compiler checkout it runs in? By the content key of
//! the sources the binary carries (`key_behind_text`; every build since build-key), else by commit (older binaries).
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

/// The content key of the sources in `checkout` (the digest build.rs bakes into a binary built from them).
pub fn checkout_inputs_key(checkout: &Path) -> String {
    content_key::inputs_key(checkout)
}

/// One line when this binary was built from other sources than `checkout` holds now: the content key it was built
/// from differs from the checkout's (build_support/content_key.rs, the same digest build.rs bakes in). Exact and
/// git-free. To stay off the hot path, the digest is computed only when an input file is newer than the build.
pub fn key_behind_text(id: &Identity, checkout: &Path) -> Option<String> {
    let mine = id.key.as_deref()?;
    // one pass, one `stat` per input, stopping at the first newer one (no path list built first)
    if let Some(built) = id.build_ts {
        if !content_key::any_input_newer_than(checkout, built) {
            return None;
        }
    }
    let theirs = content_key::inputs_key(checkout);
    if theirs.starts_with(mine) || mine.starts_with(theirs.as_str()) {
        return None;
    }
    let built = id.build_ts.map(iso_datetime).unwrap_or_else(|| "unknown".into());
    let commit = id.commit.as_deref().map(|c| format!("commit {}, ", &c[..c.len().min(12)])).unwrap_or_default();
    Some(format!(
        "wj: this wj ({commit}sources {}, built {built}) is not built from this checkout's sources ({}); run `wj install-dev --from <fresh build>` (for example `cargo build --release --bin wj && target/release/wj install-dev`)",
        &mine[..mine.len().min(10)],
        &theirs[..10]
    ))
}

/// How long a verdict of [`key_behind_text`] is reused: the check costs a `stat` per compiler source (about 33M
/// instructions per `wj` run inside a checkout, ADR-0038), and the warning a developer or an agent needs after editing the
/// compiler arrives at most this late.
pub const VERDICT_TTL_SECS: i64 = 60;

fn verdict_dir() -> std::path::PathBuf {
    crate::cargo_cache::cache_root().join("toolchain").join("behind")
}

fn unix_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// [`key_behind_text`], reused for [`VERDICT_TTL_SECS`] from a file under `dir` named by the binary's key and the
/// checkout (the shared cache root, not the temp dir). The file holds the time of the verdict and the line (empty when
/// current); anything unreadable, older or from the future recomputes. A new build has a new key, so it never reads
/// an old binary's verdict.
pub fn cached_key_behind_text(id: &Identity, checkout: &Path, dir: &Path, now: i64) -> Option<String> {
    let key = id.key.as_deref()?;
    let canon = std::fs::canonicalize(checkout).unwrap_or_else(|_| checkout.to_path_buf());
    let mut h: u64 = 0xcbf29ce484222325;
    for b in key.bytes().chain([0u8]).chain(canon.to_string_lossy().bytes()) {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    let file = dir.join(format!("{h:016x}"));
    if let Ok(text) = std::fs::read_to_string(&file) {
        if let Some((at, line)) = text.split_once('\n') {
            if let Ok(at) = at.trim().parse::<i64>() {
                if at <= now && now - at < VERDICT_TTL_SECS {
                    return (!line.is_empty()).then(|| line.to_string());
                }
            }
        }
    }
    let verdict = key_behind_text(id, checkout);
    let _ = std::fs::create_dir_all(dir);
    let tmp = dir.join(format!("{h:016x}.{}", std::process::id()));
    if std::fs::write(&tmp, format!("{now}\n{}", verdict.as_deref().unwrap_or(""))).is_ok() {
        let _ = std::fs::rename(&tmp, &file);
    }
    verdict
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

/// The line for `id` run in `checkout`, if behind. A binary that carries the content key of its sources (every build
/// since build-key) is compared by that key: exact, and blind to commits and files that are not compiler inputs, so a
/// stamped and an unstamped build of the same sources agree, and an uncommitted change to an input is seen either way.
/// The commit comparison is the fallback for older binaries without a key.
pub fn behind_for(id: &Identity, checkout: &Path) -> Option<String> {
    if id.key.is_some() {
        return key_behind_text(id, checkout);
    }
    behind_text_in(checkout, id, &checkout_head(checkout)?)
}

/// The checkout enclosing the current directory (or this executable), and the line for it, if behind ([`behind_for`]).
pub fn current_behind_text() -> Option<String> {
    let checkout = std::env::current_dir()
        .ok()
        .and_then(|d| super::stale::enclosing_checkout(&d))
        .or_else(|| std::env::current_exe().ok().and_then(|e| super::stale::enclosing_checkout(&e)))?;
    let id = super::identity::current();
    // the per-run path reuses a recent verdict ([`VERDICT_TTL_SECS`]); [`behind_for`] itself always judges afresh
    if id.key.is_some() {
        return cached_key_behind_text(&id, &checkout, &verdict_dir(), unix_now());
    }
    behind_for(&id, &checkout)
}

/// Print the line to stderr when this `wj` runs inside a compiler checkout it was not built from.
pub fn warn_if_behind() {
    // the instruction gates measure the interpreter, not this check (a `stat` per compiler source inside a checkout)
    if super::env::flag("WJ_NO_STALE_WARN") {
        return;
    }
    if let Some(line) = current_behind_text() {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A verdict is reused inside the TTL (no walk, even after an input changed), recomputed after it, and never shared
    /// with another binary key.
    #[test]
    fn the_behind_verdict_is_reused_for_its_ttl_and_per_binary_key() {
        let root = std::env::temp_dir().join(format!("wj-behind-ttl-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (checkout, cache) = (root.join("co"), root.join("cache"));
        std::fs::create_dir_all(checkout.join("src")).unwrap();
        std::fs::write(checkout.join("src/a.rs"), "fn a() {}").unwrap();
        let now = unix_now();
        let mut id = Identity { key: Some("0123456789abcdef".into()), version: "0.50.0".into(), commit: None, commit_ts: None, dirty: false, profile: None, build_ts: Some(now + 3600) };
        assert_eq!(cached_key_behind_text(&id, &checkout, &cache, now), None, "built after every input: current");
        id.build_ts = Some(now - 3600);
        assert_eq!(cached_key_behind_text(&id, &checkout, &cache, now + 1), None, "inside the TTL the cached verdict stands");
        assert!(cached_key_behind_text(&id, &checkout, &cache, now + VERDICT_TTL_SECS + 1).is_some(), "after the TTL the walk runs and sees the newer input");
        id.key = Some("fedcba9876543210".into());
        assert!(cached_key_behind_text(&id, &checkout, &cache, now + VERDICT_TTL_SECS + 2).is_some(), "another binary key computes its own verdict");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_input_newer_than_the_build_is_found_and_an_older_tree_is_not() {
        let root = std::env::temp_dir().join(format!("wj-behind-newer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src/deep")).unwrap();
        std::fs::write(root.join("src/deep/a.rs"), "fn a() {}").unwrap();
        std::fs::write(root.join("unrelated.txt"), "x").unwrap();
        let now = unix_now();
        assert!(content_key::any_input_newer_than(&root, now - 3600), "a file written now is newer than an hour ago");
        assert!(!content_key::any_input_newer_than(&root, now + 3600), "nothing is newer than an hour from now");
        let _ = std::fs::remove_dir_all(&root);
    }

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

    /// A stamped binary that carries its sources' key is judged by the key, as an unstamped one is: an uncommitted
    /// change to a compiler input is flagged although the commit matches, and a change to a non-input is not.
    /// Mutation-checked: with the commit logic for stamped builds the source case passes as current and fails here.
    #[test]
    fn a_stamped_binary_with_a_key_is_judged_by_its_sources_not_its_commit() {
        let dir = std::env::temp_dir().join(format!("wj-behind-key-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();
        std::fs::write(dir.join("src/lib.rs"), "fn a() {}\n").unwrap();
        std::fs::write(dir.join("docs/a.md"), "a\n").unwrap();
        let built_from = content_key::inputs_key(&dir);
        let head = "1234567890abcdef1234567890abcdef12345678";
        std::fs::create_dir_all(dir.join(".git/refs/heads")).unwrap();
        std::fs::write(dir.join(".git/HEAD"), format!("{head}\n")).unwrap();
        // built before the edits below (the mtime shortcut must not skip them)
        let me = Identity { key: Some(built_from), build_ts: Some(1), ..id(head, 1) };
        assert_eq!(behind_for(&me, &dir), None, "the sources it was built from");
        std::fs::write(dir.join("docs/a.md"), "b\n").unwrap();
        assert_eq!(behind_for(&me, &dir), None, "a documentation change is no compiler input");
        std::fs::write(dir.join("src/lib.rs"), "fn b() {}\n").unwrap();
        let line = behind_for(&me, &dir).expect("an uncommitted source change on the binary's own commit is flagged");
        assert!(line.contains("commit 1234567890ab") && line.contains("is not built from this checkout"), "{line}");
        // the same binary without a key (an older build) falls back to the commit: same commit, nothing claimed
        let old = Identity { key: None, ..me };
        assert_eq!(behind_for(&old, &dir), None);
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
