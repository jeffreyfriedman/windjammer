//! Compile-and-run adapters for each backend (ports; the compiler is invoked as a subprocess).

use super::compare::Trace;
use super::exec::{run_with_timeout, Exec};
use super::toolchain::Tool;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Backend {
    Rust,
    Go,
    Js,
}

pub const ALL_BACKENDS: [Backend; 3] = [Backend::Rust, Backend::Go, Backend::Js];

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Backend::Rust => "rust",
            Backend::Go => "go",
            Backend::Js => "js",
        }
    }

    pub fn wj_target(self) -> &'static str {
        match self {
            Backend::Rust => "rust",
            Backend::Go => "go",
            Backend::Js => "javascript",
        }
    }

    pub fn tool(self) -> Tool {
        match self {
            Backend::Rust => Tool::Rustc,
            Backend::Go => Tool::Go,
            Backend::Js => Tool::Node,
        }
    }
}

/// Result of compiling+running one program on one backend.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// wj refused or produced code the toolchain rejected.
    BuildFailed(String),
    Ran(Trace, Exec),
}

pub const RUN_BUDGET: Duration = Duration::from_secs(30);
pub const BUILD_BUDGET: Duration = Duration::from_secs(300);

fn wj_build(wj: &Path, backend: Backend, src: &Path, out: &Path) -> Result<(), String> {
    let mut cmd = Command::new(wj);
    cmd.args(["build", "--target", backend.wj_target(), "--no-cargo", "--no-lint", "-o"]).arg(out).arg(src);
    let r = run_with_timeout(&mut cmd, BUILD_BUDGET);
    if r.code == 0 { Ok(()) } else { Err(format!("wj build failed: {}{}", r.stderr, r.stdout)) }
}

fn find_with_ext(dir: &Path, ext: &str) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    v.sort();
    v.into_iter().find(|p| p.extension().map(|e| e == ext).unwrap_or(false) && p.file_name().unwrap() != "mod.rs")
}

fn to_outcome(e: Exec) -> Outcome {
    if e.timed_out {
        return Outcome::BuildFailed(format!("timed out after {:?}", e.elapsed));
    }
    Outcome::Ran(Trace::from_stdout(&e.stdout, e.code, &e.stderr), e)
}

fn build_artifact(wj: &Path, backend: Backend, work: &Path, source: &str) -> Result<PathBuf, String> {
    let src = work.join("prog.wj");
    std::fs::write(&src, source).map_err(|e| e.to_string())?;
    let out = work.join("out");
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    wj_build(wj, backend, &src, &out)?;
    Ok(out)
}

fn run_rust(out: &Path, work: &Path) -> Outcome {
    let Some(rs) = find_with_ext(out, "rs") else { return Outcome::BuildFailed("no .rs emitted".into()) };
    let bin = work.join("prog_bin");
    let mut c = Command::new("rustc");
    c.args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&bin).arg(&rs);
    let r = run_with_timeout(&mut c, BUILD_BUDGET);
    if r.code != 0 {
        return Outcome::BuildFailed(format!("rustc rejected generated code: {}", r.stderr));
    }
    to_outcome(run_with_timeout(&mut Command::new(&bin), RUN_BUDGET))
}

fn run_go(out: &Path) -> Outcome {
    let mut vet = Command::new("go");
    vet.args(["vet", "./..."]).current_dir(out);
    let v = run_with_timeout(&mut vet, BUILD_BUDGET);
    if v.code != 0 {
        return Outcome::BuildFailed(format!("go vet rejected generated code: {}{}", v.stderr, v.stdout));
    }
    // Build then run the binary: `go run` reports every non-zero exit status as 1.
    let bin = out.join("prog_bin");
    let mut build = Command::new("go");
    build.args(["build", "-o"]).arg(&bin).arg(".").current_dir(out);
    let b = run_with_timeout(&mut build, BUILD_BUDGET);
    if b.code != 0 {
        return Outcome::BuildFailed(format!("go build failed: {}{}", b.stderr, b.stdout));
    }
    to_outcome(run_with_timeout(&mut Command::new(&bin), RUN_BUDGET))
}

fn run_js(out: &Path) -> Outcome {
    let Some(entry) = ["main.mjs", "main.ts", "main.js"].iter().map(|n| out.join(n)).find(|p| p.exists()) else {
        return Outcome::BuildFailed("no JS entry point emitted".into());
    };
    let mut run = Command::new("node");
    run.arg(entry).current_dir(out);
    to_outcome(run_with_timeout(&mut run, RUN_BUDGET))
}

/// Hook that edits the generated artifact directory before it is built/run (mutation testing).
pub type Mutator<'a> = &'a dyn Fn(&Path) -> Result<(), String>;

/// Compile `source` for `backend` in `work` and run it.
pub fn compile_and_run(wj: &Path, backend: Backend, work: &Path, source: &str) -> Outcome {
    compile_and_run_with(wj, backend, work, source, None)
}

pub fn compile_and_run_with(wj: &Path, backend: Backend, work: &Path, source: &str, mutate: Option<Mutator>) -> Outcome {
    let out = match build_artifact(wj, backend, work, source) {
        Ok(o) => o,
        Err(e) => return Outcome::BuildFailed(e),
    };
    if let Some(m) = mutate {
        if let Err(e) = m(&out) {
            return Outcome::BuildFailed(format!("mutation did not apply: {e}"));
        }
    }
    match backend {
        Backend::Rust => run_rust(&out, work),
        Backend::Go => run_go(&out),
        Backend::Js => run_js(&out),
    }
}

/// Weaker 4th oracle: the Windjammer interpreter (`wj run --interpret`).
pub fn run_interpreter(wj: &Path, work: &Path, source: &str) -> Outcome {
    let src = work.join("interp.wj");
    if std::fs::write(&src, source).is_err() {
        return Outcome::BuildFailed("write failed".into());
    }
    let mut c = Command::new(wj);
    c.args(["run", "--interpret"]).arg(&src);
    to_outcome(run_with_timeout(&mut c, RUN_BUDGET))
}
