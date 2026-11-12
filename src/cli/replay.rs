// wj replay - step through a recorded or simulated run deterministically (ADR-0035).
//
// Adapter between the CLI and `interpreter::debugger`: reads the command script, runs the session, prints the events.

use crate::interpreter::debugger::{render, script, session};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

pub struct Args {
    /// `LOG PROGRAM`, or `PROGRAM` with `--simulate`.
    pub paths: Vec<PathBuf>,
    pub script: Option<PathBuf>,
    pub json: bool,
    pub simulate: bool,
    pub seed: u64,
    pub faults: f64,
    pub allow: Vec<String>,
}

fn read_script(p: &Option<PathBuf>) -> Result<String> {
    match p {
        Some(f) if f.as_os_str() != "-" => std::fs::read_to_string(f).with_context(|| format!("cannot read the script {}", f.display())),
        _ => {
            let mut t = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).context("cannot read the script from standard input")?;
            Ok(t)
        }
    }
}

pub fn execute(a: Args) -> Result<()> {
    let (source, program) = match (a.simulate, a.paths.as_slice()) {
        (true, [p]) => (session::Source::Simulate { seed: a.seed, faults: a.faults }, p.clone()),
        (false, [log, p]) => (session::Source::Log(log.clone()), p.clone()),
        (true, _) => bail!("wj replay --simulate takes one program: `wj replay --simulate --seed N PROGRAM`"),
        (false, _) => bail!("wj replay takes a log and a program: `wj replay LOG PROGRAM` (or `--simulate PROGRAM`)"),
    };
    if let session::Source::Log(log) = &source {
        if !log.is_file() {
            bail!("no effect log at {} (write one with `wj run --record {} PROGRAM`)", log.display(), log.display());
        }
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let entry = crate::interpreter::runner::entry_file(&program).map_err(anyhow::Error::msg)?;
    let entry = if entry.is_absolute() { entry } else { cwd.join(entry) };
    let cmds = script::parse(&read_script(&a.script)?).map_err(anyhow::Error::msg)?;
    // the interpreter never runs a program `wj check` rejects (as for `wj run --interpret`); lints do not stop it
    crate::cli::check::require_clean(&entry, false, crate::diagnostics::render::Format::Human)?;
    let events = session::run(session::SessionSpec { entry, source, allow: a.allow, script: cmds }).map_err(anyhow::Error::msg)?;
    let mut out = String::new();
    for e in &events {
        out.push_str(&if a.json { e.to_string() } else { render::text(e) });
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}
