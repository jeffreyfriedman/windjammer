//! Unified constraint solver using union-find.
//!
//! Phase 2: Full union-find unification over TypeVar, OwnershipVar, and Region,
//! replacing the sequential float → integer → ownership passes.
//!
//! Key improvements in Phase 1B:
//!   - IsNumeric/IsInteger/IsFloat **infer** (not just validate) numeric class
//!   - SharesRegion performs real region unification via a region union-find

use crate::ir::constraints::{Constraint, ConstraintSet, ConstraintVar};
#[cfg(test)]
use crate::ir::safety_type::Region;
use crate::ir::safety_type::{BaseType, OwnedType};

/// The result of solving a constraint set.
#[derive(Debug)]
pub struct SolverResult {
    /// Resolved base types for each constraint variable.
    pub types: Vec<Option<BaseType>>,
    /// Ownership requirement of each constraint variable (indexed by variable): the lattice join of
    /// its own `OwnershipIs` constraints and of every position it is `UsedAs`.
    pub ownership: Vec<Option<OwnedType>>,
    /// Join of the requirements over each variable's type class (what a local *holds*:
    /// `let x = make()` holds what `make` returns). Indexed by variable.
    pub class_ownership: Vec<Option<OwnedType>>,
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
    /// `UsedAs(value, site)` edges: `value` must satisfy `site`'s ownership requirement.
    flows: Vec<(ConstraintVar, ConstraintVar)>,
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
            flows: Vec::new(),
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

        // Pass 3: a value used at a position inherits that position's requirement
        // (OWN-CALL-FORMAL, OWN-RETURNED through `let` aliases). Monotone over a finite lattice.
        self.propagate_flows();

        // Results are indexed by variable, never by union-find root (D-SOLVER-UNION).
        let n = self.types.len() as u32;
        let types: Vec<Option<BaseType>> = (0..n)
            .map(|v| {
                let r = self.uf.find(v);
                self.types[r as usize].clone()
            })
            .collect();
        let mut class_join: std::collections::HashMap<u32, OwnedType> = std::collections::HashMap::new();
        for v in 0..n {
            if let Some(o) = self.ownership[v as usize].clone() {
                let r = self.uf.find(v);
                let joined = match class_join.remove(&r) {
                    Some(e) => join_ownership(e, o),
                    None => o,
                };
                class_join.insert(r, joined);
            }
        }
        let class_ownership: Vec<Option<OwnedType>> = (0..n)
            .map(|v| {
                let r = self.uf.find(v);
                class_join.get(&r).cloned()
            })
            .collect();

        SolverResult {
            types,
            ownership: self.ownership,
            class_ownership,
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
                // Ownership is a per-variable requirement (OWN-LATTICE), not a property of the
                // type class: `a == b` unifies the types of `a` and `b` but must not couple their modes.
                let slot = &mut self.ownership[var.0 as usize];
                *slot = Some(match slot.take() {
                    Some(existing) => join_ownership(existing, ownership.clone()),
                    None => ownership.clone(),
                });
            }

            Constraint::UsedAs(value, site) => {
                self.flows.push((*value, *site));
            }

