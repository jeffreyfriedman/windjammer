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

    /// Resolve every call of a single-file `program` (recomputed per program: keys are node addresses).
    /// Module files of a multi-file build are skipped until the multipass driver installs a project-wide
    /// table. Only calls that name one program function are recorded; `Unresolved`/`Ambiguous` keep
    /// the name-based lookup.
    pub(crate) fn ensure_call_targets(&mut self, program: &Program<'_>) {
        self.call_targets.clear();
        if self.is_module {
            return;
        }
        let file = self.current_wj_file.clone();
        let input = [crate::resolve::ParsedFile::new(Vec::new(), file, program)];
        self.call_targets = crate::resolve::resolve_program(&input).callee_def_paths();
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
}
