//! Publication of the numeric solver's widths as facts on `IrFunction` (ADR-0028).
//!
//! Facts: NUM-LITERAL-WIDTH (`literal_widths`), NUM-LOCAL-I64 (`solver_i64_locals`),
//! NUM-LOCAL-WIDTH (`solver_local_widths`) and NUM-USIZE (`index_source_widths`). A width is
//! published only when the solver fixed it (never the `int` default) and the class it belongs to
//! needs nothing the backend cannot print yet: classes in which an index source or use resolved to
//! another width (they need a NUM-USIZE-TO-INT conversion) and classes that contain a local the
//! facts cannot name (shadowed or closure-bound) stay with the legacy deciders.

use crate::ir::constraint_gen::FunctionConstraints;
use crate::ir::constraints::ConstraintVar;
use crate::ir::node::IrFunction;
use crate::ir::safety_type::BaseType;
use crate::ir::solver::{is_integer_type, SolverResult};
use std::collections::HashSet;

/// Where a NUM-USIZE-TO-INT conversion goes: the expression's source position and its kind (a
/// binary expression and the token after it can share a position).
pub type ConversionKey = (usize, usize, u8);

/// Key of an expression that may carry a conversion (`None` without a position).
pub fn conversion_key(e: &crate::parser::Expression<'_>) -> Option<ConversionKey> {
    use crate::parser::Expression as E;
    let kind = match e {
        E::Identifier { .. } => 1,
        E::Binary { .. } => 2,
        E::MethodCall { .. } => 3,
        E::Call { .. } => 4,
        E::Index { .. } => 5,
        E::FieldAccess { .. } => 6,
        _ => 0,
    };
    let l = e.location()?;
    Some((l.line, l.column, kind))
}

struct View<'a> {
    result: &'a SolverResult,
    held: HashSet<u32>,
}

impl View<'_> {
    fn class(&self, v: ConstraintVar) -> Option<u32> {
        self.result.class_of.get(v.0 as usize).copied()
    }

    fn int_type(&self, v: ConstraintVar) -> Option<BaseType> {
        self.result.types.get(v.0 as usize).cloned().flatten().filter(is_integer_type)
    }

    /// The solver fixed this variable's integer width and its class is publishable.
    fn published(&self, v: ConstraintVar) -> Option<BaseType> {
        let defaulted = self.result.defaulted.get(v.0 as usize).copied().unwrap_or(true);
        if defaulted || self.class(v).is_none_or(|c| self.held.contains(&c)) {
            return None;
        }
        self.int_type(v)
    }
}

/// Classes whose widths stay with the legacy deciders for now (see module docs).
fn held_classes(fc: &FunctionConstraints, result: &SolverResult) -> HashSet<u32> {
    let view = View { result, held: HashSet::new() };
    let mut held = HashSet::new();
    for &v in &fc.index_uses {
        if view.int_type(v) != Some(BaseType::Usize) {
            held.extend(view.class(v));
        }
    }
    // A name the walk met without a binding (a constant, a global, a closure parameter) is not named
    // by the facts; with lexical scoping every `let`/`for` binding has its own variable.
    let bound: HashSet<&str> = fc.binding_vars.iter().map(|(n, _)| n.as_str()).collect();
    for (name, v) in &fc.var_map.locals {
        if !bound.contains(name.as_str()) {
            held.extend(view.class(*v));
        }
    }
    // The solver could not decide a class (two exact widths met): legacy.
    for d in &result.diagnostics {
        if d.kind == crate::ir::solver::DiagnosticKind::TypeError {
            held.extend(d.vars.iter().filter_map(|v| view.class(*v)));
        }
    }
    // A negated literal in an unsigned class, or a loop binding of unknown element width: legacy.
    for &v in &fc.unmodelled.negated_literals {
        if matches!(view.int_type(v), Some(BaseType::U8 | BaseType::U16 | BaseType::U32 | BaseType::U64 | BaseType::U128 | BaseType::Usize)) {
            held.extend(view.class(v));
        }
    }
    let u = &fc.unmodelled;
    for &v in u.loop_bindings.iter().chain(&u.unknown_lengths).chain(&u.unknown_fields) {
        held.extend(view.class(v));
    }
    // Bindings of one name with different widths cannot be named by one fact: their classes stay legacy.
    for (_, widths) in binding_widths(fc, &view) {
        if widths.iter().any(|w| w.1 != widths[0].1) {
            held.extend(widths.iter().filter_map(|(v, _)| view.class(*v)));
        }
    }
    held
}

/// Each binding's solved width, grouped by name (`None` for a defaulted or non-integer binding).
fn binding_widths(fc: &FunctionConstraints, view: &View<'_>) -> std::collections::BTreeMap<String, Vec<(ConstraintVar, Option<BaseType>)>> {
    let mut out: std::collections::BTreeMap<String, Vec<(ConstraintVar, Option<BaseType>)>> = Default::default();
    for (name, v) in &fc.binding_vars {
        let defaulted = view.result.defaulted.get(v.0 as usize).copied().unwrap_or(true);
        let w = if defaulted { None } else { view.int_type(*v) };
        out.entry(name.clone()).or_default().push((*v, w));
    }
    out
}

