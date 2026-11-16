//! Callee identity (C3): which definition a call names. The resolver's table (`call_targets`, callee
//! expression -> `Def` path) comes first; the import-alias and lexical-scope rules below are the
//! name-based fallbacks it replaces, kept until every pipeline installs the table.

use super::CodeGenerator;
use crate::parser::{Expression, Program};

impl<'ast> CodeGenerator<'ast> {
    /// Crate-relative registry key (`mod::fn`) when `callee_name` names a same-crate definition through
    /// a module alias (`use crate::m as low`), `self::` or `super::`; resolved by `resolve`, the one
    /// path resolver. `None` for external or unqualified callees.
    pub(crate) fn same_crate_callee_key(&self, callee_name: &str) -> Option<String> {
        crate::resolve::same_crate_callee_key(&self.import_fn_alias_map, callee_name)
    }

    /// True when a path call `m::f(..)` goes through this file's `use crate::m` (or `self::` / `super::`): it names
    /// a definition of this crate, as `use crate::m::f` then `f(..)` does, even one Windjammer has no signature
    /// for (a hand-written Rust module). Such a call is never a missing dependency boundary.
    pub(crate) fn path_call_through_same_crate_use(&self, callee_name: &str) -> bool {
        let Some((head, _)) = callee_name.split_once("::") else { return false };
        self.idiom.same_crate_use_paths.get(head).is_some_and(|path| {
            let one = std::collections::HashMap::from([(head.to_string(), path.clone())]);
            crate::resolve::same_crate_callee_key(&one, callee_name).is_some()
        })
    }

    /// Resolve every call of `program` (recomputed per program: keys are node addresses). A module file of
    /// a multi-file build is resolved on its own, so only calls to its own definitions are recorded (paths
    /// relative to the file, as the name-based keys were); `Unresolved`/`Ambiguous` keep the name-based lookup.
    pub(crate) fn ensure_call_targets(&mut self, program: &Program<'_>) {
        self.call_targets.clear();
        let file = self.current_wj_file.clone();
        let input = [crate::resolve::ParsedFile::new(Vec::new(), file, program)];
        self.call_targets = crate::resolve::resolve_program(&input).callee_def_paths();
    }

    /// A bare call `name(..)` inside an impl that names a method of the impl (lowered to `self.name(..)` /
    /// `Self::name(..)`), not an imported free function of the same name.
    pub(crate) fn bare_call_is_sibling_method(&self, name: &str) -> bool {
        self.in_impl_block && self.current_impl_methods.contains(name) && !self.imported_fn_names.contains_key(name)
    }

    /// The registry key of the free function a bare call `name(..)` names through the file's `use` import, when
    /// the bare key holds a same-named method (a trait's `list_items(self, ..)`): name-keyed lookups would
    /// read the method's formals. `None` for any other call (C3 identity by declaration).
    pub(crate) fn imported_free_function_key(&self, name: &str, resolved: Option<&crate::analyzer::FunctionSignature>) -> Option<String> {
        if resolved.is_some_and(|s| s.has_self_receiver) || self.bare_call_is_sibling_method(name) {
            return None;
        }
        let path = self.imported_fn_names.get(name)?;
        let registries = std::iter::once(&self.signature_registry).chain(self.global_signature_registry.as_deref());
        registries.filter(|r| r.key_holds_method(name)).find_map(|r| r.imported_free_function(path)).map(|(key, _)| key)
    }