            Constraint::SharesRegion(a, b) => {
                let oa = self.ownership[a.0 as usize].clone();
                let ob = self.ownership[b.0 as usize].clone();

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
                if (var.0 as usize) < self.clones.needs_clone.len() {
                    self.clones.needs_clone[var.0 as usize] = true;
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

    fn propagate_flows(&mut self) {
        if self.flows.is_empty() {
            return;
        }
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..self.flows.len() {
                let (value, site) = self.flows[i];
                let Some(req) = self.ownership[site.0 as usize].clone() else { continue };
                let cur = self.ownership[value.0 as usize].clone();
                let next = match cur.clone() {
                    Some(c) => join_ownership(c, req),
                    None => req,
                };
                if cur.as_ref().map(ownership_rank) != Some(ownership_rank(&next)) {
                    self.ownership[value.0 as usize] = Some(next);
                    changed = true;
                }
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
                // ADR-0015 section 1: `int` = i64, `float` = f64.
                let default = match class {
                    NumericClass::Integer => BaseType::I64,
                    NumericClass::Float => BaseType::F64,
                    NumericClass::Numeric => BaseType::I64,
                };
                self.types[root as usize] = Some(default);
            }
        }
    }
}

/// Position in the ownership lattice `&` < `&mut` < owned (OWN-LATTICE). Regions do not matter.
fn ownership_rank(o: &OwnedType) -> u8 {
    match o {
        OwnedType::Inferred => 0,
        OwnedType::Ref(_) => 1,
        OwnedType::MutRef(_) => 2,
        OwnedType::Copy => 3,
        OwnedType::Owned => 4,
    }
}

/// Lattice join: the stronger requirement wins; equal ranks keep the existing value (and region),
/// so the result does not depend on constraint order up to region identity.
pub fn join_ownership(existing: OwnedType, new: OwnedType) -> OwnedType {
    if ownership_rank(&new) > ownership_rank(&existing) {
        new
    } else {
        existing
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

    /// D-SOLVER-UNION (docs/spec/inference-divergences.md): the call-site formal of a callee
    /// (OWN-CALL-FORMAL) must reach the parameter passed to it. Constraint generation links them with
    /// `TypeEquals` (types) and `UsedAs` (ownership requirement).
    #[test]
    fn ownership_propagates_across_type_equals() {
        let mut cs = ConstraintSet::new();
        let param = cs.fresh_var();
        let formal = cs.fresh_var();
        cs.add(Constraint::OwnershipIs(formal, OwnedType::Owned));
        cs.add(Constraint::TypeEquals(param, formal));
        cs.add(Constraint::UsedAs(param, formal));
        let result = Solver::new(&cs).solve(&cs);
        // The pipeline reads the result at the parameter's own variable.
        assert_eq!(result.ownership[param.0 as usize], Some(OwnedType::Owned));
    }

    /// Order of union and requirement must not matter (the bug lost constraints recorded on a
    /// class root that later stopped being the root).
    #[test]
    fn ownership_survives_union_in_either_order() {
        for formal_first in [true, false] {
            let mut cs = ConstraintSet::new();
            let param = cs.fresh_var();
            let formal = cs.fresh_var();
            let other = cs.fresh_var();
            if formal_first {
                cs.add(Constraint::OwnershipIs(formal, OwnedType::MutRef(Region::fresh(3))));
            }
            cs.add(Constraint::TypeEquals(other, formal));
            cs.add(Constraint::TypeEquals(param, formal));
            cs.add(Constraint::UsedAs(param, formal));
            if !formal_first {
                cs.add(Constraint::OwnershipIs(formal, OwnedType::MutRef(Region::fresh(3))));
            }
            let result = Solver::new(&cs).solve(&cs);
            assert!(matches!(result.ownership[param.0 as usize], Some(OwnedType::MutRef(_))));
        }
    }

    /// `a == b` unifies types; it must not make `b` owned because `a` is returned.
    #[test]
    fn type_equality_alone_does_not_couple_parameter_modes() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();
        cs.add(Constraint::OwnershipIs(a, OwnedType::Owned));
        cs.add(Constraint::OwnershipIs(b, OwnedType::Ref(Region::fresh(1))));
        cs.add(Constraint::TypeEquals(a, b));
        let result = Solver::new(&cs).solve(&cs);
        assert!(matches!(result.ownership[b.0 as usize], Some(OwnedType::Ref(_))));
        // ... but the class view (what a local holds) is the join.
        assert_eq!(result.class_ownership[b.0 as usize], Some(OwnedType::Owned));
    }

    /// Requirements flow transitively through `let` aliases: `let x = p; let y = x; keep(y)`.
    #[test]
    fn requirement_flows_through_aliases() {
        let mut cs = ConstraintSet::new();
        let p = cs.fresh_var();
        let x = cs.fresh_var();
        let y = cs.fresh_var();
        let formal = cs.fresh_var();
        // Edges added before the requirement exists: propagation is a fixed point, not one pass.
        cs.add(Constraint::UsedAs(p, x));
        cs.add(Constraint::UsedAs(x, y));
        cs.add(Constraint::UsedAs(y, formal));
        cs.add(Constraint::OwnershipIs(formal, OwnedType::Owned));
        let result = Solver::new(&cs).solve(&cs);
        assert_eq!(result.ownership[p.0 as usize], Some(OwnedType::Owned));
        // The requirement never flows backwards from a value to its use site.
        let mut cs = ConstraintSet::new();
        let p = cs.fresh_var();
        let site = cs.fresh_var();
        cs.add(Constraint::OwnershipIs(p, OwnedType::Owned));
        cs.add(Constraint::UsedAs(p, site));
        let result = Solver::new(&cs).solve(&cs);
        assert_eq!(result.ownership[site.0 as usize], None);
    }

    /// OWN-LATTICE: the join is order independent and never reports a conflict, even when the
    /// borrows carry different regions.
    #[test]
    fn ownership_join_is_commutative_and_region_blind() {
        let modes = [
            OwnedType::Ref(Region::fresh(1)),
            OwnedType::Ref(Region::fresh(2)),
            OwnedType::MutRef(Region::fresh(3)),
            OwnedType::MutRef(Region::fresh(4)),
            OwnedType::Owned,
        ];
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        for _ in 0..200 {
            // Random subset in random order.
            let mut picked: Vec<OwnedType> = Vec::new();
            for m in &modes {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                if seed % 2 == 0 {
                    picked.push(m.clone());
                }
            }
            if picked.is_empty() {
                continue;
            }
            let expected = picked.iter().map(ownership_rank).max().unwrap();
            let mut orders = vec![picked.clone()];
            let mut rev = picked.clone();
            rev.reverse();
            orders.push(rev);
            for order in orders {
                let mut cs = ConstraintSet::new();
                let v = cs.fresh_var();
                for m in &order {
                    cs.add(Constraint::OwnershipIs(v, m.clone()));
                }
                let result = Solver::new(&cs).solve(&cs);
                assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
                assert_eq!(ownership_rank(result.ownership[0].as_ref().unwrap()), expected);
            }
        }
    }

    /// Types are read per variable: a non-root member of a class sees the class type.
    #[test]
    fn types_are_resolved_for_every_class_member() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();
        let c = cs.fresh_var();
        cs.add(Constraint::TypeEquals(a, b));
        cs.add(Constraint::TypeEquals(c, a));
        cs.add(Constraint::TypeIs(c, BaseType::I64));
        let result = Solver::new(&cs).solve(&cs);
        for v in [a, b, c] {
            assert_eq!(result.types[v.0 as usize], Some(BaseType::I64));
        }
    }

    /// Clone requirements are read at the variable that carries them.
    #[test]
    fn clone_requirement_is_read_per_variable() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();
        let b = cs.fresh_var();
        cs.add(Constraint::TypeEquals(a, b));
        cs.add(Constraint::NeedsClone(b));
        let result = Solver::new(&cs).solve(&cs);
        assert!(result.clones.needs_clone(b));
        assert!(!result.clones.needs_clone(a));
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
            Some(BaseType::I64),
            "IsInteger should infer int (i64) by default (ADR-0015)"
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
    fn test_is_numeric_infers_i64_default() {
        let mut cs = ConstraintSet::new();
        let a = cs.fresh_var();

        cs.add(Constraint::IsNumeric(a));

        let solver = Solver::new(&cs);
        let result = solver.solve(&cs);

        assert!(result.diagnostics.is_empty());
        assert_eq!(
            result.types[0],
            Some(BaseType::I64),
            "IsNumeric should default to int (i64) (ADR-0015)"
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
}
