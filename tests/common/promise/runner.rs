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

/// `{w}` scratch dir, `{bin_dir}` directory holding the installed `wj` under test, `{root}` the compiler checkout.
fn subst(s: &str, w: &Path) -> String {
    let bin_dir = std::env::var("WJ_PROMISE_BIN_DIR").unwrap_or_default();
    s.replace("{w}", w.to_str().unwrap()).replace("{bin_dir}", &bin_dir).replace("{root}", env!("CARGO_MANIFEST_DIR"))
}

/// The host toolchain, resolved ONCE from the test process's real environment (the sandbox isolates HOME, so
/// rustup would otherwise find no toolchain): PATH, the real RUSTUP_HOME / CARGO_HOME (env, else under the
/// real HOME) and RUSTUP_TOOLCHAIN (env, else the active toolchain). `Err` is the single message to show.
fn host_toolchain() -> &'static Result<Vec<(&'static str, String)>, String> {
    static T: std::sync::OnceLock<Result<Vec<(&'static str, String)>, String>> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut env = vec![("PATH", std::env::var("PATH").unwrap_or_default())];
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).ok().map(std::path::PathBuf::from);
        for (k, dot) in [("RUSTUP_HOME", ".rustup"), ("CARGO_HOME", ".cargo")] {
            let v = std::env::var(k).ok().or_else(|| {
                home.as_ref().map(|h| h.join(dot)).filter(|p| p.is_dir()).map(|p| p.to_string_lossy().into_owned())
            });
            if let Some(v) = v {
                env.push((k, v));
            }
        }
        for k in ["CARGO_TARGET_DIR", "CARGO_BUILD_JOBS"] {
            if let Ok(v) = std::env::var(k) {
                env.push((k, v));
            }
        }
        let tc = std::env::var("RUSTUP_TOOLCHAIN").ok().or_else(|| {
            let mut c = std::process::Command::new("rustup");
            c.args(["show", "active-toolchain"]).current_dir(env!("CARGO_MANIFEST_DIR"));
            let out = c.output().ok().filter(|o| o.status.success())?;
            String::from_utf8_lossy(&out.stdout).split_whitespace().next().map(str::to_string)
        });
        if let Some(tc) = tc {
            env.push(("RUSTUP_TOOLCHAIN", tc));
        }
        // Prove it works the way the cases will use it: isolated HOME, only this environment.
        let iso = std::env::temp_dir();
        let probe = std::process::Command::new("cargo").arg("--version").env_clear().envs(env.iter().map(|(k, v)| (*k, v.as_str()))).env("HOME", &iso).output();
        match probe {
            Ok(o) if o.status.success() => Ok(env),
            Ok(o) => Err(format!(
                "no usable Rust toolchain for the cargo-backed acceptance cases: `cargo --version` failed with an isolated HOME \
                 (RUSTUP_HOME={:?}, CARGO_HOME={:?}, RUSTUP_TOOLCHAIN={:?}). Set RUSTUP_HOME/CARGO_HOME or RUSTUP_TOOLCHAIN, or run the \
                 cases without WJ_PROMISES_CARGO.\n{}",
                env.iter().find(|(k, _)| *k == "RUSTUP_HOME").map(|x| &x.1),
                env.iter().find(|(k, _)| *k == "CARGO_HOME").map(|x| &x.1),
                env.iter().find(|(k, _)| *k == "RUSTUP_TOOLCHAIN").map(|x| &x.1),
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => Err(format!("no usable Rust toolchain for the cargo-backed acceptance cases: cannot run `cargo` ({e}); put it on PATH")),
        }
    })
}

/// `Err(one message)` when cargo-backed cases cannot run on this host; call once before running them.
pub fn require_toolchain() -> Result<(), String> {
    host_toolchain().as_ref().map(|_| ()).map_err(|e| e.clone())
}

/// Extra environment for cases that need the host toolchain (cargo/rustc).
fn toolchain_env(case: &Case) -> Vec<(&'static str, String)> {
    if case.needs.is_none() {
        return Vec::new();
    }
    host_toolchain().as_ref().cloned().unwrap_or_default()
}

/// Runs one case; `Err` explains which expectation failed.
pub fn run_case(sb: &Sandbox, case: &Case) -> Result<(), String> {
    std::env::set_var("WJ_PROMISE_BIN_DIR", sb.bin.parent().unwrap());
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
    let case_env: Vec<(String, String)> = case.env.iter().map(|(k, v)| (k.clone(), subst(v, &w))).collect();
    let env: Vec<(&str, String)> = env.into_iter().chain(case_env.iter().map(|(k, v)| (k.as_str(), v.clone()))).collect();
    for step in &case.setup {
        let args: Vec<String> = step.iter().map(|a| subst(a, &w)).collect();
        let r = run_wj(sb, &w, &args, None, &env);
        if r.code != Some(0) {
            return Err(format!("setup `wj {}` exited {:?}\n{}{}", args.join(" "), r.code, r.stdout, r.stderr));
        }
    }
    let cwd = case.cwd.as_ref().map(|c| w.join(subst(c, &w))).unwrap_or_else(|| w.clone());
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
