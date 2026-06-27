//! Unified numeric constraint solver.
//!
//! Solves integer and float type constraints simultaneously using union-find.
//! This replaces the sequential float-then-integer fixpoint passes with a
//! single global resolution that handles cross-type-boundary information flow.
//!
//! The key architectural fix: when a generic container like `HashMap<u32, String>`
//! is instantiated, the type parameter `K=u32` propagates to ALL call sites
//! globally in one solve pass — not sequentially across separate inference engines.

use crate::ir::numeric_types::{NumericConstraint, NumericType, UnifiedExprId};
use std::collections::HashMap;

/// Result of solving numeric constraints.
#[derive(Debug)]
pub struct NumericSolverResult {
    /// Resolved types for each expression.
    pub resolved: HashMap<UnifiedExprId, NumericType>,
    /// Errors encountered during solving.
    pub errors: Vec<NumericSolverError>,
}

/// An error from the numeric solver.
#[derive(Debug, Clone)]
pub struct NumericSolverError {
    pub kind: NumericErrorKind,
    pub message: String,
    pub expr_ids: Vec<UnifiedExprId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumericErrorKind {
    TypeConflict,
    NonConvergence,
}

/// Union-find for expression IDs, mapping to numeric types.
#[derive(Default)]
struct NumericUnionFind {
    /// Map from ExprId to its canonical slot index.
    id_to_slot: HashMap<UnifiedExprId, u32>,
    /// Union-find parent array.
    parent: Vec<u32>,
    /// Union-find rank for path compression.
    rank: Vec<u32>,
    /// Resolved type for each slot (at the representative).
    types: Vec<NumericType>,
}

impl NumericUnionFind {
    fn new() -> Self {
        Self {
            id_to_slot: HashMap::new(),
            parent: Vec::new(),
            rank: Vec::new(),
            types: Vec::new(),
        }
    }

    fn get_or_create_slot(&mut self, id: UnifiedExprId) -> u32 {
        if let Some(&slot) = self.id_to_slot.get(&id) {
            return slot;
        }
        let slot = self.parent.len() as u32;
        self.id_to_slot.insert(id, slot);
        self.parent.push(slot);
        self.rank.push(0);
        self.types.push(NumericType::Unknown);
        slot
    }

    fn find(&mut self, x: u32) -> u32 {
        if self.parent[x as usize] != x {
            self.parent[x as usize] = self.find(self.parent[x as usize]);
        }
        self.parent[x as usize]
    }

    fn union(&mut self, x: u32, y: u32) -> u32 {
        let rx = self.find(x);
        let ry = self.find(y);
        if rx == ry {
            return rx;
        }
        if self.rank[rx as usize] < self.rank[ry as usize] {
            self.parent[rx as usize] = ry;
            ry
        } else if self.rank[rx as usize] > self.rank[ry as usize] {
            self.parent[ry as usize] = rx;
            rx
        } else {
            self.parent[ry as usize] = rx;
            self.rank[rx as usize] += 1;
            rx
        }
    }

    fn get_type(&mut self, slot: u32) -> NumericType {
        let root = self.find(slot);
        self.types[root as usize]
    }

