//! Unified constraint solver using union-find.
//!
//! Phase 2: Full union-find unification over TypeVar, OwnershipVar, and Region,
//! replacing the sequential float → integer → ownership passes.
//!
//! Key improvements in Phase 1B:
//!   - IsNumeric/IsInteger/IsFloat **infer** (not just validate) numeric class
//!   - SharesRegion performs real region unification via a region union-find
//!   - NumericSolver results can be imported via `import_numeric_results`

use crate::ir::constraints::{Constraint, ConstraintSet, ConstraintVar};
#[cfg(test)]
use crate::ir::safety_type::Region;
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

/// Numeric class constraint for deferred inference.
#[derive(Debug, Clone, Copy, PartialEq)]
enum NumericClass {
    /// Must be some numeric type (int or float).
    Numeric,
    /// Must be an integer type.
    Integer,
    /// Must be a float type.
    Float,
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

/// Region union-find for aliasing analysis.
struct RegionUnionFind {
    parent: Vec<u32>,
    rank: Vec<u32>,
    next_id: u32,
}

impl RegionUnionFind {
    fn new() -> Self {
        Self {
            parent: Vec::new(),
            rank: Vec::new(),
            next_id: 0,
        }
    }

    fn get_or_create(&mut self, region_id: u32) -> u32 {
        while self.parent.len() <= region_id as usize {
            let id = self.next_id;
            self.parent.push(id);
            self.rank.push(0);
            self.next_id += 1;
        }
        region_id
    }

    fn find(&mut self, x: u32) -> u32 {
        if x as usize >= self.parent.len() {
            return x;
        }
        if self.parent[x as usize] != x {
            self.parent[x as usize] = self.find(self.parent[x as usize]);
        }
        self.parent[x as usize]
    }

    fn union(&mut self, x: u32, y: u32) {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return;
        }
        self.get_or_create(rx.max(ry));
        if self.rank[rx as usize] < self.rank[ry as usize] {
            self.parent[rx as usize] = ry;
        } else if self.rank[rx as usize] > self.rank[ry as usize] {
            self.parent[ry as usize] = rx;
        } else {
            self.parent[ry as usize] = rx;
            self.rank[rx as usize] += 1;
        }
    }
}

