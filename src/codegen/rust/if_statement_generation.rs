//! If statement generation
//!
//! Handles code generation for if/else conditionals including:
//! - Simple if statements
//! - If-else chains
//! - Implicit return handling in if branches
//! - Owned string coercion for branch tails

use crate::parser::*;

use super::{string_analysis, string_utilities, CodeGenerator};

impl<'ast> CodeGenerator<'ast> {
    /// Generate code for an if statement
    pub(in crate::codegen::rust) fn generate_if_statement(
        &mut self,
        condition: &'ast Expression<'ast>,
        then_body: &[&'ast Statement<'ast>],
        else_body: &Option<Vec<&'ast Statement<'ast>>>,
    ) -> String {
        // WINDJAMMER PHILOSOPHY: Check if any branch explicitly uses .as_str()
        // If so, we should NOT auto-convert string literals in other branches
        let any_branch_has_as_str = string_analysis::block_has_as_str(then_body)
            || else_body
                .as_ref()
                .is_some_and(|b| string_analysis::block_has_as_str(b));

        let old_suppress = self.suppress_string_conversion.get();
        if any_branch_has_as_str {
            self.suppress_string_conversion.set(true);
        }

        let mut output = self.indent();
        output.push_str("if ");
        let cond_str = self.generate_expression(condition);
        // Auto-deref borrowed bool in if-condition: `if r` where r: &bool → `if *r`
        let cond_str = if let Expression::Identifier { name, .. } = condition {
            if self.inferred_borrowed_params.contains(name.as_str())
                || self.borrowed_iterator_vars.contains(name)
            {
                let inferred_type = self.infer_expression_type(condition);
                let is_bool_ref = inferred_type.as_ref().is_some_and(|t| {
                    matches!(t,
                        Type::Reference(inner) | Type::MutableReference(inner)
                        if matches!(&**inner, Type::Bool)
                    )
                });
                // If type is unknown (None) but the variable is borrowed and used
                // in an if-condition, the only valid Rust type is &bool — deref it.
                let type_unknown = inferred_type.is_none();
                if (is_bool_ref || type_unknown) && !cond_str.starts_with('*') {
                    format!("*{}", cond_str)
                } else {
                    cond_str
                }
            } else {
                cond_str
            }
        } else {
            cond_str
        };
        output.push_str(&cond_str);
        output.push_str(" {\n");

        // DOGFOODING FIX: Preserve explicit returns in if-without-else
        // In Rust, `if` without `else` must evaluate to `()`, so any value expression
        // (including implicit returns) is invalid: E0308 "if without else has incompatible types"
        //
        // Safe to optimize returns ONLY in if-else (both branches have values/returns)
        // Must preserve returns in if-without-else (then block evaluates to ())
        let old_in_func_body = self.in_function_body;
        let old_in_void_block = self.in_void_block;
        if else_body.is_none() || !self.current_is_last_statement {
            self.in_function_body = false;
        }
        // if-without-else must evaluate to (); suppress implicit returns
        if else_body.is_none() {
            self.in_void_block = true;
        }

        let old_coerce_lit = self.coerce_string_literals_to_owned;
        let any_branch_suggests_owned_coercion = self
            .branch_tail_suggests_owned_string_coercion(then_body)
            || else_body
                .as_ref()
                .is_some_and(|eb| self.branch_tail_suggests_owned_string_coercion(eb));
        // Coerce string literals in branches when:
        // - The enclosing function returns owned String (even if this `if` is not the last
        //   statement — otherwise `in_function_body` is cleared and inner blocks skip coercion), or
        // - We're in an expression context (`let`/`=` RHS, etc.) and a branch yields String
        //   (e.g. `parts[0].clone()` vs `"0"` while the function itself returns `()`).
        let coerce_string_in_branches = else_body.is_some()
            && (string_utilities::return_type_expects_owned_string(
                &self.current_function_return_type,
            ) || (self.in_expression_context && any_branch_suggests_owned_coercion));
        if coerce_string_in_branches {
            self.coerce_string_literals_to_owned = true;
        }

        self.indent_level += 1;
        output.push_str(&self.generate_block(then_body));
        self.indent_level -= 1;

        output.push_str(&self.indent());
        output.push('}');

        if let Some(else_b) = else_body {
            output.push_str(" else {\n");
            self.indent_level += 1;
            if coerce_string_in_branches {
                self.coerce_string_literals_to_owned = true;
            }
            output.push_str(&self.generate_block(else_b));
            self.indent_level -= 1;
            output.push_str(&self.indent());
            output.push('}');
        }

        self.in_void_block = old_in_void_block;

        self.coerce_string_literals_to_owned = old_coerce_lit;

        self.in_function_body = old_in_func_body;

        self.suppress_string_conversion.set(old_suppress);
        output.push('\n');
        output
    }
}
