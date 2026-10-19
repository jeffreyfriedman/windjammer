//! Final stage: anonymize (re-checking the oracle), format, write the output project and `repro.md`.

use super::anonymize::{apply, declared_words, plan, Key, Plan};
use super::engine::Engine;
use super::project::Project;
use crate::formatter::{format_source, FormatOptions};
use std::path::Path;

pub struct Anonymized {
    pub project: Project,
    pub renamed: usize,
    pub total: usize,
    /// Declared (user) names that could not be renamed without losing the bug.
    pub stuck_declared: usize,
}

/// Largest renamable subset that still reproduces the bug, found by bisection over plan items.
pub fn anonymize(eng: &Engine, p: &Project) -> Anonymized {
    let plan = plan(p);
    let n = plan.items.len();
    let mut enabled = vec![false; n];
    let all: Vec<usize> = (0..n).collect();
    let test = |en: &[bool]| eng.interesting(&apply(p, &plan, en));
    bisect(&all, &mut enabled, &test);
    let project = apply(p, &plan, &enabled);
    let declared = declared_words(p);
    let stuck = stuck_declared(&plan, &enabled, &declared);
    Anonymized {
        project,
        renamed: enabled.iter().filter(|b| **b).count(),
        total: n,
        stuck_declared: stuck,
    }
}

fn stuck_declared(
    plan: &Plan,
    enabled: &[bool],
    declared: &std::collections::HashSet<String>,
) -> usize {
    plan.items
        .iter()
        .zip(enabled)
        .filter(|((k, _), on)| {
            !**on
                && match k {
                    Key::Str(_) => true,
                    Key::Word(w) => declared.contains(w),
                }
        })
        .count()
}

fn bisect(group: &[usize], enabled: &mut Vec<bool>, test: &dyn Fn(&[bool]) -> bool) {
    if group.is_empty() {
        return;
    }
    for &i in group {
        enabled[i] = true;
    }
    if test(enabled) {
        return;
    }
    for &i in group {
        enabled[i] = false;
    }
    if group.len() == 1 {
        return;
    }
    let (a, b) = group.split_at(group.len() / 2);
    bisect(a, enabled, test);
    bisect(b, enabled, test);
}

/// Format every file (falls back to the raw text if the formatter refuses).
pub fn formatted(p: &Project) -> Project {
    let mut out = Project::default();
    for (k, v) in &p.files {
        let text = format_source(v, &FormatOptions::default()).unwrap_or_else(|_| v.clone());
        out.files.insert(k.clone(), text);
    }
    out
}

pub fn write_output(dir: &Path, p: &Project) -> std::io::Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    std::fs::create_dir_all(dir)?;
    p.write_to(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bisect_finds_the_maximal_enabled_set() {
        // items 3 and 6 cannot be enabled; the oracle fails whenever either is on
        let test = |en: &[bool]| !en[3] && !en[6];
        let mut en = vec![false; 10];
        bisect(&(0..10).collect::<Vec<_>>(), &mut en, &test);
        let off: Vec<usize> = (0..10).filter(|i| !en[*i]).collect();
        assert_eq!(off, [3, 6]);
    }
}
