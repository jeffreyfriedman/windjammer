//! Per-`let` codegen context: facts about the bound value that later passes need.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(in crate::codegen::rust) fn generate_let_statement_with_context(
        &mut self,
        pattern: &Pattern<'ast>,
        mutable: bool,
        type_: &Option<Type>,
        value: &'ast Expression<'ast>,
        location: &Option<crate::source_map::Location>,
    ) -> String {
        // ADR-0014: `let xs = [..]` holds a `Vec`, not a Copy array (CLONE-USE-AFTER-MOVE needs this).
        if let (true, Pattern::Identifier(n) | Pattern::MutBinding(n)) = (self.array_literal_as_list, pattern) {
            self.clone_facts.list_literal_locals.insert(n.clone());
        }
        // A let-bound closure that never leaves the function borrows its captures.
        self.idiom.next_closure_borrows_captures = matches!(value, Expression::Closure { .. })
            && matches!(pattern, Pattern::Identifier(n) | Pattern::MutBinding(n)
                if self.idiom.non_escaping_closure_names.contains(n));
        if let (Pattern::Identifier(n) | Pattern::MutBinding(n), Expression::MethodCall { method, .. }) =
            (pattern, value)
        {
            if matches!(method.as_str(), "lines" | "split" | "split_whitespace") {
                self.idiom.text_iterator_locals.insert(n.clone());
            }
        }
        if let (Pattern::Identifier(n) | Pattern::MutBinding(n), true) = (pattern, Self::is_text_slice_option(value)) {
            self.idiom.text_option_locals.insert(n.clone());
        }
        // `let rows: Vec<(string, int)> = [("a", 1)]`: the annotation gives each tuple its slot types
        let slots = type_.as_ref().filter(|_| matches!(value, Expression::Array { .. })).and_then(Self::list_element_tuple_slots);
        let outer_slots = std::mem::replace(&mut self.idiom.list_elem_tuple_slots, slots);
        self.idiom.binding_value_depth += 1;
        let out = self.generate_let_statement(pattern, mutable, type_, value, location);
        self.idiom.binding_value_depth -= 1;
        self.idiom.list_elem_tuple_slots = outer_slots;
        self.idiom.next_closure_borrows_captures = false;
        out
    }

    /// `let Some(v) = o else { .. };`: the generated `let` gets its diverging `else` block.
    pub(in crate::codegen::rust) fn append_let_else(
        &mut self,
        generated: String,
        stmt: &Statement<'ast>,
    ) -> String {
        let Statement::Let { else_block: Some(else_stmts), .. } = stmt else {
            return generated;
        };
        let mut out = generated.trim_end().trim_end_matches(';').to_string();
        out.push_str(" else {\n");
        self.indent_level += 1;
        // The else block must diverge: a trailing `return x` is never a tail value here.
        let was_in_fn_body = std::mem::replace(&mut self.in_function_body, false);
        out.push_str(&self.generate_block(else_stmts));
        self.in_function_body = was_in_fn_body;
        self.indent_level -= 1;
        out.push_str(&self.indent());
        out.push_str("};\n");
        out
    }

    /// The pattern of a `let`. A tuple destructure (`let (c, k) = p`) binds each part with the type of the
    /// value's element; a part is `mut` only when the source says so (`let (mut c, k) = p`, ADR-0002).
    pub(in crate::codegen::rust) fn generate_let_pattern(&mut self, pattern: &Pattern<'ast>, value: &Expression<'ast>) -> String {
        let Pattern::Tuple(parts) = pattern else {
            return self.generate_pattern(pattern);
        };
        if let Some(Type::Tuple(types)) = self.infer_expression_type(value) {
            for (part, ty) in parts.iter().zip(types) {
                if let Pattern::Identifier(n) = part {
                    self.local_var_types.insert(n.clone(), ty);
                }
            }
        }
        // the pattern's own `mut`s, never inferred for a local (ADR-0002)
        self.generate_pattern(&Pattern::Tuple(parts.to_vec()))
    }

    /// A non-Copy field bound out of a borrowed root (`let a = r.who.name` with `r: &Req`) cannot move: it is
    /// cloned. True when the root is behind a reference (whatever the field's type), so callers stop there.
    pub(in crate::codegen::rust) fn clone_field_of_borrowed_root(&self, value: &Expression<'ast>, value_str: &mut String) -> bool {
        if !self.field_access_root_is_behind_reference(value) {
            return false;
        }
        let ty = self.infer_expression_type(value);
        let bare = ty.as_ref().map(|t| match t {
            Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
            other => other,
        });
        if bare.is_some_and(|t| !self.is_type_copy(t)) && !value_str.ends_with(".clone()") && !value_str.starts_with('&') {
            *value_str = format!("{value_str}.clone()");
        }
        true
    }
}

impl<'ast> CodeGenerator<'ast> {
    /// An assignment statement; its value's block tails are the assigned value (see `binding_value_depth`).
    pub(in crate::codegen::rust) fn generate_assignment_with_context(
        &mut self,
        target: &'ast Expression<'ast>,
        value: &'ast Expression<'ast>,
        compound_op: &Option<crate::parser::ast::CompoundOp>,
    ) -> String {
        self.idiom.binding_value_depth += 1;
        let out = self.generate_assignment_statement(target, value, compound_op);
        self.idiom.binding_value_depth -= 1;
        out
    }
}
