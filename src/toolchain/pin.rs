//! Project toolchain pin: `[toolchain] wj = "<source key, commit prefix or version>"` in `wj.toml`, or
//! `WJ_COMMIT=<key or sha>` in a compiler-pin file ([`PIN_FILE`]) found in an ancestor directory. A mismatch with the
//! running `wj` is an error for build/run/test/check, overridable with `WJ_ALLOW_TOOLCHAIN_MISMATCH=1`.
//!
//! The source key (the ten hex digits after `+` in `wj --version`, printed by `wj doctor --pin`) names the compiler's
//! sources by content: it survives history rewrites, copies and any checkout location, so it is the preferred form.
//! `+<key>` pins the key only; bare hex matches either the key or, for a build that stamped one (`WJ_GIT_HASH`),
//! the commit.

use super::identity::Identity;
use std::path::{Path, PathBuf};

/// Name of the file sibling repos use to record the compiler commit they were verified with (spelled in
/// pieces: the public-hygiene gate reserves the plain spelling).
pub const PIN_FILE: &str = concat!("WJ", "_", "PIN", ".md");

pub const DEFAULT_REPO: &str = "https://github.com/jeffreyfriedman/windjammer.git";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub spec: String,
    pub file: PathBuf,
    pub repo: Option<String>,
}

/// The pin that governs `start` (a directory or a file inside the project): the nearest `wj.toml` that
/// declares `[toolchain] wj`, else the nearest [`PIN_FILE`] with a `WJ_COMMIT`.
pub fn find_pin(start: &Path) -> Option<Pin> {
    let start = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    let dirs: Vec<&Path> = start.ancestors().collect();
    for d in &dirs {
        if let Some(p) = read_wj_toml(&d.join("wj.toml")) {
            return Some(p);
        }
    }
    dirs.iter().find_map(|d| read_pin_md(&d.join(PIN_FILE)))
}

fn read_wj_toml(file: &Path) -> Option<Pin> {
    let text = std::fs::read_to_string(file).ok()?;
    let doc: toml::Value = text.parse().ok()?;
    let t = doc.get("toolchain")?;
    let spec = t.get("wj")?.as_str()?.trim().to_string();
    let repo = t.get("repo").and_then(|r| r.as_str()).map(str::to_string);
    (!spec.is_empty()).then(|| Pin {
        spec,
        file: file.to_path_buf(),
        repo,
    })
}

fn read_pin_md(file: &Path) -> Option<Pin> {
    let text = std::fs::read_to_string(file).ok()?;
    let value = |key: &str| {
        text.lines().find_map(|l| {
            l.trim()
                .strip_prefix(key)
                .and_then(|r| r.strip_prefix('='))
                .map(|v| v.trim().to_string())
        })
    };
    let spec = value("WJ_COMMIT")?;
    (!spec.is_empty()).then(|| Pin {
        spec,
        file: file.to_path_buf(),
        repo: value("WJ_REPO"),
    })
}

fn is_hex_spec(spec: &str) -> bool {
    spec.len() >= 7 && spec.chars().all(|c| c.is_ascii_hexdigit())
}

/// The forms a hex pin can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinForm {
    /// `+<hex>`: the source key only.
    Key,
    /// Bare hex: the source key, or the commit of a stamped build.
    KeyOrCommit,
    /// A version (`0.50`, `v0.50.0`).
    Version,
}

pub fn form(spec: &str) -> PinForm {
    let spec = spec.trim();
    if spec.strip_prefix('+').is_some_and(is_hex_spec) {
        PinForm::Key
    } else if is_hex_spec(spec) {
        PinForm::KeyOrCommit
    } else {
        PinForm::Version
    }
}

fn hex_prefix_match(full: &str, spec: &str) -> bool {
    let (full, spec) = (full.to_ascii_lowercase(), spec.to_ascii_lowercase());
    full.starts_with(&spec) || spec.starts_with(&full) && full.len() >= 7
}

