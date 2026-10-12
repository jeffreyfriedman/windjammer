//! `wj check --fix`: apply machine-applicable fixes, refusing any result that no longer parses.

use super::front;
use crate::diagnostics::{apply, Diagnostic};
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct FixOutcome {
    pub files_changed: usize,
    pub edits_applied: usize,
    pub files_skipped: Vec<String>,
}

pub fn apply_fixes(diags: &[Diagnostic], sources: &HashMap<String, String>, write: bool) -> Result<FixOutcome> {
    let mut outcome = FixOutcome::default();
    for (file, edits) in apply::collect_edits(diags) {
        let Some(text) = sources.get(&file) else { continue };
        let Some(new) = apply::apply_edits(text, &edits) else {
            outcome.files_skipped.push(file);
            continue;
        };
        if new == *text {
            continue;
        }
        if front::parse(&file, &new, &mut Vec::new()).is_none() {
            outcome.files_skipped.push(file);
            continue;
        }
        if write {
            std::fs::write(&file, &new)?;
        }
        outcome.files_changed += 1;
        outcome.edits_applied += edits.len();
    }
    Ok(outcome)
}
