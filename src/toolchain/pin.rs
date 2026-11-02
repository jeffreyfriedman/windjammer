//! Project toolchain pin: `[toolchain] wj = "<commit-prefix or version>"` in `wj.toml`, or `WJ_COMMIT=<sha>` in a
//! `WJ_PIN.md` found in an ancestor directory. A mismatch with the running `wj` is an error for
//! build/run/test/check, overridable with `WJ_ALLOW_TOOLCHAIN_MISMATCH=1`.

use super::identity::Identity;
use std::path::{Path, PathBuf};

pub const DEFAULT_REPO: &str = "https://github.com/jeffreyfriedman/windjammer.git";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pin {
    pub spec: String,
    pub file: PathBuf,
    pub repo: Option<String>,
}

/// The pin that governs `start` (a directory or a file inside the project): the nearest `wj.toml` that
/// declares `[toolchain] wj`, else the nearest `WJ_PIN.md` with a `WJ_COMMIT`.
pub fn find_pin(start: &Path) -> Option<Pin> {
    let start = if start.is_file() { start.parent()? } else { start };
    let dirs: Vec<&Path> = start.ancestors().collect();
    for d in &dirs {
        if let Some(p) = read_wj_toml(&d.join("wj.toml")) {
            return Some(p);
        }
    }
    dirs.iter().find_map(|d| read_pin_md(&d.join("WJ_PIN.md")))
}

fn read_wj_toml(file: &Path) -> Option<Pin> {
    let text = std::fs::read_to_string(file).ok()?;
    let doc: toml::Value = text.parse().ok()?;
    let t = doc.get("toolchain")?;
    let spec = t.get("wj")?.as_str()?.trim().to_string();
    let repo = t.get("repo").and_then(|r| r.as_str()).map(str::to_string);
    (!spec.is_empty()).then(|| Pin { spec, file: file.to_path_buf(), repo })
}

fn read_pin_md(file: &Path) -> Option<Pin> {
    let text = std::fs::read_to_string(file).ok()?;
    let value = |key: &str| {
        text.lines().find_map(|l| l.trim().strip_prefix(key).and_then(|r| r.strip_prefix('=')).map(|v| v.trim().to_string()))
    };
    let spec = value("WJ_COMMIT")?;
    (!spec.is_empty()).then(|| Pin { spec, file: file.to_path_buf(), repo: value("WJ_REPO") })
}

fn is_commit_spec(spec: &str) -> bool {
    spec.len() >= 7 && spec.chars().all(|c| c.is_ascii_hexdigit())
}

/// Does this binary satisfy the pin? Hex specs (7+ chars) are commit prefixes; dotted specs are versions
/// (`0.50` matches `0.50.0`).
pub fn satisfies(spec: &str, id: &Identity) -> bool {
    let spec = spec.trim().trim_start_matches('v');
    if is_commit_spec(spec) {
        let spec = spec.to_ascii_lowercase();
        return id.commit.as_deref().is_some_and(|c| c.to_ascii_lowercase().starts_with(&spec) || spec.starts_with(&c.to_ascii_lowercase()) && c.len() >= 7);
    }
    id.version == spec || id.version.strip_prefix(spec).is_some_and(|r| r.starts_with('.'))
}

/// The command that installs the pinned compiler.
pub fn install_command(pin: &Pin) -> String {
    let spec = pin.spec.trim().trim_start_matches('v');
    if is_commit_spec(spec) {
        format!(
            "cargo install --git {} --rev {} --bin wj --locked --force",
            pin.repo.as_deref().unwrap_or(DEFAULT_REPO),
            spec
        )
    } else {
        format!("cargo install windjammer --version {spec} --bin wj --locked --force")
    }
}

pub fn mismatch_message(pin: &Pin, id: &Identity) -> String {
    format!(
        "this project pins wj `{}` ({}), but the running wj is {} (commit {}).\n  fix:      {}\n  override: WJ_ALLOW_TOOLCHAIN_MISMATCH=1 (warns instead of failing)\n  diagnose: wj doctor",
        pin.spec,
        pin.file.display(),
        id.version,
        id.short(),
        install_command(pin)
    )
}

pub enum Outcome {
    Ok,
    /// Mismatch allowed by the override; print this warning.
    Warn(String),
    Fail(String),
}

/// Check the pin governing `start` against `id`. `allow` is the value of `WJ_ALLOW_TOOLCHAIN_MISMATCH`.
pub fn check(start: &Path, id: &Identity, allow: Option<&str>) -> Outcome {
    let Some(pin) = find_pin(start) else { return Outcome::Ok };
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
    let allow = std::env::var("WJ_ALLOW_TOOLCHAIN_MISMATCH").ok();
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
        Identity { version: version.into(), commit: Some(commit.into()), commit_ts: None, dirty: false, profile: None, build_ts: None }
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
    fn wj_toml_pin_wins_over_pin_md_and_nearest_wins() {
        let d = tempfile::tempdir().unwrap();
        let app = d.path().join("apps/one");
        fs::create_dir_all(&app).unwrap();
        fs::write(d.path().join("WJ_PIN.md"), "WJ_REPO=https://example.com/wj.git\nWJ_COMMIT=aaaaaaa1\n").unwrap();
        assert_eq!(find_pin(&app).unwrap().spec, "aaaaaaa1");
        assert_eq!(find_pin(&app).unwrap().repo.as_deref(), Some("https://example.com/wj.git"));
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
        assert!(matches!(check(d.path(), &id("abcdef123", "0.50.0"), None), Outcome::Ok));
        assert!(matches!(check(tempfile::tempdir().unwrap().path(), &i, None), Outcome::Ok));
    }
}
