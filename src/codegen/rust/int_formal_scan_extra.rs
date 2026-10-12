//! Extra expression forms the call-argument integer-width scan looks through.

use crate::parser::*;

use super::CodeGenerator;

impl<'ast> CodeGenerator<'ast> {
    pub(super) fn scan_extra_exprs_for_call_arg_int_formal(
        &self,
        expr: &Expression<'ast>,
        name: &str,
        peer: &mut Option<Type>,
    ) {
        match expr {
            // `println("${f(xs)}")` is a `format` macro whose arguments are ordinary expressions.
            Expression::MacroInvocation { args, .. } => {
                for arg in args {
                    self.scan_expr_for_call_arg_int_formal(arg, name, peer);
                    if peer.is_some() {
                        return;
                    }
                }
            }
            Expression::Cast { expr: inner, .. } | Expression::TryOp { expr: inner, .. } => {
                self.scan_expr_for_call_arg_int_formal(inner, name, peer);
            }
            Expression::Tuple { elements, .. } | Expression::Array { elements, .. } => {
                for element in elements {
                    self.scan_expr_for_call_arg_int_formal(element, name, peer);
                    if peer.is_some() {
                        return;
                    }
                }
            }
            _ => {}
        }
    }
}
