//! Data model of `wj explain-ownership`. Plain data only: nothing here decides anything.
//!
//! Every parameter carries the decision at each stage of the pipeline so that disagreement
//! between the "owners" (analyzer, solver, IR post-passes, codegen) is visible:
//! `analyzer` -> `solver` -> `ir_final` -> `emitted`.

use serde::Serialize;

/// A use site (or seed) that contributed a constraint, with its rule ID.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Reason {
    pub rule: String,
    pub detail: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
    /// True when the constraint reaches the parameter in the solver and has the winning mode.
    pub binding: bool,
    /// True when the constraint is in the parameter's equivalence class but the solver
    /// result for the parameter does not reflect it (see divergence D-SOLVER-UNION).
    pub dropped: bool,
}

/// A machine-readable note about a disagreement or an implementation-only decision.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    /// Divergence ID from `docs/spec/inference-divergences.md` (or a rule ID).
    pub id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParamReport {
    pub name: String,
    pub declared: String,
    /// `&` / `&mut` when the source annotates the parameter (a pin), else `None`.
    pub pinned: Option<String>,
    pub analyzer: Option<String>,
    pub solver: Option<String>,
    pub ir_final: Option<String>,
    pub emitted: Option<String>,
    /// IR stages that changed the mode, in order (`stage=mode`), from `IrFunction::ownership_trail`.
    pub trail: Vec<String>,
    /// The mode the generated program actually has (`emitted`, else `ir_final`).
    pub effective: String,
    pub reasons: Vec<Reason>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalReport {
    pub name: String,
    /// Width the IR solver resolved, e.g. `i32`.
    pub solver_width: Option<String>,
    /// Width visible in the emitted Rust (annotation or literal suffix), e.g. `i64`.
    pub emitted_width: Option<String>,
    pub mutable_in_emitted: Option<bool>,
    pub reasons: Vec<Reason>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CloneReport {
    pub var: String,
    pub solver_needs_clone: bool,
    pub emitted_clones: usize,
    pub reasons: Vec<Reason>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallArgReport {
    pub index: usize,
    /// How the argument is passed in the emitted Rust: `borrow`, `borrow-mut`, `clone`,
    /// `move`, `to_string`, or `expr`.
    pub form: String,
    /// The root variable of the argument, when it is a plain variable.
    pub var: Option<String>,
    /// Callee formal mode in the emitted Rust (`owned`, `&`, `&mut`), if the callee is in this file.
    pub formal: Option<String>,
    pub used_later: Option<bool>,
    pub rule: Option<String>,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CallReport {
    pub callee: String,
    pub args: Vec<CallArgReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionReport {
    pub name: String,
    pub params: Vec<ParamReport>,
    pub locals: Vec<LocalReport>,
    pub clones: Vec<CloneReport>,
    pub calls: Vec<CallReport>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub file: String,
    /// True when emitted Rust was available, so `emitted` columns are populated.
    pub has_emitted: bool,
    pub functions: Vec<FunctionReport>,
    pub notes: Vec<String>,
}

impl Report {
    /// All findings in the report, for summaries and tests.
    pub fn findings(&self) -> Vec<&Finding> {
        let mut out = Vec::new();
        for f in &self.functions {
            for p in &f.params {
                out.extend(p.findings.iter());
            }
            for l in &f.locals {
                out.extend(l.findings.iter());
            }
            for c in &f.clones {
                out.extend(c.findings.iter());
            }
            for c in &f.calls {
                for a in &c.args {
                    out.extend(a.findings.iter());
                }
            }
        }
        out
    }
}
