//! Same-crate callee keys: the glue between the generator's import aliases and `resolve`.

use super::CodeGenerator;

impl CodeGenerator<'_> {
    /// Crate-relative registry key (`mod::fn`) when `callee_name` names a same-crate definition through
    /// a module alias (`use crate::m as low`), `self::` or `super::`; resolved by `resolve`, the one
    /// path resolver. `None` for external or unqualified callees.
    pub(crate) fn same_crate_callee_key(&self, callee_name: &str) -> Option<String> {
        crate::resolve::same_crate_callee_key(&self.import_fn_alias_map, callee_name)
    }
}