    fn set_type(&mut self, slot: u32, ty: NumericType) {
        let root = self.find(slot);
        self.types[root as usize] = ty;
    }
}

/// The unified numeric solver.
#[derive(Default)]
pub struct NumericSolver {
    uf: NumericUnionFind,
    constraints: Vec<NumericConstraint>,
    errors: Vec<NumericSolverError>,
}

impl NumericSolver {
    pub fn new() -> Self {
        Self {
            uf: NumericUnionFind::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add a constraint to be solved.
    pub fn add_constraint(&mut self, constraint: NumericConstraint) {
        // Pre-register all ExprIds
        match &constraint {
            NumericConstraint::MustBe { expr_id, .. } => {
                self.uf.get_or_create_slot(*expr_id);
            }
            NumericConstraint::MustMatch { expr_a, expr_b, .. } => {
                self.uf.get_or_create_slot(*expr_a);
                self.uf.get_or_create_slot(*expr_b);
            }
        }
        self.constraints.push(constraint);
    }

    /// Add multiple constraints at once.
    pub fn add_constraints(&mut self, constraints: impl IntoIterator<Item = NumericConstraint>) {
        for c in constraints {
            self.add_constraint(c);
        }
    }

    /// Solve all constraints and return the result.
    pub fn solve(mut self) -> NumericSolverResult {
        // Pass 0: Seed all explicit MustBe constraints (like float inference pass 0).
        // This ensures explicit type annotations win over defaults.
        for constraint in &self.constraints.clone() {
            if let NumericConstraint::MustBe {
                expr_id,
                numeric_type,
                ..
            } = constraint
            {
                if !numeric_type.is_unknown() {
                    let slot = self.uf.get_or_create_slot(*expr_id);
                    let current = self.uf.get_type(slot);
                    if current.is_unknown() {
                        self.uf.set_type(slot, *numeric_type);
                    }
                }
            }
        }

        // Pass 1+: Fixpoint iteration (union-find + propagation).
        let max_iterations = 100;
        let mut iteration = 0;
        let mut changed = true;

        while changed && iteration < max_iterations {
            changed = false;
            iteration += 1;

            for constraint in &self.constraints.clone() {
                match constraint {
                    NumericConstraint::MustBe {
                        expr_id,
                        numeric_type,
                        reason,
                    } => {
                        let slot = self.uf.get_or_create_slot(*expr_id);
                        let current = self.uf.get_type(slot);

                        if current.is_unknown() && !numeric_type.is_unknown() {
                            self.uf.set_type(slot, *numeric_type);
                            changed = true;
                        } else if !current.is_unknown()
                            && !numeric_type.is_unknown()
                            && current != *numeric_type
                        {
                            // Conflict — try to resolve
                            if let Some(resolved) = current.resolve_conflict(numeric_type) {
                                if resolved != current {
                                    self.uf.set_type(slot, resolved);
                                    changed = true;
                                }
                            } else {
                                self.errors.push(NumericSolverError {
                                    kind: NumericErrorKind::TypeConflict,
                                    message: format!(
                                        "type conflict: {:?} vs {:?} ({})",
                                        current, numeric_type, reason
                                    ),
                                    expr_ids: vec![*expr_id],
                                });
                            }
                        }
                    }

                    NumericConstraint::MustMatch {
                        expr_a,
                        expr_b,
                        reason,
                    } => {
                        let slot_a = self.uf.get_or_create_slot(*expr_a);
                        let slot_b = self.uf.get_or_create_slot(*expr_b);
                        let type_a = self.uf.get_type(slot_a);
                        let type_b = self.uf.get_type(slot_b);

                        match (type_a.is_unknown(), type_b.is_unknown()) {
                            (true, true) => {
                                // Unify structurally so future assignments propagate
                                let root = self.uf.union(slot_a, slot_b);
                                let _ = root; // both unknown, union keeps Unknown
                            }
                            (true, false) => {
                                // Propagate b's type to a
                                let root = self.uf.union(slot_a, slot_b);
                                self.uf.set_type(root, type_b);
                                changed = true;
                            }
                            (false, true) => {
                                // Propagate a's type to b
                                let root = self.uf.union(slot_a, slot_b);
                                self.uf.set_type(root, type_a);
                                changed = true;
                            }
                            (false, false) => {
                                if type_a == type_b {
                                    // Already agree — unify
                                    self.uf.union(slot_a, slot_b);
                                } else if let Some(resolved) = type_a.resolve_conflict(&type_b) {
                                    let root = self.uf.union(slot_a, slot_b);
                                    self.uf.set_type(root, resolved);
                                    if resolved != type_a || resolved != type_b {
                                        changed = true;
                                    }
                                } else {
                                    self.errors.push(NumericSolverError {
                                        kind: NumericErrorKind::TypeConflict,
                                        message: format!(
                                            "type conflict in MustMatch: {:?} vs {:?} ({})",
                                            type_a, type_b, reason
                                        ),
                                        expr_ids: vec![*expr_a, *expr_b],
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        if changed && iteration >= max_iterations {
            self.errors.push(NumericSolverError {
                kind: NumericErrorKind::NonConvergence,
                message: format!(
                    "numeric solver did not converge after {} iterations",
                    max_iterations
                ),
                expr_ids: Vec::new(),
            });
        }

        // Collect results — snapshot id_to_slot to avoid borrow conflict
        let id_slots: Vec<(UnifiedExprId, u32)> = self
            .uf
            .id_to_slot
            .iter()
            .map(|(&id, &slot)| (id, slot))
            .collect();

        let mut resolved = HashMap::new();
        for (expr_id, slot) in id_slots {
            let ty = self.uf.get_type(slot);
            if !ty.is_unknown() {
                resolved.insert(expr_id, ty);
            }
        }

        NumericSolverResult {
            resolved,
            errors: self.errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(seq: usize, line: usize, col: usize) -> UnifiedExprId {
        UnifiedExprId::new(seq, 0, line, col)
    }

    #[test]
    fn test_simple_must_be() {
        let mut solver = NumericSolver::new();
        let id = eid(0, 1, 1);
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: id,
            numeric_type: NumericType::U32,
            reason: "explicit annotation".into(),
        });
        let result = solver.solve();
        assert!(result.errors.is_empty());
        assert_eq!(result.resolved.get(&id), Some(&NumericType::U32));
    }

    #[test]
    fn test_must_match_propagation() {
        let mut solver = NumericSolver::new();
        let a = eid(0, 1, 1);
        let b = eid(1, 2, 1);

        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: a,
            numeric_type: NumericType::U32,
            reason: "from HashMap<u32, _>".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: a,
            expr_b: b,
            reason: "insert key arg".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        assert_eq!(result.resolved.get(&a), Some(&NumericType::U32));
        assert_eq!(result.resolved.get(&b), Some(&NumericType::U32));
    }

    #[test]
    fn test_transitive_propagation() {
        // Simulates: HashMap<u32, String> → insert(key, val) → key must be u32
        let mut solver = NumericSolver::new();
        let decl = eid(0, 1, 1); // HashMap declaration K=u32
        let insert_key = eid(1, 5, 1); // key arg at insert() call
        let literal = eid(2, 5, 10); // literal 42 passed as key

        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: decl,
            numeric_type: NumericType::U32,
            reason: "HashMap<u32, _> type parameter".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: decl,
            expr_b: insert_key,
            reason: "K parameter flows to insert key".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: insert_key,
            expr_b: literal,
            reason: "argument matches parameter".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        // The literal should resolve to U32, not default I32
        assert_eq!(result.resolved.get(&literal), Some(&NumericType::U32));
    }

    #[test]
    fn test_float_strict_conflict() {
        let mut solver = NumericSolver::new();
        let id = eid(0, 1, 1);

        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: id,
            numeric_type: NumericType::F32,
            reason: "let x: f32".into(),
        });
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: id,
            numeric_type: NumericType::F64,
            reason: "fn returns f64".into(),
        });

        let result = solver.solve();
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].kind, NumericErrorKind::TypeConflict);
    }

    #[test]
    fn test_int_implicit_cast_resolution() {
        let mut solver = NumericSolver::new();
        let a = eid(0, 1, 1);
        let b = eid(1, 2, 1);

        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: a,
            numeric_type: NumericType::I32,
            reason: "default".into(),
        });
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: b,
            numeric_type: NumericType::Usize,
            reason: "array index".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: a,
            expr_b: b,
            reason: "used as index".into(),
        });

        let result = solver.solve();
        // Should resolve without error (i32 <-> usize is safe)
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_library_multipass_scenario() {
        // The bug that motivated this work:
        // File A: HashMap<u32, String> declared
        // File B: map.insert(key, val) — key should be u32, not default I64
        let mut solver = NumericSolver::new();

        // File A: HashMap<u32, String> type param K=u32
        let type_param = eid(0, 10, 5);
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: type_param,
            numeric_type: NumericType::U32,
            reason: "HashMap<u32, String> declaration".into(),
        });

        // File B: insert(key, val) — key flows from type_param
        let key_param = eid(1, 20, 10);
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: type_param,
            expr_b: key_param,
            reason: "generic K flows to insert() key parameter".into(),
        });

