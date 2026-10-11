//! Trace-level comparison and divergence reports.
//!
//! A trace is the ordered stdout lines (programs emit checkpoint hashes as lines) plus exit status
//! and a stderr class. Comparison reports the FIRST diverging line, never just "differs".

use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StderrClass {
    Clean,
    Panic,
    Other,
}

pub fn classify_stderr(stderr: &str, exit: i32) -> StderrClass {
    let s = stderr.to_lowercase();
    if s.contains("panic") || s.contains("thread 'main'") || s.contains("uncaught") {
        StderrClass::Panic
    } else if exit != 0 || !s.trim().is_empty() && s.contains("error") {
        StderrClass::Other
    } else {
        StderrClass::Clean
    }
}

#[derive(Debug, Clone)]
pub struct Trace {
    pub lines: Vec<String>,
    pub exit: i32,
    pub stderr_class: StderrClass,
}

impl Trace {
    pub fn from_expect(lines: Vec<String>, exit: i32) -> Trace {
        let stderr_class = if exit == 0 { StderrClass::Clean } else { StderrClass::Panic };
        Trace { lines, exit, stderr_class }
    }

    pub fn from_stdout(stdout: &str, exit: i32, stderr: &str) -> Trace {
        let lines = stdout.lines().map(|l| l.trim_end().to_string()).collect();
        Trace { lines, exit, stderr_class: classify_stderr(stderr, exit) }
    }
}

/// None when traces agree; otherwise a human-readable first-divergence description.
pub fn first_divergence(a: &Trace, b: &Trace) -> Option<String> {
    let n = a.lines.len().min(b.lines.len());
    for i in 0..n {
        if a.lines[i] != b.lines[i] {
            return Some(format!("line {}: {:?} vs {:?}", i + 1, a.lines[i], b.lines[i]));
        }
    }
    if a.lines.len() != b.lines.len() {
        return Some(format!("line count {} vs {}", a.lines.len(), b.lines.len()));
    }
    if a.exit != b.exit {
        return Some(format!("exit status {} vs {}", a.exit, b.exit));
    }
    if a.stderr_class != b.stderr_class {
        return Some(format!("stderr class {:?} vs {:?}", a.stderr_class, b.stderr_class));
    }
    None
}

/// One labelled oracle/backend verdict used to build a divergence report.
pub struct Voice {
    pub label: String,
    pub trace: Option<Trace>,
}

/// Which voice disagrees with which: compares every pair against the first (independent) voice,
/// and lists pairwise disagreements among the rest.
pub fn divergence_report(voices: &[Voice]) -> Option<String> {
    let mut out = String::new();
    for (i, a) in voices.iter().enumerate() {
        for b in voices.iter().skip(i + 1) {
            let (Some(ta), Some(tb)) = (&a.trace, &b.trace) else { continue };
            if let Some(d) = first_divergence(ta, tb) {
                let _ = writeln!(out, "  {} != {} ({})", a.label, b.label, d);
            }
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(lines: &[&str]) -> Trace {
        Trace::from_expect(lines.iter().map(|s| s.to_string()).collect(), 0)
    }

    #[test]
    fn reports_first_divergence_line() {
        let d = first_divergence(&t(&["a", "b", "c"]), &t(&["a", "x", "y"])).unwrap();
        assert!(d.starts_with("line 2"), "{d}");
    }

    #[test]
    fn equal_traces_agree() {
        assert!(first_divergence(&t(&["a"]), &t(&["a"])).is_none());
    }

    #[test]
    fn exit_status_is_part_of_the_trace() {
        let a = t(&["a"]);
        let mut b = t(&["a"]);
        b.exit = 3;
        assert!(first_divergence(&a, &b).unwrap().contains("exit"));
    }

    #[test]
    fn panic_stderr_classified() {
        assert_eq!(classify_stderr("thread 'main' panicked at x", 101), StderrClass::Panic);
        assert_eq!(classify_stderr("", 0), StderrClass::Clean);
    }
}