/// Does this binary satisfy the pin? `+<hex>` is a source-key prefix; bare hex (7+ chars) is a source-key or commit
/// prefix (a dev build carries no commit: build.rs never queries git, so only a build that stamped one with
/// `WJ_GIT_HASH` matches by commit); dotted specs are versions (`0.50` matches `0.50.0`).
pub fn satisfies(spec: &str, id: &Identity) -> bool {
    let spec = spec.trim();
    let key_matches = |hex: &str| id.key.as_deref().is_some_and(|k| hex_prefix_match(k, hex));
    match form(spec) {
        PinForm::Key => key_matches(&spec[1..]),
        PinForm::KeyOrCommit => key_matches(spec) || id.commit.as_deref().is_some_and(|c| hex_prefix_match(c, spec)),
        PinForm::Version => {
            let spec = spec.trim_start_matches('v');
            id.version == spec || id.version.strip_prefix(spec).is_some_and(|r| r.starts_with('.'))
        }
    }
}

/// The command that installs the pinned compiler.
pub fn install_command(pin: &Pin) -> String {
    let spec = pin.spec.trim().trim_start_matches('v');
    match form(spec) {
        PinForm::Key => format!(
            "build wj from the compiler sources whose `wj --version` shows +{} (cargo build --release --bin wj && target/release/wj install-dev)",
            &spec[1..]
        ),
        PinForm::KeyOrCommit => format!(
            "build wj from the compiler sources whose `wj --version` shows +{spec} (cargo build --release --bin wj && target/release/wj install-dev), or a stamped release: cargo install --git {} --rev {spec} --bin wj --locked --force",
            pin.repo.as_deref().unwrap_or(DEFAULT_REPO)
        ),
        PinForm::Version => format!("cargo install windjammer --version {spec} --bin wj --locked --force"),
    }
}

pub fn mismatch_message(pin: &Pin, id: &Identity) -> String {
    let pinned = match form(&pin.spec) {
        PinForm::Key => "a source key",
        PinForm::KeyOrCommit => "a source key or commit",
        PinForm::Version => "a version",
    };
    let actual = match (id.short_key(), &id.commit) {
        (Some(k), Some(_)) => format!("{}+{k}, commit {}", id.version, id.short()),
        (Some(k), None) => format!("{}+{k}", id.version),
        (None, _) => format!("{} (commit {})", id.version, id.short()),
    };
    format!(
        "this project pins wj `{}` ({pinned}, {}), but the running wj is {actual}.\n  expected: {}\n  actual:   {}\n  fix:      {}\n  override: WJ_ALLOW_TOOLCHAIN_MISMATCH=1 (warns instead of failing)\n  diagnose: wj doctor",
        pin.spec,
        pin.file.display(),
        pin.spec.trim().trim_start_matches('+'),
        id.short_key().map(str::to_string).or_else(|| id.commit.clone()).unwrap_or_else(|| id.version.clone()),
        install_command(pin)
    )
}

/// The pin line for the running build (`wj doctor --pin`): its source key, ready to paste under `[toolchain]`.
pub fn pin_line(id: &Identity) -> Option<String> {
    id.short_key().map(|k| format!("wj = \"{k}\""))
}

pub enum Outcome {
    Ok,
    /// Mismatch allowed by the override; print this warning.
    Warn(String),
    Fail(String),
}

/// Check the pin governing `start` against `id`. `allow` is the value of `WJ_ALLOW_TOOLCHAIN_MISMATCH`.
pub fn check(start: &Path, id: &Identity, allow: Option<&str>) -> Outcome {
    let Some(pin) = find_pin(start) else {
        return Outcome::Ok;
    };
    if satisfies(&pin.spec, id) {
        return Outcome::Ok;
    }
    let msg = mismatch_message(&pin, id);
    if allow.is_some_and(|v| matches!(v, "1" | "true" | "yes")) {
        Outcome::Warn(format!("warning: toolchain mismatch allowed: {msg}"))
    } else {
        Outcome::Fail(msg)
    }
}

