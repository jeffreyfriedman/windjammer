//! Per-function parameter, local and clone reports.
//!
//! The reasons come from constraint provenance (`ConstraintOrigin`) recorded by
//! `constraint_gen`. A constraint is *binding* when, alone among the ownership constraints,
//! it reaches the parameter in the solver with the winning mode. This reads the same
//! constraint set the pipeline solves; it never feeds a decision back.

use super::emitted::EmittedFn;
use super::report::{CloneReport, Finding, LocalReport, ParamReport, Reason};
use crate::analyzer::{AnalyzedFunction, OwnershipMode};
use crate::ir::constraint_gen::FunctionConstraints;
use crate::ir::constraints::{Constraint, ConstraintOrigin, ConstraintSet, ConstraintVar};
use crate::ir::node::IrFunction;
use crate::ir::safety_type::{BaseType, OwnedType};
use crate::ir::solver::Solver;
use crate::parser::OwnershipHint;

pub fn owned_label(o: &OwnedType) -> Option<&'static str> {
    match o {
        OwnedType::Owned | OwnedType::Copy => Some("owned"),
        OwnedType::Ref(_) => Some("&"),
        OwnedType::MutRef(_) => Some("&mut"),
        OwnedType::Inferred => None,
    }
}

pub fn mode_label(m: OwnershipMode) -> &'static str {
    match m {
        OwnershipMode::Owned => "owned",
        OwnershipMode::Borrowed => "&",
        OwnershipMode::MutBorrowed => "&mut",
    }
}

pub fn width_label(b: &BaseType) -> Option<String> {
    Some(match b {
        BaseType::Inferred => return None,
        BaseType::I8 => "i8".into(),
        BaseType::I16 => "i16".into(),
        BaseType::I32 => "i32".into(),
        BaseType::I64 => "i64".into(),
        BaseType::I128 => "i128".into(),
        BaseType::U8 => "u8".into(),
        BaseType::U16 => "u16".into(),
        BaseType::U32 => "u32".into(),
        BaseType::U64 => "u64".into(),
        BaseType::U128 => "u128".into(),
        BaseType::F32 => "f32".into(),
        BaseType::F64 => "f64".into(),
        other => format!("{:?}", other),
    })
}

/// Equivalence classes induced by `TypeEquals`, mirroring the solver's union-find.
struct Classes {
    parent: Vec<u32>,
}

impl Classes {
    fn new(cs: &ConstraintSet) -> Self {
        let mut c = Classes {
            parent: (0..cs.num_vars()).collect(),
        };
        for k in cs.iter() {
            if let Constraint::TypeEquals(a, b) = k {
                let (ra, rb) = (c.find(a.0), c.find(b.0));
                if ra != rb {
                    c.parent[ra as usize] = rb;
                }
            }
        }
        c
    }
    fn find(&mut self, x: u32) -> u32 {
        let mut r = x;
        while self.parent[r as usize] != r {
            r = self.parent[r as usize];
        }
        r
    }
}

/// Variables whose ownership requirement reaches `var`: itself plus every position it is
/// (transitively) `UsedAs`. Mirrors the solver's flow propagation.
fn requirement_sources(cs: &ConstraintSet, var: ConstraintVar) -> std::collections::BTreeSet<u32> {
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![var.0];
    while let Some(v) = stack.pop() {
        if !seen.insert(v) {
            continue;
        }
        for c in cs.iter() {
            if let Constraint::UsedAs(value, site) = c {
                if value.0 == v {
                    stack.push(site.0);
                }
            }
        }
    }
    seen
}

/// Ownership of `var` as the pipeline consumes it (`result.ownership[var]`).
fn solver_mode(cs: &ConstraintSet, var: ConstraintVar) -> Option<&'static str> {
    let result = Solver::new(cs).solve(cs);
    result
        .ownership
        .get(var.0 as usize)
        .and_then(|o| o.as_ref())
        .and_then(owned_label)
}

fn kind_label(c: &Constraint) -> Option<(&'static str, ConstraintVar)> {
    match c {
        Constraint::OwnershipIs(v, o) => owned_label(o).map(|l| (l, *v)),
        _ => None,
    }
}

fn reason_from(o: Option<&ConstraintOrigin>, fallback: &str) -> (String, String, Option<usize>, Option<usize>) {
    match o {
        Some(o) => (
            o.rule.to_string(),
            o.detail.clone(),
            o.at.map(|a| a.0),
            o.at.map(|a| a.1),
        ),
        None => ("UNTAGGED".into(), fallback.into(), None, None),
    }
}

