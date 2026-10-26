//! The inference rule catalog: stable IDs shared by `docs/spec/inference.md`, the constraint
//! provenance (`ConstraintOrigin::rule`) and `wj explain-ownership` output.
//!
//! A test (`tests/inference_spec_catalog_test.rs`) keeps this table and the spec headings in sync.

/// Which part of the language a rule governs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleArea {
    ParamMode,
    Receiver,
    Clone,
    Move,
    Return,
    Closure,
    Field,
    Number,
    Text,
    Mutability,
    CallSite,
    Type,
    Effect,
}

/// One declarative inference rule.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub area: RuleArea,
    pub title: &'static str,
}

macro_rules! rules {
    ($(($id:literal, $area:ident, $title:literal)),* $(,)?) => {
        &[$(Rule { id: $id, area: RuleArea::$area, title: $title }),*]
    };
}

/// Every rule ID the spec defines. Order is the order of the spec.
pub const RULES: &[Rule] = rules![
    ("OWN-LATTICE", ParamMode, "Parameter modes form the lattice ref < mut < owned; the strongest requirement wins"),
    ("OWN-READONLY", ParamMode, "A parameter that is only read is borrowed `&T`"),
    ("OWN-MUT-ASSIGN", ParamMode, "Assigning to a parameter or one of its fields/elements makes it `&mut T`"),
    ("OWN-MUT-RECEIVER", ParamMode, "Calling a `&mut self` method on a parameter makes it `&mut T`"),
    ("OWN-MUT-ANALYZER", ParamMode, "Mutation detected by the analyzer (implementation seed)"),
    ("OWN-RETURNED", ParamMode, "A parameter (or a non-Copy field of it) that is returned is owned"),
    ("OWN-STORED", ParamMode, "A parameter moved into a struct, enum, tuple or collection is owned"),
    ("OWN-COPY-SEED", ParamMode, "Copy scalars are passed by value"),
    ("OWN-READ-FIELD", ParamMode, "Reading a field of a parameter borrows it"),
    ("OWN-CONSUMING-METHOD", ParamMode, "A consuming trait method (`into`, `try_into`, `into_iter`) on a generic parameter takes it by value"),
    ("OWN-CALL-FORMAL", ParamMode, "Passing a parameter on to a callee inherits the callee's formal mode"),
    ("OWN-STR-REF", ParamMode, "Read-only `string` parameters are `&str`"),
    ("OWN-PIN-REF", ParamMode, "An explicit `&T` annotation is a pin"),
    ("OWN-PIN-MUTREF", ParamMode, "An explicit `&mut T` annotation is a pin"),
    ("OWN-ANALYZER-SEED", ParamMode, "Analyzer pre-decision fed to the solver (implementation seed; ADR-0001 divergence)"),
    ("SELF-READ", Receiver, "A method that only reads `self` takes `&self`"),
    ("SELF-MUT", Receiver, "A method that mutates `self` takes `&mut self`"),
    ("SELF-CONSUME", Receiver, "A method that returns or moves `self` takes `self`"),
    ("SELF-GETTER", Receiver, "Returning a non-Copy field of `self` borrows `self` and clones the field"),
    ("MOVE-COPY-TYPE", Move, "Copy types are copied, never moved or cloned"),
    ("MOVE-LAST-USE", Move, "The last use of an owned value moves it; no clone"),
    ("CLONE-USE-AFTER-MOVE", Clone, "A value passed by move and used again afterwards is cloned at the move"),
    ("CLONE-LOOP", Clone, "A value moved inside a loop body but declared outside it is cloned"),
    ("CLONE-BORROWED-TO-OWNED", Clone, "Producing an owned value from a borrowed one clones it"),
    ("CLONE-ANALYZER", Clone, "Clone site recorded by the analyzer's auto-clone pass (implementation seed)"),
    ("RET-OWNED", Return, "Function results are owned values"),
    ("RET-NO-BORROW", Return, "A function never returns a borrow of its parameters"),
    ("CLOS-MOVE-CAPTURE", Closure, "A closure captures the variables it names by move"),
    ("CLOS-MUT-CAPTURE", Closure, "A closure that mutates a captured variable makes it mutable"),
    ("FIELD-OWNED", Field, "Struct fields own their values"),
    ("NUM-DECLARED", Number, "A declared numeric type is exact"),
    ("NUM-LITERAL-SUFFIX", Number, "A literal suffix is exact"),
    ("NUM-PROPAGATE", Number, "An unsuffixed literal takes the type its uses require"),
    ("NUM-DEFAULT-INT", Number, "An unconstrained integer is `int` (i64)"),
    ("NUM-DEFAULT-FLOAT", Number, "An unconstrained float is `float` (f64)"),
    ("NUM-INDEX", Number, "Indexing and `len()` accept and produce `int` through a checked conversion"),
    ("NUM-MIXED", Number, "Mixed-width arithmetic requires an explicit conversion"),
    ("NUM-CAST", Number, "`as` fixes the width of its result"),
    ("NUM-LITERAL", Number, "Literal of a non-numeric kind"),
    ("NUM-CLASS", Number, "A value of known width is in that width's integer or float class"),
    ("TY-UNIFY", Type, "Connected values (operands, bindings, assignments, returns, elements) have one type"),
    ("TY-BOOL", Type, "Conditions, comparisons and logical operands are bool"),
    ("TY-CALL-RESULT", Type, "A call has its callee's return type"),
    ("TY-ANALYZER-SEED", Type, "Parameter type inferred by the analyzer (implementation seed)"),
    ("VAL-OWNED", Type, "Constructed values are owned"),
    ("EFF-CALL", Effect, "Effectful runtime calls contribute effects to the caller"),
    ("STR-OWNED", Text, "`string` is an owned, growable string"),
    ("STR-LITERAL", Text, "A string literal is a `string` value"),
    ("STR-READONLY-REF", Text, "A read-only `string` parameter is borrowed as `&str`"),
    ("MUT-LOCAL-EXPLICIT", Mutability, "A local is mutable only if declared `let mut` (ADR-0002)"),
    ("CALL-BORROW", CallSite, "An argument for a `&T` formal is lent"),
    ("CALL-BORROW-MUT", CallSite, "An argument for a `&mut T` formal is lent mutably"),
    ("CALL-MOVE", CallSite, "An argument for an owned formal at its last use is moved"),
    ("CALL-CLONE", CallSite, "An argument for an owned formal that is used again is cloned"),
];

