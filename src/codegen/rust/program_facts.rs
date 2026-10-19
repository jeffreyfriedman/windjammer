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
    /// Names a type annotation in this file can refer to: items declared here and `use` imports.
    pub(crate) file_scope_type_names: HashSet<String>,
    /// True when the file has a glob import or nested `mod` (name scope is not tracked precisely).
    pub(crate) file_scope_is_open: bool,
    /// Type names imported from another crate (`use dep::Type`); whether they implement `Serialize` is unknown.
    pub(crate) foreign_imported_types: HashSet<String>,
    /// Functions of the standard library this file imports by name or through a glob
    /// (`use std::log::*` brings `debug`): they shadow a language intrinsic of the same name (ADR-0014).
    pub(crate) imported_std_fn_names: HashSet<String>,
}

impl<'ast> CodeGenerator<'ast> {
    /// Hand-written trait impls (`impl PartialEq for T`) suppress the matching auto-derive.
    pub(super) fn collect_user_trait_impls(&mut self, program: &Program<'_>) {
        self.idiom.inline_mod_fn_names = Self::inline_mod_only_function_names(program);
        self.collect_file_scope(program);
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

    /// Names this file declares or imports (what an emitted type annotation may spell), and the type names
    /// imported from other crates.
    fn collect_file_scope(&mut self, program: &Program<'_>) {
        let idiom = &mut self.idiom;
        idiom.file_scope_type_names.clear();
        idiom.foreign_imported_types.clear();
        idiom.file_scope_is_open = false;
        idiom.imported_std_fn_names.clear();
        for item in &program.items {
            if let Item::Use { path, alias: None, .. } = item {
                super::intrinsic_lowering::std_fn_names_imported_by(path, &mut idiom.imported_std_fn_names);
            }
        }
        for item in &program.items {
            match item {
                Item::Struct { decl, .. } => {
                    idiom.file_scope_type_names.insert(decl.name.clone());
                }
                Item::Enum { decl, .. } => {
                    idiom.file_scope_type_names.insert(decl.name.clone());
                }
                Item::Trait { decl, .. } => {
                    idiom.file_scope_type_names.insert(decl.name.clone());
                }
                Item::TypeAlias { name, .. } | Item::BoundAlias { name, .. } => {
                    idiom.file_scope_type_names.insert(name.clone());
                }
                Item::Use { path, alias, .. } => {
                    if path.last().is_some_and(|l| l == "*") {
                        idiom.file_scope_is_open = true;
                    } else if let Some(n) = alias.as_ref().or(path.last()) {
                        idiom.file_scope_type_names.insert(n.clone());
                        let foreign = path.len() >= 2
                            && !matches!(path[0].as_str(), "std" | "crate" | "super" | "self")
                            && n.chars().next().is_some_and(char::is_uppercase);
                        if foreign {
                            idiom.foreign_imported_types.insert(n.clone());
                        }
                    }
                }
                Item::Mod { .. } => idiom.file_scope_is_open = true,
                _ => {}
            }
        }
    }

    /// `opt.copied().unwrap_or(d)` / `.cloned().unwrap_or(d)`: the receiver already yields owned values.
    pub(super) fn receiver_already_owned(expr_str: &str) -> bool {
        expr_str.contains(".copied().unwrap_or") || expr_str.contains(".cloned().unwrap_or")
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
