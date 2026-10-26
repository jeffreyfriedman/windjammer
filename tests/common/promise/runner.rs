//! Executes acceptance cases against the installed-binary sandbox (binary alone in a temp prefix,
//! cleared environment). Returns a list of failures; empty = every promise held.

use super::model::Case;
use super::super::installed_sandbox::Sandbox;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

pub struct Run {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

const TIMEOUT: Duration = Duration::from_secs(120);

pub fn run_wj(sb: &Sandbox, cwd: &Path, args: &[String], stdin: Option<&str>, extra_env: &[(&str, String)]) -> Run {
    let mut cmd = sb.command(cwd);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn wj {args:?}: {e}"));
    {
        let mut si = child.stdin.take().unwrap();
        if let Some(s) = stdin {
            let _ = si.write_all(s.as_bytes());
        }
    }
    let start = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if start.elapsed() > TIMEOUT {
            let _ = child.kill();
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let out = child.wait_with_output().unwrap();
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn subst(s: &str, w: &Path) -> String {
    s.replace("{w}", w.to_str().unwrap())
}

/// Extra environment for cases that need the host toolchain (cargo/rustc) on PATH.
fn toolchain_env(case: &Case) -> Vec<(&'static str, String)> {
    if case.needs.is_none() {
        return Vec::new();
    }
    let mut env = vec![("PATH", std::env::var("PATH").unwrap_or_default())];
    for k in ["CARGO_HOME", "RUSTUP_HOME", "CARGO_TARGET_DIR", "CARGO_BUILD_JOBS"] {
        if let Ok(v) = std::env::var(k) {
            env.push((k, v));
        }
    }
    env
}

/// Runs one case; `Err` explains which expectation failed.
pub fn run_case(sb: &Sandbox, case: &Case) -> Result<(), String> {
    let dir = tempfile::TempDir::new().unwrap();
    let w = dir.path().canonicalize().unwrap();
    for (rel, text) in &case.files {
        super::super::installed_sandbox::write(&w.join(rel), &subst(text, &w));
    }
    let mut env = toolchain_env(case);
    // A private temp dir: commands like `wj clean` must never see other processes' temp files.
    let tmpdir = w.join(".tmp");
    std::fs::create_dir_all(&tmpdir).unwrap();
    env.push(("TMPDIR", tmpdir.to_string_lossy().into_owned()));
    for step in &case.setup {
        let args: Vec<String> = step.iter().map(|a| subst(a, &w)).collect();
        let r = run_wj(sb, &w, &args, None, &env);
        if r.code != Some(0) {
            return Err(format!("setup `wj {}` exited {:?}\n{}{}", args.join(" "), r.code, r.stdout, r.stderr));
        }
    }
    let cwd = case.cwd.as_ref().map(|c| w.join(c)).unwrap_or_else(|| w.clone());
    let args: Vec<String> = case.args.iter().map(|a| subst(a, &w)).collect();
    let r = run_wj(sb, &cwd, &args, case.stdin.as_deref(), &env);
    let ctx = || format!("`wj {}` exit {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}", case.args.join(" "), r.code, r.stdout, r.stderr);
    let both = format!("{}\n{}", r.stdout, r.stderr);
    let mut why = Vec::new();
    match (case.exit, case.is_negative()) {
        (Some(e), _) if r.code != Some(e) => why.push(format!("exit {:?}, want {e}", r.code)),
        (None, false) if r.code != Some(0) => why.push("positive case must exit 0".into()),
        (None, true) if matches!(r.code, Some(0) | None) => why.push("negative case must exit non-zero".into()),
        _ => {}
    }
    for (hay, needles, label) in [(&r.stdout, &case.stdout_has, "stdout"), (&r.stderr, &case.stderr_has, "stderr"), (&both, &case.output_has, "output")] {
        for n in needles {
            if !hay.contains(&subst(n, &w)) {
                why.push(format!("{label} lacks {n:?}"));
            }
        }
    }
    for n in &case.output_lacks {
        if both.contains(&subst(n, &w)) {
            why.push(format!("output must not contain {n:?}"));
        }
    }
    for p in &case.creates {
        if !w.join(p).exists() {
            why.push(format!("expected file {p} was not created"));
        }
    }
    for p in &case.absent {
        if w.join(p).exists() {
            why.push(format!("file {p} must not exist"));
        }
    }
    for (p, n) in &case.file_has {
        let t = std::fs::read_to_string(w.join(p)).unwrap_or_default();
        if !t.contains(n.as_str()) {
            why.push(format!("{p} lacks {n:?}"));
        }
    }
    if case.json && serde_json::from_str::<serde_json::Value>(r.stdout.trim()).is_err() {
        why.push("stdout is not valid JSON".into());
    }
    if why.is_empty() {
        Ok(())
    } else {
        Err(format!("{}\n{}", why.join("; "), ctx()))
    }
}
