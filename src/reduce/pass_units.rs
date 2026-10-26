//! Hierarchical delta debugging over bracket groups: files are processed top-down by nesting depth;
//! inside each group the units (items, impl members, statements, match arms, fields, parameters,
//! generic parameters, arguments) are minimised with ddmin.

use super::ddmin::ddmin;
use super::session::Session;
use super::toks::{groups, render, scan, units, without_ranges, Group};
use crate::formatter::Tok;
use std::ops::Range;

pub fn run(s: &mut Session) -> bool {
    let mut changed = false;
    for file in s.file_order() {
        changed |= reduce_file(s, &file);
    }
    changed
}

fn reduce_file(s: &mut Session, file: &str) -> bool {
    let mut changed = false;
    let mut depth = 0;
    loop {
        if s.eng.exhausted() {
            return changed;
        }
        let Some(text) = s.project.files.get(file) else {
            return changed;
        };
        let Some(mut toks) = scan(text) else {
            return changed;
        };
        let level: Vec<Group> = groups(&toks)
            .into_iter()
            .filter(|g| g.depth == depth)
            .collect();
        if level.is_empty() {
            return changed;
        }
        for g in level.iter().rev() {
            let us = units(&toks, g);
            if us.is_empty() {
                continue;
            }
            let before = toks.len();
            toks = reduce_units(s, file, toks, &us);
            changed |= toks.len() != before;
        }
        depth += 1;
    }
}

/// Minimise `us` (units of `base`) and return the reduced token vector (also adopted in `s`).
fn reduce_units(s: &mut Session, file: &str, base: Vec<Tok>, us: &[Range<usize>]) -> Vec<Tok> {
    let build = |p: &super::project::Project, removed: &[usize]| {
        let mut r: Vec<Range<usize>> = removed.iter().map(|&i| us[i].clone()).collect();
        r.sort_by_key(|x| x.start);
        p.with_file(file, render(&without_ranges(&base, &r)))
    };
    let keep = ddmin(s, us.len(), &build);
    let removed: Vec<Range<usize>> = (0..us.len())
        .filter(|i| !keep.contains(i))
        .map(|i| us[i].clone())
        .collect();
    without_ranges(&base, &removed)
}