        // File B: literal 42 passed as key
        let literal_42 = eid(2, 20, 20);
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: key_param,
            expr_b: literal_42,
            reason: "argument matches parameter".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        // Critical: literal 42 resolves to U32, not I32/I64
        assert_eq!(result.resolved.get(&literal_42), Some(&NumericType::U32));
        assert_eq!(result.resolved.get(&key_param), Some(&NumericType::U32));
    }

    #[test]
    fn test_mixed_int_float_independence() {
        // Int and float constraints don't cross-contaminate
        let mut solver = NumericSolver::new();
        let int_expr = eid(0, 1, 1);
        let float_expr = eid(1, 2, 1);

        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: int_expr,
            numeric_type: NumericType::I32,
            reason: "int var".into(),
        });
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: float_expr,
            numeric_type: NumericType::F64,
            reason: "float var".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        assert_eq!(result.resolved.get(&int_expr), Some(&NumericType::I32));
        assert_eq!(result.resolved.get(&float_expr), Some(&NumericType::F64));
    }

    #[test]
    fn test_chain_propagation() {
        // a = b = c = d, only d has a type
        let mut solver = NumericSolver::new();
        let a = eid(0, 1, 1);
        let b = eid(1, 2, 1);
        let c = eid(2, 3, 1);
        let d = eid(3, 4, 1);

        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: a,
            expr_b: b,
            reason: "chain".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: b,
            expr_b: c,
            reason: "chain".into(),
        });
        solver.add_constraint(NumericConstraint::MustMatch {
            expr_a: c,
            expr_b: d,
            reason: "chain".into(),
        });
        solver.add_constraint(NumericConstraint::MustBe {
            expr_id: d,
            numeric_type: NumericType::I64,
            reason: "annotated".into(),
        });

        let result = solver.solve();
        assert!(result.errors.is_empty());
        assert_eq!(result.resolved.get(&a), Some(&NumericType::I64));
        assert_eq!(result.resolved.get(&b), Some(&NumericType::I64));
        assert_eq!(result.resolved.get(&c), Some(&NumericType::I64));
        assert_eq!(result.resolved.get(&d), Some(&NumericType::I64));
    }
}
