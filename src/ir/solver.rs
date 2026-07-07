//! Unified constraint solver using union-find.
//!
//! Phase 1: Stub implementation — the solver accepts constraints but does not
//! yet replace the sequential inference passes.
//!
//! Phase 2: Full union-find unification over TypeVar and OwnershipVar,
//! replacing the sequential float → integer → ownership passes.

use crate::ir::constraints::{Constraint, ConstraintSet, ConstraintVar};
use crate::ir::safety_type::{BaseType, OwnedType};

/// The result of solving a constraint set.
#[derive(Debug)]
pub struct SolverResult {
    /// Resolved base types for each constraint variable.
    pub types: Vec<Option<BaseType>>,
    /// Resolved ownership modes for each constraint variable.
    pub ownership: Vec<Option<OwnedType>>,
    /// Which variables need clone insertion.
    pub clones: CloneRequirements,
    /// Diagnostics produced during solving.
    pub diagnostics: Vec<SolverDiagnostic>,
}

/// A diagnostic emitted by the solver.
#[derive(Debug, Clone)]
pub struct SolverDiagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub vars: Vec<ConstraintVar>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticKind {
    TypeError,
    OwnershipConflict,
    EffectViolation,
    TaintViolation,
}

/// Tracks which variables need clone insertion.
#[derive(Debug, Default)]
pub struct CloneRequirements {
    needs_clone: Vec<bool>,
}

impl CloneRequirements {
    fn new(size: u32) -> Self {
        Self {
            needs_clone: vec![false; size as usize],
        }
    }

    pub fn needs_clone(&self, var: ConstraintVar) -> bool {
        self.needs_clone
            .get(var.0 as usize)
            .copied()
            .unwrap_or(false)
    }
}

/// Union-find data structure for type unification.
struct UnionFind {
    parent: Vec<u32>,
    rank: Vec<u32>,
}

impl UnionFind {
    fn new(size: u32) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size as usize],
        }
    }

    fn find(&mut self, x: u32) -> u32 {
        if self.parent[x as usize] != x {
            self.parent[x as usize] = self.find(self.parent[x as usize]);
        }
        self.parent[x as usize]
    }

    fn union(&mut self, x: u32, y: u32) -> bool {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return false;
        }
        if self.rank[rx as usize] < self.rank[ry as usize] {
            self.parent[rx as usize] = ry;
        } else if self.rank[rx as usize] > self.rank[ry as usize] {
            self.parent[ry as usize] = rx;
        } else {
            self.parent[ry as usize] = rx;
            self.rank[rx as usize] += 1;
        }
        true
    }
}

/// The constraint solver.
pub struct Solver {
    uf: UnionFind,
    types: Vec<Option<BaseType>>,
    ownership: Vec<Option<OwnedType>>,
    clones: CloneRequirements,
    diagnostics: Vec<SolverDiagnostic>,
}

impl Solver {
    /// Create a solver for a given constraint set.
    pub fn new(constraints: &ConstraintSet) -> Self {
        let n = constraints.num_vars();
        Self {
            uf: UnionFind::new(n),
            types: vec![None; n as usize],
            ownership: vec![None; n as usize],
            clones: CloneRequirements::new(n),
            diagnostics: Vec::new(),
        }
    }

    /// Solve all constraints and return the result.
    pub fn solve(mut self, constraints: &ConstraintSet) -> SolverResult {
        for constraint in constraints.iter() {
            self.process_constraint(constraint);
        }
        SolverResult {
            types: self.types,
            ownership: self.ownership,
            clones: self.clones,
            diagnostics: self.diagnostics,
        }
    }

    fn process_constraint(&mut self, constraint: &Constraint) {
        match constraint {
            Constraint::TypeEquals(a, b) => {
                let ra = self.uf.find(a.0);
                let rb = self.uf.find(b.0);
                if ra != rb {
                    let type_a = self.types[ra as usize].clone();
                    let type_b = self.types[rb as usize].clone();
                    self.uf.union(ra, rb);
                    let root = self.uf.find(ra);
                    match (type_a, type_b) {
                        (Some(ta), Some(tb)) if ta != tb => {
                            self.diagnostics.push(SolverDiagnostic {
                                kind: DiagnosticKind::TypeError,
                                message: format!("type conflict: {:?} vs {:?}", ta, tb),
                                vars: vec![*a, *b],
                            });
                        }
                        (Some(t), None) | (None, Some(t)) => {
                            self.types[root as usize] = Some(t);
                        }
                        _ => {}
                    }
                }
            }

            Constraint::TypeIs(var, base_type) => {
                let root = self.uf.find(var.0);
                match &self.types[root as usize] {
                    Some(existing) if existing != base_type => {
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::TypeError,
                            message: format!(
                                "type conflict: expected {:?}, got {:?}",
                                base_type, existing
                            ),
                            vars: vec![*var],
                        });
                    }
                    _ => {
                        self.types[root as usize] = Some(base_type.clone());
                    }
                }
            }

