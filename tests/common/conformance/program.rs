//! Corpus program model: header parsing and seed templating.

use std::path::{Path, PathBuf};

pub const SEED_PLACEHOLDER: &str = "__SEED__";

#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub path: PathBuf,
    pub template: String,
    pub seeds: Vec<u64>,
    pub known_expect: Vec<String>,
    pub expect_exit: i32,
    pub has_ref: bool,
    pub seeded: bool,
    pub stderr: StderrExpect,
}

/// Independent expectations about the stderr stream (ADR-0014 intrinsics write to it):
/// `// stderr: line` (exact lines, in order), `// stderr-has: text` (substring), `// stderr-empty`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StderrExpect {
    pub exact: Vec<String>,
    pub has: Vec<String>,
    pub empty: bool,
}

impl StderrExpect {
    pub fn is_set(&self) -> bool {
        self.empty || !self.exact.is_empty() || !self.has.is_empty()
    }

    /// None when `stderr` satisfies every expectation.
    pub fn check(&self, stderr: &str) -> Option<String> {
        if self.empty && !stderr.trim().is_empty() {
            return Some(format!("expected empty stderr, got {:?}", stderr.trim()));
        }
        if !self.exact.is_empty() {
            let got: Vec<&str> = stderr.lines().map(|l| l.trim_end()).collect();
            if got != self.exact.iter().map(|s| s.as_str()).collect::<Vec<_>>() {
                return Some(format!("stderr lines {:?} != expected {:?}", got, self.exact));
            }
        }
        for h in &self.has {
            if !stderr.contains(h.as_str()) {
                return Some(format!("stderr lacks {h:?}; got {:?}", stderr.trim()));
            }
        }
        None
    }
}

pub fn parse_stderr_header(template: &str) -> StderrExpect {
    let mut e = StderrExpect::default();
    for line in template.lines() {
        let Some(rest) = line.strip_prefix("// ") else { continue };
        let value = |v: &str| v.strip_prefix(' ').unwrap_or(v).to_string();
        if let Some(v) = rest.strip_prefix("stderr-has:") {
            e.has.push(value(v));
        } else if let Some(v) = rest.strip_prefix("stderr:") {
            e.exact.push(value(v));
        } else if rest.trim_end() == "stderr-empty" {
            e.empty = true;
        }
    }
    e
}

pub fn parse_header(template: &str) -> (Vec<u64>, Vec<String>, i32) {
    let mut seeds = Vec::new();
    let mut expect = Vec::new();
    let mut exit = 0;
    for line in template.lines() {
        let Some(rest) = line.strip_prefix("// ") else { continue };
        if let Some(v) = rest.strip_prefix("seeds:") {
            seeds = v.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        } else if let Some(v) = rest.strip_prefix("expect:") {
            expect.push(v.strip_prefix(' ').unwrap_or(v).to_string());
        } else if let Some(v) = rest.strip_prefix("exit:") {
            exit = v.trim().parse().unwrap_or(0);
        }
    }
    (seeds, expect, exit)
}

pub fn load(path: &Path) -> Program {
    let template = std::fs::read_to_string(path).expect("read program");
    let (mut seeds, known_expect, expect_exit) = parse_header(&template);
    let seeded = template.contains(SEED_PLACEHOLDER);
    if seeds.is_empty() {
        seeds = if seeded { vec![1, 2, 3, 4, 5] } else { vec![0] };
    }
    let name = path.file_stem().unwrap().to_string_lossy().to_string();
    let has_ref = path.with_extension("ref.py").exists();
    let stderr = parse_stderr_header(&template);
    Program { name, path: path.to_path_buf(), template, seeds, known_expect, expect_exit, has_ref, seeded, stderr }
}

impl Program {
    pub fn source_for(&self, seed: u64) -> String {
        self.template.replace(SEED_PLACEHOLDER, &seed.to_string())
    }

    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap()
    }
}

/// All `*.wj` corpus programs, sorted by name.
pub fn discover(dir: &Path) -> Vec<Program> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("corpus dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|e| e == "wj").unwrap_or(false))
        .collect();
    paths.sort();
    paths.iter().map(|p| load(p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_parses_expect_seeds_exit() {
        let (s, e, x) = parse_header("// seeds: 3 4\n// expect: a b\n// expect: c\n// exit: 101\nfn main(){}");
        assert_eq!(s, vec![3, 4]);
        assert_eq!(e, vec!["a b", "c"]);
        assert_eq!(x, 101);
    }

    #[test]
    fn stderr_header_and_check() {
        let e = parse_stderr_header("// stderr: a\n// stderr: b\n// stderr-has: zz\nfn main(){}");
        assert_eq!(e.exact, vec!["a", "b"]);
        assert!(e.check("a\nb\nzz here").is_some(), "exact lines must match exactly");
        let h = parse_stderr_header("// stderr-has: ] x = 1\n");
        assert!(h.check("[f:1:2] x = 1").is_none());
        assert!(h.check("nothing").is_some());
        assert!(parse_stderr_header("// stderr-empty\n").check("oops").is_some());
        assert!(!parse_stderr_header("fn main(){}").is_set());
    }
}
