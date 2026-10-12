//! Program-level facts gathered before item emission.

use crate::parser::*;
use std::collections::HashSet;

use super::CodeGenerator;

/// Facts and flags consumed while emitting one program.
#[derive(Default)]
pub(crate) struct IdiomScratch {
    /// `(type, trait)` pairs the user implements by hand; the auto-derive must not also derive them (E0119).
    pub(crate) user_trait_impls: HashSet<(String, String)>,
    /// Let-bound closures of the current function that are only called / passed to free functions.
    pub(crate) non_escaping_closure_names: HashSet<String>,
    /// Set by `let` generation for the closure it is about to emit (by-reference captures).
    pub(crate) next_closure_borrows_captures: bool,
    /// Locals bound to `lines()` / `split(..)` / `split_whitespace()` (their `next()` yields `&str`).
    pub(crate) text_iterator_locals: HashSet<String>,
    /// Functions declared only inside inline `mod`s (emitted inside their module, not at top level).
    pub(crate) inline_mod_fn_names: HashSet<String>,
}

impl<'ast> CodeGenerator<'ast> {
    /// Hand-written trait impls (`impl PartialEq for T`) suppress the matching auto-derive.
    pub(super) fn collect_user_trait_impls(&mut self, program: &Program<'_>) {
        self.idiom.inline_mod_fn_names = Self::inline_mod_only_function_names(program);
        self.idiom.user_trait_impls.clear();
        for item in &program.items {
            if let Item::Impl { block, .. } = item {
                if let Some(tr) = &block.trait_name {
                    let short = tr.rsplit("::").next().unwrap_or(tr).to_string();
                    self.idiom.user_trait_impls.insert((block.type_name.clone(), short));
                }
            }
        }
    }

    /// Functions declared inside an inline `mod` are analyzed (signature registry) but emitted inside
    /// their module by `generate_inline_module_item`, never again at the top level.
    pub(super) fn inline_mod_only_function_names(program: &Program<'_>) -> HashSet<String> {
        let top_level: HashSet<&str> = program
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Function { decl, .. } => Some(decl.name.as_str()),
                _ => None,
            })
            .collect();
        program
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Mod { items, .. } => Some(items),
                _ => None,
            })
            .flatten()
            .filter_map(|item| match item {
                Item::Function { decl, .. } => Some(decl.name.as_str()),
                _ => None,
            })
            .filter(|n| !top_level.contains(n))
            .map(|n| n.to_string())
            .collect()
    }

    /// `&k` where `k` is an owned local (`let k = "x"` emits `String`): the user's `&` is the only borrow
    /// there is, so call-site rules must not treat it as "already a reference" (`map.get(&k)`).
    pub(super) fn ref_operand_is_owned_local(&self, operand: &Expression<'ast>) -> bool {
        matches!(operand, Expression::Identifier { name, .. }
            if self.local_var_types.contains_key(name.as_str())
                && !self.emitted_rust_ref_formals.contains(name.as_str())
                && !self.str_ref_optimized_params.contains(name.as_str())
                && !self.inferred_borrowed_params.contains(name.as_str())
                && !self.borrowed_iterator_vars.contains(name)
                && !self.explicit_ref_let_bindings.contains(name.as_str())
                && !self.current_function_params.iter().any(|p| p.name == *name))
    }
}