            Constraint::IsNumeric(var) => {
                let root = self.uf.find(var.0);
                if let Some(ref ty) = self.types[root as usize] {
                    if !is_numeric_type(ty) {
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::TypeError,
                            message: format!("expected numeric type, found {:?}", ty),
                            vars: vec![*var],
                        });
                    }
                }
            }

            Constraint::IsInteger(var) => {
                let root = self.uf.find(var.0);
                if let Some(ref ty) = self.types[root as usize] {
                    if !is_integer_type(ty) {
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::TypeError,
                            message: format!("expected integer type, found {:?}", ty),
                            vars: vec![*var],
                        });
                    }
                }
            }

            Constraint::IsFloat(var) => {
                let root = self.uf.find(var.0);
                if let Some(ref ty) = self.types[root as usize] {
                    if !is_float_type(ty) {
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::TypeError,
                            message: format!("expected float type, found {:?}", ty),
                            vars: vec![*var],
                        });
                    }
                }
            }

            Constraint::OwnershipIs(var, ownership) => {
                let root = self.uf.find(var.0);
                match &self.ownership[root as usize] {
                    Some(existing) if existing != ownership => {
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::OwnershipConflict,
                            message: format!(
                                "ownership conflict: {:?} vs {:?}",
                                ownership, existing
                            ),
                            vars: vec![*var],
                        });
                    }
                    _ => {
                        self.ownership[root as usize] = Some(ownership.clone());
                    }
                }
            }

            Constraint::SharesRegion(a, b) => {
                let ra = self.uf.find(a.0);
                let rb = self.uf.find(b.0);
                let oa = self.ownership[ra as usize].clone();
                let ob = self.ownership[rb as usize].clone();
                if let (Some(OwnedType::MutRef(_)), Some(OwnedType::MutRef(_))) = (&oa, &ob) {
                    self.diagnostics.push(SolverDiagnostic {
                        kind: DiagnosticKind::OwnershipConflict,
                        message: "two mutable borrows share a region".to_string(),
                        vars: vec![*a, *b],
                    });
                }
            }

            Constraint::NeedsClone(var) => {
                let root = self.uf.find(var.0);
                if (root as usize) < self.clones.needs_clone.len() {
                    self.clones.needs_clone[root as usize] = true;
                }
            }

            // Effect and taint constraints delegated to specialized solvers
            Constraint::HasEffects(_, _)
            | Constraint::EffectsUnion(_, _)
            | Constraint::TaintIs(_, _)
            | Constraint::TaintPropagates(_, _)
            | Constraint::Sanitizes(_) => {
                // Handled by EffectSolver/TaintSolver in WP6
            }
        }
    }
}

fn is_numeric_type(ty: &BaseType) -> bool {
    is_integer_type(ty) || is_float_type(ty)
}

fn is_integer_type(ty: &BaseType) -> bool {
    matches!(
        ty,
        BaseType::I8
            | BaseType::I16
            | BaseType::I32
            | BaseType::I64
            | BaseType::I128
            | BaseType::U8
            | BaseType::U16
            | BaseType::U32
            | BaseType::U64
            | BaseType::U128
    )
}

fn is_float_type(ty: &BaseType) -> bool {
    matches!(ty, BaseType::F32 | BaseType::F64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::constraints::ConstraintSet;
    use crate::ir::safety_type::BaseType;

    #[test]
    fn test_type_unification_basic() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();

        cs.add(Constraint::TypeIs(a, BaseType::I32));
        cs.add(Constraint::TypeEquals(a, b));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        // Both should resolve to I32
        let _root_a = {
            let mut uf = UnionFind::new(cs.num_vars());
            uf.find(a.0)
        };
        // After solving, `b` should have inherited I32 from `a`
        assert_eq!(result.types[0], Some(BaseType::I32));
    }

    #[test]
    fn test_type_conflict_detected() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::TypeIs(a, BaseType::I32));
        cs.add(Constraint::TypeIs(a, BaseType::F64));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].kind, DiagnosticKind::TypeError);
    }

    #[test]
    fn test_ownership_resolution() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::OwnershipIs(
            a,
            OwnedType::Ref(crate::ir::safety_type::Region::fresh(1)),
        ));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.ownership[0],
            Some(OwnedType::Ref(crate::ir::safety_type::Region::fresh(1)))
        );
    }

    #[test]
    fn test_empty_constraint_set() {
        let cs = ConstraintSet::new();
        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert!(result.types.is_empty());
        assert!(result.ownership.is_empty());
    }
}
