//! Solver types of locals, published per binding: `IrFunction::local_types` by name for a name bound once (or bound
//! several times with one type), and `IrFunction::use_local_types` for each read of a name whose bindings disagree
//! (`for t in words { .. }` then `let t: int = ..`), so a read answers for the binding it names.

use super::constraint_gen::FunctionConstraints;
use super::safety_type::{BaseType, OwnedType, SafetyType};
use super::solver::SolverResult;
use super::IrFunction;
use std::collections::{BTreeMap, BTreeSet};

/// Classes in which two exact integer widths met (the solver could not decide them).
fn width_conflict_classes(result: &SolverResult) -> BTreeSet<u32> {
    result.width_conflicts.iter().filter_map(|v| result.class_of.get(v.0 as usize).copied()).collect()
}

/// A local's solved type, except an integer width of a class two exact widths met: which width the union kept
/// depends on constraint order, so the class stays with the legacy deciders (as its literals and name do).
fn solved_local_base<'r>(result: &'r SolverResult, conflicted: &BTreeSet<u32>, var: usize) -> Option<&'r BaseType> {
    let undecided = result.class_of.get(var).is_some_and(|c| conflicted.contains(c));
    result.types.get(var).and_then(|t| t.as_ref()).filter(|b| !(undecided && crate::ir::solver::is_integer_type(b)))
}

/// The solved type of one binding variable (`None` when the solver knows neither base nor mode).
fn solved(result: &SolverResult, conflicted: &BTreeSet<u32>, var: usize) -> Option<SafetyType> {
    let base = solved_local_base(result, conflicted, var).filter(|b| **b != BaseType::Inferred);
    // A local's mode is what it holds (join over its type class), not a requirement.
    let own = result.class_ownership.get(var).and_then(|o| o.as_ref()).filter(|o| **o != OwnedType::Inferred);
    if base.is_none() && own.is_none() {
        return None;
    }
    let mut t = SafetyType::owned(BaseType::Inferred);
    if let Some(b) = base {
        t.base = b.clone();
    }
    if let Some(o) = own {
        t.ownership = o.clone();
    }
    Some(t)
}

/// Write the solved local types of `fc` into `ir_fn`.
pub(crate) fn publish(fc: &FunctionConstraints, result: &SolverResult, ir_fn: &mut IrFunction) {
    let conflicted = width_conflict_classes(result);
    let mut by_name: BTreeMap<&str, Vec<Option<SafetyType>>> = BTreeMap::new();
    for (name, var) in &fc.binding_vars {
        by_name.entry(name.as_str()).or_default().push(solved(result, &conflicted, var.0 as usize));
    }
    // Bindings agree when their base and ownership kind agree (regions are per binding).
    let kind = |t: &Option<SafetyType>| t.as_ref().map(|t| (t.base.clone(), std::mem::discriminant(&t.ownership)));
    let ambiguous: BTreeSet<&str> =
        by_name.iter().filter(|(_, ts)| ts.windows(2).any(|w| kind(&w[0]) != kind(&w[1]))).map(|(n, _)| *n).collect();
    for (local_name, &var) in &fc.var_map.locals {
        if ambiguous.contains(local_name.as_str()) {
            continue;
        }
        let Some(t) = solved(result, &conflicted, var.0 as usize) else { continue };
        let entry = ir_fn.local_types.entry(local_name.clone()).or_insert_with(|| SafetyType::owned(BaseType::Inferred));
        if t.base != BaseType::Inferred {
            entry.base = t.base;
        }
        if t.ownership != OwnedType::Inferred {
            entry.ownership = t.ownership;
        }
    }
    if ambiguous.is_empty() {
        return;
    }
    // Every read of an ambiguous name gets an entry (unsolved: `Inferred`), so no reader falls back to a lookup by name.
    let names: BTreeMap<u32, &str> = fc.binding_vars.iter().map(|(n, v)| (v.0, n.as_str())).collect();
    for (key, var) in &fc.ident_uses {
        if names.get(&var.0).is_some_and(|n| ambiguous.contains(n)) {
            let unknown = || SafetyType { ownership: OwnedType::Inferred, ..SafetyType::owned(BaseType::Inferred) };
            let t = solved(result, &conflicted, var.0 as usize).unwrap_or_else(unknown);
            ir_fn.use_local_types.insert(*key, t);
        }
    }
}
