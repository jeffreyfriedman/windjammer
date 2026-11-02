//! Age-based staleness: a `wj` whose commit date is more than 30 days older than the newest commit in an
//! enclosing compiler checkout, or more than 60 days old when there is none, warns once per day on stderr.
//! `WJ_NO_STALE_WARN=1` silences it.
//!
//! Hot path: a binary younger than 30 days returns after one clock read and one subtraction. Only an old
//! binary reads the daily stamp, and the checkout's newest commit date is recomputed (one `git log`) only when
//! the checkout's HEAD or reflog changed since the stamp was written.

use super::timefmt::{iso_date, now_ts};
use std::path::{Path, PathBuf};

pub const DAY: i64 = 86_400;
pub const BEHIND_CHECKOUT_DAYS: i64 = 30;
pub const STANDALONE_DAYS: i64 = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staleness {
    pub days: i64,
    /// `true` when measured against a compiler checkout, `false` when against the clock.
    pub vs_checkout: bool,
}

/// Is a binary built from a commit at `binary_ts` stale? `newest_in_checkout` is the newest commit date
/// visible in an enclosing compiler checkout.
pub fn assess(binary_ts: i64, now: i64, newest_in_checkout: Option<i64>) -> Option<Staleness> {
    match newest_in_checkout {
        Some(newest) => {
            let days = (newest - binary_ts) / DAY;
            (days > BEHIND_CHECKOUT_DAYS).then_some(Staleness { days, vs_checkout: true })
        }
        None => {
            let days = (now - binary_ts) / DAY;
            (days > STANDALONE_DAYS).then_some(Staleness { days, vs_checkout: false })
        }
    }
}

/// Cheap pre-check: can this binary possibly be stale? A binary at most 30 days old cannot be more than
/// 30 days behind any commit that exists, nor more than 60 days old.
pub fn may_be_stale(binary_ts: i64, now: i64) -> bool {
    now - binary_ts > BEHIND_CHECKOUT_DAYS * DAY
}

pub fn warning_text(binary_commit: &str, binary_ts: i64, s: &Staleness, checkout: Option<&Path>) -> String {
    let against = match (s.vs_checkout, checkout) {
        (true, Some(c)) => format!("{} days older than the newest commit in {}", s.days, c.display()),
        _ => format!("{} days old", s.days),
    };
    format!(
        "wj: warning: this wj (commit {}, {}) is {}; run `wj doctor` for the fix (silence: WJ_NO_STALE_WARN=1)",
        &binary_commit[..binary_commit.len().min(12)],
        iso_date(binary_ts),
        against
    )
}

// ---- enclosing checkout ----

/// The compiler checkout enclosing `start`: a directory whose Cargo.toml names the `windjammer` package and
/// that has a `.git`.
pub fn enclosing_checkout(start: &Path) -> Option<PathBuf> {
    start.ancestors().find_map(|d| {
        let cargo = std::fs::read_to_string(d.join("Cargo.toml")).ok()?;
        let named = cargo.lines().any(|l| l.trim().replace(' ', "") == "name=\"windjammer\"");
        (named && d.join(".git").exists()).then(|| d.to_path_buf())
    })
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

/// Changes whenever the checkout's HEAD moves (commit, checkout, pull).
fn fingerprint(checkout: &Path) -> String {
    let Some(g) = git_dir(checkout) else { return String::new() };
    let stamp = |rel: &str| {
        std::fs::metadata(g.join(rel))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format!("{}.{}", d.as_secs(), d.subsec_nanos()))
            .unwrap_or_default()
    };
    format!("{}:{}", stamp("HEAD"), stamp("logs/HEAD"))
}

fn newest_commit_ts(checkout: &Path) -> Option<i64> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["log", "-1", "--format=%ct", "--all"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok()
}

// ---- daily stamp ----

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stamp {
    pub warned_day: i64,
    pub fingerprint: String,
    pub newest: i64,
}

