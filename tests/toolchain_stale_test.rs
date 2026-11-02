//! Gates: age-based staleness warning (30 days behind an enclosing compiler checkout, 60 days otherwise),
//! once per day, silenceable, and free on the hot path. The clock is injected with `WJ_TOOLCHAIN_NOW`.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use windjammer::toolchain::identity;
use windjammer::toolchain::stale::{warn_with, DAY};

fn commit_ts() -> Option<i64> {
    identity::current().commit_ts
}

fn run(cwd: &Path, cache: &Path, now: i64, extra: &[(&str, &str)]) -> Output {
    let f = cwd.join("t.wj");
    if !f.exists() {
        fs::write(&f, "fn main() {\n    println(\"hi\")\n}\n").unwrap();
    }
    let mut c = Command::new(env!("CARGO_BIN_EXE_wj"));
    c.args(["check", "t.wj"])
        .current_dir(cwd)
        .env("WJ_CARGO_TARGET_ROOT", cache)
        .env("WJ_TOOLCHAIN_NOW", now.to_string());
    c.env_remove("WJ_NO_STALE_WARN");
    for (k, v) in extra {
        c.env(k, v);
    }
    c.output().unwrap()
}

fn warned(o: &Output) -> bool {
    String::from_utf8_lossy(&o.stderr).contains("wj: warning:")
}

#[test]
fn standalone_binary_warns_after_sixty_days_once_per_day() {
    let Some(ts) = commit_ts() else { return };
    let (work, cache) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert!(
        !warned(&run(work.path(), cache.path(), ts + 59 * DAY, &[])),
        "59 days is fine"
    );
    let first = run(work.path(), cache.path(), ts + 61 * DAY, &[]);
    assert!(warned(&first), "{}", String::from_utf8_lossy(&first.stderr));
    let err = String::from_utf8_lossy(&first.stderr);
    assert_eq!(
        err.lines().filter(|l| l.contains("wj: warning:")).count(),
        1,
        "{err}"
    );
    assert!(
        err.contains("wj doctor") && err.contains("WJ_NO_STALE_WARN=1"),
        "{err}"
    );
    assert!(
        !warned(&run(work.path(), cache.path(), ts + 61 * DAY + 3600, &[])),
        "second run the same day stays quiet"
    );
    assert!(
        warned(&run(work.path(), cache.path(), ts + 62 * DAY + 3600, &[])),
        "next day warns again"
    );
}

#[test]
fn silence_switch_suppresses_the_warning_and_the_stamp() {
    let Some(ts) = commit_ts() else { return };
    let (work, cache) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    assert!(!warned(&run(
        work.path(),
        cache.path(),
        ts + 90 * DAY,
        &[("WJ_NO_STALE_WARN", "1")]
    )));
    assert!(
        warned(&run(work.path(), cache.path(), ts + 90 * DAY, &[])),
        "silencing must not consume the day's warning"
    );
}

fn fake_checkout(newest: i64) -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("Cargo.toml"),
        "[package]\nname = \"windjammer\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();
    let date = format!("{newest} +0000");
    let git = |args: &[&str]| {
        let o = Command::new("git")
            .current_dir(d.path())
            .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
            .args(args)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    };
    git(&["init", "-q"]);
    git(&["add", "Cargo.toml"]);
    git(&["commit", "-q", "-m", "x"]);
    d
}

#[test]
fn inside_a_checkout_the_binary_is_measured_against_the_newest_commit_there() {
    let Some(ts) = commit_ts() else { return };
    let cache = tempfile::tempdir().unwrap();
    // checkout 40 days ahead of the binary: stale even though "now" is only 41 days after the binary
    let ahead = fake_checkout(ts + 40 * DAY);
    let o = run(ahead.path(), cache.path(), ts + 41 * DAY, &[]);
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(warned(&o) && err.contains("newest commit in"), "{err}");
    // checkout only 20 days ahead: fresh, although "now" is 70 days after the binary (the 60-day rule is for no checkout)
    let near = fake_checkout(ts + 20 * DAY);
    let cache2 = tempfile::tempdir().unwrap();
    let o = run(near.path(), cache2.path(), ts + 70 * DAY, &[]);
    assert!(!warned(&o), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn a_binary_younger_than_thirty_days_pays_almost_nothing() {
    let Some(ts) = commit_ts() else { return };
    let missing = std::env::temp_dir().join(format!("wj-stale-hotpath-{}", std::process::id()));
    std::env::set_var("WJ_CARGO_TARGET_ROOT", &missing);
    std::env::remove_var("WJ_NO_STALE_WARN");
    let id = identity::current();
    let now = ts + 10 * DAY;
    let n = 20_000;
    let start = std::time::Instant::now();
    for _ in 0..n {
        warn_with(&id, now);
    }
    let per_call = start.elapsed() / n;
    assert!(
        per_call < std::time::Duration::from_micros(20),
        "hot path costs {per_call:?} per call"
    );
    assert!(
        !missing.join("toolchain").exists(),
        "a fresh binary must not touch the stamp directory"
    );
}
