//! Self-analysis methods for the analyzer.
//! Determines whether impl methods need &self, &mut self, or owned self
//! by analyzing field access patterns, mutations, and return types.
//!
//! Core (`function_modifies_self_fields`); sibling `self_*` modules contain the specialized passes.

use std::collections::HashSet;

use crate::parser::*;

use super::Analyzer;

impl<'ast> Analyzer<'ast> {
    /// Check if a function modifies self fields (for impl methods)
    #[allow(dead_code)]
    pub(super) fn function_modifies_self_fields(&self, func: &FunctionDecl) -> bool {
        self.function_modifies_self_fields_with_registry(func, None)
    }

    /// Check if a function modifies self fields, with optional registry for cross-type resolution
    pub(super) fn function_modifies_self_fields_with_registry(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
    ) -> bool {
        let mut visited = HashSet::new();
        self.function_modifies_self_fields_with_registry_inner(func, registry, &mut visited)
    }

    fn function_modifies_self_fields_with_registry_inner(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        let calls_mut =
            self.function_calls_mutating_self_methods_with_registry(func, registry, visited);
        if calls_mut {
            return true;
        }

        if self.function_mutates_through_self_option_scrutinee(func, registry) {
            return true;
        }

        for stmt in func.body.iter() {
            if self.statement_modifies_self_fields(stmt, registry, visited) {
                return true;
            }
        }
        false
    }

    /// Recursively check if a function modifies self fields.
    ///
    /// `visited` prevents re-analysis: once a function has been analyzed,
    /// its result is cached. On cycle (A→B→A), returns false — the cycle
    /// itself provides no evidence of mutation. Actual mutations are caught
    /// on non-recursive paths.
    fn function_modifies_self_fields_recursive(
        &self,
        func: &FunctionDecl,
        registry: Option<&super::SignatureRegistry>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(func.name.clone()) {
            return false;
        }

        if let Some(return_type) = &func.return_type {
            if self.type_is_mut_ref(return_type) {
                return true;
            }
        }

        func.body
            .iter()
            .any(|stmt| self.statement_modifies_self_fields(stmt, registry, visited))
    }

    /// Check if a type contains a mutable reference (&mut T)
    /// This includes Option<&mut T>, Result<&mut T, E>, Vec<&mut T>, etc.
    fn type_is_mut_ref(&self, ty: &Type) -> bool {
        match ty {
            Type::MutableReference(_) => true,
            Type::Option(inner) | Type::Vec(inner) | Type::Reference(inner) => {
                self.type_is_mut_ref(inner)
            }
            Type::Result(ok, err) => self.type_is_mut_ref(ok) || self.type_is_mut_ref(err),
            Type::Tuple(types) => types.iter().any(|t| self.type_is_mut_ref(t)),
            Type::Parameterized(_, args) => args.iter().any(|t| self.type_is_mut_ref(t)),
            _ => false,
        }
    }
}
