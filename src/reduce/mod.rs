//! `wj reduce`: automatic test-case reducer for Windjammer projects. See `MODULE.md`.

pub mod anonymize;
mod ddmin;
mod engine;
mod finish;
mod names;
mod oracle;
mod pass_files;
mod pass_sites;
mod pass_units;
mod proc;
mod project;
mod report;
mod rng;
mod session;
mod state;
mod toks;

pub use oracle::Oracle;
pub use project::Project;

use anyhow::{bail, Result};
use engine::Engine;
use oracle::OracleEnv;
use session::Session;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct ReduceOptions {
    pub input: PathBuf,
    pub oracle: Oracle,
    pub out: PathBuf,
    pub jobs: usize,
    pub seed: u64,
    pub max_steps: usize,
    pub time_budget: Duration,
    pub candidate_timeout: Duration,
    pub keep_names: bool,
    pub wj_bin: PathBuf,
    /// Where cargo target directories for `rustc-error` oracles live (the driver passes `CARGO_TARGET_DIR`).
    pub target_base: Option<PathBuf>,
    /// Print progress counts (never source text).
    pub verbose: bool,
}

#[derive(Debug)]
pub struct Outcome {
    pub files_before: usize,
    pub files_after: usize,
    pub lines_before: usize,
    pub lines_after: usize,
    pub steps: usize,
    pub anonymized: bool,
    pub warnings: Vec<String>,
}

pub fn run(opts: &ReduceOptions) -> Result<Outcome> {
    let original = Project::load(&opts.input)?;
    let scratch = tempfile::tempdir()?;
    let env = OracleEnv {
        wj_bin: opts.wj_bin.clone(),
        timeout: opts.candidate_timeout,
        scratch: scratch.path().to_path_buf(),
        aux: ["wj.toml"]
            .iter()
            .filter_map(|n| {
                std::fs::read_to_string(opts.input.join(n))
                    .ok()
                    .map(|t| (n.to_string(), t))
            })
            .collect(),
        target_base: opts
            .target_base
            .clone()
            .unwrap_or_else(|| scratch.path().join("target")),
    };
    let eng = Engine::new(
        opts.oracle.clone(),
        env,
        opts.jobs,
        opts.max_steps,
        Instant::now() + opts.time_budget,
        &original,
    );
    let say = |m: String| {
        if opts.verbose {
            eprintln!("reduce: {m}");
        }
    };

    let in_hash = format!("{:016x}", original.hash());
    let desc = opts.oracle.describe();
    let sdir = state::dir_for(&opts.out);
    let (mut st, start) = match state::load(&sdir, &in_hash, &desc, opts.seed) {
        Some((st, p)) => {
            say(format!(
                "resuming at round {} ({} files)",
                st.round,
                p.files.len()
            ));
            eng.preload_cache(state::cache_from_state(&st.cache));
            (st, p)
        }
        None => {
            if !eng.baseline_holds(&original) {
                bail!("the oracle `{desc}` does not hold on the input project; nothing to reduce");
            }
            (
                state::State {
                    input_hash: in_hash,
                    oracle: desc.clone(),
                    seed: opts.seed,
                    ..Default::default()
                },
                original.clone(),
            )
        }
    };
    let mut s = Session::new(&eng, start, opts.seed.wrapping_add(st.round as u64));
    if !st.finished {
        say(format!(
            "start: {} files, {} lines",
            s.project.files.len(),
            s.project.line_count()
        ));
        loop {
            let mut changed = pass_files::strip_comments(&mut s);
            changed |= pass_files::drop_files(&mut s);
            changed |= pass_units::run(&mut s);
            changed |= pass_sites::run(&mut s);
            st.round += 1;
            st.steps = eng.steps();
            st.cache = state::cache_to_state(&eng.cache_snapshot());
            st.finished = !changed && !eng.exhausted();
            state::save(&sdir, &st, &s.project)?;
            say(format!(
                "round {}: {} files, {} lines, {} oracle runs",
                st.round,
                s.project.files.len(),
                s.project.line_count(),
                eng.steps()
            ));
            if st.finished || eng.exhausted() {
                break;
            }
        }
    }
    let reduced = s.project.clone();
    let mut warnings = Vec::new();
    if !eng.env.aux.is_empty() {
        warnings.push("the input's wj.toml was used while reducing but is not part of the output (it may name private dependencies)".to_string());
    }
    let (final_project, anonymized) = finish_project(&eng, &reduced, opts, &mut warnings)?;
    let pretty = finish::formatted(&final_project);
    let final_project = if pretty == final_project || eng.interesting(&pretty) {
        pretty
    } else {
        warnings.push("formatting lost the bug; wrote the unformatted candidate".into());
        final_project
    };
    finish::write_output(&opts.out, &final_project)?;
    let md = report::repro_md(&report::Summary {
        oracle: desc,
        before: &original,
        after: &final_project,
        steps: eng.steps(),
        cache_hits: eng.cache_hits(),
        rejected: eng.rejected(),
        seed: opts.seed,
        anonymized,
        warnings: &warnings,
    });
    std::fs::write(opts.out.join("repro.md"), md)?;
    Ok(Outcome {
        files_before: original.files.len(),
        files_after: final_project.files.len(),
        lines_before: original.line_count(),
        lines_after: final_project.line_count(),
        steps: eng.steps(),
        anonymized,
        warnings,
    })
}

fn finish_project(
    eng: &Engine,
    reduced: &Project,
    opts: &ReduceOptions,
    warnings: &mut Vec<String>,
) -> Result<(Project, bool)> {
    eng.set_unlimited();
    let a = finish::anonymize(eng, reduced);
    if opts.verbose {
        eprintln!(
            "reduce: anonymized {}/{} names, {} user names could not be renamed",
            a.renamed, a.total, a.stuck_declared
        );
    }
    if a.stuck_declared == 0 {
        return Ok((a.project, true));
    }
    if opts.keep_names {
        warnings.push(format!(
            "{} user-declared names or strings could not be anonymized without losing the bug (kept because of --keep-names); review before sharing",
            a.stuck_declared
        ));
        return Ok((a.project, false));
    }
    bail!(
        "anonymization is incomplete: {} user-declared names or strings cannot be renamed without losing the bug; nothing was written (rerun with --keep-names to accept a partially anonymized result)",
        a.stuck_declared
    )
}
