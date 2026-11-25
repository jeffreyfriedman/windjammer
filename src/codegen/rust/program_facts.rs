//! Program-level facts gathered before item emission.

use crate::parser::*;
use std::collections::HashSet;

use super::CodeGenerator;

/// Facts and flags consumed while emitting one program.
#[derive(Default)]
pub(crate) struct IdiomScratch {
    /// Every struct and enum name the crate declares (all modules): a runtime type of the same name is not that type.
    pub(crate) crate_user_type_names: HashSet<String>,
    /// `(type, trait)` pairs the user implements by hand; the auto-derive must not also derive them (E0119).
    pub(crate) user_trait_impls: HashSet<(String, String)>,
    /// Let-bound closures of the current function that are only called / passed to free functions.
    pub(crate) non_escaping_closure_names: HashSet<String>,
    /// Set by `let` generation for the closure it is about to emit (by-reference captures).
    pub(crate) next_closure_borrows_captures: bool,
    /// Inside the value of a `let` or an assignment: a block tail there is the binding's value, never the function's
    /// return value, so the return-width tail casts do not apply.
    pub(crate) binding_value_depth: u32,
    /// Locals bound to `lines()` / `split(..)` / `split_whitespace()` (their `next()` yields `&str`).
    pub(crate) text_iterator_locals: HashSet<String>,
    /// Locals bound to `text.split(..).next()` and the like: `Option<&str>` values.
    pub(crate) text_option_locals: HashSet<String>,
    /// A call argument is being generated: a block value in it is built into a fresh value (moved into a by-value
    /// formal, or borrowed as a temporary for a shared one), so its tails are never references to a binding.
    pub(crate) call_arg_block_value: bool,
    /// Parameters of the comparator closure being generated (`sort_by(|a, b| ..)`): they are `&T`.
    pub(crate) ref_closure_params: HashSet<String>,
    /// Parameters of an iterator adapter's closure, with the reference depth of their type from the adapter's
    /// declaration (`v.filter(|n| ..)`: `&&T`, `v.map(|x| ..)`: `&T`); operators dereference them to the value.
    pub(crate) adapter_closure_param_depth: std::collections::HashMap<String, usize>,
    /// Names a type annotation in this file can refer to: items declared here and `use` imports.
    pub(crate) file_scope_type_names: HashSet<String>,
    /// True when the file has a glob import or nested `mod` (name scope is not tracked precisely).
    pub(crate) file_scope_is_open: bool,
    /// Type names imported from another crate (`use dep::Type`); whether they implement `Serialize` is unknown.
    pub(crate) foreign_imported_types: HashSet<String>,
    /// `use crate::m` / `use crate::m as n` / `use self::..` / `use super::..` of this file, by the name it binds: a path
    /// call through it (`m::f(..)`) names a same-crate definition, as the imported-name form `use crate::m::f` does.
    pub(crate) same_crate_use_paths: std::collections::HashMap<String, String>,
    /// Crates the project's `wj.toml` declares as dependencies: `use dep::m::T` names that crate, never a module
    /// of this one, whatever directories sit next to the build output.
    pub(crate) dependency_crates: HashSet<String>,
    /// Free functions this file declares at top level: a bare call names one of them before a same-named
    /// function of any other module (lexical scope beats imports and other files).
    pub(crate) file_scope_fn_names: HashSet<String>,
    /// Functions of the standard library this file imports by name or through a glob
    /// (`use std::log::*` brings `debug`): they shadow a language intrinsic of the same name (ADR-0014).
    pub(crate) imported_std_fn_names: HashSet<String>,
    /// Loop bindings that are `&str` slices (`for w in text.split_whitespace()`), live during the loop body.
    pub(crate) str_slice_loop_vars: HashSet<String>,
    /// ADR-0014: list literals (by element-slice address) in return position of a function whose
    /// declared return type is a `Vec`: tail expressions, `return` values, `if`/`match` arm tails.
    pub(crate) return_position_list_literals: HashSet<usize>,
    /// ADR-0014: list literals in value position of a `let` declared `Vec<T>` (tails of its `if`/`match`/block value).
    pub(crate) declared_let_list_literals: HashSet<usize>,
    /// Tuple slot types for the next tuple literal only (a match arm next to a sibling arm of tuple type).
    pub(crate) expected_tuple_slots: Option<Vec<Type>>,
    /// Tuple slot types for every tuple literal element of the list literal being generated.
    pub(crate) list_elem_tuple_slots: Option<Vec<Type>>,
}

impl<'ast> CodeGenerator<'ast> {
    /// Hand-written trait impls (`impl PartialEq for T`) suppress the matching auto-derive.
    pub(super) fn collect_user_trait_impls(&mut self, program: &Program<'_>) {
        self.ensure_call_targets(program);
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
        idiom.same_crate_use_paths.clear();
        idiom.dependency_crates = if self.current_wj_file.as_os_str().is_empty() {
            HashSet::new()
        } else {
            crate::module_system::project::find_manifest_dir(&self.current_wj_file)
                .map(|root| crate::module_system::project::dependency_crate_names(&root))
                .unwrap_or_default()
        };
        idiom.file_scope_fn_names.clear();
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
                    let same_crate = path.len() >= 2 && matches!(path[0].as_str(), "crate" | "self" | "super");
                    if let Some(n) = alias.as_ref().or(path.last()).filter(|n| same_crate && n.as_str() != "*") {
                        idiom.same_crate_use_paths.insert(n.clone(), path.join("::"));
                    }
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
                Item::Function { decl, .. } => {
                    idiom.file_scope_fn_names.insert(decl.name.clone());
                }
                _ => {}
            }
        }
    }

    /// `opt.copied().unwrap_or(d)` / `.cloned().unwrap_or(d)`: the receiver already yields owned values.
    pub(super) fn receiver_already_owned(expr_str: &str) -> bool {
        expr_str.contains(".copied().unwrap_or") || expr_str.contains(".cloned().unwrap_or")
    }

    /// `&k` where `k` is an owned local (`let k = "x"` emits `String`): the user's `&` is the only borrow
    /// there is, so call-site rules must not treat it as "already a reference" (`map.get(&k)`).
    /// A parameter declared as a scalar (`k: int`) is emitted by value whatever the analyzer inferred about its uses
    /// (a key only passed to a lookup is "inferred borrowed" but still `k: i64`), so it is an owned local too.
    pub(super) fn ref_operand_is_owned_local(&self, operand: &Expression<'ast>) -> bool {
        let Expression::Identifier { name, .. } = operand else { return false };
        let n = name.as_str();
        if self.emitted_rust_ref_formals.contains(n) || self.str_ref_optimized_params.contains(n) {
            return false;
        }
        if let Some(p) = self.current_function_params.iter().find(|p| p.name == *name) {
            return !matches!(p.type_, Type::Reference(_) | Type::MutableReference(_))
                && crate::type_classification::is_copy_pass_by_value_formal(&p.type_);
        }
        self.local_var_types.contains_key(n)
            && !self.inferred_borrowed_params.contains(n)
            && !self.borrowed_iterator_vars.contains(name)
            && !self.explicit_ref_let_bindings.contains(n)
    }
}