/// Look up a rule by ID.
pub fn rule(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn rule_ids_are_unique() {
        let mut seen = HashSet::new();
        for r in RULES {
            assert!(seen.insert(r.id), "duplicate rule id {}", r.id);
        }
    }

    #[test]
    fn every_tagged_rule_in_constraint_gen_is_cataloged() {
        let src = include_str!("../constraint_gen.rs");
        let mut n = 0;
        for part in src.split("tagged(\"").skip(1) {
            let id = &part[..part.find('"').unwrap()];
            assert!(rule(id).is_some(), "rule {id} used via tagged() but not in RULES");
            n += 1;
        }
        assert!(n > 40, "expected the structural constraints to be tagged, found {n}");
    }

    #[test]
    fn every_provenance_rule_used_by_constraint_gen_is_cataloged() {
        let src = include_str!("../constraint_gen.rs");
        for line in src.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some(end) = rest.find('"') {
                    let id = &rest[..end];
                    let looks_like_rule = id
                        .split_once('-')
                        .is_some_and(|(p, _)| ["OWN", "CLONE", "NUM", "SELF", "TY", "VAL", "EFF", "RET"].contains(&p))
                        && id.chars().all(|c| c.is_ascii_uppercase() || c == '-');
                    if looks_like_rule && t.ends_with(',') {
                        assert!(rule(id).is_some(), "rule {id} used in constraint_gen but not in RULES");
                    }
                }
            }
        }
    }
}
