//! Gates: `wj doctor` finds every `wj` on PATH, resolves symlinks, flags duplicates with different commits,
//! stale binaries and a missing shared cache, prints the fix commands and exits non-zero; `--json` mirrors it.

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DAY: i64 = 86_400;

fn current() -> serde_json::Value {
    let o = Command::new(env!("CARGO_BIN_EXE_wj")).args(["version", "--json"]).output().unwrap();
    serde_json::from_slice(&o.stdout).unwrap()
}

/// A stand-in `wj` that prints `line` for `--version`.
fn fake(dir: &Path, line: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let p = dir.join("wj");
    fs::write(&p, format!("#!/bin/sh\necho '{line}'\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    p
}

struct Env {
    cache: tempfile::TempDir,
    work: tempfile::TempDir,
}

fn env(with_cache: bool) -> Env {
    let e = Env { cache: tempfile::tempdir().unwrap(), work: tempfile::tempdir().unwrap() };
    if with_cache {
        fs::create_dir_all(e.cache.path().join("cargo-target").join("shared")).unwrap();
    }
    e
}

fn doctor(e: &Env, path: &[&Path], now: Option<i64>, args: &[&str]) -> Output {
    let joined = std::env::join_paths(path.iter().map(|p| p.to_path_buf())).unwrap();
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    c.arg("doctor").args(args).current_dir(e.work.path()).env("PATH", joined).env("WJ_CARGO_TARGET_ROOT", e.cache.path());
    if let Some(n) = now {
        c.env("WJ_TOOLCHAIN_NOW", n.to_string());
    }
    c.output().unwrap()
}

fn report(o: &Output) -> serde_json::Value {
    serde_json::from_slice(&o.stdout).unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&o.stdout)))
}

fn kinds(r: &serde_json::Value) -> Vec<String> {
    r["problems"].as_array().unwrap().iter().map(|p| p["kind"].as_str().unwrap().to_string()).collect()
}

fn ts() -> i64 {
    current()["commit_timestamp"].as_i64().unwrap_or(1_790_000_000)
}

fn date(t: i64) -> String {
    windjammer::toolchain::timefmt::iso_date(t)
}

fn line(commit: &str, t: i64) -> String {
    format!("wj 0.50.0 (commit {commit} {}, clean, release, built {}T00:00:00Z)", date(t), date(t))
}

#[test]
fn healthy_path_exits_zero() {
    let e = env(true);
    let bin = tempfile::tempdir().unwrap();
    let c = current();
    let l = c["commit"].as_str().map(|h| line(&h[..12], ts())).unwrap_or_else(|| line("unknown", ts()));
    fake(bin.path(), &l);
    let o = doctor(&e, &[bin.path()], Some(ts() + DAY), &[]);
    let text = String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(0), "{text}");
    assert!(text.contains("no problems found"), "{text}");
}

#[test]
fn duplicates_with_different_commits_are_flagged_with_fix_commands() {
    let e = env(true);
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    fake(a.path(), &line("aaaaaaaaaaaa", ts()));
    fake(b.path(), &line("bbbbbbbbbbbb", ts()));
    let o = doctor(&e, &[a.path(), b.path()], Some(ts() + DAY), &["--json"]);
    assert_eq!(o.status.code(), Some(1));
    let r = report(&o);
    assert_eq!(r["binaries"].as_array().unwrap().len(), 2);
    assert!(kinds(&r).contains(&"duplicate".to_string()), "{r}");
    let dup = r["problems"].as_array().unwrap().iter().find(|p| p["kind"] == "duplicate").unwrap();
    let fix = dup["fix"].to_string();
    assert!(fix.contains("install-dev") && fix.contains(&format!("rm {}", b.path().join("wj").display())), "{fix}");
}

#[test]
fn identical_duplicates_and_symlinks_are_not_problems() {
    let e = env(true);
    let (a, b, c) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let real = fake(a.path(), &line("aaaaaaaaaaaa", ts()));
    symlink(&real, b.path().join("wj")).unwrap();
    fake(c.path(), &line("aaaaaaaaaaaa", ts()));
    let o = doctor(&e, &[b.path(), a.path(), c.path()], Some(ts() + DAY), &["--json"]);
    let r = report(&o);
    let bins = r["binaries"].as_array().unwrap();
    assert_eq!(bins.len(), 2, "the symlink and its target are one binary: {r}");
    assert_eq!(bins[0]["path"], b.path().join("wj").display().to_string());
    assert_eq!(bins[0]["resolved"], std::fs::canonicalize(&real).unwrap().display().to_string());
    assert!(!kinds(&r).contains(&"duplicate".to_string()), "{r}");
}

#[test]
fn old_and_unstamped_binaries_are_stale() {
    let e = env(true);
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    fake(a.path(), &line("aaaaaaaaaaaa", ts() - 100 * DAY));
    fake(b.path(), "wj 0.50.0");
    let o = doctor(&e, &[a.path(), b.path()], Some(ts()), &["--json"]);
    assert_eq!(o.status.code(), Some(1));
    let r = report(&o);
    let stale: Vec<_> = r["problems"].as_array().unwrap().iter().filter(|p| p["kind"] == "stale").collect();
    assert_eq!(stale.len(), 2, "{r}");
    assert!(stale.iter().any(|p| p["message"].as_str().unwrap().contains("predates build identity stamping")));
    assert!(stale.iter().all(|p| !p["fix"].as_array().unwrap().is_empty()));
}

#[test]
fn missing_cache_and_missing_wj_are_reported() {
    let e = env(false);
    let empty = tempfile::tempdir().unwrap();
    let o = doctor(&e, &[empty.path()], None, &["--json"]);
    assert_eq!(o.status.code(), Some(1));
    let r = report(&o);
    assert!(kinds(&r).contains(&"missing".to_string()) && kinds(&r).contains(&"cache".to_string()), "{r}");
    let cache = r["problems"].as_array().unwrap().iter().find(|p| p["kind"] == "cache").unwrap();
    assert!(cache["fix"].to_string().contains("wj cache setup"));
    assert_eq!(r["cache"]["exists"], false);
}

#[test]
fn text_report_lists_version_commit_and_mtime_per_binary() {
    let e = env(true);
    let a = tempfile::tempdir().unwrap();
    fake(a.path(), &line("abcdef123456", ts()));
    let o = doctor(&e, &[a.path()], Some(ts() + DAY), &[]);
    let t = String::from_utf8_lossy(&o.stdout);
    assert!(t.contains("version 0.50.0") && t.contains("commit abcdef123456") && t.contains("mtime 20"), "{t}");
}

fn checkout_with_head() -> (tempfile::TempDir, String) {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("Cargo.toml"), "[package]\nname = \"windjammer\"\n").unwrap();
    let git = |args: &[&str]| {
        let o = Command::new("git").current_dir(d.path()).args(["-c", "user.name=t", "-c", "user.email=t@example.com"]).args(args).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_string()
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "x"]);
    let head = git(&["rev-parse", "HEAD"]);
    (d, head)
}

#[test]
fn expect_head_accepts_only_a_wj_built_from_the_checkout_head() {
    let (co, head) = checkout_with_head();
    let (good, bad) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    fake(good.path(), &line(&head[..12], ts()));
    fake(bad.path(), &line("aaaaaaaaaaaa", ts()));
    let e = Env { cache: tempfile::tempdir().unwrap(), work: co };
    let o = doctor(&e, &[good.path(), bad.path()], None, &["--expect-head"]);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    let o = doctor(&e, &[bad.path(), good.path()], None, &["--expect-head"]);
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("HEAD") && err.contains("install-dev"), "{err}");
}
