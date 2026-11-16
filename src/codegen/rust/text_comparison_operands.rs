//! Comparison operands that mix owned text (`String`) with borrowed text (`&str`). Rust has `PartialEq<&str> for
//! String` but no `PartialOrd<&str>`, so an ordering (`it.date < from` with `from` a read-only `string` parameter
//! emitted as `&str`) views the owned side as `&str` (`it.date.as_str() < from`). Decided from the operands' types:
//! a `&str` local (a slice, a `&str` result) on one side; for an ordering also a parameter emitted as `&str`, and an
//! owned `string` local or field on the other. Equality with a parameter or field keeps its native `String == &str`.
//! Also: which side of a borrowed/owned comparison is compared by value (`compares_by_value`).

use crate::parser::{BinaryOp, Expression, Type};

use super::CodeGenerator;

fn is_ordering(op: &BinaryOp) -> bool {
    matches!(op, BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge)
}

impl<'ast> CodeGenerator<'ast> {
    /// `e` is text held by reference in the emitted Rust.
    pub(in crate::codegen::rust) fn borrowed_text_operand(&self, e: &Expression<'ast>, op: &BinaryOp) -> bool {
        if self.is_str_slice_local(e) {
            return true;
        }
        let Expression::Identifier { name, .. } = e else { return false };
        is_ordering(op)
            && self.current_function_params.iter().any(|p| p.name == *name && super::types::is_windjammer_text_type(&p.type_))
            && (self.str_ref_optimized_params.contains(name.as_str()) || self.emitted_rust_ref_formals.contains(name.as_str()))
    }

    /// `e` is owned text (`String`) in the emitted Rust: an owned `string` local, or (for an ordering) a `string` field.
    pub(in crate::codegen::rust) fn owned_text_operand(&self, e: &Expression<'ast>, op: &BinaryOp) -> bool {
        if self.is_owned_text_local(e) {
            return true;
        }
        is_ordering(op)
            && matches!(e, Expression::FieldAccess { .. })
            && self.infer_expression_type(e).is_some_and(|t| !matches!(t, Type::Reference(_) | Type::MutableReference(_)) && super::types::is_windjammer_text_type(&t))
    }

    /// A comparison between a borrowed binding and an owned peer compares values when `e` is a Copy value (`*amount > 0`)
    /// or a value with no `PartialEq` against a reference to itself (`r == *root` with `root: &Vec<u8>`); text keeps its
    /// `String`/`&str` cross impls.
    pub(in crate::codegen::rust) fn compares_by_value(&self, e: &Expression<'ast>) -> bool {
        self.expression_is_copy(e)
            || self.infer_expression_type(e).is_some_and(|ty| {
                let bare = match &ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => inner.as_ref(),
                    other => other,
                };
                self.is_type_copy(bare) || matches!(bare, Type::Vec(_) | Type::Array(..) | Type::Tuple(_))
            })
    }
}
