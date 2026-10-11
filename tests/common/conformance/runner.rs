//! Runs corpus programs over backends x seeds and judges each cell against the independent oracle.

use super::backends::{compile_and_run, Backend, Outcome, ALL_BACKENDS};
use super::compare::{divergence_report, first_divergence, Trace, Voice};
use super::oracle;
use super::program::Program;
use super::support::Support;
use super::toolchain::ToolProbe;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    /// Failed, but listed in SUPPORT.toml (explicit, visible).
    Unsupported(String),
    Skip(String),
    Fail(String),
}

#[derive(Default)]
pub struct Report {
    pub cells: BTreeMap<(String, Backend), Vec<Verdict>>,
}

impl Report {
    pub fn counts(&self, b: Backend) -> (usize, usize, usize, usize) {
        let (mut p, mut u, mut s, mut f) = (0, 0, 0, 0);
        for ((_, bb), vs) in &self.cells {
            if *bb != b {
                continue;
            }
            for v in vs {
                match v {
                    Verdict::Pass => p += 1,
                    Verdict::Unsupported(_) => u += 1,
                    Verdict::Skip(_) => s += 1,
                    Verdict::Fail(_) => f += 1,
                }
            }
        }
        (p, u, s, f)
    }

    pub fn failures(&self) -> Vec<String> {
        let mut out = Vec::new();
        for ((p, b), vs) in &self.cells {
            for v in vs {
                if let Verdict::Fail(msg) = v {
                    out.push(format!("{p} [{}]: {msg}", b.name()));
                }
            }
        }
        out
    }

    pub fn summary(&self) -> String {
        let mut s = String::from("backend  pass  unsupported  skip  fail\n");
        for b in ALL_BACKENDS {
            let (p, u, k, f) = self.counts(b);
            s += &format!("{:<8} {p:>4}  {u:>11}  {k:>4}  {f:>4}\n", b.name());
        }
        s
    }
}

/// Judge a single trace against the independent expectation (and report which voice disagrees).
pub fn judge(label: &str, expected: &Trace, got: &Trace) -> Option<String> {
    let voices = [
        Voice { label: "oracle".into(), trace: Some(expected.clone()) },
        Voice { label: label.into(), trace: Some(got.clone()) },
    ];
    divergence_report(&voices)
}

fn run_cell(wj: &Path, p: &Program, b: Backend, seed: u64, work: &Path) -> Result<Trace, String> {
    match compile_and_run(wj, b, work, &p.source_for(seed)) {
        Outcome::BuildFailed(e) => Err(format!("seed {seed}: build/toolchain failure: {}", e.lines().take(12).collect::<Vec<_>>().join(" | "))),
        Outcome::Ran(t, _) => Ok(t),
    }
}

fn verdict_for(support: &Support, p: &Program, b: Backend, raw: Result<(), String>) -> Verdict {
    match (raw, support.reason(&p.name, b)) {
        (Ok(()), None) => Verdict::Pass,
        (Ok(()), Some(r)) => Verdict::Fail(format!("passes but SUPPORT.toml lists it unsupported ({r}); remove the entry (ratchet)")),
        (Err(e), None) => Verdict::Fail(e),
        (Err(_), Some(r)) => Verdict::Unsupported(r.to_string()),
    }
}

/// Evaluate one program on one backend across all its seeds.
pub fn eval_program(wj: &Path, p: &Program, b: Backend, support: &Support, work_root: &Path) -> Verdict {
    let mut result = Ok(());
    for &seed in &p.seeds {
        let work = work_root.join(format!("{}_{}_{}", p.name, b.name(), seed));
        std::fs::create_dir_all(&work).unwrap();
        let step = oracle::expected(p, seed).and_then(|exp| {
            let got = run_cell(wj, p, b, seed, &work)?;
            match first_divergence(&exp, &got) {
                None => Ok(()),
                Some(d) => Err(format!("seed {seed}: {d}\n{}", judge(b.name(), &exp, &got).unwrap_or_default())),
            }
        });
        if let Err(e) = step {
            result = Err(e);
            break;
        }
    }
    verdict_for(support, p, b, result)
}

/// Everything a worker needs, shared immutably across threads.
pub struct Ctx {
    pub wj: std::path::PathBuf,
    pub programs: Vec<Program>,
    pub support: Support,
    pub probe: ToolProbe,
    pub work_root: std::path::PathBuf,
}

pub fn run_all(ctx: std::sync::Arc<Ctx>) -> Report {
    let mut report = Report::default();
    let jobs: Vec<(usize, Backend)> = (0..ctx.programs.len()).flat_map(|i| ALL_BACKENDS.map(|b| (i, b))).collect();
    let c = ctx.clone();
    let results = super::parallel::map(jobs, 3, move |(i, b)| {
        let p = &c.programs[i];
        let v = if !c.probe.has(b.tool()) {
            Verdict::Skip(format!("toolchain {} missing", b.tool().name()))
        } else {
            eval_program(&c.wj, p, b, &c.support, &c.work_root)
        };
        ((p.name.clone(), b), v)
    });
    for (k, v) in results {
        report.cells.entry(k).or_default().push(v);
    }
    report
}
