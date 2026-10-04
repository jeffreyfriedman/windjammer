#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! P3.619: cross-crate owned struct formals must not receive `&mut` at the call site.
//!
//! Product `wj-scheduler` → `wj_cron::matches_cron(expr: CronExpr, …)` /
//! `next_run(expr: CronExpr, …)`: tip emits `matches_cron(&mut cron, …)` /
//! `next_run(&mut cron, …)` → E0308 (expected `CronExpr`, found `&mut CronExpr`).
//! Source uses bare `matches_cron(cron, …)` after `Ok(cron)`.
//!
//! Distinct from P3.574 (package formal should demote to `&CronExpr` for field
//! reads). Either formal stays owned → call must **move**/clone, or formal
//! demotes → call must pass `&cron` — never `&mut cron` into an owned formal.
//! Do not reshape scheduler with `.clone()` / borrow hacks.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn scheduler_app() -> Option<PathBuf> {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..8 {
        let sibling = dir.join("windjammer-ecosystem/apps/wj-scheduler");
        if sibling.join("src").exists() {
            return Some(sibling);
        }
        if let Some(parent) = dir.parent() {
            let uncle = parent.join("windjammer-ecosystem/apps/wj-scheduler");
            if uncle.join("src").exists() {
                return Some(uncle);
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

#[test]
fn cross_crate_owned_struct_formal_must_not_receive_mut_ref() {
    let app = scheduler_app().unwrap_or_else(|| {
        panic!(
            "wj-scheduler src not found by walking from {}",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let tmp = TempDir::new().expect("tempdir");
    let out = tmp.path().join("out");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .current_dir(&app)
        .args([
            "build",
            "src",
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--no-cargo",
            "--module-file",
        ])
        .output()
        .expect("wj build wj-scheduler");
    assert!(
        build.status.success(),
        "P3.619 wj-scheduler transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let crontab = fs::read_to_string(out.join("domain").join("crontab.rs")).unwrap_or_default();
    let schedule = fs::read_to_string(out.join("domain").join("schedule.rs")).unwrap_or_default();
    eprintln!(
        "P3.619 crontab matches:\n{}",
        crontab
            .lines()
            .filter(|l| l.contains("matches_cron"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!(
        "P3.619 schedule calls:\n{}",
        schedule
            .lines()
            .filter(|l| l.contains("matches_cron") || l.contains("next_run"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let bad = crontab.contains("matches_cron(&mut cron")
        || schedule.contains("matches_cron(&mut cron")
        || schedule.contains("next_run(&mut cron");
    assert!(
        !bad,
        "P3.619 RED: owned CronExpr formal must not receive `&mut cron` \
         (move or `&cron` if demoted):\n--- crontab.rs ---\n{crontab}\n--- schedule.rs ---\n{schedule}"
    );
}