pub fn publish(fc: &FunctionConstraints, result: &SolverResult, ir_fn: &mut IrFunction) {
    let held = held_classes(fc, result);
    let view = View { result, held };
    for (key, var) in &fc.literal_vars {
        if let Some(base) = view.published(*var) {
            ir_fn.literal_widths.insert(*key, base);
        }
    }
    for name in &fc.single_binding_locals {
        let Some(&var) = fc.var_map.locals.get(name) else { continue };
        let defaulted = result.defaulted.get(var.0 as usize).copied().unwrap_or(true);
        if !defaulted && matches!(result.types.get(var.0 as usize), Some(Some(BaseType::I64))) {
            ir_fn.solver_i64_locals.insert(name.clone());
        }
    }
    // NUM-LOCAL-WIDTH: a name all of whose bindings the solver fixed to one width.
    for (name, widths) in binding_widths(fc, &view) {
        let ws: Vec<Option<BaseType>> = widths.iter().map(|(v, _)| view.published(*v)).collect();
        if let Some(Some(w)) = ws.first() {
            if ws.iter().all(|x| x.as_ref() == Some(w)) {
                ir_fn.solver_local_widths.insert(name, w.clone());
            }
        }
    }
    // NUM-USIZE-TO-INT: an index source in a class of another width is converted where it is produced.
    for (key, var) in &fc.index_sources {
        if let Some(w) = view.published(*var) {
            ir_fn.index_source_widths.insert(*key, w);
        }
    }
    // ... and a `usize` value pushed into a `Vec` of another integer width is converted at the argument.
    for (key, arg, obj) in &fc.element_stores {
        let elem = match result.types.get(obj.0 as usize) {
            Some(Some(BaseType::Vec(e))) => (**e).clone(),
            _ => continue,
        };
        if view.published(*arg) == Some(BaseType::Usize) && is_integer_type(&elem) && elem != BaseType::Usize {
            ir_fn.usize_conversions.insert(*key, elem);
        }
    }
}

#[cfg(test)]
mod tests {
    fn facts(src: &str, f: &str) -> crate::ir::node::IrFunction {
        let mut lexer = crate::lexer::Lexer::new(src);
        let tokens = lexer.tokenize_with_locations();
        let parser = Box::leak(Box::new(crate::parser::Parser::new(tokens)));
        let program = Box::leak(Box::new(parser.parse().expect("parse")));
        let mut analyzer = crate::analyzer::Analyzer::new();
        let (analyzed, registry, _) = analyzer.analyze_program(program).expect("analyze");
        let module = crate::ir::IrPipeline::new().lower_to_ir(&analyzed, &registry, None);
        module.functions.into_iter().find(|x| x.name == f).expect("function")
    }

    use crate::ir::safety_type::BaseType;

    /// ADR-0028: a counter bounded by `len()` is published `usize`, with its literals and the `len()`.
    #[test]
    fn len_bounded_counter_publishes_usize_widths() {
        let f = facts("pub fn f(v: Vec<string>) -> bool {\n    let mut i = 0\n    while i < v.len() {\n        i = i + 1\n    }\n    true\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        assert!(f.literal_widths.values().all(|w| *w == BaseType::Usize) && f.literal_widths.len() == 2, "{:?}", f.literal_widths);
        assert_eq!(f.index_source_widths.len(), 1);
    }

    /// NUM-USIZE-TO-INT: a `len()` that meets a declared `int` is converted at the call; its class is int.
    #[test]
    fn len_meeting_an_int_is_converted_at_the_call() {
        let f = facts("pub fn f(v: Vec<string>, n: int) -> int {\n    let mut t = v.len() + n\n    t = t + 1\n    t\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("t"), Some(&BaseType::I64));
        assert_eq!(f.index_source_widths.values().collect::<Vec<_>>(), vec![&BaseType::I64]);
    }

    /// An index use in a class of another width needs a conversion at the index: held.
    #[test]
    fn index_use_of_an_int_class_is_held() {
        let f = facts("pub fn f(v: Vec<string>, n: int) -> string {\n    let k = n + 1\n    v[k]\n}\n", "f");
        assert!(f.solver_local_widths.get("k").is_none(), "{:?}", f.solver_local_widths);
        assert!(f.literal_widths.is_empty(), "{:?}", f.literal_widths);
    }

    /// A `usize` counter pushed into a returned `Vec<int>` is converted at the argument.
    #[test]
    fn usize_counter_pushed_into_int_vec_is_converted() {
        let f = facts("pub fn f(lines: Vec<string>) -> Vec<int> {\n    let mut out = Vec::new()\n    let mut i = 0\n    while i < lines.len() {\n        out.push(i + 1)\n        i = i + 1\n    }\n    out\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
        assert_eq!(f.usize_conversions.values().collect::<Vec<_>>(), vec![&BaseType::I64], "{:?}", f.usize_conversions);
    }

    /// Rebinding a name in a later scope (`for j in ..` then `let mut j = ..`) is a separate class;
    /// both bindings fix one width, so the name is published.
    #[test]
    fn rebound_name_with_one_width_is_published() {
        let f = facts("pub fn f(v: Vec<int>) -> int {\n    let mut s = 0\n    for j in 0..v.len() {\n        s = s + v[j]\n    }\n    let mut i = 0\n    while i < v.len() {\n        let mut j = i + 1\n        while j < v.len() {\n            j = j + 1\n        }\n        i = i + 1\n    }\n    s\n}\n", "f");
        assert_eq!(f.solver_local_widths.get("j"), Some(&BaseType::Usize), "{:?}", f.solver_local_widths);
        assert_eq!(f.solver_local_widths.get("i"), Some(&BaseType::Usize));
    }

    /// Bindings of one name with different widths cannot be named by one fact: held.
    #[test]
    fn rebound_name_with_two_widths_is_held() {
        let f = facts("pub fn f(n: i32, v: Vec<int>) -> i32 {\n    let x = n + 1\n    let mut k = 0\n    while k < v.len() {\n        k = k + 1\n    }\n    let x = k\n    n\n}\n", "f");
        assert!(f.solver_local_widths.get("x").is_none(), "{:?}", f.solver_local_widths);
    }
}
