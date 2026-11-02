//! Build identity of this `wj`: crate version, commit, commit date, dirty flag, profile, build date.
//!
//! Values come from `build.rs`. The commit date is the only time value that may influence generated output;
//! the wall-clock build date (`WJ_BUILD_TS`) is read here and nowhere else (gate:
//! tests/toolchain_identity_test.rs).

use super::timefmt::{iso_date, iso_datetime, parse_iso_date};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const COMMIT: &str = env!("WJ_GIT_HASH");
const COMMIT_TS: &str = env!("WJ_GIT_TS");
const DIRTY: &str = env!("WJ_GIT_DIRTY");
pub const PROFILE: &str = env!("WJ_BUILD_PROFILE");
const BUILD_TS: &str = env!("WJ_BUILD_TS");

/// What a `wj` binary says about itself (own or parsed from another binary's `--version`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub version: String,
    /// Full commit hash, or a prefix when parsed from `--version` text; `None` for binaries that predate stamping.
    pub commit: Option<String>,
    /// Commit date as unix seconds (midnight UTC when parsed from text).
    pub commit_ts: Option<i64>,
    pub dirty: bool,
    pub profile: Option<String>,
    pub build_ts: Option<i64>,
}

pub fn current() -> Identity {
    let known = COMMIT != "unknown";
    Identity {
        version: VERSION.to_string(),
        commit: known.then(|| COMMIT.to_string()),
        commit_ts: COMMIT_TS.parse().ok().filter(|t| *t > 0),
        dirty: DIRTY == "true",
        profile: Some(PROFILE.to_string()),
        build_ts: BUILD_TS.parse().ok().filter(|t| *t > 0),
    }
}

impl Identity {
    pub fn short(&self) -> &str {
        match &self.commit {
            Some(c) => &c[..c.len().min(12)],
            None => "unknown",
        }
    }

    /// `wj --version` text: `wj 0.50.0 (commit 2b6db230c4a1 2026-11-02, clean, release, built 2026-11-02T10:11:12Z)`.
    pub fn version_line(&self) -> String {
        let date = self
            .commit_ts
            .map(iso_date)
            .unwrap_or_else(|| "unknown".into());
        let built = self
            .build_ts
            .map(iso_datetime)
            .unwrap_or_else(|| "unknown".into());
        format!(
            "wj {} (commit {} {}, {}, {}, built {})",
            self.version,
            self.short(),
            date,
            if self.dirty { "dirty" } else { "clean" },
            self.profile.as_deref().unwrap_or("unknown"),
            built
        )
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "name": "wj",
            "version": self.version,
            "commit": self.commit,
            "commit_short": self.commit.as_ref().map(|_| self.short()),
            "commit_date": self.commit_ts.map(iso_date),
            "commit_timestamp": self.commit_ts,
            "dirty": self.dirty,
            "profile": self.profile,
            "build_date": self.build_ts.map(iso_datetime),
            "build_timestamp": self.build_ts,
        })
    }
}

/// Parse the output of `wj --version`. Old binaries print only `wj <version>`; those parse with no commit.
pub fn parse_version_line(text: &str) -> Option<Identity> {
    let line = text.lines().next()?.trim();
    let rest = line.strip_prefix("wj ")?;
    let (version, detail) = match rest.split_once(' ') {
        Some((v, d)) => (v, Some(d)),
        None => (rest, None),
    };
    if version.is_empty() || !version.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let mut id = Identity {
        version: version.to_string(),
        commit: None,
        commit_ts: None,
        dirty: false,
        profile: None,
        build_ts: None,
    };
    let Some(inner) = detail
        .and_then(|d| d.strip_prefix('('))
        .and_then(|d| d.strip_suffix(')'))
    else {
        return Some(id);
    };
    for (i, part) in inner.split(", ").enumerate() {
        if i == 0 {
            let mut w = part.strip_prefix("commit ")?.split(' ');
            let hash = w.next()?;
            if hash != "unknown" {
                id.commit = Some(hash.to_string());
            }
            id.commit_ts = w.next().and_then(parse_iso_date);
        } else if part == "dirty" {
            id.dirty = true;
        } else if part == "clean" {
        } else if let Some(b) = part.strip_prefix("built ") {
            id.build_ts = parse_utc_datetime(b);
        } else {
            id.profile = Some(part.to_string());
        }
    }
    Some(id)
}

fn parse_utc_datetime(s: &str) -> Option<i64> {
    let (d, t) = s.strip_suffix('Z')?.split_once('T')?;
    let mut it = t.splitn(3, ':');
    let h: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let sec: i64 = it.next()?.parse().ok()?;
    Some(parse_iso_date(d)? + h * 3600 + m * 60 + sec)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Identity {
        Identity {
            version: "0.50.0".into(),
            commit: Some("2b6db230c4a1ffffffffffffffffffffffffffff".into()),
            commit_ts: Some(1_787_961_600),
            dirty: true,
            profile: Some("release".into()),
            build_ts: Some(1_787_999_999),
        }
    }

    #[test]
    fn version_line_round_trips() {
        let id = sample();
        let line = id.version_line();
        assert_eq!(line, "wj 0.50.0 (commit 2b6db230c4a1 2026-08-29, dirty, release, built 2026-08-29T10:39:59Z)");
        let back = parse_version_line(&line).unwrap();
        assert_eq!(back.commit.as_deref(), Some("2b6db230c4a1"));
        assert_eq!(back.commit_ts, id.commit_ts);
        assert!(back.dirty);
        assert_eq!(back.profile.as_deref(), Some("release"));
        assert_eq!(back.build_ts, id.build_ts);
    }

    #[test]
    fn old_binaries_parse_without_a_commit() {
        let id = parse_version_line("wj 0.50.0\n").unwrap();
        assert_eq!(id.version, "0.50.0");
        assert!(id.commit.is_none() && id.commit_ts.is_none());
        assert!(parse_version_line("not wj").is_none());
    }
}
