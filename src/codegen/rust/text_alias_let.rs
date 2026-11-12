//! `let mut body = raw` where `raw` holds borrowed text (`let raw = s.trim()`, `&str` by `trim`'s declared result):
//! a mutable local that is later given owned text must own its text from the start, so the alias copies the text
//! (`raw.to_string()`), never `raw.clone()` (which is still `&str`, and a later `body = <String>` is rustc E0308).
//! Decided from the IR local type of `raw` (text held by reference).

use crate::parser::Expression;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn own_mut_alias_of_borrowed_text(&self, mutable: bool, value: &Expression<'ast>, value_str: &mut String) {
        let Expression::Identifier { name, .. } = value else { return };
        if !mutable || *value_str != format!("{name}.clone()") {
            return;
        }
        let Some(ir) = self.current_ir_function.as_ref() else { return };
        let ty = ir.use_local_type(value).or_else(|| ir.local_types.get(name));
        // Text by the solver (`trim(self) -> &str` types `raw` as text) and not known to the backend as an owned
        // `String`: `to_string()` copies either way, and stays correct when `raw` holds `&str`.
        let text = ty.is_some_and(|t| t.base == crate::ir::safety_type::BaseType::String);
        let known_owned = matches!(self.local_var_types.get(name), Some(crate::parser::Type::String))
            && !self.idiom.str_slice_loop_vars.contains(name);
        if text && !known_owned {
            *value_str = format!("{name}.to_string()");
        }
    }
}
