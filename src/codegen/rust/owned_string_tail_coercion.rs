//! Owned-`String` coercion of a returned or tail expression, and the `.clone()` of a borrowed `self.field`.
//! Used by block implicit returns, `return` statements and `if`/`else` values that must be owned text.

use crate::parser::Type;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    /// Apply owned-String tail coercion to an implicit-return or explicit-return expression.
    ///
    /// When a function returns `String`, this converts string literals to owned form,
    /// rewrites borrowed-param `.clone()` to `.to_string()`, and clones `self.field`
    /// when `self` is borrowed. Used by block implicit returns and `return` statements.
    ///
    /// `respect_suppress`: if true, checks `suppress_string_conversion` and `.as_str()` usage.
    pub(crate) fn apply_owned_string_tail_coercion(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
        respect_suppress: bool,
    ) {
        let returns_string = super::string_utilities::return_type_expects_owned_string(
            &self.current_function_return_type,
        );
        let in_match_needing_string = self.in_match_arm_needing_string;
        // An `if`/`else` value whose sibling branch is an owned `String` (`coerce_string_literals_to_owned`).
        let owned_branch_value = self.coerce_string_literals_to_owned && self.in_expression_context;

        if !returns_string && !in_match_needing_string && !owned_branch_value {
            return;
        }

        if respect_suppress {
            if expr_str.contains(".as_str()") {
                return;
            }
            if self.suppress_string_conversion.get() {
                return;
            }
        }

        if matches!(
            expr,
            crate::parser::Expression::Literal {
                value: crate::parser::Literal::String(_),
                ..
            }
        ) && !super::string_utilities::already_owned_string_expr(expr_str)
        {
            *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
        } else {
            super::string_utilities::rewrite_borrowed_str_clone_to_to_string(
                expr_str,
                expr,
                &self.emitted_rust_ref_formals,
                &self.current_function_params,
            );
            if !super::string_utilities::already_owned_string_expr(expr_str) {
                // `text[a..b]` is a borrowed slice; the owned `string` result copies it.
                if self.is_text_slice_expr(expr) {
                    *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
                    return;
                }
                if let crate::parser::Expression::FieldAccess { field, .. } = expr {
                    if self.module_string_consts.contains(field) {
                        *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
                        return;
                    }
                }
                if let crate::parser::Expression::Identifier { name, .. } = expr {
                    if self.module_string_consts.contains(name) {
                        *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
                        return;
                    }
                    let is_ref_text = self.infer_expression_type(expr).as_ref().is_some_and(|t| {
                        matches!(
                            t,
                            Type::Reference(inner) | Type::MutableReference(inner)
                                if super::types::is_windjammer_text_type(inner)
                        )
                    });
                    let is_borrowed_string_param = self.inferred_borrowed_params.contains(name)
                        || self.str_ref_optimized_params.contains(name)
                        || self.current_function_params.iter().any(|p| {
                            p.name == *name
                                && matches!(
                                    &p.type_,
                                    Type::Reference(inner)
                                        if super::types::is_windjammer_text_type(inner)
                                )
                        });
                    if is_ref_text || is_borrowed_string_param {
                        *expr_str = super::string_utilities::coerce_expr_to_owned_string(expr_str);
                    }
                }
            }
        }

        self.maybe_clone_borrowed_self_field(expr_str, expr);
    }

    /// If `expr` is `self.field` and `self` is borrowed, append `.clone()` for non-Copy fields.
    pub(crate) fn maybe_clone_borrowed_self_field(
        &self,
        expr_str: &mut String,
        expr: &crate::parser::Expression,
    ) {
        if let crate::parser::Expression::FieldAccess { object, .. } = expr {
            if let crate::parser::Expression::Identifier { name: obj_name, .. } = &**object {
                if obj_name == "self"
                    && !expr_str.ends_with(".clone()")
                    && !self.suppress_borrowed_clone
                {
                    let self_is_borrowed = self.current_function_params.iter().any(|p| {
                        p.name == "self"
                            && matches!(
                                p.ownership,
                                crate::parser::OwnershipHint::Ref
                                    | crate::parser::OwnershipHint::Mut
                            )
                    }) || self.current_method_self_emits_borrowed_receiver();
                    if self_is_borrowed {
                        let is_copy = self
                            .infer_expression_type(expr)
                            .as_ref()
                            .is_some_and(|t| self.is_type_copy(t));
                        if !is_copy {
                            *expr_str = format!("{}.clone()", expr_str);
                        }
                    }
                }
            }
        }
    }
}
