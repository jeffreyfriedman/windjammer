//! Constraint definitions for the unified solver.
//!
//! Phase 1: Type definitions only — the solver is a stub.
//! Phase 2: Constraint generation from AST + union-find unification.

use crate::ir::safety_type::{BaseType, EffectSet, OwnedType, TaintStatus};

/// A constraint variable — an opaque handle into the solver's union-find.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstraintVar(pub u32);

impl ConstraintVar {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// A constraint emitted during the AST walk.
#[derive(Debug, Clone)]
pub enum Constraint {
    // --- Type constraints ---
    /// Two type variables must unify to the same type.
    TypeEquals(ConstraintVar, ConstraintVar),
    /// A type variable must be a specific concrete type.
    TypeIs(ConstraintVar, BaseType),
    /// A type variable must be numeric (int or float).
    IsNumeric(ConstraintVar),
    /// A type variable must be an integer type.
    IsInteger(ConstraintVar),
    /// A type variable must be a float type.
    IsFloat(ConstraintVar),

    // --- Ownership constraints ---
    /// A variable must have a specific ownership mode.
    OwnershipIs(ConstraintVar, OwnedType),
    /// Two variables share a borrow region (aliasing constraint).
    SharesRegion(ConstraintVar, ConstraintVar),
    /// A variable needs a clone (used at multiple owned sites).
    NeedsClone(ConstraintVar),
    /// `UsedAs(value, site)`: `value` is used at a position whose ownership requirement is
    /// `site`'s (a callee formal, the return slot, a `let` binding that aliases it). The value's
    /// requirement is at least the site's (OWN-CALL-FORMAL, OWN-LATTICE). Directional, unlike
    /// `TypeEquals`: `a == b` unifies types without coupling modes.
    UsedAs(ConstraintVar, ConstraintVar),

    // --- Effect constraints (Phase 3) ---
    /// A function's effect set must include these effects.
    HasEffects(ConstraintVar, EffectSet),
    /// A function's effects are the union of its callees' effects.
    EffectsUnion(ConstraintVar, Vec<ConstraintVar>),

    // --- Taint constraints (Phase 4) ---
    /// A value has a specific taint status.
    TaintIs(ConstraintVar, TaintStatus),
    /// Taint propagates from source to target.
    TaintPropagates(ConstraintVar, ConstraintVar),
    /// A sanitizer clears taint.
    Sanitizes(ConstraintVar),
}

/// Why a constraint was emitted: a stable rule ID (see `docs/spec/inference.md`) plus the
/// source position of the use site. Provenance is a side table for reporting
/// (`wj explain-ownership`); the solver never reads it, so it cannot change a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintOrigin {
    pub rule: &'static str,
    pub detail: String,
    /// `(line, column)` of the use site, when the AST node carries a location.
    pub at: Option<(usize, usize)>,
}

/// A collection of constraints to be solved.
#[derive(Debug, Clone, Default)]
pub struct ConstraintSet {
    constraints: Vec<Constraint>,
    /// Parallel to `constraints`: `None` for constraints emitted without provenance.
    origins: Vec<Option<ConstraintOrigin>>,
    next_var: u32,
}

impl ConstraintSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a fresh constraint variable.
    pub fn fresh_var(&mut self) -> ConstraintVar {
        let var = ConstraintVar(self.next_var);
        self.next_var += 1;
        var
    }

    /// Add a constraint to the set.
    pub fn add(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
        self.origins.push(None);
    }

    /// Add a constraint together with the rule and use site that produced it.
    pub fn add_because(&mut self, constraint: Constraint, origin: ConstraintOrigin) {
        self.constraints.push(constraint);
        self.origins.push(Some(origin));
    }

    /// A copy of this set keeping only the constraints for which `keep(index, constraint)` holds.
    /// Variables and provenance are preserved; used by the explain tooling to test which
    /// constraints actually reach a variable.
    pub fn filtered(&self, mut keep: impl FnMut(usize, &Constraint) -> bool) -> ConstraintSet {
        let mut out = ConstraintSet {
            constraints: Vec::new(),
            origins: Vec::new(),
            next_var: self.next_var,
        };
        for (i, (c, o)) in self.constraints.iter().zip(self.origins.iter()).enumerate() {
            if keep(i, c) {
                out.constraints.push(c.clone());
                out.origins.push(o.clone());
            }
        }
        out
    }

    /// Iterate over constraints with their provenance (if recorded).
    pub fn iter_with_origin(&self) -> impl Iterator<Item = (&Constraint, Option<&ConstraintOrigin>)> {
        self.constraints
            .iter()
            .zip(self.origins.iter().map(|o| o.as_ref()))
    }

    /// Number of constraints.
    pub fn len(&self) -> usize {
        self.constraints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.constraints.is_empty()
    }

    /// Iterate over all constraints.
    pub fn iter(&self) -> impl Iterator<Item = &Constraint> {
        self.constraints.iter()
    }

    /// Number of variables allocated.
    pub fn num_vars(&self) -> u32 {
        self.next_var
    }
}
