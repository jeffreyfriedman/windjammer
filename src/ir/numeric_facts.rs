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
    let index_vars = fc.index_sources.iter().map(|(_, v)| *v).chain(fc.index_uses.iter().copied());
    let mut held = HashSet::new();
    for v in index_vars {
        if view.int_type(v) != Some(BaseType::Usize) {
            held.extend(view.class(v));
        }
    }
    for (name, v) in &fc.var_map.locals {
        if !fc.single_binding_locals.contains(name) {
            held.extend(view.class(*v));
        }
    }
    held
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
        if let Some(base) = view.published(var) {
            ir_fn.solver_local_widths.insert(name.clone(), base);
        }
    }
    for (key, var) in &fc.index_sources {
        if view.published(*var) == Some(BaseType::Usize) {
            ir_fn.index_source_widths.insert(*key, BaseType::Usize);
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

    /// A class whose `len()` meets a declared `int` needs a conversion: nothing is published yet.
    #[test]
    fn class_needing_a_conversion_is_held() {
        let f = facts("pub fn f(v: Vec<string>, n: int) -> int {\n    let mut t = v.len() + n\n    t = t + 1\n    t\n}\n", "f");
        assert!(f.solver_local_widths.get("t").is_none(), "{:?} {:?} {:?}", f.solver_local_widths, f.literal_widths, f.index_source_widths);
        assert!(f.literal_widths.is_empty(), "{:?}", f.literal_widths);
    }

    /// A shadowed local cannot be named by a fact, so its class is held.
    #[test]
    fn shadowed_local_class_is_held() {
        let f = facts("pub fn f(n: i32) -> i32 {\n    let x = 1\n    let x = x + n\n    x\n}\n", "f");
        assert!(f.solver_local_widths.get("x").is_none());
        assert!(f.literal_widths.is_empty(), "{:?}", f.literal_widths);
    }
}
