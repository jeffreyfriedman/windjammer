//! Signatures of let-bound closures with annotated parameters (`let f = |s: string| ..`).

use crate::analyzer::FunctionSignature;
use crate::parser::{Expression, Type};
use std::collections::HashMap;

/// Closure signatures by local name, valid within one function body.
#[derive(Default)]
pub(crate) struct ClosureSignatures(HashMap<String, FunctionSignature>);

impl ClosureSignatures {
    /// Bind `name` to `sig`, or forget any earlier binding when the new `let` is not such a closure.
    pub(crate) fn record(&mut self, name: &str, sig: Option<FunctionSignature>) {
        match sig {
            Some(sig) => {
                self.0.insert(name.to_string(), sig);
            }
            None => {
                self.0.remove(name);
            }
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&FunctionSignature> {
        self.0.get(name)
    }

    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    /// `let f = |s: string, n: int| ..`: the closure's formals as a signature, so `f("x", 1)` converts its
    /// arguments like a call of a declared function. `None` unless every parameter is annotated.
    pub(crate) fn of_let(value: &Expression<'_>, name: &str) -> Option<crate::analyzer::FunctionSignature> {
        let Expression::Closure { parameters, .. } = value else { return None };
        let types: Option<Vec<Type>> = parameters
            .iter()
            .map(|p| {
                let (_, ty) = p.split_once(": ")?;
                Some(crate::codegen::rust::types::parse_wj_type_name(ty.trim()))
            })
            .collect();
        let types = types?;
        Some(crate::analyzer::FunctionSignature {
            name: name.to_string(),
            param_ownership: vec![crate::analyzer::OwnershipMode::Owned; types.len()],
            formal_param_types: types.clone(),
            param_types: types,
            ..Default::default()
        })
    }
}
