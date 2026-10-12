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
        let out = self.generate_let_statement(pattern, mutable, type_, value, location);
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
}
