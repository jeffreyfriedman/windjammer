//! Oracles: "is the bug still present in this project?". Specs are parsed from the CLI; evaluation
//! writes the candidate to a scratch directory and runs the compiler (or a user command) on it.

use super::proc;
use super::project::Project;
use anyhow::{bail, Result};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub enum Oracle {
    /// rustc rejects the generated crate with `code` (and, if given, a message matching `msg`).
    RustcError { code: String, msg: Option<Regex> },
    /// The compiler panics or crashes while building the project.
    Panic,
    /// Interpreted and compiled runs disagree.
    Miscompile,
    /// Shell command run in the candidate directory; exit status 0 means the bug is present.
    Cmd(String),
    /// In-process predicate (library use and tests).
    Predicate(Arc<dyn Fn(&Project) -> bool + Send + Sync>),
}

impl Oracle {
    pub fn parse(spec: &str) -> Result<Oracle> {
        let spec = spec.trim();
        if let Some(rest) = spec.strip_prefix("rustc-error") {
            let rest = rest.trim();
            let (code, msg) = match rest.split_once(':') {
                Some((c, m)) => (c.trim(), Some(m.trim())),
                None => (rest, None),
            };
            let valid = code.len() == 5
                && code.starts_with('E')
                && code[1..].chars().all(|c| c.is_ascii_digit());
            if !valid {
                bail!("oracle `rustc-error` needs an error code like E0308 (optionally `E0308:regex`)");
            }
            let msg = match msg {
                Some(m) if !m.is_empty() => {
                    Some(Regex::new(m).map_err(|e| anyhow::anyhow!("bad message regex: {e}"))?)
                }
                _ => None,
            };
            return Ok(Oracle::RustcError {
                code: code.to_string(),
                msg,
            });
        }
        if let Some(rest) = spec.strip_prefix("cmd") {
            let cmd = rest.trim();
            if cmd.is_empty() {
                bail!("oracle `cmd` needs a shell command: --oracle \"cmd 'grep -q foo main.wj'\"");
            }
            return Ok(Oracle::Cmd(cmd.trim_matches('\'').to_string()));
        }
        match spec {
            "panic" | "crash" => Ok(Oracle::Panic),
            "miscompile" => Ok(Oracle::Miscompile),
            _ => bail!("unknown oracle `{spec}`; expected rustc-error E0308[:regex] | panic | crash | miscompile | cmd '<shell>'"),
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Oracle::RustcError { code, msg: None } => format!("rustc-error {code}"),
            Oracle::RustcError { code, msg: Some(m) } => {
                format!("rustc-error {code}:{}", m.as_str())
            }
            Oracle::Panic => "panic".into(),
            Oracle::Miscompile => "miscompile".into(),
            Oracle::Cmd(c) => format!("cmd '{c}'"),
            Oracle::Predicate(_) => "predicate".into(),
        }
    }
}

/// Where and how oracles run.
#[derive(Clone)]
pub struct OracleEnv {
    pub wj_bin: PathBuf,
    pub timeout: Duration,
    pub scratch: PathBuf,
    /// Cargo target directories live under here, one per worker, reused across candidates so
    /// dependencies (the runtime crate) compile once.
    pub target_base: PathBuf,
    /// Non-source files of the input project (its `wj.toml`), copied verbatim into every candidate
    /// directory so oracles build under the original configuration. They are never part of the output.
    pub aux: Vec<(String, String)>,
}

impl Oracle {
    /// Whether the bug is present in `project`. `worker` selects a private scratch directory.
    pub fn holds(&self, env: &OracleEnv, project: &Project, worker: usize) -> bool {
        if let Oracle::Predicate(f) = self {
            return f(project);
        }
        let dir = env.scratch.join(format!("w{worker}"));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        if project.write_to(&src).is_err() {
            return false;
        }
        for (rel, text) in &env.aux {
            let _ = std::fs::write(src.join(rel), text);
        }
        let r = self.run_in(env, &src, &dir.join("out"), worker);
        let _ = std::fs::remove_dir_all(&dir);
        r
    }