/// The constraint solver.
pub struct Solver {
    uf: UnionFind,
    types: Vec<Option<BaseType>>,
    ownership: Vec<Option<OwnedType>>,
    clones: CloneRequirements,
    diagnostics: Vec<SolverDiagnostic>,
    /// Deferred numeric class constraints — resolved after all other constraints.
    numeric_classes: Vec<(ConstraintVar, NumericClass)>,
    /// Region union-find for aliasing analysis.
    region_uf: RegionUnionFind,
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
            numeric_classes: Vec::new(),
            region_uf: RegionUnionFind::new(),
        }
    }

    /// Solve all constraints and return the result.
    pub fn solve(mut self, constraints: &ConstraintSet) -> SolverResult {
        // Pass 1: Process all constraints
        for constraint in constraints.iter() {
            self.process_constraint(constraint);
        }

        // Pass 2: Resolve deferred numeric class constraints.
        // For vars that have no type yet but are constrained to be numeric,
        // infer a default type.
        self.resolve_numeric_classes();

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
                } else {
                    self.numeric_classes.push((*var, NumericClass::Numeric));
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
                } else {
                    self.numeric_classes.push((*var, NumericClass::Integer));
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
                } else {
                    self.numeric_classes.push((*var, NumericClass::Float));
                }
            }

            Constraint::OwnershipIs(var, ownership) => {
                let root = self.uf.find(var.0);
                match &self.ownership[root as usize] {
                    Some(existing) if existing != ownership => {
                        // Formal lattice: Owned > MutRef > Ref.
                        // Owned wins over MutRef so returned/moved params stay by-value
                        // even when the body also mutates (constraint_gen seeds both).
                        // MutRef wins over Ref (mutation / `&mut self` method).
                        match (existing, ownership) {
                            (OwnedType::Ref(_), OwnedType::MutRef(r)) => {
                                self.ownership[root as usize] = Some(OwnedType::MutRef(r.clone()));
                            }
                            (OwnedType::MutRef(_), OwnedType::Ref(_)) => {
                                // Already MutRef, keep it
                            }
                            (OwnedType::MutRef(_), OwnedType::Owned)
                            | (OwnedType::Ref(_), OwnedType::Owned) => {
                                // Move/return requirement beats borrow.
                                self.ownership[root as usize] = Some(OwnedType::Owned);
                            }
                            (OwnedType::Owned, OwnedType::MutRef(_))
                            | (OwnedType::Owned, OwnedType::Ref(_)) => {
                                // Formal stays owned; mutation/read does not demote to &mut/&T.
                            }
                            _ => {
                                self.diagnostics.push(SolverDiagnostic {
                                    kind: DiagnosticKind::OwnershipConflict,
                                    message: format!(
                                        "ownership conflict: {:?} vs {:?}",
                                        ownership, existing
                                    ),
                                    vars: vec![*var],
                                });
                            }
                        }
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

                // Unify regions if both have region-based ownership
                match (&oa, &ob) {
                    (Some(OwnedType::Ref(r1)), Some(OwnedType::Ref(r2)))
                    | (Some(OwnedType::Ref(r1)), Some(OwnedType::MutRef(r2)))
                    | (Some(OwnedType::MutRef(r1)), Some(OwnedType::Ref(r2))) => {
                        let rid1 = self.region_uf.get_or_create(r1.0);
                        let rid2 = self.region_uf.get_or_create(r2.0);
                        self.region_uf.union(rid1, rid2);
                    }
                    (Some(OwnedType::MutRef(r1)), Some(OwnedType::MutRef(r2))) => {
                        let rid1 = self.region_uf.get_or_create(r1.0);
                        let rid2 = self.region_uf.get_or_create(r2.0);
                        // Unify regions
                        self.region_uf.union(rid1, rid2);
                        // Two mutable borrows to the same region = error
                        self.diagnostics.push(SolverDiagnostic {
                            kind: DiagnosticKind::OwnershipConflict,
                            message: "two mutable borrows share a region".to_string(),
                            vars: vec![*a, *b],
                        });
                    }
                    _ => {}
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
                // Handled by EffectSolver/TaintSolver in pipeline
            }
        }
    }

    /// Resolve deferred numeric class constraints by inferring defaults
    /// for variables that have no explicit type yet.
    fn resolve_numeric_classes(&mut self) {
        // Collect the strongest numeric class per root variable
        let mut root_classes: std::collections::HashMap<u32, NumericClass> =
            std::collections::HashMap::new();

        for &(var, class) in &self.numeric_classes {
            let root = self.uf.find(var.0);
            // If the var already has a resolved type, skip
            if self.types[root as usize].is_some() {
                continue;
            }
            let entry = root_classes.entry(root).or_insert(class);
            // More specific class wins: Integer/Float beats Numeric
            match (*entry, class) {
                (NumericClass::Numeric, NumericClass::Integer) => *entry = NumericClass::Integer,
                (NumericClass::Numeric, NumericClass::Float) => *entry = NumericClass::Float,
                (NumericClass::Integer, NumericClass::Float)
                | (NumericClass::Float, NumericClass::Integer) => {
                    // Conflicting: both Integer and Float on same var
                    // This is rare but possible (e.g., generic numeric code)
                    // Default to the first encountered
                }
                _ => {}
            }
        }

        for (root, class) in root_classes {
            if self.types[root as usize].is_none() {
                let default = match class {
                    NumericClass::Integer => BaseType::I32,
                    NumericClass::Float => BaseType::F64,
                    NumericClass::Numeric => BaseType::I32,
                };
                self.types[root as usize] = Some(default);
            }
        }
    }

    /// Import results from the NumericSolver, mapping ConstraintVars to BaseTypes.
    ///
    /// This bridges the specialized numeric inference engine into the unified solver.
    /// Call after `solve()` on the numeric solver, before `solve()` on this solver.
    pub fn import_numeric_type(
        &mut self,
        var: ConstraintVar,
        numeric_type: &crate::ir::numeric_types::NumericType,
    ) {
        if let Some(base) = numeric_type_to_base(numeric_type) {
            let root = self.uf.find(var.0);
            if self.types[root as usize].is_none() {
                self.types[root as usize] = Some(base);
            }
        }
    }
}