pub fn param_reports(
    af: &AnalyzedFunction<'_>,
    fc: &FunctionConstraints,
    ir_fn: &IrFunction,
    emitted: Option<&EmittedFn>,
) -> Vec<ParamReport> {
    let mut out = Vec::new();
    for param in &af.decl.parameters {
        let name = &param.name;
        let var = fc.var_map.params.get(name).copied();
        let sources = var.map(|v| requirement_sources(&fc.constraints, v));
        let analyzer = af.inferred_ownership.get(name).map(|m| mode_label(*m).to_string());
        let solver = var.and_then(|v| solver_mode(&fc.constraints, v)).map(String::from);
        let ir_final = ir_fn
            .param_types
            .get(name)
            .and_then(|st| owned_label(&st.ownership))
            .map(String::from);
        let emitted_mode = emitted
            .and_then(|e| e.params.iter().find(|p| &p.name == name))
            .map(|p| p.mode.label().to_string());
        let trail: Vec<String> = ir_fn
            .ownership_trail
            .get(name)
            .map(|t| {
                t.iter()
                    .map(|(stage, o)| format!("{}={}", stage, owned_label(o).unwrap_or("-")))
                    .collect()
            })
            .unwrap_or_default();
        // IR passes that overrode the solver (C5 deletion targets).
        let overriders: Vec<&str> = ir_fn
            .ownership_trail
            .get(name)
            .map(|t| {
                t.iter()
                    .skip_while(|(stage, _)| *stage != "solver")
                    .skip(1)
                    .map(|(stage, _)| *stage)
                    .collect()
            })
            .unwrap_or_default();
        let pinned = match param.ownership {
            OwnershipHint::Ref => Some("&".to_string()),
            OwnershipHint::Mut => Some("&mut".to_string()),
            _ => None,
        };
        let effective = emitted_mode
            .clone()
            .or_else(|| ir_final.clone())
            .unwrap_or_else(|| "unknown".into());

        // Reasons: every ownership constraint on the parameter or on a position it flows to.
        let mut reasons = Vec::new();
        if let (Some(v), Some(sources)) = (var, sources.as_ref()) {
            for (i, (c, o)) in fc.constraints.iter_with_origin().enumerate() {
                let Some((label, cvar)) = kind_label(c) else { continue };
                if !sources.contains(&cvar.0) {
                    continue;
                }
                // Does this constraint, alone among ownership constraints, reach the parameter?
                let only_this = fc.constraints.filtered(|j, k| {
                    !matches!(k, Constraint::OwnershipIs(..)) || j == i
                });
                let reaches = solver_mode(&only_this, v) == Some(label);
                let (rule, detail, line, col) = reason_from(o, "untagged ownership constraint");
                reasons.push(Reason {
                    rule,
                    detail,
                    line,
                    col,
                    binding: reaches && solver.as_deref() == Some(label),
                    dropped: !reaches,
                });
            }
        }

        let mut findings = Vec::new();
        if reasons.iter().any(|r| r.dropped) {
            findings.push(Finding {
                id: "D-SOLVER-UNION".into(),
                message: format!(
                    "{} ownership constraint(s) on `{}` never reach the solver result",
                    reasons.iter().filter(|r| r.dropped).count(),
                    name
                ),
            });
        }
        let stages: Vec<(&str, &Option<String>)> = vec![
            ("analyzer", &analyzer),
            ("solver", &solver),
            ("ir", &ir_final),
            ("emitted", &emitted_mode),
        ];
        let distinct: std::collections::BTreeSet<&str> =
            stages.iter().filter_map(|(_, m)| m.as_deref()).collect();
        if distinct.len() > 1 {
            let chain = stages
                .iter()
                .map(|(s, m)| format!("{}={}", s, m.as_deref().unwrap_or("-")))
                .collect::<Vec<_>>()
                .join(" ");
            let by = if overriders.is_empty() {
                String::new()
            } else {
                format!(" (overridden after the solver by {})", overriders.join(", "))
            };
            findings.push(Finding {
                id: "D-OWN-STAGES".into(),
                message: format!("decision changed between stages: {}{}", chain, by),
            });
        }
        if let Some(pin) = &pinned {
            if emitted_mode.as_deref().is_some_and(|m| m != pin) || (emitted_mode.is_none() && ir_final.as_deref().is_some_and(|m| m != pin)) {
                findings.push(Finding {
                    id: "D-PIN-NOT-HONORED".into(),
                    message: format!(
                        "`{}` is annotated `{}` but the final mode is {}",
                        name, pin, effective
                    ),
                });
            }
        }
        if reasons.is_empty() && effective == "&" {
            reasons.push(Reason {
                rule: "OWN-READONLY".into(),
                detail: format!("no use of `{}` requires more than a shared borrow", name),
                line: None,
                col: None,
                binding: true,
                dropped: false,
            });
        }
        out.push(ParamReport {
            name: name.clone(),
            declared: crate::type_display::format_wj_type(&param.type_),
            pinned,
            analyzer,
            solver,
            ir_final,
            emitted: emitted_mode,
            trail,
            effective,
            reasons,
            findings,
        });
    }
    out
}

