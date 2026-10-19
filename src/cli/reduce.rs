// wj reduce - shrink a failing project to a minimal, anonymized reproduction.
//
// Adapter between the CLI and the `reduce` library module. Prints counts only, never source text.

use crate::reduce::{self, Oracle, ReduceOptions};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;

pub struct Args {
    pub path: PathBuf,
    pub oracle: String,
    pub output: PathBuf,
    pub jobs: usize,
    pub seed: u64,
    pub max_steps: usize,
    pub time_budget: u64,
    pub candidate_timeout: u64,
    pub keep_names: bool,
    pub wj_bin: Option<PathBuf>,
    pub verbose: bool,
}

pub fn execute(a: Args) -> Result<()> {
    let oracle = Oracle::parse(&a.oracle)?;
    let wj_bin = match a.wj_bin {
        Some(b) => b,
        None => {
            std::env::current_exe().context("cannot locate the wj executable; pass --wj-bin")?
        }
    };
    let outcome = reduce::run(&ReduceOptions {
        input: a.path,
        oracle,
        out: a.output.clone(),
        jobs: a.jobs.max(1),
        seed: a.seed,
        max_steps: a.max_steps,
        time_budget: Duration::from_secs(a.time_budget),
        candidate_timeout: Duration::from_secs(a.candidate_timeout),
        keep_names: a.keep_names,
        wj_bin,
        target_base: Some(crate::cargo_cache::shared_target_dir()),
        verbose: a.verbose,
    })?;
    for w in &outcome.warnings {
        eprintln!("warning: {w}");
    }
    println!(
        "reduced {} -> {} files, {} -> {} lines in {} oracle runs{}; wrote {}",
        outcome.files_before,
        outcome.files_after,
        outcome.lines_before,
        outcome.lines_after,
        outcome.steps,
        if outcome.anonymized {
            " (anonymized)"
        } else {
            ""
        },
        a.output.display()
    );
    Ok(())
}