/// Convert a NumericType to a BaseType.
fn numeric_type_to_base(nt: &crate::ir::numeric_types::NumericType) -> Option<BaseType> {
    use crate::ir::numeric_types::NumericType;
    match nt {
        NumericType::I8 => Some(BaseType::I8),
        NumericType::I16 => Some(BaseType::I16),
        NumericType::I32 => Some(BaseType::I32),
        NumericType::I64 => Some(BaseType::I64),
        NumericType::I128 => Some(BaseType::I128),
        NumericType::U8 => Some(BaseType::U8),
        NumericType::U16 => Some(BaseType::U16),
        NumericType::U32 => Some(BaseType::U32),
        NumericType::U64 => Some(BaseType::U64),
        NumericType::U128 => Some(BaseType::U128),
        NumericType::Usize => Some(BaseType::U64),
        NumericType::Isize => Some(BaseType::I64),
        NumericType::F32 => Some(BaseType::F32),
        NumericType::F64 => Some(BaseType::F64),
        NumericType::Unknown => None,
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

        cs.add(Constraint::OwnershipIs(a, OwnedType::Ref(Region::fresh(1))));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(result.ownership[0], Some(OwnedType::Ref(Region::fresh(1))));
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

    // --- Phase 1B: Numeric inference tests ---

    #[test]
    fn test_is_integer_infers_default_type() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        // Only say it must be integer, no explicit type
        cs.add(Constraint::IsInteger(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::I32),
            "IsInteger should infer I32 by default"
        );
    }

    #[test]
    fn test_is_float_infers_default_type() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::IsFloat(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::F64),
            "IsFloat should infer F64 by default"
        );
    }

    #[test]
    fn test_is_numeric_infers_i32_default() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::IsNumeric(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::I32),
            "IsNumeric should default to I32"
        );
    }

    #[test]
    fn test_is_integer_respects_explicit_type() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::TypeIs(a, BaseType::U64));
        cs.add(Constraint::IsInteger(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::U64),
            "explicit U64 should be kept, not overridden to I32"
        );
    }

    #[test]
    fn test_is_numeric_with_specific_constraint_wins() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::IsNumeric(a));
        cs.add(Constraint::IsFloat(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::F64),
            "IsFloat is more specific than IsNumeric, should infer F64"
        );
    }

    // --- Phase 1B: Region unification tests ---

    #[test]
    fn test_shares_region_unifies_refs() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();

        cs.add(Constraint::OwnershipIs(a, OwnedType::Ref(Region::fresh(1))));
        cs.add(Constraint::OwnershipIs(b, OwnedType::Ref(Region::fresh(2))));
        cs.add(Constraint::SharesRegion(a, b));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        // Shared refs to the same region: no error
        assert!(
            result.diagnostics.is_empty(),
            "two shared refs to same region should not conflict"
        );
    }

    #[test]
    fn test_shares_region_mut_refs_conflict() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();

        cs.add(Constraint::OwnershipIs(
            a,
            OwnedType::MutRef(Region::fresh(1)),
        ));
        cs.add(Constraint::OwnershipIs(
            b,
            OwnedType::MutRef(Region::fresh(2)),
        ));
        cs.add(Constraint::SharesRegion(a, b));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.kind == DiagnosticKind::OwnershipConflict),
            "two mutable refs to same region should conflict"
        );
    }

    #[test]
    fn test_ownership_owned_beats_mutref_for_returned_params() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        // Body mutates (MutRef) then returns the param (Owned) — formal stays Owned.
        cs.add(Constraint::OwnershipIs(
            a,
            OwnedType::MutRef(Region::fresh(1)),
        ));
        cs.add(Constraint::OwnershipIs(a, OwnedType::Owned));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(
            result.diagnostics.is_empty(),
            "MutRef then Owned should not conflict: {:?}",
            result.diagnostics
        );
        assert!(
            matches!(result.ownership[0], Some(OwnedType::Owned)),
            "returned/moved Owned must beat MutRef, got {:?}",
            result.ownership[0]
        );
    }

    #[test]
    fn test_ownership_owned_seed_not_demoted_by_mutref() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::OwnershipIs(a, OwnedType::Owned));
        cs.add(Constraint::OwnershipIs(
            a,
            OwnedType::MutRef(Region::fresh(1)),
        ));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(
            result.diagnostics.is_empty(),
            "Owned then MutRef should not conflict: {:?}",
            result.diagnostics
        );
        assert!(
            matches!(result.ownership[0], Some(OwnedType::Owned)),
            "Owned formal must not demote to MutRef, got {:?}",
            result.ownership[0]
        );
    }

    #[test]
    fn test_ownership_ref_then_mutref_upgrades() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::OwnershipIs(a, OwnedType::Ref(Region::fresh(1))));
        cs.add(Constraint::OwnershipIs(
            a,
            OwnedType::MutRef(Region::fresh(2)),
        ));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        // MutRef should win over Ref (mutation detected after initial read-only inference)
        assert!(
            result.diagnostics.is_empty(),
            "Ref → MutRef upgrade should not produce a conflict"
        );
        assert!(
            matches!(result.ownership[0], Some(OwnedType::MutRef(_))),
            "should be upgraded to MutRef"
        );
    }

    // --- Phase 1B: Numeric bridge integration test ---

    #[test]
    fn test_import_numeric_type() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();

        cs.add(Constraint::TypeEquals(a, b));

        let mut solver = Solver::new(&cs);
        solver.import_numeric_type(a, &crate::ir::numeric_types::NumericType::U32);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(result.types[0], Some(BaseType::U32));
    }
}