    /// The resolver's definition path for this callee expression (`a::take`, `Counter::new`).
    pub(crate) fn resolved_callee_path(&self, callee: &Expression<'_>) -> Option<&str> {
        self.call_targets.get(&(callee as *const Expression<'_> as usize)).map(String::as_str)
    }

    /// Callee key for registry / IR signature lookup (`use dep::fn as alias` → `dep::fn`).
    /// Same-crate `crate::mod::fn` registers as `mod::fn` in library multipass.
    pub(in crate::codegen::rust) fn signature_lookup_callee_name<'a>(
        &self,
        callee_name: &'a str,
    ) -> std::borrow::Cow<'a, str> {
        if let Some(rest) = callee_name.strip_prefix("crate::") {
            // `crate::mod::fn` registers as `mod::fn`. Do not collapse `crate::fn`
            // to a bare name — that ORs HashMap::get / log::get homonyms.
            if rest.contains("::") {
                if let Some(q) = self.import_fn_alias_map.get(rest) {
                    return std::borrow::Cow::Owned(q.clone());
                }
                return std::borrow::Cow::Owned(rest.to_string());
            }
        }
        if callee_name.contains("::") {
            let same_crate = self.same_crate_callee_key(callee_name).filter(|k| !callee_name.starts_with("crate::") && k.contains("::"));
            if let Some(key) = same_crate {
                return std::borrow::Cow::Owned(key);
            }
            return std::borrow::Cow::Borrowed(callee_name);
        }
        // Lexical scope (C3): inside `mod m`, bare `f` is `m::f` when `m` defines it.
        let scoped = self.signature_registry.scoped_key(&self.current_module_path, callee_name);
        if let Some(q) = scoped.or_else(|| {
            let g = self.global_signature_registry.as_ref()?;
            g.scoped_key(&self.current_module_path, callee_name)
        }) {
            return std::borrow::Cow::Owned(q);
        }
        if let Some(q) = self.import_fn_alias_map.get(callee_name) {
            return std::borrow::Cow::Owned(q.clone());
        }
        if let Some(q) = self.imported_runtime_qualified_callee(callee_name) {
            return std::borrow::Cow::Owned(q);
        }
        std::borrow::Cow::Borrowed(callee_name)
    }

    /// Shared-borrow slot at a plain call `f(..)` whose signature codegen resolved (`resolved`). The name-keyed
    /// lookups ([`Self::preregistered_free_call_arg_expects_borrow`], [`Self::callee_arg_expects_shared_borrow_at_call`])
    /// read the wrong formals in two cases, where the resolved signature decides instead:
    /// - `f` is a parameter or local (`accept: fn(int) -> bool`): it shadows every function named `f`;
    /// - the bare registry key `f` holds a method of the same name (a trait's `list_items(self, ..)`) while
    ///   the call resolved to a free function.
    pub(in crate::codegen::rust) fn free_call_arg_expects_shared_borrow(
        &self,
        callee: &str,
        resolved: Option<&crate::analyzer::FunctionSignature>,
        arg_index: usize,
    ) -> bool {
        let bare = !callee.contains("::");
        let shadowed = bare
            && (self.current_function_params.iter().any(|p| p.name == callee)
                || self.local_variable_scopes.iter().any(|s| s.contains(callee)));
        if shadowed {
            return resolved.is_some_and(|sig| self.ir_sig_arg_expects_shared_borrow(sig, arg_index));
        }
        match resolved.filter(|_| self.imported_free_function_key(callee, resolved).is_some()) {
            Some(sig) => {
                self.preregistered_free_call_arg_expects_borrow(callee, arg_index)
                    || self.ir_sig_arg_expects_shared_borrow(sig, arg_index)
            }
            None => {
                self.preregistered_free_call_arg_expects_borrow(callee, arg_index)
                    || self.callee_arg_expects_shared_borrow_at_call(callee, arg_index)
            }
        }
    }

    /// The signature of the function this file declares at top level as `name`, when a bare call `name(..)` names
    /// it (lexical scope beats every other module): not inside an inline `mod`, not an impl's sibling method. The
    /// file's registry holds its own definitions under their bare names (`keep_own_definitions`).
    pub(crate) fn own_file_definition(&self, name: &str) -> Option<crate::analyzer::FunctionSignature> {
        if name.contains("::")
            || !self.current_module_path.is_empty()
            || self.bare_call_is_sibling_method(name)
            || !self.idiom.file_scope_fn_names.contains(name)
        {
            return None;
        }
        self.signature_registry.own_definition(name).filter(|s| !s.has_self_receiver).cloned()
    }
}
