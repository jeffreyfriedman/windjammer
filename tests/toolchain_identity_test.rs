//! Gates: `wj --version` and `wj version --json` report the identity the binary was built with, checked
//! against git itself (an independent oracle), and the wall-clock build date never reaches generated output.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wj(args: &[&str]) -> (String, i32) {
    let o = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(args)
        .env("WJ_NO_STALE_WARN", "1")
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&o.stdout).to_string(),
        o.status.code().unwrap_or(-1),
    )
}

fn git(args: &[&str]) -> Option<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(root())
        .args(args)
        .output()
        .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn json() -> serde_json::Value {
    let (out, code) = wj(&["version", "--json"]);
    assert_eq!(code, 0);
    serde_json::from_str(&out).unwrap()
}

#[test]
fn version_flag_reports_every_identity_field() {
    let (out, code) = wj(&["--version"]);
    assert_eq!(code, 0);
    let line = out.trim_end();
    assert_eq!(out.lines().count(), 1, "{out}");
    let j = json();
    assert!(
        line.starts_with(&format!("wj {} (commit ", env!("CARGO_PKG_VERSION"))),
        "{line}"
    );
    assert!(
        line.contains(&format!("commit {}", j["commit_short"].as_str().unwrap())),
        "{line}"
    );
    assert!(line.contains(j["commit_date"].as_str().unwrap()), "{line}");
    assert!(
        line.contains(if j["dirty"].as_bool().unwrap() {
            ", dirty,"
        } else {
            ", clean,"
        }),
        "{line}"
    );
    assert!(
        line.contains(&format!(", {}, built ", j["profile"].as_str().unwrap())),
        "{line}"
    );
    assert!(line.contains(j["build_date"].as_str().unwrap()), "{line}");
    let (short, _) = wj(&["-V"]);
    assert_eq!(short, out);
}

#[test]
fn identity_matches_git_when_built_from_a_checkout() {
    let Some(head) = git(&["rev-parse", "HEAD"]) else {
        return;
    };
    let j = json();
    assert_eq!(j["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        j["commit"].as_str().unwrap(),
        head,
        "the binary must report the commit it was built from"
    );
    let ts: i64 = git(&["log", "-1", "--format=%ct"])
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(j["commit_timestamp"].as_i64().unwrap(), ts);
    let dirty = !git(&["status", "--porcelain", "--untracked-files=no"])
        .unwrap()
        .is_empty();
    assert_eq!(
        j["dirty"].as_bool().unwrap(),
        dirty,
        "dirty flag must reflect tracked changes"
    );
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    assert_eq!(j["profile"], profile);
    assert!(
        j["build_timestamp"].as_i64().unwrap() >= ts,
        "built before it was committed?"
    );
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Wall-clock build time is for `--version` only: anything in the compiler that read it could make generated
/// output differ between two builds of the same commit (the cold/warm determinism gate would catch that late).
#[test]
fn wall_clock_build_date_stays_inside_the_toolchain_module() {
    let mut files = Vec::new();
    rust_files(&root().join("src"), &mut files);
    let offenders: Vec<_> = files
        .iter()
        .filter(|p| !p.starts_with(root().join("src/toolchain")))
        .filter(|p| {
            let t = std::fs::read_to_string(p).unwrap();
            t.contains("WJ_BUILD_TS") || t.contains("build_ts")
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "build time must not leave the toolchain module: {offenders:?}"
    );
    let ident = std::fs::read_to_string(root().join("src/toolchain/identity.rs")).unwrap();
    assert!(ident.contains("WJ_BUILD_TS"));
}
