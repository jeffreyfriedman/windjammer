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

/// A component of a structured type (see `Constraint::HasPart`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Part {
    /// The element of a `Vec`, an array, or an iterator.
    Elem,
    /// Field `i` of a tuple.
    Field(u8),
    /// The payload of `Some` / `Ok`.
    Payload,
    /// The payload of `Err`.
    ErrPayload,
    /// The key of a map.
    Key,
    /// The value of a map.
    Value,
    /// Parameter `i` of a function value (a closure, a `fn(..)` formal).
    Param(u8),
    /// The result of a function value.
    Ret,
}

impl Part {
    /// This part of a known type, when the type has it and the part is fully known.
    pub fn of(self, ty: &BaseType) -> Option<BaseType> {
        let t = match (self, ty) {
            (Part::Elem, BaseType::Vec(e) | BaseType::Array(e, _)) => (**e).clone(),
            (Part::Field(i), BaseType::Tuple(ts)) => ts.get(i as usize)?.clone(),
            (Part::Payload, BaseType::Option(t) | BaseType::Result(t, _)) => (**t).clone(),
            (Part::ErrPayload, BaseType::Result(_, e)) => (**e).clone(),
            (Part::Key, BaseType::HashMap(k, _)) => (**k).clone(),
            (Part::Value, BaseType::HashMap(_, v)) => (**v).clone(),
            (Part::Param(i), BaseType::FunctionPointer { params, .. } | BaseType::Fn(params, _)) => params.get(i as usize)?.clone(),
            (Part::Ret, BaseType::FunctionPointer { return_type: r, .. } | BaseType::Fn(_, r)) => (**r).clone(),
            _ => return None,
        };
        // A named type (a struct, or a signature's generic spelled as a name) does not type a part: only structure and
        // numbers do.
        (is_fully_known(&t) && !contains_named(&t)).then_some(t)
    }
}

/// A type with no `Inferred` or type-variable component.
pub fn is_fully_known(t: &BaseType) -> bool {
    match t {
        BaseType::Inferred | BaseType::TypeVar(_) => false,
        BaseType::Vec(x) | BaseType::Option(x) | BaseType::Array(x, _) => is_fully_known(x),
        BaseType::Result(a, b) | BaseType::HashMap(a, b) => is_fully_known(a) && is_fully_known(b),
        BaseType::Tuple(ts) => ts.iter().all(is_fully_known),
        BaseType::Fn(ps, r) | BaseType::FunctionPointer { params: ps, return_type: r } => ps.iter().all(is_fully_known) && is_fully_known(r),
        _ => true,
    }
}

/// A type with a named component (a struct, a trait, or a generic parameter spelled as a name).
pub fn contains_named(t: &BaseType) -> bool {
    match t {
        BaseType::Custom(_) | BaseType::TraitObject(_) | BaseType::ImplTrait(_) => true,
        BaseType::Vec(x) | BaseType::Option(x) | BaseType::Array(x, _) => contains_named(x),
        BaseType::Result(a, b) | BaseType::HashMap(a, b) => contains_named(a) || contains_named(b),
        BaseType::Tuple(ts) => ts.iter().any(contains_named),
        BaseType::Fn(ps, r) | BaseType::FunctionPointer { params: ps, return_type: r } => ps.iter().any(contains_named) || contains_named(r),
        _ => false,
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
    /// NUM-USIZE (ADR-0028): the value is an index source (`len()`, `count()`) or an index use
    /// (`v[i]`). Soft: a class with no exact width resolves to `usize` instead of the `int` default;
    /// a class with an exact width keeps it (the backend converts at the site, NUM-USIZE-TO-INT).
    PrefersUsize(ConstraintVar),
    /// `HasPart(container, part, var)`: `var` is that part of the container's type (the element of a
    /// `Vec`/array/iterator, a tuple field, an `Option`/`Ok` payload, an `Err` payload). Structural:
    /// two containers in one class have equal parts, and a container's known type fixes its parts' types.
    HasPart(ConstraintVar, Part, ConstraintVar),
    /// `FlowsInto(value, slot)`: a value passed to a std method's generic slot (`v.push(i + 1)`). It unifies like
    /// `TypeEquals`, except that an index class (no exact width, meets an index source or use) entering a slot of an
    /// exact integer width stays an index class and is converted at the argument (NUM-USIZE-TO-INT).
    FlowsInto(ConstraintVar, ConstraintVar),
    /// `NumericPeers(a, b)`: `a` and `b` meet in arithmetic or a compound assignment. One class, unless one side is an
    /// integer (an exact integer type) and the other a float (an exact float type or a float literal): that is
    /// mixed arithmetic, which the backend converts, and the classes stay apart.
    /// The third variable, when present, is the arithmetic's result: the shared class, or for mixed arithmetic the
    /// float side's class (`i * 0.5` is a float).
    NumericPeers(ConstraintVar, ConstraintVar, Option<ConstraintVar>),

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

    /// Add a constraint without provenance. Test-only: compiler code must say why a constraint
    /// exists (`add_because`), so `wj explain-ownership` never reports `UNTAGGED` (plan C2).
    #[cfg(test)]
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