impl Stamp {
    pub fn parse(s: &str) -> Stamp {
        let mut st = Stamp::default();
        for l in s.lines() {
            match l.split_once('=') {
                Some(("warned_day", v)) => st.warned_day = v.parse().unwrap_or(0),
                Some(("fingerprint", v)) => st.fingerprint = v.to_string(),
                Some(("newest", v)) => st.newest = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        st
    }

    pub fn render(&self) -> String {
        format!("warned_day={}\nfingerprint={}\nnewest={}\n", self.warned_day, self.fingerprint, self.newest)
    }
}

pub fn stamp_path() -> PathBuf {
    crate::cargo_cache::cache_root().join("toolchain").join("stale-warning.stamp")
}

/// Print the once-per-day stale warning if due. Never fails; never slows a fresh binary.
pub fn warn_if_stale() {
    warn_with(&super::identity::current(), now_ts());
}

/// `warn_if_stale` with the identity and clock supplied (the hot-path gate times this).
pub fn warn_with(id: &super::identity::Identity, now: i64) {
    let (Some(ts), Some(commit)) = (id.commit_ts, id.commit.as_deref()) else { return };
    if !may_be_stale(ts, now) || std::env::var_os("WJ_NO_STALE_WARN").is_some_and(|v| v != "0" && !v.is_empty()) {
        return;
    }
    let path = stamp_path();
    let before = std::fs::read_to_string(&path).unwrap_or_default();
    let mut stamp = Stamp::parse(&before);
    if stamp.warned_day == now / DAY {
        return;
    }
    let cwd = std::env::current_dir().ok();
    let checkout = cwd
        .as_deref()
        .and_then(enclosing_checkout)
        .or_else(|| std::env::current_exe().ok().and_then(|e| enclosing_checkout(&e)));
    let newest = checkout.as_deref().and_then(|c| {
        let fp = fingerprint(c);
        if stamp.fingerprint != fp || stamp.newest == 0 {
            stamp.newest = newest_commit_ts(c).unwrap_or(0);
            stamp.fingerprint = fp;
        }
        (stamp.newest > 0).then_some(stamp.newest)
    });
    if let Some(s) = assess(ts, now, newest) {
        eprintln!("{}", warning_text(commit, ts, &s, checkout.as_deref()));
        stamp.warned_day = now / DAY;
    }
    let after = stamp.render();
    if after != before {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&path, after);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn behind_checkout_threshold_is_thirty_days() {
        let newest = NOW;
        assert!(assess(newest - 30 * DAY, NOW, Some(newest)).is_none());
        let s = assess(newest - 31 * DAY - 5, NOW, Some(newest)).unwrap();
        assert!(s.vs_checkout && s.days == 31);
        // a standalone-old binary is fine when the checkout is equally old
        assert!(assess(NOW - 90 * DAY, NOW, Some(NOW - 90 * DAY)).is_none());
    }

    #[test]
    fn standalone_threshold_is_sixty_days() {
        assert!(assess(NOW - 60 * DAY, NOW, None).is_none());
        assert!(assess(NOW - 61 * DAY, NOW, None).is_some());
        assert!(!may_be_stale(NOW - 30 * DAY, NOW) && may_be_stale(NOW - 31 * DAY, NOW));
    }

    #[test]
    fn stamp_round_trips() {
        let s = Stamp { warned_day: 20_000, fingerprint: "1.2:3.4".into(), newest: 99 };
        assert_eq!(Stamp::parse(&s.render()), s);
    }

    #[test]
    fn warning_is_one_line_and_names_the_escape_hatch() {
        let s = assess(NOW - 90 * DAY, NOW, None).unwrap();
        let w = warning_text("2b6db230c4a1ffff", NOW - 90 * DAY, &s, None);
        assert_eq!(w.lines().count(), 1);
        assert!(w.contains("wj doctor") && w.contains("WJ_NO_STALE_WARN=1"));
    }
}
