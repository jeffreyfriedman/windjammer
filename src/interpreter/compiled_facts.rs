//! The compiler's own decisions, read by the interpreter where it would otherwise guess differently.
//!
//! Parameter passing: most mutations (`p.x = ..`, `v.push(..)`, `n += 1`) make a parameter `&mut` in both
//! the interpreter and the generated Rust. A plain reassignment of the whole parameter (`slot = f(slot)`)
//! does not always: the IR keeps it `&mut`, then code generation takes some such parameters by value (one
//! it also passes on or returns; `wj explain-ownership` shows the stages). The decision is the emitted
//! signature itself, so for such parameters the interpreter reads it: the module's file is generated to
//! Rust in-process (no cargo) and its signatures parsed (`ir::explain::emitted`, the reader
//! `explain-ownership` uses).
//!
//! Computed once per module, and only for programs that reassign a parameter.

use super::engine::{FuncDef, Interpreter};
use crate::ir::explain::emitted::{parse_emitted, EmittedMode};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Per function (`name`, or `Type::name` for a method): the parameters compiled code takes by `&mut`.
pub(crate) type MutRefParams = HashMap<String, HashSet<String>>;

/// Generate `program` to Rust and read each function's `&mut` parameters; `None` when generation fails
/// (the interpreter then keeps its own answer).
fn compute(program: &'static crate::parser::Program<'static>) -> Option<MutRefParams> {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::codegen::generate_rust_source(program).ok()));
    std::panic::set_hook(previous);
    let file = parse_emitted(&rust.ok()??)?;
    Some(
        file.fns
            .into_iter()
            .map(|(name, f)| (name, f.params.into_iter().filter(|p| p.mode == EmittedMode::MutRef).map(|p| p.name).collect()))
            .collect(),
    )
}

impl Interpreter {
    /// Does compiled code take parameter `param` of `f` by `&mut`? `None`: no fact (the module was not
    /// loaded from a program, the analysis failed, or it does not know the function).
    pub(crate) fn compiled_mut_ref(&mut self, f: &FuncDef, param: &str) -> Option<bool> {
        if !self.compiled_facts.contains_key(&f.module) {
            let facts = self.programs.get(&f.module).and_then(|p| compute(p)).map(Rc::new);
            self.compiled_facts.insert(f.module, facts);
        }
        let facts = self.compiled_facts.get(&f.module)?.as_ref()?;
        let key = match &f.self_ty {
            Some(t) => format!("{t}::{}", f.name),
            None => f.name.clone(),
        };
        facts.get(&key).map(|ps| ps.contains(param))
    }
}