    fn run_in(&self, env: &OracleEnv, src: &Path, out: &Path, worker: usize) -> bool {
        match self {
            Oracle::Predicate(_) => unreachable!(),
            Oracle::Cmd(c) => {
                let mut cmd = Command::new("/bin/sh");
                cmd.arg("-c")
                    .arg(c)
                    .current_dir(src)
                    .env("WJ", &env.wj_bin)
                    .env("WJ_REDUCE_OUT", out);
                proc::run(cmd, env.timeout).success()
            }
            Oracle::Panic => {
                let r = build(env, src, out);
                r.signaled
                    || r.code == Some(101)
                    || r.stderr.contains("panicked at")
                    || r.stderr.contains("internal compiler error")
            }
            Oracle::RustcError { code, msg } => {
                if !build(env, src, out).success() {
                    return false;
                }
                let mut cmd = Command::new("cargo");
                cmd.args(["check", "--message-format=json", "--quiet"])
                    .current_dir(out)
                    .env(
                        "CARGO_TARGET_DIR",
                        env.target_base.join(format!("reduce-w{worker}")),
                    );
                let r = proc::run(cmd, env.timeout);
                r.stdout
                    .lines()
                    .any(|l| rustc_line_matches(l, code, msg.as_ref()))
            }
            Oracle::Miscompile => {
                let mut i = Command::new(&env.wj_bin);
                i.arg("run").arg(src).arg("--interpret");
                let mut c = Command::new(&env.wj_bin);
                c.arg("run").arg(src).arg("--compiled");
                let (a, b) = (proc::run(i, env.timeout), proc::run(c, env.timeout));
                // a build failure of the compiled side is a different bug (use `rustc-error`), not a miscompile
                let build_failed = b.stderr.contains("could not compile")
                    || b.stderr.contains("error[E")
                    || b.stderr.contains("Transpilation failed");
                a.code == Some(0)
                    && !a.timed_out
                    && !b.timed_out
                    && !b.signaled
                    && b.code != Some(127)
                    && !build_failed
                    && (a.stdout != b.stdout || a.code != b.code)
            }
        }
    }
}

fn build(env: &OracleEnv, src: &Path, out: &Path) -> proc::Captured {
    let mut cmd = Command::new(&env.wj_bin);
    cmd.arg("build")
        .arg(src)
        .arg("--no-cargo")
        .arg("--no-lint")
        .arg("-o")
        .arg(out);
    proc::run(cmd, env.timeout)
}

fn rustc_line_matches(line: &str, code: &str, msg: Option<&Regex>) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    let m = &v["message"];
    if m["level"] != "error" || m["code"]["code"] != code {
        return false;
    }
    msg.is_none_or(|re| m["message"].as_str().is_some_and(|t| re.is_match(t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_spec_form() {
        assert!(matches!(
            Oracle::parse("rustc-error E0308").unwrap(),
            Oracle::RustcError { msg: None, .. }
        ));
        assert!(matches!(
            Oracle::parse("rustc-error E0308:mismatched.*").unwrap(),
            Oracle::RustcError { msg: Some(_), .. }
        ));
        assert!(matches!(Oracle::parse("panic").unwrap(), Oracle::Panic));
        assert!(matches!(Oracle::parse("crash").unwrap(), Oracle::Panic));
        assert!(matches!(
            Oracle::parse("miscompile").unwrap(),
            Oracle::Miscompile
        ));
        assert!(
            matches!(Oracle::parse("cmd 'grep -q x main.wj'").unwrap(), Oracle::Cmd(c) if c == "grep -q x main.wj")
        );
    }

    #[test]
    fn rejects_malformed_specs() {
        assert!(Oracle::parse("rustc-error").is_err());
        assert!(Oracle::parse("rustc-error 308").is_err());
        assert!(Oracle::parse("rustc-error E0308:(").is_err());
        assert!(Oracle::parse("cmd").is_err());
        assert!(Oracle::parse("telepathy").is_err());
    }

    #[test]
    fn matches_rustc_json_by_code_and_message() {
        let line = r#"{"reason":"compiler-message","message":{"level":"error","code":{"code":"E0308"},"message":"mismatched types"}}"#;
        assert!(rustc_line_matches(line, "E0308", None));
        assert!(rustc_line_matches(
            line,
            "E0308",
            Some(&Regex::new("mismatched").unwrap())
        ));
        assert!(!rustc_line_matches(
            line,
            "E0308",
            Some(&Regex::new("borrow").unwrap())
        ));
        assert!(!rustc_line_matches(line, "E0382", None));
    }
}