pub fn local_reports(
    fc: &FunctionConstraints,
    ir_fn: &IrFunction,
    emitted: Option<&EmittedFn>,
) -> Vec<LocalReport> {
    let mut classes = Classes::new(&fc.constraints);
    let mut names: Vec<&String> = fc.var_map.locals.keys().collect();
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let var = fc.var_map.locals[name];
        let class = classes.find(var.0);
        let solver_width = ir_fn
            .local_types
            .get(name)
            .and_then(|st| width_label(&st.base));
        let el = emitted.and_then(|e| e.lets.iter().rev().find(|l| &l.name == name));
        let emitted_width = el.and_then(|l| l.width.clone());
        let mut reasons = Vec::new();
        for (c, o) in fc.constraints.iter_with_origin() {
            if let Constraint::TypeIs(v, b) = c {
                if classes.find(v.0) == class {
                    let (rule, detail, line, col) = reason_from(o, "untagged type constraint");
                    reasons.push(Reason {
                        rule,
                        detail,
                        line,
                        col,
                        binding: width_label(b) == solver_width,
                        dropped: false,
                    });
                }
            }
        }
        let mut findings = Vec::new();
        if let (Some(s), Some(e)) = (&solver_width, &emitted_width) {
            if s != e {
                findings.push(Finding {
                    id: "D-WIDTH-OWNERS".into(),
                    message: format!(
                        "solver resolved `{}` as {} but the emitted program uses {} (width decided outside the solver)",
                        name, s, e
                    ),
                });
            }
        }
        out.push(LocalReport {
            name: name.clone(),
            solver_width,
            emitted_width,
            mutable_in_emitted: el.map(|l| l.mutable),
            reasons,
            findings,
        });
    }
    out
}

pub fn clone_reports(
    fc: &FunctionConstraints,
    af: &AnalyzedFunction<'_>,
    emitted: Option<&EmittedFn>,
) -> Vec<CloneReport> {
    let mut names: Vec<String> = fc.var_map.params.keys().cloned().collect();
    names.extend(fc.var_map.locals.keys().cloned());
    names.sort();
    names.dedup();
    let mut out = Vec::new();
    for name in names {
        let var = fc
            .var_map
            .params
            .get(&name)
            .or_else(|| fc.var_map.locals.get(&name))
            .copied();
        let Some(var) = var else { continue };
        let mut reasons = Vec::new();
        for (c, o) in fc.constraints.iter_with_origin() {
            if let Constraint::NeedsClone(v) = c {
                if *v == var {
                    let (rule, detail, line, col) = reason_from(o, "untagged clone constraint");
                    reasons.push(Reason {
                        rule,
                        detail,
                        line,
                        col,
                        binding: true,
                        dropped: false,
                    });
                }
            }
        }
        let emitted_clones = emitted.and_then(|e| e.clones.get(&name)).copied().unwrap_or(0);
        if reasons.is_empty() && emitted_clones == 0 {
            continue;
        }
        let solver_needs_clone = !reasons.is_empty();
        let mut findings = Vec::new();
        if emitted_clones > 0 && !solver_needs_clone {
            findings.push(Finding {
                id: "D-CLONE-OUTSIDE-SOLVER".into(),
                message: format!(
                    "`{}` is cloned {}x in the emitted program but no solver clone constraint exists (the clone decision was made in codegen)",
                    name, emitted_clones
                ),
            });
        }
        let _ = af;
        out.push(CloneReport {
            var: name,
            solver_needs_clone,
            emitted_clones,
            reasons,
            findings,
        });
    }
    out
}