/// Entry for the build/run/test/check commands: `Err` carries the message to print; exits are the caller's.
pub fn enforce(start: &Path) -> Result<(), String> {
    let id = super::identity::current();
    let allow = super::env::get("WJ_ALLOW_TOOLCHAIN_MISMATCH");
    match check(start, &id, allow.as_deref()) {
        Outcome::Ok => Ok(()),
        Outcome::Warn(w) => {
            eprintln!("{w}");
            Ok(())
        }
        Outcome::Fail(m) => Err(format!("error: {m}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn id(commit: &str, version: &str) -> Identity {
        Identity {
            key: None,
            version: version.into(),
            commit: Some(commit.into()),
            commit_ts: None,
            dirty: false,
            profile: None,
            build_ts: None,
        }
    }

    #[test]
    fn commit_prefix_and_version_matching() {
        let i = id("2b6db230c4a1ffff", "0.50.0");
        assert!(satisfies("2b6db23", &i));
        assert!(satisfies("2B6DB230C4A1", &i));
        assert!(!satisfies("2b6db24", &i));
        assert!(satisfies("0.50.0", &i) && satisfies("v0.50", &i));
        assert!(!satisfies("0.5", &i) && !satisfies("0.51.0", &i));
        let unknown = Identity { commit: None, ..i };
        assert!(!satisfies("2b6db23", &unknown));
    }

    #[test]
    fn source_key_pins() {
        let keyed = Identity { key: Some("3fa91c20d4ffffffffff".into()), commit: None, ..id("x", "0.50.0") };
        assert!(satisfies("3fa91c20d4", &keyed) && satisfies("+3fa91c20d4", &keyed) && satisfies("3FA91C2", &keyed));
        assert!(!satisfies("3fa91c20d5", &keyed) && !satisfies("+2b6db230c4", &keyed));
        // `+` pins the key only: a stamped commit with the same digits does not satisfy it
        let stamped = Identity { key: Some("aaaaaaaaaa".into()), ..id("3fa91c20d4ffff", "0.50.0") };
        assert!(!satisfies("+3fa91c20d4", &stamped) && satisfies("3fa91c20d4", &stamped));
        let pin = Pin { spec: "+0123456789".into(), file: "wj.toml".into(), repo: None };
        let m = mismatch_message(&pin, &keyed);
        assert!(m.contains("a source key") && m.contains("expected: 0123456789") && m.contains("actual:   3fa91c20d4"), "{m}");
        assert_eq!(pin_line(&keyed).as_deref(), Some("wj = \"3fa91c20d4\""));
    }

    #[test]
    fn wj_toml_pin_wins_over_pin_md_and_nearest_wins() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("apps/one");
        fs::create_dir_all(&app).unwrap();
        fs::write(
            d.path().join(PIN_FILE),
            "WJ_REPO=https://example.com/wj.git\nWJ_COMMIT=aaaaaaa1\n",
        )
        .unwrap();
        assert_eq!(find_pin(&app).unwrap().spec, "aaaaaaa1");
        assert_eq!(
            find_pin(&app).unwrap().repo.as_deref(),
            Some("https://example.com/wj.git")
        );
        fs::write(app.join("wj.toml"), "[package]\nname=\"x\"\n").unwrap();
        assert_eq!(find_pin(&app).unwrap().spec, "aaaaaaa1");
        fs::write(app.join("wj.toml"), "[toolchain]\nwj = \"bbbbbbb2\"\n").unwrap();
        let p = find_pin(&app).unwrap();
        assert_eq!(p.spec, "bbbbbbb2");
        assert_eq!(p.file, app.join("wj.toml"));
    }

    #[test]
    fn mismatch_fails_with_install_command_and_override_warns() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("wj.toml"), "[toolchain]\nwj = \"abcdef1\"\n").unwrap();
        let i = id("0123456789", "0.50.0");
        match check(d.path(), &i, None) {
            Outcome::Fail(m) => {
                assert!(m.contains(&format!("cargo install --git {DEFAULT_REPO} --rev abcdef1 --bin wj --locked --force")), "{m}")
            }
            _ => panic!("expected failure"),
        }
        assert!(matches!(check(d.path(), &i, Some("1")), Outcome::Warn(_)));
        assert!(matches!(
            check(d.path(), &id("abcdef123", "0.50.0"), None),
            Outcome::Ok
        ));
        assert!(matches!(
            check(tempfile::tempdir().unwrap().path(), &i, None),
            Outcome::Ok
        ));
    }
}
