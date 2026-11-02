//! Gates: `wj install-dev` installs atomically, keeps a timestamped backup (last five), refuses to downgrade
//! without --force, and refuses a binary that cannot report `--version`.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const DAY: i64 = 86_400;
const T0: i64 = 1_790_000_000;

fn date(t: i64) -> String {
    windjammer::toolchain::timefmt::iso_date(t)
}

fn fake(dir: &Path, name: &str, commit: &str, t: i64, built: i64) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let p = dir.join(name);
    let line = format!(
        "wj 0.50.0 (commit {commit} {}, clean, release, built {}T00:00:00Z)",
        date(t),
        date(built)
    );
    fs::write(&p, format!("#!/bin/sh\necho '{line}'\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    p
}

fn install(from: &Path, to: &Path, now: i64, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["install-dev", "--from"])
        .arg(from)
        .arg("--to")
        .arg(to)
        .args(extra)
        .env("WJ_TOOLCHAIN_NOW", now.to_string())
        .output()
        .unwrap()
}

fn entries(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    v.sort();
    v
}

fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).to_string()
}

#[test]
fn fresh_install_then_upgrade_keeps_a_timestamped_backup() {
    let (src, dst) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let dest = dst.path().join("bin").join("wj");
    let v1 = fake(src.path(), "v1", "111111111111", T0, T0);
    let v2 = fake(src.path(), "v2", "222222222222", T0 + 5 * DAY, T0 + 5 * DAY);
    let o = install(&v1, &dest, T0 + DAY, &[]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(
        entries(dest.parent().unwrap()),
        ["wj"],
        "no temp files or backups on a first install"
    );
    let o = install(&v2, &dest, T0 + 6 * DAY, &[]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let names = entries(dest.parent().unwrap());
    assert_eq!(names.len(), 2, "{names:?}");
    let bak = names
        .iter()
        .find(|n| n.starts_with("wj.") && n.ends_with(".bak"))
        .unwrap();
    assert!(
        bak.contains(&windjammer::toolchain::timefmt::compact_datetime(
            T0 + 6 * DAY
        )),
        "{bak}"
    );
    assert_eq!(
        fs::read(dest.parent().unwrap().join(bak)).unwrap(),
        fs::read(&v1).unwrap()
    );
    assert_eq!(fs::read(&dest).unwrap(), fs::read(&v2).unwrap());
    assert!(String::from_utf8_lossy(&o.stdout).contains("previous binary kept at"));
}

#[test]
fn downgrade_is_refused_without_force_and_leaves_the_install_untouched() {
    let (src, dst) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let dest = dst.path().join("wj");
    let new = fake(
        src.path(),
        "new",
        "222222222222",
        T0 + 5 * DAY,
        T0 + 5 * DAY,
    );
    let old = fake(src.path(), "old", "111111111111", T0, T0);
    assert_eq!(
        install(&new, &dest, T0 + 6 * DAY, &[]).status.code(),
        Some(0)
    );
    let before = fs::read(&dest).unwrap();
    let o = install(&old, &dest, T0 + 7 * DAY, &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        err(&o).contains("refusing to downgrade") && err(&o).contains("--force"),
        "{}",
        err(&o)
    );
    assert_eq!(fs::read(&dest).unwrap(), before);
    assert_eq!(entries(dst.path()), ["wj"]);
    let o = install(&old, &dest, T0 + 7 * DAY, &["--force"]);
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    assert_eq!(fs::read(&dest).unwrap(), fs::read(&old).unwrap());
}

#[test]
fn a_binary_that_cannot_report_its_version_is_never_installed() {
    let (src, dst) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let dest = dst.path().join("wj");
    let good = fake(src.path(), "good", "111111111111", T0, T0);
    assert_eq!(install(&good, &dest, T0, &[]).status.code(), Some(0));
    let before = fs::read(&dest).unwrap();
    let broken = src.path().join("broken");
    fs::write(&broken, "#!/bin/sh\nexit 3\n").unwrap();
    fs::set_permissions(&broken, fs::Permissions::from_mode(0o755)).unwrap();
    let o = install(&broken, &dest, T0 + DAY, &["--force"]);
    assert_eq!(o.status.code(), Some(1), "{}", err(&o));
    assert_eq!(fs::read(&dest).unwrap(), before);
    assert_eq!(
        entries(dst.path()),
        ["wj"],
        "no staging file may be left behind"
    );
}

#[test]
fn reinstalling_the_same_build_is_a_no_op_and_dry_run_writes_nothing() {
    let (src, dst) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let dest = dst.path().join("wj");
    let v1 = fake(src.path(), "v1", "111111111111", T0, T0);
    assert_eq!(
        install(&v1, &dest, T0, &["--dry-run"]).status.code(),
        Some(0)
    );
    assert!(!dest.exists());
    assert_eq!(install(&v1, &dest, T0, &[]).status.code(), Some(0));
    let o = install(&v1, &dest, T0 + DAY, &[]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("already holds this build"));
    assert_eq!(entries(dst.path()), ["wj"]);
}

#[test]
fn only_the_five_newest_backups_are_kept() {
    let (src, dst) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let dest = dst.path().join("wj");
    for i in 0..8 {
        let v = fake(
            src.path(),
            &format!("v{i}"),
            &format!("{i:012}"),
            T0 + i * DAY,
            T0 + i * DAY,
        );
        let o = install(&v, &dest, T0 + 30 * DAY + i * 3600, &[]);
        assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    }
    let baks: Vec<_> = entries(dst.path())
        .into_iter()
        .filter(|n| n.ends_with(".bak"))
        .collect();
    assert_eq!(baks.len(), 5, "{baks:?}");
}

#[test]
fn the_running_binary_installs_and_the_copy_reports_the_same_identity() {
    let dst = tempfile::tempdir().unwrap();
    let dest = dst.path().join("wj");
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["install-dev", "--to"])
        .arg(&dest)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(0), "{}", err(&o));
    let a = Command::new(&dest).arg("--version").output().unwrap();
    let b = Command::new(env!("CARGO_BIN_EXE_wj"))
        .arg("--version")
        .env("WJ_NO_STALE_WARN", "1")
        .output()
        .unwrap();
    assert_eq!(a.stdout, b.stdout);
}
