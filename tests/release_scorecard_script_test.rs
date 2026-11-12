//! `scripts/wj/release_scorecard.wj`: the fixtures prove every criterion can turn RED (and GREEN, STALE, MISSING), and the
//! committed page and JSON match the script's definition.
//!
//! Interpreted from the repository checkout; it uses packages from windjammer-ecosystem (`WJ_ECOSYSTEM`, or checked out at the
//! repository root). Without it the tests skip and say so.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ecosystem() -> Option<PathBuf> {
    let dir = match std::env::var("WJ_ECOSYSTEM") {
        Ok(d) if !d.is_empty() => PathBuf::from(d),
        _ => root().join("windjammer-ecosystem"),
    };
    if dir.join("packages").is_dir() {
        Some(dir)
    } else {
        eprintln!("SKIPPED: windjammer-ecosystem is not available (set WJ_ECOSYSTEM)");
        None
    }
}

fn run(eco: &Path, flag: &str) -> (bool, String) {
    let out = Command::new(WJ)
        .args([
            "run",
            "--interpret",
            "--allow",
            &format!("fs.read={}/**", eco.display()),
        ])
        .arg("scripts/wj/release_scorecard.wj")
        .arg("--")
        .arg(flag)
        .env("WJ_ECOSYSTEM", eco)
        .current_dir(root())
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

#[test]
fn every_criterion_turns_red_green_stale_and_missing() {
    let Some(eco) = ecosystem() else { return };
    let (ok, text) = run(&eco, "--selftest");
    assert!(ok, "{text}");
    assert!(text.contains("0 failures"), "{text}");
}

#[test]
fn committed_page_and_json_match_the_definition() {
    let Some(eco) = ecosystem() else { return };
    let (ok, text) = run(&eco, "--check");
    assert!(
        ok,
        "regenerate with `wj run scripts/wj/release_scorecard.wj -- --write`:\n{text}"
    );
}
