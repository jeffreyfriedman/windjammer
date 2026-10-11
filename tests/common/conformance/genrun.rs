//! Running generated program bundles against every backend, shrinking failures.

use super::backends::{compile_and_run, Backend, Outcome};
use super::gen_ast::*;
use super::gen_build::gen_case;
use super::shrink::shrink;
use std::path::Path;

/// Outcome of comparing one backend's trace with the evaluator for a bundle of cases.
pub enum Verdict {
    Agree,
    BuildFailed(String),
    /// First diverging case id, 1-based line within the whole trace, expected vs got.
    Diverged { case: usize, line: usize, expected: String, got: String },
}

pub fn make_cases(first_seed: u64, count: usize) -> Vec<(usize, Case)> {
    (0..count).map(|i| (i, gen_case(first_seed + i as u64))).collect()
}

fn case_of_line(expected: &[String], line: usize) -> usize {
    expected[..=line.min(expected.len() - 1)].iter().rev().find_map(|l| l.strip_prefix('#').and_then(|n| n.parse().ok())).unwrap_or(0)
}

pub fn run_bundle(wj: &Path, b: Backend, work: &Path, cases: &[(usize, Case)], names: &Names) -> Verdict {
    let src = print_program(cases, names);
    let expected = eval_program(cases);
    match compile_and_run(wj, b, work, &src) {
        Outcome::BuildFailed(e) => Verdict::BuildFailed(e),
        Outcome::Ran(t, _) => {
            for (i, e) in expected.iter().enumerate() {
                if t.lines.get(i) != Some(e) {
                    let got = t.lines.get(i).cloned().unwrap_or_else(|| "<missing>".into());
                    return Verdict::Diverged { case: case_of_line(&expected, i), line: i + 1, expected: e.clone(), got };
                }
            }
            if t.lines.len() != expected.len() {
                return Verdict::Diverged { case: cases.last().map(|c| c.0).unwrap_or(0), line: expected.len() + 1, expected: "<end>".into(), got: t.lines[expected.len()].clone() };
            }
            Verdict::Agree
        }
    }
}

/// Shrink the failing case of a bundle to a minimal program; returns a printable report.
pub fn shrink_report(wj: &Path, b: Backend, work: &Path, case_id: usize, case: &Case, names: &Names) -> String {
    let mut n = 0usize;
    let small = shrink(case, |cand| {
        n += 1;
        let w = work.join(format!("shrink{n}"));
        std::fs::create_dir_all(&w).ok();
        !matches!(run_bundle(wj, b, &w, &[(case_id, cand.clone())], names), Verdict::Agree)
    });
    let w = work.join("shrink_final");
    std::fs::create_dir_all(&w).ok();
    let detail = match run_bundle(wj, b, &w, &[(case_id, small.clone())], names) {
        Verdict::Diverged { line, expected, got, .. } => format!("first diverging checkpoint: trace line {line}: expected `{expected}` but {} printed `{got}`", b.name()),
        Verdict::BuildFailed(e) => format!("backend build failure: {}", e.lines().take(6).collect::<Vec<_>>().join(" | ")),
        Verdict::Agree => "(minimal program no longer fails - flaky)".into(),
    };
    format!("minimal program:\n{}\n{detail}", print_program(&[(case_id, small)], names))
}
